//! generate_random_map: ported from TankAttack.cpp L5635-6053.
//!
//! DEVIATION (per continuation spec §C.3): the C++ implements "retry
//! placement failed" by clearing `EntityArr` and **recursively calling
//! `generate_random_map`** again (sometimes with the same type, sometimes
//! re-randomizing). Unbounded recursion on a hostile/unlucky RNG seed
//! could blow the stack in Rust in a way it wouldn't necessarily in C++
//! (tail-callish but not guaranteed optimized). This port converts that
//! into a bounded outer loop (8 attempts, matching the spec's suggestion)
//! that re-runs shared setup + the chosen type's placement logic from
//! scratch each time, and falls back to a deterministic grid placement
//! (one fort per team, no jitter, guaranteed to terminate) if all 8 fail.
//! This does not distinguish "retry same type" vs "retry with a fresh
//! random type" the way different C++ call sites do (type 0's anchor
//! retry re-picks type 0 specifically; its own fort-placement retry and
//! types 1/2's retries re-randomize the type; type 3 retries type 3) --
//! this port always retries the originally-selected type. Each of the
//! 1000-iteration *inner* per-fort/anchor placement retry loops (C++'s
//! `MAX_RETRIES`) is preserved exactly as a bounded loop, not recursion.

use crate::entity::EntityType;
use crate::state::GameState;
use crate::util::displacement;
use sfml::graphics::{Color, FloatRect};
use sfml::system::Vector2f;

const MAX_RETRIES: u32 = 1000;
const OUTER_RETRIES: u32 = 8;
const DEG_TO_RAD: f32 = std::f32::consts::PI / 180.0;
const MAP_SIZES: [f32; 12] = [
    800.0, 1600.0, 2400.0, 3200.0, 4000.0, 4800.0, 5600.0, 6400.0, 7200.0, 8000.0, 9600.0, 12000.0,
];

struct RetryExceeded;

/// C++: void generate_random_map(int type = -1)
pub fn generate_random_map(state: &mut GameState, map_type: Option<i32>) {
    if state.event_logging {
        println!("/DEMOGAME()/");
    }

    reset_for_map_gen(state);

    let map_type = map_type.unwrap_or_else(|| state.rand_range(0..4));

    for _outer in 0..OUTER_RETRIES {
        state.entities.clear();

        match try_generate(state, map_type) {
            Ok(setup) => {
                post_setup(state, map_type, &setup);
                return;
            }
            Err(RetryExceeded) => continue,
        }
    }

    // All 8 attempts failed (only realistically reachable with a
    // pathological seed + tiny map): fall back to one fort per team on a
    // deterministic grid, guaranteed to terminate and stay in bounds.
    state.entities.clear();
    let setup = fallback_grid_placement(state);
    post_setup(state, map_type, &setup);
}

/// C++'s top-of-function state reset (L5641-5657).
fn reset_for_map_gen(state: &mut GameState) {
    state.current_game_timer = 0;
    state.extra_population = 0;

    state.entities.clear();
    state.lasers.clear();
    state.missiles.clear();
    state.tiles.clear();
    state.particles.clear();
    state.game_texts.clear();
    state.dead_tanks.clear();
    state.buttons.clear();
    state.game_log_arr.clear();

    let n = state.number_of_teams as usize;
    state.tanks_killed = vec![0; n];
    state.tanks_lost = vec![0; n];
    state.forts_destroyed = vec![0; n];
    state.forts_lost = vec![0; n];
    state.pvp_tanks_killed = vec![0; n];
    state.pvp_tanks_lost = vec![0; n];

    state.tank_pop_arr.clear();
    state.fort_pop_arr.clear();
    state.captured_fort_pop_arr.clear();
    state.dead_tank_pop_arr.clear();
    state.captured_fort_ratio_arr.clear();
    state.special_tank_pop_arr.clear();
    state.special_tank_ratio_arr.clear();
    state.special_fort_pop_arr.clear();
    state.special_fort_ratio_arr.clear();
    state.tanks_killed_arr.clear();
    state.tanks_lost_arr.clear();
    state.average_tank_age_arr.clear();
    state.team_territory_arr.clear();
    state.tank_age_team_time_arr.clear();
    state.tank_kills_team_time_arr.clear();
}

