//! Laser: ported from `class Laser` (TankAttack.cpp L1055-1305).
//! `sender: Entity*` becomes `sender_uid: Option<u32>` per spec §4.3.

use crate::entity::{entity_type_string, EntityType};
use crate::state::GameState;
use crate::util::{check_line_collision, dim_color, opposite_color};
use sfml::graphics::Color;
use sfml::system::Vector2f;

pub struct Laser {
    pub pos: Vector2f,
    pub last_pos: Vector2f,
    pub vel: Vector2f,
    pub team: i32,
    pub ia: bool,
    pub col: Color,
    pub actual_col: Color,
    pub unique_id: i32,
    pub dam: f32,
    pub life: f32,
    pub max_life: f32,
    pub chunks: [i32; 2],
    pub size: Vec<f32>,
    pub lost_sender: bool,

    // C++: Entity* sender + sender_entity_type/sender_unique_id/sender_entity_index/sender_level
    pub sender_uid: Option<u32>,
    pub sender_entity_index_hint: Option<usize>,
    pub sender_entity_type: EntityType,
    pub sender_level: i32,
}

impl Laser {
    /// C++: Laser()
    pub fn new() -> Self {
        Laser {
            pos: Vector2f::new(0.0, 0.0),
            last_pos: Vector2f::new(0.0, 0.0),
            vel: Vector2f::new(0.0, 0.0),
            team: 0,
            ia: false,
            col: Color::rgba(0, 0, 0, 0),
            actual_col: Color::rgba(0, 0, 0, 0),
            unique_id: -1,
            dam: 0.0,
            life: 60.0,
            max_life: 60.0,
            chunks: [0, 0],
            size: Vec::new(),
            lost_sender: false,
            sender_uid: None,
            sender_entity_index_hint: None,
            sender_entity_type: EntityType::Null,
            sender_level: 0,
        }
    }
}

impl Default for Laser {
    fn default() -> Self {
        Self::new()
    }
}

/// C++: bool Laser::spawn(spawn_pos, spawn_vel, ang, spawn_dam, spawn_team,
/// sender_specs, sp_size)
/// `sender_specs` was `[entity_index, unique_id]`; here it's just the uid --
/// the index hint is looked up fresh, since Rust can't cache a stale index
/// across the call boundary safely.
#[allow(clippy::too_many_arguments)]
pub fn laser_spawn(
    state: &mut GameState,
    laser_index: usize,
    spawn_pos: Vector2f,
    spawn_vel: f32,
    ang: f32,
    spawn_dam: f32,
    spawn_team: i32,
    sender_uid: u32,
    sp_size: Vec<f32>,
    dim_color_perc: f32,
) -> bool {
    let sender_idx = match state.find_entity_index_by_uid(sender_uid) {
        Some(i) => i,
        None => {
            state.lasers[laser_index].ia = false;
            return false;
        }
    };

    if spawn_team >= state.number_of_teams || spawn_team < 0 {
        return false;
    }

    let sender_entity_type = state.entities[sender_idx].entity_type;
    let sender_level = state.entities[sender_idx].level;

    let l = &mut state.lasers[laser_index];
    l.sender_uid = Some(sender_uid);
    l.sender_entity_index_hint = Some(sender_idx);
    l.ia = true;
    l.dam = spawn_dam;
    l.pos = spawn_pos;
    l.last_pos = l.pos;
    l.team = spawn_team;
    let rad = (ang as f64) * std::f64::consts::PI / 180.0;
    l.vel = Vector2f::new(
        (rad.cos() as f32) * spawn_vel,
        (rad.sin() as f32) * spawn_vel,
    );
    l.life = l.max_life;
    l.size = sp_size;

    l.col = opposite_color(state.color_arr[spawn_team as usize]);
    l.actual_col = dim_color(l.col, dim_color_perc);

    l.sender_entity_type = sender_entity_type;
    l.sender_level = sender_level;

    true
}

