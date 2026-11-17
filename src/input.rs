//! Stage D.5: input handling, ported from `handle_events` (TankAttack.cpp
//! L4258-4536) and `check_keyboard_inputs` (L4537-4595).
//!
//! Split for testability: the actual per-event dispatch logic lives in
//! `handle_single_event`, which takes a plain `sfml::window::Event` value
//! -- constructible directly in a test with no window/event-queue
//! involved (`rust-sfml` 0.21 doesn't expose a `push_event`-style testing
//! hook, so this port creates the same effect by factoring the pollable
//! loop apart from the handling logic, rather than testing through a
//! live window). `handle_events` itself (the `while let Some(event) =
//! window.poll_event()` loop, plus the pre-loop camera-follow/scroll
//! logic) needs a real `RenderWindow` and isn't unit-tested directly --
//! it's a thin wrapper that just calls the tested pieces per event.

use crate::button::{button_fire, ButtonAction};
use crate::entity::EntityType;
use crate::mapgen::generate_random_map;
use crate::spawn::{spawn_fort, spawn_game_text, spawn_tank};
use crate::state::GameState;
use sfml::graphics::{Color, FloatRect, RenderWindow};
use sfml::system::Vector2f;
use sfml::window::mouse::Button as MouseButton;
use sfml::window::{Event, Key};

const SCROLL_SENSITIVITY: f32 = 15.0;

/// C++'s pre-event-loop camera logic in `handle_events` (L4260-4327):
/// minimap-drag highlighting, edge-of-screen mouse scroll (when not
/// camera-following), and camera-follow-tank tracking (including
/// re-picking a random tank if the followed one died, and giving up on
/// follow mode if no tanks remain).
pub fn apply_pre_event_camera_logic(state: &mut GameState) {
    if state.minimap_highlight {
        let mut dx = state.mouse_pos.x - state.minimap_rect.left;
        let mut dy = state.mouse_pos.y - state.minimap_rect.top;
        dx *= state.game_map_width / state.minimap_width;
        dy *= state.game_map_height / state.minimap_height;
        state.camera_pos[0] = dx - state.config.graphics.screen_width / 2.0;
        state.camera_pos[1] = dy - state.config.graphics.screen_height / 2.0;
    }

    if !state.camera_follow_tank {
        let screen_scroll_width = 10.0f32;
        let fps = state.game_fps_ratio;
        let (sw, sh) = (
            state.config.graphics.screen_width,
            state.config.graphics.screen_height,
        );
        if state.mouse_pos.x <= screen_scroll_width {
            state.camera_pos[0] -= fps * SCROLL_SENSITIVITY;
        }
        if state.mouse_pos.x >= sw - screen_scroll_width {
            state.camera_pos[0] += fps * SCROLL_SENSITIVITY;
        }
        if state.mouse_pos.y <= screen_scroll_width {
            state.camera_pos[1] -= fps * SCROLL_SENSITIVITY;
        }
        if state.mouse_pos.y >= sh - screen_scroll_width {
            state.camera_pos[1] += fps * SCROLL_SENSITIVITY;
        }
    }

    if state.camera_follow_tank {
        // Re-validate the followed tank by uid (the C++ additionally
        // checks a raw-pointer `Tank_Followed` against `EntityArr` for
        // "is this pointer still one of our live entities" -- redundant
        // here, since a uid lookup already answers that question).
        let still_valid = match state.tank_followed_uid {
            Some(uid) => state
                .find_entity_index_by_uid(uid)
                .map(|i| state.entities[i].ia)
                .unwrap_or(false),
            None => false,
        };

        if !still_valid {
            let tank_indices: Vec<usize> = state
                .entities
                .iter()
                .enumerate()
                .filter(|(_, e)| e.entity_type == EntityType::Tank)
                .map(|(i, _)| i)
                .collect();
            if !tank_indices.is_empty() {
                let pick = tank_indices[state.rand_range(0..tank_indices.len() as i32) as usize];
                state.tank_followed_uid = Some(state.entities[pick].unique_id);
            } else {
                state.camera_follow_tank = false;
                state.tank_followed_uid = None;
            }
        } else if let Some(uid) = state.tank_followed_uid {
            if let Some(i) = state.find_entity_index_by_uid(uid) {
                let pos = state.entities[i].pos;
                let (sw, sh) = (
                    state.config.graphics.screen_width,
                    state.config.graphics.screen_height,
                );
                state.camera_pos[0] = pos.x - sw / 2.0;
                state.camera_pos[1] = pos.y - sh / 2.0;
            }
        }
    }
}

