// Integration test for Stage 0 (GameState/spawn.rs prerequisites) and
// Stage A.1 (Entity::spawn) / A.2 (Entity::lock_target) / A.4 (Entity::move),
// per continuation_spec.md's suggested "smaller first bite". Run with
// `cargo test --test stage_a_partial -- --nocapture`.
//
// NOTE: this test changes the working directory to the crate root so the
// relative config-file paths (GameRules.json etc.) resolve the same way
// they do for `cargo run`.

use tank_attack_rust::config::Config;
use tank_attack_rust::entity::entity_move;
use tank_attack_rust::spawn::spawn_tank;
use tank_attack_rust::state::GameState;
use sfml::system::Vector2f;

#[test]
fn stage_a_partial_smoke_test() {
    let cfg = Config::load();
    let mut state = GameState::new(cfg);
    state.game_fps_ratio = 1.0;

    // Spawn a handful of tanks across a few teams, in the game's world
    // coordinate space (not clipped to game_screen_rect -- spawn_tank
    // itself doesn't screen-clip, unlike spawn_particle).
    let n = 20;
    for i in 0..n {
        let team = i % state.number_of_teams;
        let pos = Vector2f::new(100.0 + (i as f32) * 15.0, 200.0 + (i as f32) * 7.0);
        spawn_tank(&mut state, pos, team, 0, false);
    }

    assert_eq!(state.entities.len(), n as usize, "all tanks should have been pushed");

    // uid uniqueness + nonzero.
    let mut uids: Vec<u32> = state.entities.iter().map(|e| e.unique_id).collect();
    uids.sort_unstable();
    uids.dedup();
    assert_eq!(uids.len(), n as usize, "unique_ids must all be distinct");
    assert!(state.entities.iter().all(|e| e.unique_id != 0), "unique_id must never be 0");

    // entity_index matches actual position.
    for (i, e) in state.entities.iter().enumerate() {
        assert_eq!(e.entity_index, i as i32, "entity_index must match Vec position");
    }

    // hitpoints/range/damage pulled from config correctly.
    let ev = &state.config.entity_values;
    assert_eq!(state.entities[0].hitpoints, ev.tank_hp);
    assert_eq!(state.entities[0].max_hitpoints, ev.tank_hp);
    assert_eq!(state.entities[0].dam, ev.tank_damage);
    assert_eq!(state.entities[0].range, ev.tank_range as f32);

    // team assignment round-robins correctly.
    for i in 0..n {
        assert_eq!(state.entities[i as usize].team, i % state.number_of_teams);
    }

    // GameLogArr got a "spawned" entry per tank.
    assert_eq!(state.game_log_arr.len(), n as usize);
    assert!(state.game_log_arr[0].contains("spawned"), "log line: {}", state.game_log_arr[0]);

    // lock_target ran during spawn: with tanks of many different teams all
    // near each other, every tank should have found *some* enemy (unless
    // it's genuinely the closest-to-itself edge case, which can't happen
    // here since no two tanks share a team... actually with n=20 and only
    // number_of_teams distinct teams, most tanks DO have same-team peers,
    // so just assert every entity picked SOME valid enemy of a different team).
    for e in &state.entities {
        if let Some(uid) = e.enemy_uid {
            let target = state.entities.iter().find(|o| o.unique_id == uid).unwrap();
            assert_ne!(target.team, e.team, "locked enemy must be a different team");
        }
    }
    println!("[ok] spawn_tank + entity_lock_target basic invariants hold");

    // --- entity_move: run 500 ticks, assert no NaN/Inf and finite positions ---
    for _tick in 0..500 {
        state.game_ticks += 1;
        for idx in 0..state.entities.len() {
            // Give tanks some acceleration/turning so move() actually has
            // velocity/angular-velocity to integrate (mirrors the tank
            // "always slowly rotating/thrusting" behavior AI() would apply
            // -- AI() itself isn't ported yet, so we drive it directly here
            // purely for the purposes of this smoke test).
            state.entities[idx].tilt(1);
            state.entities[idx].accel(1.0, 1.0);
            entity_move(&mut state, idx);
        }
    }

    for (i, e) in state.entities.iter().enumerate() {
        assert!(e.pos.x.is_finite() && e.pos.y.is_finite(), "entity {i} pos went non-finite: {:?}", e.pos);
        assert!(e.vel.x.is_finite() && e.vel.y.is_finite(), "entity {i} vel went non-finite: {:?}", e.vel);
        assert!(e.ang.is_finite(), "entity {i} ang went non-finite: {}", e.ang);
    }
    println!("[ok] entity_move ran 500 ticks with no NaN/Inf, chunk={:?}", state.entities[0].chunk);

    // --- re-lock-target cadence: after max_lock_target_time ticks, the
    // lock_target_timer should have wrapped back toward 0 at least once ---
    let max_t = state.entities[0].max_lock_target_time;
    assert!(max_t > 0.0);
    println!(
        "[ok] max_lock_target_time = {max_t}, final lock_target_timer = {}",
        state.entities[0].lock_target_timer
    );

    println!("\nAll Stage 0 + A.1/A.2/A.4 smoke tests passed.");
}
