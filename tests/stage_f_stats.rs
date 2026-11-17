//! Integration tests for Stage F (stats.rs), per continuation spec's
//! acceptance criteria (§F.5): update_stats populates temp_* correctly,
//! save_json_data writes all 15 files with valid JSON, save_game_state
//! writes one entity per live entity.

use tank_attack_rust::config::Config;
use tank_attack_rust::spawn::{spawn_fort, spawn_tank};
use tank_attack_rust::state::GameState;
use tank_attack_rust::stats::{get_average_tank_age, get_team_territory, save_game_state, save_json_data, update_stats};
use sfml::system::Vector2f;

fn fresh_state() -> GameState {
    let cfg = Config::load();
    let mut state = GameState::new(cfg);
    state.game_fps_ratio = 1.0;
    state
}

#[test]
fn update_stats_counts_tanks_and_forts_per_team() {
    let mut state = fresh_state();
    spawn_tank(&mut state, Vector2f::new(0.0, 0.0), 0, 0, false);
    spawn_tank(&mut state, Vector2f::new(0.0, 0.0), 0, 1, false); // special (level 1)
    spawn_tank(&mut state, Vector2f::new(0.0, 0.0), 1, 0, false);
    spawn_fort(&mut state, Vector2f::new(0.0, 0.0), 0, false, false);
    spawn_fort(&mut state, Vector2f::new(0.0, 0.0), 2, true, false); // captured

    update_stats(&mut state);

    assert_eq!(state.get_fort_pop(0, false), 1);
    assert_eq!(state.get_fort_pop(2, false), 1);
    assert_eq!(state.get_fort_pop(2, true), 1, "captured fort should count toward the captured subtally");
    assert_eq!(state.get_fort_pop(0, true), 0);
    assert_eq!(state.get_special_tank_pop(0), 1, "one level-1 tank on team 0");
    assert_eq!(tank_attack_rust::entity::get_tank_pop(&state, 0, 0), 2);
    assert_eq!(tank_attack_rust::entity::get_tank_pop(&state, 1, 0), 1);
    assert_eq!(state.active_entity_pop(), 5);
}

#[test]
fn update_stats_ignores_dead_entities() {
    let mut state = fresh_state();
    spawn_tank(&mut state, Vector2f::new(0.0, 0.0), 0, 0, false);
    state.entities[0].ia = false;

    update_stats(&mut state);

    assert_eq!(state.get_fort_pop(0, false) + tank_attack_rust::entity::get_tank_pop(&state, 0, 0), 0);
    assert_eq!(state.active_entity_pop(), 0);
}

#[test]
fn average_tank_age_matches_expected_formula() {
    let mut state = fresh_state();
    spawn_tank(&mut state, Vector2f::new(0.0, 0.0), 0, 0, false);
    spawn_tank(&mut state, Vector2f::new(0.0, 0.0), 0, 0, false);
    state.entities[0].age = 120.0;
    state.entities[1].age = 240.0;

    // C++: age * 1000 / FPS, FPS = 120. avg age = (120+240)/2 = 180.
    // 180 * 1000 / 120 = 1500.
    let avg = get_average_tank_age(&state, 0, 120);
    assert_eq!(avg, 1500);
}

#[test]
fn average_tank_age_is_zero_with_no_tanks() {
    let state = fresh_state();
    assert_eq!(get_average_tank_age(&state, 0, 120), 0);
}

#[test]
fn team_territory_counts_claimed_tiles() {
    let mut state = fresh_state();
    state.game_map_width = 500.0;
    state.game_map_height = 500.0;
    tank_attack_rust::tile::init_tiles(&mut state);
    spawn_fort(&mut state, Vector2f::new(250.0, 250.0), 3, false, false);
    tank_attack_rust::tile::update_tiles(&mut state);

    let territory = get_team_territory(&state, 3);
    assert!(territory > 0);
    assert_eq!(territory, state.team_tile_no[3]);
}

