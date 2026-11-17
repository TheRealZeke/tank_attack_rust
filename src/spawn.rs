//! Global spawn functions, ported from the free `spawn_*` functions in
//! TankAttack.cpp (~L5137-5241). Each allocates/pushes into the relevant
//! `GameState` Vec and delegates to the struct-level spawn logic already
//! written in Stages 1-6 (`laser_spawn`, `missile_spawn`, `Particle::spawn`,
//! `DeadTank::spawn`, ...).
//!
//! Only `spawn_tank`, `spawn_fort`, `spawn_particle`, `spawn_button` are
//! wired up so far (needed by Stage A.1's `entity_spawn` test harness and
//! Stage B). `spawn_laser`/`spawn_missile`/`spawn_dead_tank`/
//! `spawn_floating_text`/`spawn_game_text` are included per continuation
//! spec §0.2 since they're small and self-contained, even though nothing
//! calls them yet (Entity::AI in Stage A.3 will).

use crate::button::Button;
use crate::dead_tank::DeadTank;
use crate::entity::{entity_spawn, Entity, EntityType};
use crate::floating_text::{FloatingText, FontChoice};
use crate::game_display_text::GameDisplayText;
use crate::laser::{laser_spawn, Laser};
use crate::missile::{missile_spawn, Missile};
use crate::particle::Particle;
use crate::state::GameState;
use sfml::graphics::Color;
use sfml::system::Vector2f;

/// C++: void spawn_tank(sf::Vector2f pos, int team, int level, bool no_ammo)
/// (L5137-5155).
pub fn spawn_tank(state: &mut GameState, pos: Vector2f, team: i32, level: i32, no_ammo: bool) {
    if state.event_logging {
        println!("\n/SPAWNTANK()/");
    }

    let ev = &state.config.entity_values;
    let (
        tank_laser_cooldown,
        tank_max_ammo,
        tank_supplyammo_ticks,
        special_tank_age,
        special_tank_killcount,
        fort_gun_length,
        fort_gun_width,
        fort_laser_cooldown,
        fort_missle_cooldown_factor,
        fort_spawntank_time,
        special_fort_tanks_healed,
        special_fort_tanks_spawned,
    ) = (
        ev.tank_laser_cooldown,
        ev.tank_max_ammo,
        ev.tank_supplyammo_ticks,
        ev.special_tank_age,
        ev.special_tank_killcount,
        ev.fort_gun_length,
        ev.fort_gun_width,
        ev.fort_laser_cooldown,
        ev.fort_missle_cooldown_factor,
        ev.fort_spawntank_time,
        ev.special_fort_tanks_healed,
        ev.special_fort_tanks_spawned,
    );

    use rand::Rng;
    let mut rng = state.rng.clone();
    let mut rng_calls = 0u64;
    let new_tank = Entity::new(
        EntityType::Tank,
        tank_laser_cooldown,
        tank_max_ammo,
        tank_supplyammo_ticks,
        special_tank_age,
        special_tank_killcount,
        fort_gun_length,
        fort_gun_width,
        fort_laser_cooldown,
        fort_missle_cooldown_factor,
        fort_spawntank_time,
        special_fort_tanks_healed,
        special_fort_tanks_spawned,
        |n| {
            rng_calls += 1;
            rng.gen_range(0..n)
        },
    );
    state.rng = rng;
    state.rng_calls += rng_calls;

    let idx = state.entities.len();
    state.entities.push(new_tank);

    let (tw, th) = state.tank_texture_size;
    entity_spawn(state, idx, pos, team, tw, th);

    state.entities[idx].level = level;
    if no_ammo {
        state.entities[idx].no_ammo = true;
        state.entities[idx].ammo_supply = 0;
        state.entities[idx].hitpoints = 1.0;
    }
    // C++ re-derives entity_index via a search loop after push_back; since
    // Rust's push always lands at `idx`, the search is redundant and we
    // assign directly.
    state.entities[idx].entity_index = idx as i32;

    if state.event_logging {
        println!("  SPAWNTANK() executed");
    }
}