/// Values computed during shared setup / the per-type branch that
/// `post_setup` needs afterward. Replaces the C++ locals that stay in
/// scope for the rest of the function body (`forts_per_team`,
/// `doubledForts`, ...).
struct MapSetup {
    forts_per_team: f32,
    doubled_forts: bool,
    triplet_forts: bool,
    quad_forts: bool,
}

fn select_teams(state: &mut GameState, n: i32) -> Vec<i32> {
    let mut out: Vec<i32> = Vec::new();
    while (out.len() as i32) < n {
        let t = state.rand_range(0..state.number_of_teams);
        if !out.contains(&t) {
            out.push(t);
        }
    }
    out
}

/// C++'s `spawnFortCluster` closure (L5698-5709).
fn spawn_fort_cluster(
    state: &mut GameState,
    center: Vector2f,
    team: i32,
    doubled: bool,
    triplet: bool,
    quad: bool,
) {
    if !doubled && !triplet && !quad {
        crate::spawn::spawn_fort(state, center, team, false, false);
        return;
    }
    let n = if doubled {
        2
    } else if triplet {
        3
    } else {
        4
    };
    let mut ang = state.rand_range(0..360) as f32;
    let fort_range = state.config.entity_values.fort_range as f32;
    let r = fort_range * 0.3;
    for _ in 0..n {
        let pos = Vector2f::new(
            (ang * DEG_TO_RAD).cos() * r + center.x,
            (ang * DEG_TO_RAD).sin() * r + center.y,
        );
        crate::spawn::spawn_fort(state, pos, team, false, false);
        ang += 360.0 / n as f32;
    }
}

/// Runs shared setup then dispatches to the chosen type's placement logic.
/// Returns `Err(RetryExceeded)` if any inner MAX_RETRIES loop was
/// exhausted (caller clears entities and tries the whole thing again).
fn try_generate(state: &mut GameState, map_type: i32) -> Result<MapSetup, RetryExceeded> {
    let fort_multipliers: Vec<f32> = MAP_SIZES
        .iter()
        .map(|&s| (s * s * 2.5) / (8000.0 * 8000.0))
        .collect();
    let size_idx = state.rand_range(0..MAP_SIZES.len() as i32) as usize;

    state.game_map_width = MAP_SIZES[size_idx];
    state.game_map_height = MAP_SIZES[size_idx];
    let map_bounds = FloatRect::new(0.0, 0.0, state.game_map_width, state.game_map_height);

    let mut forts_per_team = (state.rand_range(0..13) + 2) as f32 * fort_multipliers[size_idx];
    let mut doubled_forts = false;
    let mut triplet_forts = false;
    let mut quad_forts = false;
    let mut dist_factor = 1.0f32;

    match map_type {
        0 => try_generate_type0(
            state,
            map_bounds,
            &mut forts_per_team,
            &mut doubled_forts,
            &mut triplet_forts,
            &mut quad_forts,
            &mut dist_factor,
        )?,
        1 => try_generate_type1(
            state,
            map_bounds,
            &mut forts_per_team,
            &mut doubled_forts,
            &mut triplet_forts,
            &mut quad_forts,
            &mut dist_factor,
        )?,
        2 => try_generate_type2(state, map_bounds, &mut forts_per_team)?,
        3 => try_generate_type3(
            state,
            map_bounds,
            &mut forts_per_team,
            &mut doubled_forts,
            &mut triplet_forts,
            &mut quad_forts,
        )?,
        _ => try_generate_type0(
            state,
            map_bounds,
            &mut forts_per_team,
            &mut doubled_forts,
            &mut triplet_forts,
            &mut quad_forts,
            &mut dist_factor,
        )?,
    }

    Ok(MapSetup {
        forts_per_team,
        doubled_forts,
        triplet_forts,
        quad_forts,
    })
}