/// C++'s per-event `switch` body inside `handle_events`'s
/// `while (myWindow.pollEvent(event))` loop (L4331-4530).
pub fn handle_single_event(state: &mut GameState, event: Event) {
    match event {
        Event::Closed => {
            state.quit_game = true;
        }
        Event::MouseButtonReleased { .. } => {
            state.minimap_highlight = false;
        }
        Event::MouseButtonPressed { button, x, y } => {
            handle_mouse_button_pressed(state, button, x as f32, y as f32);
        }
        Event::KeyPressed { code, .. } => {
            handle_key_pressed(state, code);
        }
        Event::MouseMoved { x, y } => {
            state.mouse_pos = Vector2f::new(x as f32, y as f32);
        }
        _ => {}
    }
}

pub fn handle_mouse_button_pressed(state: &mut GameState, button: MouseButton, x: f32, y: f32) {
    match button {
        MouseButton::Left => {
            state.mouse_pos = Vector2f::new(x, y);

            // Buttons first.
            let mut button_clicked = false;
            for i in 0..state.buttons.len() {
                if state.buttons[i].ia && state.buttons[i].rect.contains(state.mouse_pos) {
                    let action = button_fire(state.buttons[i].action, state.number_of_teams);
                    apply_button_action(state, action);
                    button_clicked = true;
                }
            }
            if button_clicked {
                return;
            }

            // Minimap next.
            if state.minimap_rect.contains(state.mouse_pos) && state.draw_minimap {
                let mut dx = state.mouse_pos.x - state.minimap_rect.left;
                let mut dy = state.mouse_pos.y - state.minimap_rect.top;
                dx *= state.game_map_width / state.minimap_width;
                dy *= state.game_map_height / state.minimap_height;
                state.camera_pos[0] = dx - state.config.graphics.screen_width / 2.0;
                state.camera_pos[1] = dy - state.config.graphics.screen_height / 2.0;
                state.minimap_highlight = true;
                return;
            }

            // Forts next.
            let click_world = Vector2f::new(x + state.camera_pos[0], y + state.camera_pos[1]);
            let mut fort_clicked = false;
            for i in 0..state.entities.len() {
                if state.entities[i].entity_type == EntityType::Fort
                    && state.entities[i].collision_rect().contains(click_world)
                {
                    fort_clicked = true;
                    let fort_uid = state.entities[i].unique_id;
                    let fort_col = state.entities[i].col;
                    for r in 0..state.tiles.len() {
                        let cols = state.tiles[r].len();
                        for c in 0..cols {
                            if state.tiles[r][c].fort_uid == Some(fort_uid) {
                                crate::tile::tile_flash(state, r, c, 5.0, fort_col, 64.0, 1);
                            }
                        }
                    }
                }
            }
            if fort_clicked {
                return;
            }

            if state.summon_team == -1 {
                return;
            }

            // Spawn tanks (LShift = level 1, LControl = no-ammo, else normal).
            let temp_rect = FloatRect::new(
                -state.camera_pos[0],
                -state.camera_pos[1],
                state.game_map_width,
                state.game_map_height,
            );
            if temp_rect.contains(Vector2f::new(x, y)) {
                let world_pos = Vector2f::new(x + state.camera_pos[0], y + state.camera_pos[1]);
                if Key::LShift.is_pressed() {
                    spawn_tank(state, world_pos, state.summon_team, 1, false);
                } else if Key::LControl.is_pressed() {
                    spawn_tank(state, world_pos, state.summon_team, 0, true);
                } else {
                    spawn_tank(state, world_pos, state.summon_team, 0, false);
                }
            }
        }
        MouseButton::Right => {
            state.mouse_pos = Vector2f::new(x, y);
            if state.summon_team == -1 {
                return;
            }

            let temp_rect = FloatRect::new(
                -state.camera_pos[0],
                -state.camera_pos[1],
                state.game_map_width,
                state.game_map_height,
            );
            if temp_rect.contains(Vector2f::new(x, y)) {
                let world_pos = Vector2f::new(x + state.camera_pos[0], y + state.camera_pos[1]);
                if Key::LShift.is_pressed() {
                    spawn_fort(state, world_pos, state.summon_team, false, true);
                } else if Key::LControl.is_pressed() {
                    spawn_fort(state, world_pos, state.summon_team, true, false);
                } else {
                    spawn_fort(state, world_pos, state.summon_team, false, false);
                }
            }
        }
        _ => {}
    }
}

