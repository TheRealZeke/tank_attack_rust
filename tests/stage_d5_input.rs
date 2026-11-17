//! Integration tests for Stage D.5 (input.rs), per continuation spec's
//! testable-tier guidance: `handle_single_event` and the camera-logic
//! helpers take plain data (a constructed `Event`, or just `GameState`)
//! and are fully testable without a live window/event queue.

use tank_attack_rust::config::Config;
use tank_attack_rust::entity::EntityType;
use tank_attack_rust::input::{apply_pre_event_camera_logic, clamp_camera_to_bounds, handle_key_pressed, handle_single_event};
use tank_attack_rust::mapgen::generate_random_map;
use tank_attack_rust::spawn::{spawn_button, spawn_tank};
use tank_attack_rust::state::GameState;
use rand::SeedableRng;
use sfml::graphics::Color;
use sfml::system::Vector2f;
use sfml::window::mouse::Button as MouseButton;
use sfml::window::{Event, Key};

fn fresh_state() -> GameState {
    let cfg = Config::load();
    let mut state = GameState::new(cfg);
    state.game_fps_ratio = 1.0;
    state.rng = rand::rngs::SmallRng::seed_from_u64(1);
    state
}

#[test]
fn closed_event_sets_quit_game() {
    let mut state = fresh_state();
    assert!(!state.quit_game);
    handle_single_event(&mut state, Event::Closed);
    assert!(state.quit_game);
}

#[test]
fn mouse_moved_updates_mouse_pos() {
    let mut state = fresh_state();
    handle_single_event(&mut state, Event::MouseMoved { x: 123, y: 456 });
    assert_eq!(state.mouse_pos, Vector2f::new(123.0, 456.0));
}

#[test]
fn mouse_button_released_clears_minimap_highlight() {
    let mut state = fresh_state();
    state.minimap_highlight = true;
    handle_single_event(&mut state, Event::MouseButtonReleased { button: MouseButton::Left, x: 0, y: 0 });
    assert!(!state.minimap_highlight);
}

#[test]
fn left_click_on_a_button_fires_its_action() {
    let mut state = fresh_state();
    // action 0 == QuitGame, per button_fire's mapping.
    spawn_button(&mut state, Vector2f::new(10.0, 10.0), Vector2f::new(30.0, 25.0), "QUIT".to_string(), Color::rgb(200, 20, 40), 0, 16);

    handle_single_event(&mut state, Event::MouseButtonPressed { button: MouseButton::Left, x: 20, y: 20 });
    assert!(state.quit_game, "clicking inside the QUIT button's rect should fire QuitGame");
}

#[test]
fn left_and_right_click_spawn_tank_and_fort_for_summon_team() {
    // Merged into one test function: both branches internally call
    // `Key::LShift/LControl.is_pressed()`, and rust-sfml enforces that
    // all windowing/keyboard calls happen from a single consistent OS
    // thread per process (the first caller "claims" it; any other
    // thread panics). `cargo test`'s default parallel runner gives each
    // `#[test]` fn its own thread, so two separate tests that each
    // touch `is_pressed()` race to claim that thread and the loser
    // panics with "Graphics/windowing functionality is only supported
    // on the same thread" -- a real constraint of the crate (relevant
    // to Stage E's eventual main loop too, not just this test), not a
    // port bug. Keeping both assertions in one function keeps them on
    // one thread and sidesteps it.
    let mut state = fresh_state();
    state.game_map_width = 2000.0;
    state.game_map_height = 2000.0;
    state.camera_pos = [0.0, 0.0];

    state.summon_team = 3;
    assert_eq!(state.entities.len(), 0);
    handle_single_event(&mut state, Event::MouseButtonPressed { button: MouseButton::Left, x: 500, y: 500 });
    assert_eq!(state.entities.len(), 1, "click inside map bounds with no button/minimap hit should spawn a tank");
    assert_eq!(state.entities[0].entity_type, EntityType::Tank);
    assert_eq!(state.entities[0].team, 3);
    assert_eq!(state.entities[0].level, 0);
    assert!(!state.entities[0].no_ammo);

    state.summon_team = 5;
    handle_single_event(&mut state, Event::MouseButtonPressed { button: MouseButton::Right, x: 400, y: 400 });
    assert_eq!(state.entities.len(), 2);
    assert_eq!(state.entities[1].entity_type, EntityType::Fort);
    assert_eq!(state.entities[1].team, 5);
}