/// TYPE 0 -- Random Map (C++ L5712-5782).
fn try_generate_type0(
    state: &mut GameState,
    map_bounds: FloatRect,
    forts_per_team: &mut f32,
    doubled_forts: &mut bool,
    triplet_forts: &mut bool,
    quad_forts: &mut bool,
    dist_factor: &mut f32,
) -> Result<(), RetryExceeded> {
    if state.event_logging {
        println!("\n  Demogame 1: Random Map. Setting up...");
    }
    let (sw, sh) = (
        state.config.graphics.screen_width,
        state.config.graphics.screen_height,
    );
    crate::spawn::spawn_game_text(
        state,
        "Random Game".to_string(),
        Vector2f::new(sw / 2.0, sh / 2.0),
        Color::rgb(255, 255, 255),
        40,
        5.0,
        2.0,
        true,
    );

    let team_count = state.rand_range(0..state.number_of_teams - 1) + 2;
    // DEVIATION: none -- `16 / team_count` is genuine C++ integer division
    // (both operands ints), preserved exactly via i32 division before the
    // sqrt, per the C++ comment "integer division intentional".
    *forts_per_team *= ((16 / team_count) as f32).sqrt();

    match state.rand_range(0..7) {
        2 | 3 => {
            *doubled_forts = true;
            *forts_per_team /= 2.0;
            *dist_factor = 2.0f32.sqrt();
        }
        4 | 5 => {
            *triplet_forts = true;
            *forts_per_team /= 3.0;
            *dist_factor = 3.0f32.sqrt();
        }
        6 => {
            *quad_forts = true;
            *forts_per_team /= 4.0;
            *dist_factor = 4.0f32.sqrt();
        }
        _ => {}
    }

    let teams = select_teams(state, team_count);
    let mut anchors: Vec<Vector2f> = Vec::new();

    for _ in 0..team_count {
        let mut c;
        let mut retries = 0u32;
        loop {
            retries += 1;
            if retries > MAX_RETRIES {
                if state.event_logging {
                    println!("Random Map: anchor placement failed!");
                }
                return Err(RetryExceeded);
            }
            c = Vector2f::new(
                (state.rand_range(0..100) as f32) * state.game_map_width / 100.0,
                (state.rand_range(0..100) as f32) * state.game_map_height / 100.0,
            );
            let too_close = anchors.iter().any(|a: &Vector2f| {
                let dx = c.x - a.x;
                let dy = c.y - a.y;
                (dx * dx + dy * dy).sqrt() < 100.0
            });
            if !too_close {
                break;
            }
        }
        anchors.push(c);
    }

    let n_teams_total = state.number_of_teams as f32;
    for _j in 0..(*forts_per_team as i32) {
        let mut retries = 0u32;
        for i in 0..team_count as usize {
            let mut pos;
            loop {
                retries += 1;
                if retries > MAX_RETRIES {
                    if state.event_logging {
                        println!("Random Map: fort placement failed!");
                    }
                    return Err(RetryExceeded);
                }

                let dis = (state.rand_range(0..500) as f32) * state.game_map_width / 2500.0
                    * (n_teams_total / team_count as f32).powf(0.75);
                let ang = state.rand_range(0..360) as f32;
                pos = Vector2f::new(
                    (ang * DEG_TO_RAD).cos() * dis + anchors[i].x,
                    (ang * DEG_TO_RAD).sin() * dis + anchors[i].y,
                );

                let mut repeat = !map_bounds.contains(pos);
                for e in &state.entities {
                    if repeat || e.entity_type != EntityType::Fort {
                        continue;
                    }
                    let dx = pos.x - e.pos.x;
                    let dy = pos.y - e.pos.y;
                    let dist = (dx * dx + dy * dy).sqrt();
                    let fort_range = state.config.entity_values.fort_range as f32;
                    if e.team != teams[i] && dist <= fort_range * 1.25 * *dist_factor {
                        repeat = true;
                    }
                    if e.team == teams[i] && dist < fort_range * 0.5 * *dist_factor {
                        repeat = true;
                    }
                }
                if !repeat {
                    break;
                }
            }
            spawn_fort_cluster(
                state,
                pos,
                teams[i],
                *doubled_forts,
                *triplet_forts,
                *quad_forts,
            );
        }
    }

    if state.event_logging {
        println!("    Demogame 1: Random Map. Setup complete.");
    }
    Ok(())
}