/// C++: `Button::fire()`'s effects, applied to `GameState` by the caller
/// (per the design noted in `button.rs`).
fn apply_button_action(state: &mut GameState, action: ButtonAction) {
    let (sw, sh) = (
        state.config.graphics.screen_width,
        state.config.graphics.screen_height,
    );
    match action {
        ButtonAction::QuitGame => state.quit_game = true,
        ButtonAction::RegenerateMap => generate_random_map(state, None),
        ButtonAction::DeleteEntities => {
            state.game_texts.clear();
            state.entities.clear();
            state.dead_tanks.clear();
            state.game_log_arr.clear();
            spawn_game_text(
                state,
                "ENTITIES DELETED".to_string(),
                Vector2f::new(sw / 2.0, sh / 2.0),
                Color::rgb(255, 255, 255),
                30,
                5.0,
                2.0,
                true,
            );
        }
        ButtonAction::SetSummonTeamRandom => state.summon_team = -1,
        ButtonAction::ToggleGangUp => {
            state.gang_up = !state.gang_up;

            // DEVIATION: C++ reads `FortPopArr[FortPopArr.size()-1]` (the
            // latest tick's per-team fort population, from `update_stats`
            // in stats.rs, not yet ported) to find the team with the most
            // forts as the "gang-up victim". Until stats.rs lands,
            // `fort_pop_arr` is empty; fall back to `victim_team = -1`
            // (no victim) rather than indexing an empty Vec.
            let mut highest = 0;
            let mut index = -1i32;
            if let Some(latest) = state.fort_pop_arr.last() {
                for i in 0..state.number_of_teams as usize {
                    let pop = latest.get(i).copied().unwrap_or(0);
                    if pop > highest {
                        highest = pop;
                        index = i as i32;
                    }
                }
            }
            state.victim_team = index;

            let text = if state.gang_up {
                "Game Up: ON"
            } else {
                "Game Up: OFF"
            };
            spawn_game_text(
                state,
                text.to_string(),
                Vector2f::new(sw / 2.0, sh / 2.0),
                Color::rgb(255, 255, 255),
                30,
                1.0,
                2.0,
                false,
            );
        }
        ButtonAction::SetSummonTeam(team) => state.summon_team = team,
        ButtonAction::None => {}
    }
}

pub fn handle_key_pressed(state: &mut GameState, code: Key) {
    let (sw, sh) = (
        state.config.graphics.screen_width,
        state.config.graphics.screen_height,
    );
    match code {
        Key::Escape => {
            if state.event_logging {
                println!("  *ESC key pressed*");
            }
            // C++: `// quitGame = true;` -- commented out. Deliberately
            // does nothing, matching the original.
        }
        Key::Delete => {
            state.game_texts.clear();
            state.entities.clear();
            state.dead_tanks.clear();
            spawn_game_text(
                state,
                "ENTITIES DELETED".to_string(),
                Vector2f::new(sw / 2.0, sh / 2.0),
                Color::rgb(255, 255, 255),
                30,
                5.0,
                2.0,
                true,
            );
        }
        Key::Space => {} // C++: empty event, deliberately does nothing.
        Key::R => {
            state.camera_follow_tank = !state.camera_follow_tank;
            state.tank_followed_uid = None;
        }
        Key::O => state.draw_particles = !state.draw_particles,
        Key::T => state.draw_tiles = !state.draw_tiles,
        Key::B => state.draw_softborders = !state.draw_softborders,
        Key::D => state.debug_features = !state.debug_features,
        Key::F3 => state.draw_stats = !state.draw_stats,
        Key::F2 => {
            // TODO(Stage E): take_screenshot(window) -- needs the live
            // RenderWindow, which this function intentionally doesn't
            // take (see module doc comment); Stage E's main loop should
            // call it directly on this key rather than plumbing it
            // through here.
            spawn_game_text(
                state,
                "SCREENSHOT TAKEN".to_string(),
                Vector2f::new(sw / 2.0, sh / 2.0),
                Color::rgb(255, 255, 255),
                20,
                5.0,
                2.0,
                true,
            );
        }
        Key::A => state.autoplay = !state.autoplay,
        Key::U => state.draw_ui = !state.draw_ui,
        Key::M => state.draw_minimap = !state.draw_minimap,
        Key::P => state.paused = !state.paused,
        Key::W => state.wind_physics = !state.wind_physics,
        Key::J => {
            // C++ ignores the Result (`if (outputFile.is_open())` just
            // skips a file it can't write); matched here via `.ok()`.
            let _ = crate::stats::save_json_data(state);
        }
        Key::S => {
            let _ = crate::stats::save_game_state(state, "SaveGame.json");
        }
        _ => {}
    }
}