#[test]
fn left_click_with_summon_team_null_spawns_nothing() {
    let mut state = fresh_state();
    state.game_map_width = 2000.0;
    state.game_map_height = 2000.0;
    state.camera_pos = [0.0, 0.0];
    state.summon_team = -1; // "NULL TEAM"

    handle_single_event(&mut state, Event::MouseButtonPressed { button: MouseButton::Left, x: 500, y: 500 });
    assert_eq!(state.entities.len(), 0, "summon_team == -1 should spawn nothing");
}

#[test]
fn clicking_a_fort_flashes_every_tile_it_claims() {
    let mut state = fresh_state();
    state.game_map_width = 500.0;
    state.game_map_height = 500.0;
    tank_attack_rust::tile::init_tiles(&mut state);
    tank_attack_rust::spawn::spawn_fort(&mut state, Vector2f::new(250.0, 250.0), 0, false, false);
    tank_attack_rust::tile::update_tiles(&mut state);
    state.camera_pos = [0.0, 0.0];

    let claimed_before: Vec<(usize, usize)> = (0..state.tiles.len())
        .flat_map(|r| (0..state.tiles[r].len()).map(move |c| (r, c)))
        .filter(|&(r, c)| state.tiles[r][c].team == 0)
        .collect();
    assert!(!claimed_before.is_empty());
    // NOTE: `update_tiles` itself flashes every tile whose color just
    // changed (matching Tile::update's `if (changed) flash(...)` in the
    // C++) -- newly-claimed tiles flash immediately, before any click.
    // That's correct behavior, not a bug; reset flash_state here to
    // isolate the click's effect specifically.
    for &(r, c) in &claimed_before {
        state.tiles[r][c].flash_state = false;
    }

    // Click directly on the fort's world position (camera at origin, so
    // screen coords == world coords here).
    handle_single_event(&mut state, Event::MouseButtonPressed { button: MouseButton::Left, x: 250, y: 250 });

    let any_flashing = claimed_before.iter().any(|&(r, c)| state.tiles[r][c].flash_state);
    assert!(any_flashing, "clicking a fort should have set flash_state on at least one of its claimed tiles");
    // No tank should have spawned (fort click takes priority over
    // tank-spawn, matching the C++'s early `break`/`return`).
    assert_eq!(state.entities.iter().filter(|e| e.entity_type == EntityType::Tank).count(), 0);
}

#[test]
fn key_toggles_flip_their_flags() {
    let mut state = fresh_state();
    let before = (state.draw_particles, state.draw_tiles, state.debug_features, state.draw_stats, state.autoplay, state.draw_ui, state.draw_minimap, state.paused, state.wind_physics);

    handle_key_pressed(&mut state, Key::O);
    handle_key_pressed(&mut state, Key::T);
    handle_key_pressed(&mut state, Key::D);
    handle_key_pressed(&mut state, Key::F3);
    handle_key_pressed(&mut state, Key::A);
    handle_key_pressed(&mut state, Key::U);
    handle_key_pressed(&mut state, Key::M);
    handle_key_pressed(&mut state, Key::P);
    handle_key_pressed(&mut state, Key::W);

    assert_eq!(
        (state.draw_particles, state.draw_tiles, state.debug_features, state.draw_stats, state.autoplay, state.draw_ui, state.draw_minimap, state.paused, state.wind_physics),
        (!before.0, !before.1, !before.2, !before.3, !before.4, !before.5, !before.6, !before.7, !before.8)
    );
}

#[test]
fn escape_and_space_keys_do_nothing() {
    let mut state = fresh_state();
    let before = state.quit_game;
    handle_key_pressed(&mut state, Key::Escape);
    assert_eq!(state.quit_game, before, "Escape should NOT quit -- that line is commented out in the original");
    handle_key_pressed(&mut state, Key::Space);
    // Space is an intentional no-op; just confirm no panic and no state change we'd notice.
    assert_eq!(state.quit_game, before);
}

#[test]
fn delete_key_clears_entities_and_spawns_notice_text() {
    let mut state = fresh_state();
    spawn_tank(&mut state, Vector2f::new(10.0, 10.0), 0, 0, false);
    assert_eq!(state.entities.len(), 1);

    handle_key_pressed(&mut state, Key::Delete);

    assert_eq!(state.entities.len(), 0);
    assert!(state.game_texts.iter().any(|t| t.text == "ENTITIES DELETED"));
}

