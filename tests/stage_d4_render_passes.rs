//! Stage D.4 smoke test, per the tapered verification plan: build a real
//! game state (generated map + spawned tanks/forts/projectiles/effects),
//! run one full frame through every render_* pass into an off-screen
//! RenderTexture, save it to disk, and assert the result isn't just the
//! clear color -- i.e. "something drew", the cheap catch-all for "nothing
//! drew at all". Side-by-side comparison against the real C++ build is
//! for the user to do, not something this test can automate.

use tank_attack_rust::assets::Assets;
use tank_attack_rust::config::Config;
use tank_attack_rust::mapgen::generate_random_map;
use tank_attack_rust::render::*;
use tank_attack_rust::spawn::{spawn_dead_tank, spawn_floating_text, spawn_game_text, spawn_laser, spawn_missile, spawn_particle, spawn_tank};
use tank_attack_rust::state::GameState;
use rand::SeedableRng;
use sfml::graphics::{Color, RenderTarget, RenderTexture, Texture};
use sfml::system::Vector2f;

#[test]
fn full_frame_render_pass_draws_something_and_does_not_panic() {
    let config = Config::load();
    let mut state = GameState::new(config);
    state.game_fps_ratio = 1.0;
    state.rng = rand::rngs::SmallRng::seed_from_u64(7);
    state.draw_softborders = true; // exercise that code path too
    state.debug_features = true; // exercise debug overlays too

    let color_arr = state.color_arr.clone();
    let assets = Assets::load(&state.config, &color_arr).expect("assets should load");

    generate_random_map(&mut state, Some(0));

    // Add a bit of everything so every render_* pass has something to do.
    let center = Vector2f::new(state.game_map_width / 2.0, state.game_map_height / 2.0);
    spawn_tank(&mut state, center, 0, 0, false);
    spawn_tank(&mut state, Vector2f::new(center.x + 30.0, center.y), 1, 1, false);
    let (uid0, uid1) = (state.entities[0].unique_id, state.entities[1].unique_id);
    spawn_laser(&mut state, center.x, center.y, 5.0, 0.0, 10.0, 0, uid0, vec![4.0, 0.66]);
    spawn_missile(&mut state, center, 3.0, 90.0, 10.0, 3, 0, uid0, uid1, 6.0, 10, 40.0, 7.5, 1.0);
    spawn_particle(&mut state, center, 4.0, 2.0, 1, 1.0, Color::WHITE);
    spawn_particle(&mut state, center, 4.0, 2.0, 2, 1.0, Color::WHITE);
    spawn_dead_tank(&mut state, center, 45.0, 0);
    spawn_floating_text(&mut state, "TEST".to_string(), center, Color::WHITE, Color::BLACK, 20, 5.0, tank_attack_rust::floating_text::FontChoice::Font1, 0.5);
    spawn_game_text(&mut state, "GAME TEXT".to_string(), center, Color::WHITE, 20, 5.0, 1.0, true);

    // Center the camera on the action so entities land inside the
    // render texture rather than off in some corner of a large map.
    let (w, h) = (state.config.graphics.screen_width as u32, state.config.graphics.screen_height as u32);
    state.camera_pos = [state.game_map_width / 2.0 - w as f32 / 2.0, state.game_map_height / 2.0 - h as f32 / 2.0];
    state.game_screen_rect = sfml::graphics::FloatRect::new(state.camera_pos[0], state.camera_pos[1], w as f32, h as f32);

    let mut rt = RenderTexture::new(w, h).expect("RenderTexture::new should succeed under Xvfb");
    rt.clear(Color::BLACK);

    // One full frame, in the same rough order the real main loop would
    // call these (world first, then UI/minimap on top).
    draw_tiles(&mut state, &assets, &mut rt);
    render_entities(&mut state, &assets, &mut rt);
    render_lasers(&mut state, &mut rt);
    render_missiles(&mut state, &mut rt);
    render_particles(&mut state, &mut rt);
    render_dead_tanks(&mut state, &assets, &mut rt);
    render_floating_texts(&mut state, &assets, &mut rt);
    render_minimap(&mut state, &mut rt);
    draw_ui(&mut state, &assets, &mut rt, true);

    rt.display();
    let tex: &Texture = rt.texture();
    let img = tex.copy_to_image().expect("copy_to_image should succeed");

    // Save to disk for optional human/side-by-side inspection.
    let out_path = "/tmp/stage_d4_smoke_test_render.png";
    let _ = img.save_to_file(out_path);

    let mut any_nonblack = false;
    'outer: for x in (0..w).step_by(2) {
        for y in (0..h).step_by(2) {
            let p = unsafe { img.pixel_at(x, y) };
            if p.r != 0 || p.g != 0 || p.b != 0 {
                any_nonblack = true;
                break 'outer;
            }
        }
    }
    assert!(any_nonblack, "full-frame render pass produced a completely blank (all-black) image -- nothing drew at all");

    // Run a second frame to make sure per-frame state (culling, retained
    // Vecs, etc.) doesn't panic on the next tick either.
    draw_tiles(&mut state, &assets, &mut rt);
    render_entities(&mut state, &assets, &mut rt);
    render_lasers(&mut state, &mut rt);
    render_missiles(&mut state, &mut rt);
    render_particles(&mut state, &mut rt);
    render_dead_tanks(&mut state, &assets, &mut rt);
    render_floating_texts(&mut state, &assets, &mut rt);
    render_minimap(&mut state, &mut rt);
    draw_ui(&mut state, &assets, &mut rt, true);
}