/// C++: void spawn_fort(sf::Vector2f pos, int team, bool captured, bool special_fort)
/// (L5157-5178).
pub fn spawn_fort(
    state: &mut GameState,
    pos: Vector2f,
    team: i32,
    captured: bool,
    special_fort: bool,
) {
    if state.event_logging {
        println!("\n/SPAWNFORT()/");
    }

    let ev = &state.config.entity_values;
    let (
        tank_laser_cooldown,
        tank_max_ammo,
        tank_supplyammo_ticks,
        special_tank_age,
        special_tank_killcount,
        fort_gun_length,
        fort_gun_width,
        fort_laser_cooldown,
        fort_missle_cooldown_factor,
        fort_spawntank_time,
        special_fort_tanks_healed,
        special_fort_tanks_spawned,
        fort_assimilation_ticks,
    ) = (
        ev.tank_laser_cooldown,
        ev.tank_max_ammo,
        ev.tank_supplyammo_ticks,
        ev.special_tank_age,
        ev.special_tank_killcount,
        ev.fort_gun_length,
        ev.fort_gun_width,
        ev.fort_laser_cooldown,
        ev.fort_missle_cooldown_factor,
        ev.fort_spawntank_time,
        ev.special_fort_tanks_healed,
        ev.special_fort_tanks_spawned,
        ev.fort_assimilation_ticks,
    );

    use rand::Rng;
    let mut rng = state.rng.clone();
    let mut rng_calls = 0u64;
    let new_fort = Entity::new(
        EntityType::Fort,
        tank_laser_cooldown,
        tank_max_ammo,
        tank_supplyammo_ticks,
        special_tank_age,
        special_tank_killcount,
        fort_gun_length,
        fort_gun_width,
        fort_laser_cooldown,
        fort_missle_cooldown_factor,
        fort_spawntank_time,
        special_fort_tanks_healed,
        special_fort_tanks_spawned,
        |n| {
            rng_calls += 1;
            rng.gen_range(0..n)
        },
    );
    state.rng = rng;
    state.rng_calls += rng_calls;

    let idx = state.entities.len();
    state.entities.push(new_fort);

    let (fw, fh) = state.fort_texture_size;
    entity_spawn(state, idx, pos, team, fw, fh);

    if !captured {
        state.entities[idx].tank_spawn_timer = 0.0;
    } else {
        state.entities[idx].tank_spawn_timer = -(fort_assimilation_ticks as f32);
    }

    if special_fort {
        let healed_threshold = state.entities[idx].fort_tanks_healed_threshold;
        let spawned_threshold = state.entities[idx].fort_tanks_spawned_threshold;
        state.entities[idx].fort_tanks_healed = healed_threshold;
        state.entities[idx].fort_tanks_spawned = spawned_threshold;
        state.entities[idx].special_fort = true;
    }

    state.entities[idx].entity_index = idx as i32;

    if state.event_logging {
        println!("\n  SPAWNFORT() executed");
    }
}

/// C++: void spawn_particle(...) (L5180-5192).
pub fn spawn_particle(
    state: &mut GameState,
    pos: Vector2f,
    size: f32,
    range: f32,
    kind: i32,
    dur_rate: f32,
    col: Color,
) {
    if !state.game_screen_rect.contains(pos) || state.paused {
        return;
    }
    use rand::Rng;
    let mut rng = state.rng.clone();
    let mut rng_calls = 0u64;
    let mut new_part = Particle::new();
    new_part.spawn(
        pos,
        size,
        range,
        kind,
        dur_rate,
        col,
        state.config.graphics.default_particle_ticks as f32,
        |n| {
            rng_calls += 1;
            rng.gen_range(0..n)
        },
    );
    state.rng = rng;
    state.rng_calls += rng_calls;
    state.particles.push(new_part);
}