/// TYPE 1 -- 1 vs 1 (C++ L5786-5845).
fn try_generate_type1(
    state: &mut GameState,
    map_bounds: FloatRect,
    forts_per_team: &mut f32,
    doubled_forts: &mut bool,
    triplet_forts: &mut bool,
    quad_forts: &mut bool,
    dist_factor: &mut f32,
) -> Result<(), RetryExceeded> {
    if state.event_logging {
        println!("  Demogame 2: 1 VS 1. Setting up...");
    }
    let (sw, sh) = (
        state.config.graphics.screen_width,
        state.config.graphics.screen_height,
    );
    crate::spawn::spawn_game_text(
        state,
        "1 VS 1".to_string(),
        Vector2f::new(sw / 2.0, sh / 2.0),
        Color::rgb(255, 255, 255),
        40,
        5.0,
        3.0,
        true,
    );

    const TEAM_COUNT: i32 = 2;
    // DEVIATION: none -- `16 / TEAM_COUNT` is genuine C++ integer division.
    *forts_per_team *= ((16 / TEAM_COUNT) as f32).sqrt() * 2.0;

    match state.rand_range(0..7) {
        2 | 3 => {
            *doubled_forts = true;
            *forts_per_team /= 2.0;
            *dist_factor = 2.0;
        }
        4 | 5 => {
            *triplet_forts = true;
            *forts_per_team /= 3.0;
            *dist_factor = 3.0;
        }
        6 => {
            *quad_forts = true;
            *forts_per_team /= 4.0;
            *dist_factor = 4.0;
        }
        _ => {}
    }

    let teams = select_teams(state, TEAM_COUNT);
    let map_center = Vector2f::new(state.game_map_width / 2.0, state.game_map_height / 2.0);
    let arc_range = 360.0 / TEAM_COUNT as f32;
    let angle_offset = state.rand_range(0..360) as f32;
    let max_dis =
        ((state.game_map_width / 2.0).powi(2) + (state.game_map_height / 2.0).powi(2)).sqrt();

    for _i in 0..(*forts_per_team as i32) {
        for j in 0..TEAM_COUNT as usize {
            let mut pos;
            let mut retries = 0u32;
            loop {
                retries += 1;
                if retries > MAX_RETRIES {
                    if state.event_logging {
                        println!("1 VS 1: fort placement failed!");
                    }
                    return Err(RetryExceeded);
                }

                let team_ang = arc_range * j as f32 + angle_offset;
                let ang = team_ang + state.rand_range(0..arc_range as i32) as f32;
                let dis = state.rand_range(0..max_dis as i32) as f32;
                pos = Vector2f::new(
                    (ang * DEG_TO_RAD).cos() * dis + map_center.x,
                    (ang * DEG_TO_RAD).sin() * dis + map_center.y,
                );

                let mut repeat = !map_bounds.contains(pos);
                for e in &state.entities {
                    if repeat || e.entity_type != EntityType::Fort {
                        continue;
                    }
                    let dx = e.pos.x - pos.x;
                    let dy = e.pos.y - pos.y;
                    let dist = (dx * dx + dy * dy).sqrt();
                    let fort_range = state.config.entity_values.fort_range as f32;
                    if e.team != teams[j] && dist < fort_range * 1.1 * *dist_factor {
                        repeat = true;
                    }
                    if e.team == teams[j] && dist < fort_range * 0.3 * *dist_factor {
                        repeat = true;
                    }
                }
                if !repeat {
                    break;
                }
            }
            spawn_fort_cluster(
                state,
                pos,
                teams[j],
                *doubled_forts,
                *triplet_forts,
                *quad_forts,
            );
        }
    }

    if state.event_logging {
        println!("    Demogame 2: 1 VS 1. Setup complete.");
    }
    Ok(())
}