/// C++: void Laser::move()
/// Returns a list of (killer_uid, killed_index) kill events so the caller
/// can push GameLogArr / spawn floating text / update stats -- kept out of
/// this function to avoid borrowing state.entities mutably twice.
///
/// NOTE: sound playback (laserhit01.play()) and spawn_particle() calls are
/// left as `// TODO` -- they depend on the audio/particle wiring that's
/// part of the still-unported main loop / render module.
pub fn laser_move(
    state: &mut GameState,
    laser_index: usize,
    enemies_fn: impl Fn(i32, i32) -> bool,
) {
    if state.event_logging {
        println!("Laser.move()");
    }

    if state.lasers[laser_index].lost_sender {
        state.lasers[laser_index].ia = false;
        return;
    }

    let sender_uid = state.lasers[laser_index].sender_uid;
    let sender_uid = match sender_uid {
        Some(u) => u,
        None => {
            state.lasers[laser_index].ia = false;
            return;
        }
    };

    // Self-healing sender lookup (mirrors the C++ index-then-uid-scan pattern).
    let sender_idx = match state.lasers[laser_index]
        .sender_entity_index_hint
        .filter(|&i| i < state.entities.len() && state.entities[i].unique_id == sender_uid)
    {
        Some(i) => i,
        None => match state.find_entity_index_by_uid(sender_uid) {
            Some(i) => i,
            None => {
                let l = &mut state.lasers[laser_index];
                l.ia = false;
                l.sender_uid = None;
                l.sender_entity_index_hint = None;
                return;
            }
        },
    };
    state.lasers[laser_index].sender_entity_index_hint = Some(sender_idx);

    let ratio = state.game_fps_ratio;
    {
        let l = &mut state.lasers[laser_index];
        l.life -= ratio;
        if l.life <= 0.0 {
            l.ia = false;
        }
        l.last_pos = l.pos;
        l.pos.x += l.vel.x * ratio;
        l.pos.y += l.vel.y * ratio;
    }

    let (pos, last_pos, team, dam) = {
        let l = &state.lasers[laser_index];
        (l.pos, l.last_pos, l.team, l.dam)
    };

    let sender_entity_type = state.entities[sender_idx].entity_type;
    let forts_attack_forts = state.config.rules.forts_attack_forts;

    let mut collision_found = false;
    let mut kill_events: Vec<(usize, usize)> = Vec::new(); // (sender_idx, killed entity idx)

    for i in 0..state.entities.len() {
        let (e_ia, e_team, e_type, e_hp) = {
            let e = &state.entities[i];
            (e.ia, e.team, e.entity_type, e.hitpoints)
        };
        if e_ia && enemies_fn(team, e_team) {
            if (forts_attack_forts
                || sender_entity_type != EntityType::Fort
                || e_type == EntityType::Tank)
                && e_hp > 0.0
            {
                let rect = state.entities[i].collision_rect();
                if check_line_collision(rect, pos, last_pos) {
                    collision_found = true;
                    state.entities[i].hitpoints -= dam;
                    state.entities[i].last_bullet_team = team;

                    if state.entities[i].hitpoints <= 0.0 && e_type == EntityType::Tank {
                        state.entities[sender_idx].kill_count += 1;
                        kill_events.push((sender_idx, i));
                    }
                }
            }
        }
    }

    if collision_found {
        state.lasers[laser_index].ia = false;
        let (pos, col) = {
            let l = &state.lasers[laser_index];
            (l.pos, l.col)
        };
        if state.game_screen_rect.contains(pos) {
            state.sound_queue.push(crate::assets::SoundEffect::LaserHit);
            for _ in 0..4 {
                crate::spawn::spawn_particle(state, pos, 1.0, 4.0, 1, 0.5, col);
            }
        }
    }

    // Kill handling: log line, kill_count (already applied above), PvP
    // stats, and floating text ("+1 Kill" / "Veteren" / "KillStreak!").
    for (killer_idx, victim_idx) in kill_events {
        let (killer_team, killer_type, killer_uid, killer_pos) = {
            let k = &state.entities[killer_idx];
            (k.team, k.entity_type, k.unique_id, k.pos)
        };
        let (victim_team, victim_type, victim_uid, victim_pos) = {
            let v = &state.entities[victim_idx];
            (v.team, v.entity_type, v.unique_id, v.pos)
        };

        state.game_log_arr.push(format!(
            "{{{}}}[{}#{}] killed {{{}}}[{}#{}]",
            state.hex_color_arr[killer_team as usize],
            entity_type_string(killer_type),
            killer_uid,
            state.hex_color_arr[victim_team as usize],
            entity_type_string(victim_type),
            victim_uid,
        ));

        if killer_type == EntityType::Tank {
            state.pvp_tanks_killed[killer_team as usize] += 1;
            state.pvp_tanks_lost[victim_team as usize] += 1;

            let killer_col = state.color_arr[killer_team as usize];
            let outline_col = dim_color(opposite_color(killer_col), 25.0);

            if state.game_screen_rect.contains(victim_pos) {
                crate::spawn::spawn_floating_text(
                    state,
                    "+1 Kill".to_string(),
                    victim_pos,
                    killer_col,
                    outline_col,
                    18,
                    1.0,
                    crate::floating_text::FontChoice::Font2,
                    0.25,
                );
            }
            if state.game_screen_rect.contains(killer_pos) {
                let (kill_count, kill_count_threshold, special_tank_killcount) = {
                    let k = &state.entities[killer_idx];
                    (
                        k.kill_count,
                        k.kill_count_threshold,
                        state.config.entity_values.special_tank_killcount,
                    )
                };
                if special_tank_killcount != -1 {
                    if kill_count == special_tank_killcount {
                        crate::spawn::spawn_floating_text(
                            state,
                            "Veteren".to_string(),
                            killer_pos,
                            outline_col,
                            Color::BLACK,
                            20,
                            2.5,
                            crate::floating_text::FontChoice::Font2,
                            0.25,
                        );
                    } else if kill_count_threshold != 0
                        // DEVIATION: added `kill_count_threshold != 0` guard.
                        // C++'s `sender->kill_count % sender->kill_count_threshold`
                        // has no such check and is UB (crash) if the threshold is
                        // ever 0; in practice `kill_count_threshold` is always
                        // `special_tank_killcount` from EntityValues.json, which
                        // is non-zero in the shipped config, but Rust's `%`
                        // panics on a zero divisor where C++ would just corrupt
                        // state or crash less predictably, so this guard avoids
                        // an avoidable panic without changing behavior for any
                        // config value that would have worked in C++ anyway.
                        && kill_count % kill_count_threshold == 0
                        && kill_count > special_tank_killcount
                    {
                        crate::spawn::spawn_floating_text(
                            state,
                            "KillStreak!".to_string(),
                            killer_pos,
                            outline_col,
                            Color::BLACK,
                            20,
                            2.5,
                            crate::floating_text::FontChoice::Font2,
                            0.25,
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------
// Stage D.4: Laser::draw(), ported from TankAttack.cpp L1275-1299.
// ---------------------------------------------------------------------

use sfml::graphics::RenderTarget;

/// C++: void Laser::draw() (TankAttack.cpp L1275-1299).
pub fn laser_draw<T: RenderTarget>(state: &mut GameState, target: &mut T, idx: usize) {
    if state.event_logging {
        println!("Laser.draw()");
    }

    let camera = Vector2f::new(state.camera_pos[0], state.camera_pos[1]);
    let (pos, vel, size1, life, max_life, mut actual_col) = {
        let l = &state.lasers[idx];
        (
            l.pos,
            l.vel,
            l.size.get(1).copied().unwrap_or(0.0),
            l.life,
            l.max_life,
            l.actual_col,
        )
    };
    let g_pos = pos - camera;

    let mut ratio = life * 2.0 * 255.0 / max_life;
    ratio = ratio.clamp(0.0, 255.0);
    actual_col.a = ratio as u8;
    state.lasers[idx].actual_col = actual_col;

    let size0 = state.lasers[idx].size.first().copied().unwrap_or(0.0);
    crate::render::draw_thick_line(target, g_pos, g_pos - vel * size1, size0, actual_col);

    if state.event_logging {
        println!("  Laser Drawn");
    }

    // Draw particles (chance depends on sender type).
    let (sender_entity_type, pos, col) = {
        let l = &state.lasers[idx];
        (l.sender_entity_type, l.pos, l.col)
    };
    match sender_entity_type {
        crate::entity::EntityType::Tank => {
            if state.rand_range(0..100) as f32 <= 20.0 * state.game_fps_ratio {
                crate::spawn::spawn_particle(state, pos, 2.0, 0.1, 2, 1.0, col);
            }
        }
        crate::entity::EntityType::Fort => {
            if state.rand_range(0..100) as f32 <= 33.0 * state.game_fps_ratio {
                crate::spawn::spawn_particle(state, pos, 2.0, 0.1, 2, 1.0, col);
            }
        }
        crate::entity::EntityType::Null => {}
    }
}