/// C++: void spawn_laser(...) (L5194-5203).
#[allow(clippy::too_many_arguments)]
pub fn spawn_laser(
    state: &mut GameState,
    posx: f32,
    posy: f32,
    vel: f32,
    ang: f32,
    dam: f32,
    team: i32,
    sender_uid: u32,
    size: Vec<f32>,
) {
    if state.event_logging {
        println!("/SPAWNLASER()/");
    }
    let idx = state.lasers.len();
    state.lasers.push(Laser::new());
    let spawned = laser_spawn(
        state,
        idx,
        Vector2f::new(posx, posy),
        vel,
        ang,
        dam,
        team,
        sender_uid,
        size,
        // C++'s dim_color percentage for `actual_col` isn't specified at
        // the spawn_laser call site in the excerpt this port has seen;
        // defaults to 100 (no dimming) until the exact call-site value
        // (from Entity::AI's laser-shoot block, Stage A.3) is ported.
        100.0,
    );
    if !spawned {
        state.lasers.remove(idx);
    }
    if state.event_logging {
        println!("  SPAWNLASER() executed");
    }
}

/// C++: void spawn_missle(...) (L5205-5224).
#[allow(clippy::too_many_arguments)]
pub fn spawn_missile(
    state: &mut GameState,
    pos: Vector2f,
    vel: f32,
    ang: f32,
    dam: f32,
    no_of_proj: i32,
    team: i32,
    sender_uid: u32,
    target_uid: u32,
    size: f32,
    trail_length: i32,
    splash_radius: f32,
    terminal_vel: f32,
    power: f32,
) {
    if state.event_logging {
        println!("/SPAWNMISSLE()/");
    }
    let rad = (ang as f64) * std::f64::consts::PI / 180.0;
    let t_vel = Vector2f::new((rad.cos() as f32) * vel, (rad.sin() as f32) * vel);

    let idx = state.missiles.len();
    state.missiles.push(Missile::new());
    missile_spawn(
        state,
        idx,
        pos,
        t_vel,
        dam,
        no_of_proj,
        team,
        sender_uid,
        target_uid,
        size,
        trail_length,
        splash_radius,
        terminal_vel,
        power,
    );
    if state.event_logging {
        println!("  SPAWNMISSLE() executed");
    }
}

/// C++: void spawn_game_text(...) (L5226-5229).
pub fn spawn_game_text(
    state: &mut GameState,
    text: String,
    pos: Vector2f,
    col: Color,
    size: i32,
    dur: f32,
    thickness: f32,
    shadow: bool,
) {
    state.game_texts.push(GameDisplayText::new(
        text, pos, col, size, dur, thickness, shadow,
    ));
}

/// C++: void spawn_dead_tank(sf::Vector2f pos, float ang, int team) (L5231-5235).
pub fn spawn_dead_tank(state: &mut GameState, pos: Vector2f, ang: f32, team: i32) {
    use rand::Rng;
    let mut rng = state.rng.clone();
    let mut rng_calls = 0u64;
    let mut new_dt = DeadTank::new(|n| {
        rng_calls += 1;
        rng.gen_range(0..n)
    });
    state.rng = rng;
    state.rng_calls += rng_calls;
    let dead_tank_life = state.config.graphics.dead_tank_ticks as f32;
    new_dt.spawn(pos, ang, team, dead_tank_life, &state.color_arr);
    state.dead_tanks.push(new_dt);
}

/// C++: void spawn_button(...) (L5237-5240).
pub fn spawn_button(
    state: &mut GameState,
    pos: Vector2f,
    size: Vector2f,
    text: String,
    colour: Color,
    action: i32,
    text_size: i32,
) {
    state
        .buttons
        .push(Button::new(pos, size, text, colour, action, text_size));
}

/// C++: void spawn_floating_text(...) (L5243-5246).
#[allow(clippy::too_many_arguments)]
pub fn spawn_floating_text(
    state: &mut GameState,
    text: String,
    pos: Vector2f,
    col: Color,
    outline_col: Color,
    size: i32,
    dur: f32,
    font: FontChoice,
    vel: f32,
) {
    state.floating_texts.push(FloatingText::new(
        text,
        pos,
        col,
        outline_col,
        size,
        dur,
        font,
        vel,
    ));
}