/// C++: void check_keyboard_inputs() (TankAttack.cpp L4537-4595).
/// Needs a live window for real keyboard state (`Key::is_pressed`
/// queries the OS), so isn't itself unit-tested; the camera-bounds
/// clamp at the end is factored out into `clamp_camera_to_bounds`,
/// which is.
pub fn check_keyboard_inputs(state: &mut GameState) {
    if state.event_logging {
        println!("/KEYBOARDINPUTS()/");
    }

    let fps = state.game_fps_ratio;
    if Key::Left.is_pressed() {
        state.camera_pos[0] -= fps * SCROLL_SENSITIVITY;
        state.camera_follow_tank = false;
    }
    if Key::Right.is_pressed() {
        state.camera_pos[0] += fps * SCROLL_SENSITIVITY;
        state.camera_follow_tank = false;
    }
    if Key::Up.is_pressed() {
        state.camera_pos[1] -= fps * SCROLL_SENSITIVITY;
        state.camera_follow_tank = false;
    }
    if Key::Down.is_pressed() {
        state.camera_pos[1] += fps * SCROLL_SENSITIVITY;
        state.camera_follow_tank = false;
    }

    let number_keys = [
        (Key::Num1, 0),
        (Key::Num2, 1),
        (Key::Num3, 2),
        (Key::Num4, 3),
        (Key::Num5, 4),
        (Key::Num6, 5),
        (Key::Num7, 6),
        (Key::Num8, 7),
        (Key::Num9, 8),
        (Key::Num0, 9),
    ];
    for (key, team) in number_keys {
        if key.is_pressed() {
            state.summon_team = team;
        }
    }
    let shifted_number_keys = [
        (Key::Num1, 10),
        (Key::Num2, 11),
        (Key::Num3, 12),
        (Key::Num4, 13),
    ];
    for (key, team) in shifted_number_keys {
        if key.is_pressed() && Key::LShift.is_pressed() {
            state.summon_team = team;
        }
    }

    if Key::H.is_pressed() && state.game_volume > 0.0 {
        state.game_volume -= 1.0;
    }
    if Key::Y.is_pressed() && state.game_volume < 100.0 {
        state.game_volume += 1.0;
    }
    if Key::Y.is_pressed() || Key::H.is_pressed() {
        // TODO(Stage E audio): apply state.game_volume to every live
        // sf::Sound/sf::Music instance's volume, same as the C++'s block
        // of `*.setVolume(*_Volume * game_volume / 100)` calls. Nothing
        // to do here yet since Stage D.1's SoundAssets are buffers, not
        // playing Sound instances (those get created in Stage E).
    }

    clamp_camera_to_bounds(state);

    if state.event_logging {
        println!("  KEYBOARDINPUTS() executed");
    }
}

/// C++'s camera-bounds clamp at the end of `check_keyboard_inputs`
/// (L4589-4594).
pub fn clamp_camera_to_bounds(state: &mut GameState) {
    let (sw, sh) = (
        state.config.graphics.screen_width,
        state.config.graphics.screen_height,
    );

    if state.camera_pos[0] <= -sw / 3.0 {
        state.camera_pos[0] = -sw / 3.0;
    }
    if state.camera_pos[0] >= state.game_map_width - 2.0 * sw / 3.0 {
        state.camera_pos[0] = state.game_map_width - 2.0 * sw / 3.0;
    }
    if state.camera_pos[1] <= -sh / 3.0 {
        state.camera_pos[1] = -sh / 3.0;
    }
    if state.camera_pos[1] >= state.game_map_height - 2.0 * sh / 3.0 {
        state.camera_pos[1] = state.game_map_height - 2.0 * sh / 3.0;
    }
}

/// C++: void handle_events(sf::RenderWindow& myWindow) (TankAttack.cpp
/// L4258-4536). The live-window wrapper: pre-loop camera logic, then
/// drains the real event queue through `handle_single_event`.
pub fn handle_events(state: &mut GameState, window: &mut RenderWindow) {
    if state.event_logging {
        println!("/HANDLEEVENTS()/");
    }

    apply_pre_event_camera_logic(state);

    while let Some(event) = window.poll_event() {
        handle_single_event(state, event);
    }

    if state.event_logging {
        println!("  HANDLEEVENTS() executed.");
    }
}

/// C++: void draw_borders() (TankAttack.cpp L4602-4609). Small enough to
/// live here rather than render.rs; only meaningful once the camera can
/// actually move (Stage D.5's territory).
pub fn draw_borders<T: sfml::graphics::RenderTarget>(state: &GameState, target: &mut T) {
    if state.camera_pos[0] <= 0.0 {
        let rect = FloatRect::new(
            0.0,
            0.0,
            -state.camera_pos[0],
            state.config.graphics.screen_height,
        );
        crate::render::draw_filled_rect(target, rect, Color::BLACK);
    }
}