/// TYPE 2 -- Roundtable (C++ L5849-5901). No cluster forts (doubled/
/// triplet/quad don't apply -- always plain `spawn_fort`, matching C++).
fn try_generate_type2(
    state: &mut GameState,
    map_bounds: FloatRect,
    forts_per_team: &mut f32,
) -> Result<(), RetryExceeded> {
    if state.event_logging {
        println!("  Demogame 3: Roundtable. Setting up...");
    }
    let (sw, sh) = (
        state.config.graphics.screen_width,
        state.config.graphics.screen_height,
    );
    crate::spawn::spawn_game_text(
        state,
        "Roundable Game".to_string(),
        Vector2f::new(sw / 2.0, sh / 2.0),
        Color::rgb(255, 255, 255),
        40,
        5.0,
        3.0,
        true,
    );

    let team_count = state.rand_range(0..state.number_of_teams - 2) + 3;
    // DEVIATION: none -- integer division intentional, as in type 0.
    *forts_per_team *= ((16 / team_count) as f32).sqrt();
    let arc_range = 360.0 / team_count as f32;
    let angle_offset = state.rand_range(0..360) as f32;
    let max_dis =
        ((state.game_map_width / 2.0).powi(2) + (state.game_map_height / 2.0).powi(2)).sqrt();
    let map_center = Vector2f::new(state.game_map_width / 2.0, state.game_map_height / 2.0);

    let teams = select_teams(state, team_count);

    for _i in 0..(*forts_per_team as i32) {
        for j in 0..team_count as usize {
            let mut pos;
            let mut retries = 0u32;
            loop {
                retries += 1;
                if retries > MAX_RETRIES {
                    if state.event_logging {
                        println!("Roundtable: fort placement failed!");
                    }
                    return Err(RetryExceeded);
                }

                let team_ang = arc_range * j as f32 + angle_offset;
                let ang = team_ang + state.rand_range(0..arc_range as i32) as f32;
                let dis = state.rand_range(0..max_dis as i32) as f32;
                pos = Vector2f::new(
                    (ang * DEG_TO_RAD).cos() * dis + map_center.x,
                    (ang * DEG_TO_RAD).sin() * dis + map_center.y,
                );

                let mut repeat = !map_bounds.contains(pos);
                let fort_range = state.config.entity_values.fort_range as f32;
                for e in &state.entities {
                    if repeat || e.entity_type != EntityType::Fort {
                        continue;
                    }
                    let dx = e.pos.x - pos.x;
                    let dy = e.pos.y - pos.y;
                    let dist = (dx * dx + dy * dy).sqrt();
                    if e.team != teams[j] && dist < fort_range {
                        repeat = true;
                    }
                    if e.team == teams[j] && dist < fort_range * 0.4 {
                        repeat = true;
                    }
                }
                let cdx = map_center.x - pos.x;
                let cdy = map_center.y - pos.y;
                if (cdx * cdx + cdy * cdy).sqrt() > state.game_map_height / 2.0 {
                    repeat = true;
                }
                if !repeat {
                    break;
                }
            }
            crate::spawn::spawn_fort(state, pos, teams[j], false, false);
        }
    }

    if state.event_logging {
        println!("    Demogame 3: Roundtable. Setup complete.");
    }
    Ok(())
}

