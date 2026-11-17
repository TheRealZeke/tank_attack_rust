//! Stage F: population statistics, ported from TankAttack.cpp's
//! `update_stats` (L6091-6141), `get_average_tank_age`/`get_team_territory`
//! (L5452-5484), `save_json_data` (L6141-6353), and `save_game_state`
//! (L6660-6690).

use crate::entity::EntityType;
use crate::state::GameState;
use std::fs;
use std::io::Write;

/// C++: void update_stats() (TankAttack.cpp L6091-6141). Recomputes the
/// `temp_*` per-team population snapshots that `get_fort_pop`/
/// `get_special_tank_pop`/`get_special_fort_pop` (already on `GameState`,
/// see state.rs) read from.
pub fn update_stats(state: &mut GameState) {
    let n = state.number_of_teams as usize;
    let mut tank_num = vec![0i32; n];
    let mut fort_num = vec![0i32; n];
    let mut captured_num = vec![0i32; n];
    let mut special_tank_num = vec![0i32; n];
    let mut special_fort_num = vec![0i32; n];

    for e in &state.entities {
        if !e.ia {
            continue;
        }
        let team = e.team;
        if team < 0 || team as usize >= n {
            continue;
        }
        match e.entity_type {
            EntityType::Tank => {
                tank_num[team as usize] += 1;
                if e.level != 0 {
                    special_tank_num[team as usize] += 1;
                }
            }
            EntityType::Fort => {
                fort_num[team as usize] += 1;
                if e.tank_spawn_timer < 0.0 {
                    captured_num[team as usize] += 1;
                }
                if e.fort_level != 0 {
                    special_fort_num[team as usize] += 1;
                }
            }
            EntityType::Null => {}
        }
    }

    state.temp_tank_pop = tank_num;
    state.temp_fort_pop = fort_num;
    state.temp_captured_fort_pop = captured_num;
    state.temp_special_tank_pop = special_tank_num;
    state.temp_special_fort_pop = special_fort_num;
}

/// C++: int get_average_tank_age(int team) (TankAttack.cpp L5452-5463).
/// `fps` is C++'s `FPS` constant (120), passed in rather than hardcoded
/// as a magic number at the call site.
pub fn get_average_tank_age(state: &GameState, team: i32, fps: i32) -> i32 {
    let mut num = 0.0f32;
    let mut age = 0.0f32;
    for e in &state.entities {
        if e.entity_type == EntityType::Tank && e.team == team {
            num += 1.0;
            age += e.age * 1000.0 / fps as f32;
        }
    }
    if num == 0.0 {
        return 0;
    }
    (age / num).round() as i32
}

/// C++: int get_team_territory(int team) (TankAttack.cpp L5476-5484).
pub fn get_team_territory(state: &GameState, team: i32) -> i32 {
    let mut num = 0;
    for row in &state.tiles {
        for t in row {
            if t.team == team {
                num += 1;
            }
        }
    }
    num
}

/// C++: void save_json_data() (TankAttack.cpp L6141-6353). Writes all 15
/// files under `Game Data/`. Returns `Err` with a description if the
/// directory can't be created or a file can't be written, rather than
/// C++'s silent `if (outputFile.is_open())` skip -- callers that want
/// the original's "just skip it" behavior can `.ok()` the result.
pub fn save_json_data(state: &GameState) -> std::io::Result<()> {
    let folder = "Game Data";
    fs::create_dir_all(folder)?;

    write_json(folder, "TankPop.json", &state.tank_pop_arr)?;
    write_json(folder, "FortPop.json", &state.fort_pop_arr)?;
    write_json(folder, "CapturedFortPop.json", &state.captured_fort_pop_arr)?;
    write_json(folder, "DeadTankPopArr.json", &state.dead_tank_pop_arr)?;
    write_json(
        folder,
        "CapturedFortRatio.json",
        &state.captured_fort_ratio_arr,
    )?;
    write_json(folder, "SpecialTankPop.json", &state.special_tank_pop_arr)?;
    write_json(
        folder,
        "SpecialTankRatio.json",
        &state.special_tank_ratio_arr,
    )?;
    write_json(folder, "SpecialFortPop.json", &state.special_fort_pop_arr)?;
    write_json(
        folder,
        "SpecialFortRatio.json",
        &state.special_fort_ratio_arr,
    )?;
    write_json(folder, "TanksKilled.json", &state.tanks_killed_arr)?;
    write_json(folder, "TanksLost.json", &state.tanks_lost_arr)?;
    write_json(folder, "AverageTankAge.json", &state.average_tank_age_arr)?;
    write_json(folder, "Territory.json", &state.team_territory_arr)?;
    write_json(
        folder,
        "Tank_Age_Team_Time.json",
        &state.tank_age_team_time_arr,
    )?;
    write_json(
        folder,
        "Tank_Kills_Team_Time.json",
        &state.tank_kills_team_time_arr,
    )?;

    Ok(())
}

fn write_json<T: serde::Serialize>(folder: &str, name: &str, data: &T) -> std::io::Result<()> {
    let path = format!("{folder}/{name}");
    let json = serde_json::to_string_pretty(data).unwrap_or_else(|_| "[]".to_string());
    let mut f = fs::File::create(path)?;
    f.write_all(json.as_bytes())?;
    Ok(())
}

#[derive(serde::Serialize)]
struct SavedEntity {
    entity_type: i32,
    unique_id: u32,
    entity_index: i32,
    team: i32,
    hitpoints: f32,
    max_hitpoints: f32,
    ang: f32,
    pos: [f32; 2],
    vel: [f32; 2],
    acc: [f32; 2],
}

#[derive(serde::Serialize)]
struct SavedGameState {
    entities: Vec<SavedEntity>,
}

/// C++: void save_game_state(const std::string& filename) (TankAttack.cpp
/// L6660-6690).
///
/// DEVIATION: `entity_type` serializes as the C++ enum's underlying
/// integer (`tank=0, fort=1, null=2`, TankAttack.cpp L36-39) to match
/// the C++'s output shape exactly, rather than as a Rust-idiomatic
/// string tag.
pub fn save_game_state(state: &GameState, filename: &str) -> std::io::Result<()> {
    let entities: Vec<SavedEntity> = state
        .entities
        .iter()
        .map(|e| SavedEntity {
            entity_type: match e.entity_type {
                EntityType::Tank => 0,
                EntityType::Fort => 1,
                EntityType::Null => 2,
            },
            unique_id: e.unique_id,
            entity_index: e.entity_index,
            team: e.team,
            hitpoints: e.hitpoints,
            max_hitpoints: e.max_hitpoints,
            ang: e.ang,
            pos: [e.pos.x, e.pos.y],
            vel: [e.vel.x, e.vel.y],
            acc: [e.acc.x, e.acc.y],
        })
        .collect();

    let saved = SavedGameState { entities };
    let json = serde_json::to_string_pretty(&saved).unwrap_or_else(|_| "{}".to_string());
    let mut f = fs::File::create(filename)?;
    f.write_all(json.as_bytes())?;
    Ok(())
}