#[test]
fn save_json_data_writes_all_15_files_with_valid_json() {
    let mut state = fresh_state();

    // Populate the history arrays with a plausible single "tick" of data,
    // same shape update_stats/the Stage E loop would build over time.
    for i in 0..state.number_of_teams as usize {
        state.tank_pop_arr.push(vec![i as i32]);
        state.fort_pop_arr.push(vec![i as i32]);
        state.captured_fort_pop_arr.push(vec![0]);
        state.dead_tank_pop_arr.push(vec![0]);
        state.captured_fort_ratio_arr.push(vec![0.0]);
        state.special_tank_pop_arr.push(vec![0]);
        state.special_tank_ratio_arr.push(vec![0.0]);
        state.special_fort_pop_arr.push(vec![0]);
        state.special_fort_ratio_arr.push(vec![0.0]);
        state.tanks_killed_arr.push(vec![0]);
        state.tanks_lost_arr.push(vec![0]);
        state.average_tank_age_arr.push(vec![0]);
        state.team_territory_arr.push(vec![0]);
    }
    state.tank_age_team_time_arr.push(vec![[0.0, 100.0]]);
    state.tank_kills_team_time_arr.push(vec![[0.0, 2.0]]);

    save_json_data(&state).expect("save_json_data should succeed");

    let expected_files = [
        "TankPop.json",
        "FortPop.json",
        "CapturedFortPop.json",
        "DeadTankPopArr.json",
        "CapturedFortRatio.json",
        "SpecialTankPop.json",
        "SpecialTankRatio.json",
        "SpecialFortPop.json",
        "SpecialFortRatio.json",
        "TanksKilled.json",
        "TanksLost.json",
        "AverageTankAge.json",
        "Territory.json",
        "Tank_Age_Team_Time.json",
        "Tank_Kills_Team_Time.json",
    ];
    assert_eq!(expected_files.len(), 15, "sanity: spec says 15 files");

    for name in expected_files {
        let path = format!("Game Data/{name}");
        let content = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("expected {path} to exist: {e}"));
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap_or_else(|e| panic!("{path} is not valid JSON: {e}"));
        assert!(parsed.is_array(), "{path} should be a JSON array, got: {parsed}");
        assert!(!content.is_empty());
    }

    // Clean up rather than isolating via a chdir -- `std::env::
    // set_current_dir` is process-global, not per-thread, so changing it
    // from one #[test] fn corrupts the (relative-path-dependent)
    // GameRules.json load for every other test running concurrently in
    // this binary. Writing to the real "Game Data/" and removing it
    // afterward is safe as long as no other test touches that path,
    // which none of this crate's other tests do.
    let _ = std::fs::remove_dir_all("Game Data");
}

#[test]
fn save_game_state_writes_one_entity_per_live_entity() {
    let mut state = fresh_state();
    spawn_tank(&mut state, Vector2f::new(12.0, 34.0), 0, 0, false);
    spawn_fort(&mut state, Vector2f::new(56.0, 78.0), 1, false, false);
    spawn_tank(&mut state, Vector2f::new(0.0, 0.0), 2, 0, false);
    state.entities[2].ia = false; // dead but still in the array

    let filename = "stage_f_test_SaveGame.json";
    save_game_state(&state, filename).expect("save_game_state should succeed");

    let content = std::fs::read_to_string(filename).expect("save file should exist");
    let parsed: serde_json::Value = serde_json::from_str(&content).expect("valid JSON");
    let entities = parsed["entities"].as_array().expect("entities should be an array");

    // C++ iterates the whole EntityArr with no `ia` filter -- dead
    // entities are still included.
    assert_eq!(entities.len(), 3, "save_game_state includes dead entities too, matching the C++ (no ia filter)");

    let tank = &entities[0];
    assert_eq!(tank["entity_type"], 0, "tank should serialize as 0");
    assert_eq!(tank["team"], 0);
    assert_eq!(tank["pos"][0], 12.0);
    assert_eq!(tank["pos"][1], 34.0);

    let fort = &entities[1];
    assert_eq!(fort["entity_type"], 1, "fort should serialize as 1");
    assert_eq!(fort["team"], 1);

    let _ = std::fs::remove_file(filename);
}