/// TYPE 3 -- Chaos (C++ L5905-5989). Always uses every team.
fn try_generate_type3(
    state: &mut GameState,
    map_bounds: FloatRect,
    forts_per_team: &mut f32,
    doubled_forts: &mut bool,
    triplet_forts: &mut bool,
    quad_forts: &mut bool,
) -> Result<(), RetryExceeded> {
    if state.event_logging {
        println!("  Demogame 4: Chaos. Setting up...");
    }
    let (sw, sh) = (
        state.config.graphics.screen_width,
        state.config.graphics.screen_height,
    );
    crate::spawn::spawn_game_text(
        state,
        "Chaos Game".to_string(),
        Vector2f::new(sw / 2.0, sh / 2.0),
        Color::rgb(255, 255, 255),
        40,
        5.0,
        3.0,
        true,
    );

    let team_count = state.number_of_teams;
    // C++ computes `forts_per_team *= sqrt(16/teamCount) * 0.6` and then
    // immediately overwrites it with the formula below -- the first line
    // has no effect (dead code, `forts_per_team` is unconditionally
    // reassigned next). Omitted; only the effective formula is kept.
    *forts_per_team = (state.game_map_width * state.game_map_width * 12.0) / (8000.0 * 8000.0);

    match state.rand_range(0..7) {
        2 | 3 => *doubled_forts = true,
        4 | 5 => *triplet_forts = true,
        6 => *quad_forts = true,
        _ => {}
    }

    if state.game_map_width >= 8000.0 {
        if *doubled_forts {
            *forts_per_team /= 2.0;
        }
        if *triplet_forts {
            *forts_per_team /= 3.0;
        }
        if *quad_forts {
            *forts_per_team /= 4.0;
        }
    }

    let teams = select_teams(state, team_count);
    let map_center = Vector2f::new(state.game_map_width / 2.0, state.game_map_height / 2.0);

    for _i in 0..(*forts_per_team as i32) {
        for j in 0..team_count as usize {
            let mut pos;
            let mut retries = 0u32;
            loop {
                retries += 1;
                if retries > MAX_RETRIES {
                    if state.event_logging {
                        println!("Chaos Map: fort placement failed!");
                    }
                    return Err(RetryExceeded);
                }

                pos = Vector2f::new(
                    state.rand_range(0..state.game_map_width as i32) as f32,
                    state.rand_range(0..state.game_map_height as i32) as f32,
                );

                let cdx = map_center.x - pos.x;
                let cdy = map_center.y - pos.y;
                let mut repeat = !map_bounds.contains(pos)
                    || (cdx * cdx + cdy * cdy).sqrt() > state.game_map_height / 2.0;

                let fort_range = state.config.entity_values.fort_range as f32;
                for e in &state.entities {
                    if repeat || e.entity_type != EntityType::Fort {
                        continue;
                    }
                    let dx = pos.x - e.pos.x;
                    let dy = pos.y - e.pos.y;
                    let dist = (dx * dx + dy * dy).sqrt();
                    if e.team != teams[j] && dist <= fort_range * 1.2 {
                        repeat = true;
                    }
                    if e.team == teams[j] && dist < fort_range * 2.5 {
                        repeat = true;
                    }
                }
                if !repeat {
                    break;
                }
            }
            crate::spawn::spawn_fort(state, pos, teams[j], false, false);

            if *doubled_forts || *triplet_forts || *quad_forts {
                let ang = state.rand_range(0..360) as f32;
                let fort_range = state.config.entity_values.fort_range as f32;
                if *doubled_forts {
                    let p = Vector2f::new(
                        (ang * DEG_TO_RAD).cos() * fort_range / 2.0 + pos.x,
                        (ang * DEG_TO_RAD).sin() * fort_range / 2.0 + pos.y,
                    );
                    crate::spawn::spawn_fort(state, p, teams[j], false, false);
                } else if *triplet_forts {
                    let p1 = Vector2f::new(
                        (ang * DEG_TO_RAD).cos() * fort_range / 2.0 + pos.x,
                        (ang * DEG_TO_RAD).sin() * fort_range / 2.0 + pos.y,
                    );
                    crate::spawn::spawn_fort(state, p1, teams[j], false, false);
                    let ang2 = ang + 60.0;
                    let p2 = Vector2f::new(
                        (ang2 * DEG_TO_RAD).cos() * fort_range / 2.0 + pos.x,
                        (ang2 * DEG_TO_RAD).sin() * fort_range / 2.0 + pos.y,
                    );
                    crate::spawn::spawn_fort(state, p2, teams[j], false, false);
                } else {
                    // quadForts
                    for offset in [0.0, 120.0, 240.0] {
                        let a = ang + offset;
                        let p = Vector2f::new(
                            (a * DEG_TO_RAD).cos() * fort_range / 1.5 + pos.x,
                            (a * DEG_TO_RAD).sin() * fort_range / 1.5 + pos.y,
                        );
                        crate::spawn::spawn_fort(state, p, teams[j], false, false);
                    }
                }
            }
        }
    }

    Ok(())
}

/// DEVIATION: not present in C++. Deterministic last-resort fallback if
/// all `OUTER_RETRIES` attempts fail (see module doc comment) -- one fort
/// per team, evenly spaced on a ring around the map center, no jitter, so
/// it always terminates and always stays in bounds.
fn fallback_grid_placement(state: &mut GameState) -> MapSetup {
    let n = state.number_of_teams;
    state.game_map_width = MAP_SIZES[MAP_SIZES.len() / 2];
    state.game_map_height = state.game_map_width;
    let center = Vector2f::new(state.game_map_width / 2.0, state.game_map_height / 2.0);
    let radius = state.game_map_width * 0.3;

    for team in 0..n {
        let ang = (360.0 / n as f32) * team as f32;
        let pos = Vector2f::new(
            (ang * DEG_TO_RAD).cos() * radius + center.x,
            (ang * DEG_TO_RAD).sin() * radius + center.y,
        );
        crate::spawn::spawn_fort(state, pos, team, false, false);
    }

    MapSetup {
        forts_per_team: 1.0,
        doubled_forts: false,
        triplet_forts: false,
        quad_forts: false,
    }
}

