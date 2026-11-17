//! Integration test for Stage E (game_loop.rs), per continuation spec's
//! §E.3 acceptance criteria: runs for many frames with no panic, using
//! the REAL production `tick`/`run` functions and a real (headless,
//! under Xvfb) RenderWindow -- not a parallel test-only reimplementation
//! of the loop.
//!
//! `state.last_fps` sitting "near 120" and the 5-real-minute run from the
//! spec aren't practical in an automated test (wall-clock-bound, and
//! this environment has no real display to hit a true 120fps limit
//! against) -- this test instead runs a large bounded number of frames
//! (`max_frames`) and checks the loop completes cleanly and state stays
//! sane, which is what an automated CI-style check can actually verify.
//!
//! Both scenarios below (a populated generated map, and the bare
//! just-started state) are combined into ONE #[test] function rather
//! than two: `RenderWindow::new` calls `sfml`'s
//! `thread_safety::set_window_thread()`, which -- like `Key::is_pressed`
//! in Stage D.5 -- permanently binds all subsequent windowing calls in
//! the process to whichever thread creates the first window. `cargo
//! test`'s parallel-per-function threading means a second `#[test]` that
//! also creates a `RenderWindow` panics if it lands on a different
//! thread. Same fix as D.5: keep both scenarios on one thread.

use tank_attack_rust::assets::SoundPlayers;
use tank_attack_rust::config::Config;
use tank_attack_rust::game_loop::{run, setup};
use sfml::graphics::RenderWindow;
use sfml::window::{ContextSettings, Style};

#[test]
fn game_loop_runs_many_frames_with_no_panic() {
    // Scenario 1: bare just-started state, matching what main.rs's setup()
    // actually produces before the player interacts.
    let cfg = Config::load();
    let (mut state, assets) = setup(cfg).expect("setup should succeed with the shipped files/ directory");

    assert!(!state.tiles.is_empty(), "setup() should have called init_tiles");
    assert_eq!(state.entities.len(), 0, "no map/entities should exist until the player presses RANDOM or spawns something");
    assert!(!state.buttons.is_empty(), "setup() should have loaded the UI buttons");
    assert!(state.game_texts.iter().any(|t| t.text == "GAME STARTED"));

    let (sw, sh) = (state.config.graphics.screen_width as u32, state.config.graphics.screen_height as u32);
    let mut window = RenderWindow::new((sw, sh), "Tank Attack (test)", Style::CLOSE, &ContextSettings::default());
    window.set_framerate_limit(0); // don't throttle -- this test wants to burn through frames fast

    let mut sound_players = SoundPlayers::new(&assets, &state.config.audio.volumes);
    run(&mut state, &assets, &mut sound_players, &mut window, Some(200));
    assert!(state.game_ticks >= 200);
    assert!(!state.quit_game);

    // Scenario 2: a populated generated map, so every render/AI path in
    // the loop body has real work to do, not just an empty grid. Reuses
    // the same window (and therefore the same claimed thread) rather
    // than creating a second one.
    tank_attack_rust::mapgen::generate_random_map(&mut state, Some(0));
    run(&mut state, &assets, &mut sound_players, &mut window, Some(500));

    assert!(state.game_ticks >= 700, "expected at least 700 total ticks across both scenarios, got {}", state.game_ticks);
    assert!(!state.quit_game, "nothing in this test should have triggered quit_game");

    for e in &state.entities {
        assert!(e.pos.x.is_finite() && e.pos.y.is_finite(), "entity position went non-finite after {} ticks", state.game_ticks);
        assert!(e.hitpoints.is_finite());
    }
    assert!(state.game_fps_ratio.is_finite() && state.game_fps_ratio > 0.0);
    assert!(!state.tiles.is_empty());

    println!(
        "[ok] {} ticks, {} entities, {} lasers, {} missiles, {} particles, {} dead_tanks, last_fps={}",
        state.game_ticks,
        state.entities.len(),
        state.lasers.len(),
        state.missiles.len(),
        state.particles.len(),
        state.dead_tanks.len(),
        state.last_fps
    );
}