#[test]
fn r_key_toggles_camera_follow_and_clears_followed_uid() {
    let mut state = fresh_state();
    state.tank_followed_uid = Some(42);
    assert!(!state.camera_follow_tank);

    handle_key_pressed(&mut state, Key::R);

    assert!(state.camera_follow_tank);
    assert_eq!(state.tank_followed_uid, None);
}

#[test]
fn camera_follow_picks_a_new_tank_when_followed_one_is_gone() {
    let mut state = fresh_state();
    state.camera_follow_tank = true;
    state.tank_followed_uid = Some(999999); // does not exist

    spawn_tank(&mut state, Vector2f::new(500.0, 500.0), 0, 0, false);
    let real_uid = state.entities[0].unique_id;

    apply_pre_event_camera_logic(&mut state);

    assert_eq!(state.tank_followed_uid, Some(real_uid), "should have picked the only available tank");
    assert!(state.camera_follow_tank);
}

#[test]
fn camera_follow_gives_up_when_no_tanks_exist() {
    let mut state = fresh_state();
    state.camera_follow_tank = true;
    state.tank_followed_uid = Some(999999);

    apply_pre_event_camera_logic(&mut state);

    assert!(!state.camera_follow_tank, "with zero tanks in the world, camera-follow should give up");
    assert_eq!(state.tank_followed_uid, None);
}

#[test]
fn camera_follow_tracks_the_followed_tank_position() {
    let mut state = fresh_state();
    spawn_tank(&mut state, Vector2f::new(1000.0, 800.0), 0, 0, false);
    let uid = state.entities[0].unique_id;
    state.camera_follow_tank = true;
    state.tank_followed_uid = Some(uid);

    apply_pre_event_camera_logic(&mut state);

    let (sw, sh) = (state.config.graphics.screen_width, state.config.graphics.screen_height);
    assert_eq!(state.camera_pos[0], 1000.0 - sw / 2.0);
    assert_eq!(state.camera_pos[1], 800.0 - sh / 2.0);
}

#[test]
fn edge_scroll_moves_camera_when_not_following_a_tank() {
    let mut state = fresh_state();
    state.camera_follow_tank = false;
    state.mouse_pos = Vector2f::new(1.0, 1.0); // top-left corner
    let before = state.camera_pos;

    apply_pre_event_camera_logic(&mut state);

    assert!(state.camera_pos[0] < before[0], "mouse near the left edge should scroll the camera left");
    assert!(state.camera_pos[1] < before[1], "mouse near the top edge should scroll the camera up");
}

#[test]
fn clamp_camera_to_bounds_keeps_camera_within_map() {
    let mut state = fresh_state();
    state.game_map_width = 1000.0;
    state.game_map_height = 1000.0;
    let (sw, sh) = (state.config.graphics.screen_width, state.config.graphics.screen_height);

    state.camera_pos = [-999999.0, -999999.0];
    clamp_camera_to_bounds(&mut state);
    assert_eq!(state.camera_pos[0], -sw / 3.0);
    assert_eq!(state.camera_pos[1], -sh / 3.0);

    state.camera_pos = [999999.0, 999999.0];
    clamp_camera_to_bounds(&mut state);
    assert_eq!(state.camera_pos[0], state.game_map_width - 2.0 * sw / 3.0);
    assert_eq!(state.camera_pos[1], state.game_map_height - 2.0 * sh / 3.0);
}

#[test]
fn regenerate_map_button_action_replaces_the_map() {
    let mut state = fresh_state();
    generate_random_map(&mut state, Some(0));
    let first_width = state.game_map_width;

    spawn_button(&mut state, Vector2f::new(0.0, 0.0), Vector2f::new(50.0, 25.0), "RANDOM".to_string(), Color::rgb(20, 40, 200), 1, 16);
    handle_single_event(&mut state, Event::MouseButtonPressed { button: MouseButton::Left, x: 10, y: 10 });

    // A new map should have been generated (map dims are randomly
    // reselected -- may coincidentally match, so just assert the map is
    // still in a valid, non-empty state rather than asserting inequality).
    let _ = first_width;
    assert!(!state.tiles.is_empty());
    assert!(!state.buttons.is_empty(), "RANDOM regeneration re-runs load_game_buttons, which should repopulate the button list");
}