/// C++'s post-setup block (L6000-6050).
fn post_setup(state: &mut GameState, map_type: i32, setup: &MapSetup) {
    let max_spawn_times: Vec<(usize, f32)> = state
        .entities
        .iter()
        .enumerate()
        .filter(|(_, e)| e.entity_type == EntityType::Fort)
        .map(|(i, e)| (i, e.max_tank_spawn_time))
        .collect();
    for (i, max_t) in max_spawn_times {
        state.entities[i].tank_spawn_timer = max_t;
    }

    let mut total_forts = setup.forts_per_team as i32;
    if setup.doubled_forts {
        total_forts *= 2;
    }
    if setup.triplet_forts {
        total_forts *= 3;
    }
    if setup.quad_forts {
        total_forts *= 4;
    }
    state.extra_population = total_forts - state.config.entity_values.tank_min_pop;

    if map_type != 3 {
        for team in 0..state.number_of_teams {
            let mut team_center = Vector2f::new(0.0, 0.0);
            let mut fort_count = 0.0f32;
            for e in &state.entities {
                if e.entity_type == EntityType::Fort && e.team == team {
                    team_center.x += e.pos.x;
                    team_center.y += e.pos.y;
                    fort_count += 1.0;
                }
            }
            if fort_count == 0.0 {
                continue;
            }
            team_center.x /= fort_count;
            team_center.y /= fort_count;

            let mut closest: Option<usize> = None;
            let mut closest_dis = 100_000.0f32;
            for i in 0..state.entities.len() {
                let e = &state.entities[i];
                if e.entity_type != EntityType::Fort || e.team != team {
                    continue;
                }
                let d = displacement(e.pos, team_center);
                if d < closest_dis {
                    closest_dis = d;
                    closest = Some(i);
                }
            }
            if let Some(i) = closest {
                state.entities[i].fort_level = 1;
            }
        }
    }

    crate::tile::init_tiles(state);
    crate::tile::update_tiles(state);
    crate::mapgen::load_game_buttons(state);

    let (sw, sh) = (
        state.config.graphics.screen_width,
        state.config.graphics.screen_height,
    );
    let text = format!(
        "{} x {}",
        state.game_map_width as i32, state.game_map_height as i32
    );
    crate::spawn::spawn_game_text(
        state,
        text,
        Vector2f::new(sw / 2.0, sh / 2.0 + 50.0),
        Color::rgb(255, 255, 255),
        20,
        5.0,
        1.0,
        true,
    );

    if state.event_logging {
        println!("  DEMOGAME() executed");
    }
}

/// C++: void load_game_buttons() (TankAttack.cpp L6077-6088).
pub fn load_game_buttons(state: &mut GameState) {
    let sh = state.config.graphics.screen_height;
    let sw = state.config.graphics.screen_width;
    let n = state.number_of_teams;

    crate::spawn::spawn_button(
        state,
        Vector2f::new(10.0, sh - 40.0),
        Vector2f::new(30.0, 25.0),
        "QUIT".to_string(),
        Color::rgb(200, 20, 40),
        0,
        16,
    );
    crate::spawn::spawn_button(
        state,
        Vector2f::new(50.0, sh - 40.0),
        Vector2f::new(50.0, 25.0),
        "RANDOM".to_string(),
        Color::rgb(20, 40, 200),
        1,
        16,
    );
    crate::spawn::spawn_button(
        state,
        Vector2f::new(110.0, sh - 40.0),
        Vector2f::new(50.0, 25.0),
        "DELETE".to_string(),
        Color::rgb(40, 40, 40),
        n + 2,
        16,
    );
    crate::spawn::spawn_button(
        state,
        Vector2f::new(170.0, sh - 40.0),
        Vector2f::new(75.0, 25.0),
        "NULL TEAM".to_string(),
        Color::rgb(200, 200, 200),
        n + 3,
        16,
    );
    crate::spawn::spawn_button(
        state,
        Vector2f::new(255.0, sh - 40.0),
        Vector2f::new(60.0, 25.0),
        "GANG UP".to_string(),
        Color::rgb(100, 160, 20),
        n + 4,
        16,
    );

    for i in 0..n {
        let col = state.color_arr[(n - i - 1) as usize];
        crate::spawn::spawn_button(
            state,
            Vector2f::new(sw - ((i + 1) as f32 * 30.0), sh - 40.0),
            Vector2f::new(20.0, 25.0),
            (n - i).to_string(),
            col,
            (n - i - 1) + 2,
            16,
        );
    }
}
