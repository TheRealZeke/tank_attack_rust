//! Entity: tanks and forts. Ported from `class Entity` (TankAttack.cpp ~L551-1053)
//! and `Entity::AI()` / `Entity::move()` / `Entity::draw()` (L1676-2955).
//!
//! Pointer members (`enemy`, `ammo_target`, `last_bullet_sender`) become
//! `Option<u32>` unique-id handles per spec §4.3. Dereferencing is done via
//! `GameState::find_entity_by_uid` at the call site; on a failed lookup the
//! handle is cleared, mirroring the C++ "self-healing" index/uid checks.

use crate::floating_text::FontChoice;
use crate::state::GameState;
use crate::util::{delta_angle, enemies, intercept_point, opposite_color};
use sfml::graphics::{Color, FloatRect};
use sfml::system::Vector2f;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityType {
    Tank,
    Fort,
    Null,
}

/// C++: std::string entity_type_string(EntityType type)
pub fn entity_type_string(t: EntityType) -> &'static str {
    match t {
        EntityType::Tank => "tank",
        EntityType::Fort => "fort",
        EntityType::Null => "null",
    }
}

pub struct Entity {
    // --- handles, replacing raw Entity* members ---
    pub ammo_target_uid: Option<u32>,
    pub enemy_uid: Option<u32>,
    pub last_bullet_sender_uid: Option<u32>,
    /// cached index hints (like C++'s *_entity_index members); always
    /// re-validated against the uid before use, never trusted alone.
    pub ammo_target_index_hint: Option<usize>,
    pub enemy_index_hint: Option<usize>,

    pub ia: bool,
    pub no_ammo: bool,
    pub special_fort: bool,
    pub special_tank: bool,
    pub valid_target: bool,

    pub tank_spawn_timer: f32,
    pub ang_acc: f32,
    pub age: f32,
    pub ammo_supply_timer: f32,
    pub ang: f32,
    pub dam: f32,
    pub flicker_timer: f32,
    pub flicker_vel: f32,
    pub fort_ang: f32,
    pub fort_assimilation_arc_angle: f32,
    pub fort_health_arc_angle: f32,
    pub fort_ang_vel: f32,
    pub hitpoints: f32,
    pub laser_cooldown: f32,
    pub lock_target_timer: f32,
    pub max_tank_spawn_time: f32,
    pub max_ammo_supply: f32,
    pub max_ammo_supply_time: f32,
    pub max_flicker_time: f32,
    pub max_hitpoints: f32,
    pub max_laser_cooldown: f32,
    pub max_lock_target_time: f32,
    pub max_missle_cooldown: f32,
    pub missle_cooldown: f32,
    pub old_age: f32,
    pub range: f32,
    pub size: f32,
    pub speed_factor: f32,
    pub tank_health_arc_angle: f32,
    pub targ_ang: f32,
    pub ang_vel: f32,

    pub ammo_supply: i32,
    pub chunk: [i32; 2],
    pub entity_index: i32,
    pub fort_level: i32,
    pub fort_tanks_healed: i32,
    pub fort_tanks_healed_threshold: i32,
    pub fort_tanks_spawned: i32,
    pub fort_tanks_spawned_threshold: i32,
    pub gun_barrel: [i32; 2],
    pub kill_count: i32,
    pub kill_count_threshold: i32,
    pub last_bullet_team: i32,
    pub level: i32,
    pub team: i32,
    pub tex_height: i32,
    pub tex_width: i32,
    pub tilt_preference: i32,
    pub unique_id: u32,

    pub col: Color,
    pub opp_col: Color,

    pub rect: FloatRect,

    pub acc: Vector2f,
    pub pos: Vector2f,
    pub target_pos: Vector2f,
    pub vel: Vector2f,

    pub entity_type: EntityType,

    pub heal_tank_array: Vec<i32>,
}

impl Entity {
    /// C++: Entity(const EntityType _entity_type)
    /// `rand_u32`/`rand_360` are injected so callers control RNG (spec §6.7).
    pub fn new(
        entity_type: EntityType,
        // config-derived defaults, passed in rather than read from globals
        tank_laser_cooldown: i32,
        tank_max_ammo: i32,
        tank_supplyammo_ticks: i32,
        special_tank_age: i32,
        special_tank_killcount: i32,
        fort_gun_length: i32,
        fort_gun_width: i32,
        fort_laser_cooldown: i32,
        fort_missle_cooldown_factor: f32,
        fort_spawntank_time: i32,
        special_fort_tanks_healed: i32,
        special_fort_tanks_spawned: i32,
        mut rand_fn: impl FnMut(u32) -> u32,
    ) -> Self {
        let mut e = Entity {
            ammo_target_uid: None,
            enemy_uid: None,
            last_bullet_sender_uid: None,
            ammo_target_index_hint: None,
            enemy_index_hint: None,
            ia: false,
            no_ammo: false,
            special_fort: false,
            special_tank: false,
            valid_target: false,
            tank_spawn_timer: 0.0,
            ang_acc: 0.0,
            age: 0.0,
            ammo_supply_timer: 0.0,
            ang: 0.0,
            dam: 0.0,
            flicker_timer: 0.0,
            flicker_vel: 0.0,
            fort_ang: 0.0,
            fort_assimilation_arc_angle: 0.0,
            fort_health_arc_angle: 0.0,
            fort_ang_vel: 0.0,
            hitpoints: 0.0,
            laser_cooldown: 0.0,
            lock_target_timer: 0.0,
            max_tank_spawn_time: 0.0,
            max_ammo_supply: 0.0,
            max_ammo_supply_time: 0.0,
            max_flicker_time: 0.0,
            max_hitpoints: 0.0,
            max_laser_cooldown: 0.0,
            max_lock_target_time: 0.0,
            max_missle_cooldown: 0.0,
            missle_cooldown: 0.0,
            old_age: 0.0,
            range: 0.0,
            size: 0.0,
            speed_factor: 1.0,
            tank_health_arc_angle: 0.0,
            targ_ang: 0.0,
            ang_vel: 0.0,
            ammo_supply: 0,
            chunk: [0, 0],
            entity_index: -1,
            fort_level: 0,
            fort_tanks_healed: 0,
            fort_tanks_healed_threshold: 0,
            fort_tanks_spawned: 0,
            fort_tanks_spawned_threshold: 0,
            gun_barrel: [0, 0],
            kill_count: 0,
            kill_count_threshold: 0,
            last_bullet_team: -1,
            level: 0,
            team: -1,
            tex_height: 0,
            tex_width: 0,
            tilt_preference: 0,
            unique_id: 0,
            col: Color::rgba(0, 0, 0, 0),
            opp_col: Color::rgba(0, 0, 0, 0),
            rect: FloatRect::new(0.0, 0.0, 0.0, 0.0),
            acc: Vector2f::new(0.0, 0.0),
            pos: Vector2f::new(0.0, 0.0),
            target_pos: Vector2f::new(0.0, 0.0),
            vel: Vector2f::new(0.0, 0.0),
            entity_type,
            heal_tank_array: Vec::new(),
        };

        match entity_type {
            EntityType::Tank => {
                e.max_laser_cooldown = tank_laser_cooldown as f32;
                e.max_ammo_supply = tank_max_ammo as f32;
                e.ammo_supply = e.max_ammo_supply as i32;
                e.max_ammo_supply_time = tank_supplyammo_ticks as f32;
                e.old_age = special_tank_age as f32;
                e.kill_count_threshold = special_tank_killcount;
            }
            EntityType::Fort => {
                e.gun_barrel[0] = fort_gun_length;
                e.gun_barrel[1] = fort_gun_width;

                e.ang = (rand_fn(360)) as f32;
                e.fort_ang = (rand_fn(360)) as f32;
                let mut fort_ang_vel = (rand_fn(4) + 1) as f32 / 4.0;
                fort_ang_vel = if rand_fn(2) == 0 {
                    fort_ang_vel
                } else {
                    -fort_ang_vel
                };
                e.fort_ang_vel = fort_ang_vel;
                e.max_laser_cooldown = fort_laser_cooldown as f32;
                e.max_missle_cooldown = e.max_laser_cooldown * 33.0 * fort_missle_cooldown_factor;
                e.max_tank_spawn_time = fort_spawntank_time as f32;
                e.max_flicker_time = 120.0;
                e.flicker_vel = 1.0;
                e.flicker_timer = rand_fn(e.max_flicker_time as u32) as f32;
                e.fort_tanks_healed_threshold = special_fort_tanks_healed;
                e.fort_tanks_spawned_threshold = special_fort_tanks_spawned;
            }
            EntityType::Null => {}
        }
        e
    }

    /// C++: void tilt(int dirc)
    pub fn tilt(&mut self, dirc: i32) {
        match self.entity_type {
            EntityType::Tank => self.ang_acc = 0.3 * dirc as f32,
            EntityType::Fort => self.ang_acc = 0.1 * dirc as f32,
            EntityType::Null => {}
        }
    }

    /// C++: void accel(float dirc)
    pub fn accel(&mut self, dirc: f32, tank_speed_factor: f32) {
        if self.entity_type == EntityType::Tank {
            let rad = std::f32::consts::PI * self.ang / 180.0;
            self.acc = Vector2f::new(
                rad.cos() * 0.075 * dirc * tank_speed_factor,
                rad.sin() * 0.075 * dirc * tank_speed_factor,
            );
        }
    }

    /// C++: void deccel()
    pub fn deccel(&mut self, game_fps_ratio: f32) {
        if self.entity_type == EntityType::Tank {
            self.acc = Vector2f::new(0.0, 0.0);
            let factor = 0.99f32.powf(game_fps_ratio);
            self.vel.x *= factor;
            self.vel.y *= factor;
        }
    }

    /// C++: void deccelAng()
    pub fn deccel_ang(&mut self, game_fps_ratio: f32) {
        if self.entity_type == EntityType::Tank {
            self.ang_acc = 0.0;
            self.ang_vel *= 0.9f32.powf(game_fps_ratio);
        }
    }

    /// C++: sf::FloatRect collisionRect()
    pub fn collision_rect(&self) -> FloatRect {
        match self.entity_type {
            EntityType::Tank => {
                let radians = ((self.ang - 90.0) as f64) * std::f64::consts::PI / 180.0;
                let width = (self.tex_width as f64 * radians.cos()).abs()
                    + (self.tex_height as f64 * radians.sin()).abs();
                let height = (self.tex_width as f64 * radians.sin()).abs()
                    + (self.tex_height as f64 * radians.cos()).abs();
                FloatRect::new(
                    self.pos.x - (width as f32) / 2.0,
                    self.pos.y - (height as f32) / 2.0,
                    width as f32,
                    height as f32,
                )
            }
            EntityType::Fort => self.rect,
            EntityType::Null => FloatRect::new(0.0, 0.0, 0.0, 0.0),
        }
    }

    /// C++: float get_fort_territory_power()
    pub fn get_fort_territory_power(
        &self,
        assimilating_fort_territory_power: f32,
        special_fort_territory_power: f32,
    ) -> f32 {
        debug_assert_eq!(self.entity_type, EntityType::Fort);
        if self.tank_spawn_timer < 0.0 {
            assimilating_fort_territory_power
        } else if self.special_fort || self.fort_level != 0 {
            special_fort_territory_power
        } else {
            1.0
        }
    }

    /// C++: bool isAssimilating()
    pub fn is_assimilating(&self) -> bool {
        self.entity_type == EntityType::Fort && self.tank_spawn_timer < 0.0
    }

    /// C++: void upgradeTank(int lev) -- only lev == 1 is used in the source.
    pub fn upgrade_tank(
        &mut self,
        tank_damage: f32,
        special_tank_damage_factor: f32,
        tank_max_ammo: i32,
        special_tank_ammo_factor: f32,
        tank_hp: f32,
        special_tank_hitpoints_factor: f32,
        special_tank_speed_factor: f32,
        tank_range: i32,
        special_tank_range_factor: f32,
    ) {
        self.dam = tank_damage * special_tank_damage_factor;
        self.max_ammo_supply = tank_max_ammo as f32 * special_tank_ammo_factor;
        let ratio = self.hitpoints / self.max_hitpoints;
        self.max_hitpoints = tank_hp * special_tank_hitpoints_factor;
        self.hitpoints = self.max_hitpoints * ratio;
        self.speed_factor = special_tank_speed_factor;
        self.range = tank_range as f32 * special_tank_range_factor;
        self.level = 1;
    }
}

// ---------------------------------------------------------------------
// draw() is NOT yet ported (Stage D, needs the render module). spawn(),
// lock_target(), move(), and AI() are implemented below as free functions
// taking (state, idx), per the continuation spec's mandatory borrow
// pattern (spec §0.1/§0.3) -- this avoids holding `&mut Entity` and
// `&GameState`/`&mut GameState` simultaneously, which is impossible once
// the entity lives inside `state.entities`.
// ---------------------------------------------------------------------

/// C++: void upgradeTank(int lev) as called from Entity::spawn()/AI(),
/// pulling the relevant config values out of `state.config` for the caller.
pub fn entity_upgrade_tank(state: &mut GameState, idx: usize) {
    let ev = &state.config.entity_values;
    let (
        tank_damage,
        special_tank_damage_factor,
        tank_max_ammo,
        special_tank_ammo_factor,
        tank_hp,
        special_tank_hitpoints_factor,
        special_tank_speed_factor,
        tank_range,
        special_tank_range_factor,
    ) = (
        ev.tank_damage,
        ev.special_tank_damage_factor,
        ev.tank_max_ammo,
        ev.special_tank_ammo_factor,
        ev.tank_hp,
        ev.special_tank_hitpoints_factor,
        ev.special_tank_speed_factor,
        ev.tank_range,
        ev.special_tank_range_factor,
    );
    state.entities[idx].upgrade_tank(
        tank_damage,
        special_tank_damage_factor,
        tank_max_ammo,
        special_tank_ammo_factor,
        tank_hp,
        special_tank_hitpoints_factor,
        special_tank_speed_factor,
        tank_range,
        special_tank_range_factor,
    );
}

/// C++: void Entity::spawn(sf::Vector2f spawn_pos, int spawn_team)
/// (TankAttack.cpp L747-843).
///
/// The C++ builds a local `Entity` and calls `.spawn()` on it *before*
/// pushing it into `EntityArr`; here `state.entities[idx]` is already the
/// freshly-pushed (mostly-default) entity from `Entity::new()`, and this
/// function finishes initializing it in place. This is behaviorally
/// equivalent for the unique-id collision search: C++'s search never
/// compares the new entity against itself either (it isn't in the array
/// yet), and here we explicitly skip index `idx` for the same reason.
///
/// DEVIATION: `tex_width`/`tex_height` are passed in as parameters rather
/// than read directly from `CompTankTextArr[team].getSize()` /
/// `FortTextArr[team].getSize()` -- callers (`spawn_tank`/`spawn_fort`)
/// read `state.tank_texture_size`/`fort_texture_size` instead, which
/// `game_loop::setup` populates from the real loaded textures right
/// after `Assets::load` (every team's texture is the same size, being a
/// recolor of one source image, so this is equivalent to indexing
/// `CompTankTextArr[team]` directly). `entity_sprite.setTexture`
/// is dropped entirely per the Stage 0.5 decision to not store `Sprite`
/// on `Entity` at all.
///
/// DEVIATION: the C++ source's very first statement --
/// `col = colorArr[team]; opp_col = opposite_color(colorArr[team]);` --
/// reads `this->team` while it's still the constructor default (-1) before
/// either branch below overwrites `team` and re-derives `col`/`opp_col`
/// correctly. That's an out-of-bounds `colorArr[-1]` read in C++ that
/// happens to compile (UB, but harmless in practice) and has zero
/// observable effect since both branches unconditionally overwrite `col`
/// immediately after. It is omitted here rather than reproduced as a panic.
///
/// DEVIATION: the tank branch's `sf::Color col = colorArr[team];` right
/// before `kill_count = 0;` declares a *local* variable that shadows the
/// member and is never used -- dead code, omitted.
pub fn entity_spawn(
    state: &mut GameState,
    idx: usize,
    spawn_pos: Vector2f,
    spawn_team: i32,
    tex_width: i32,
    tex_height: i32,
) {
    if state.event_logging {
        println!("  Finding Unique Id");
    }

    // Unique-id search loop (C++ L753-762): reject 0, reject collisions
    // with any *other* existing entity.
    // DEVIATION: capped at 10_000 iterations (defensive; C++ can spin
    // forever on a pathological RNG, Rust shouldn't).
    let mut new_uid: u32;
    let mut attempts = 0u32;
    loop {
        new_uid = state.rand_u32();
        attempts += 1;
        let collision = new_uid == 0
            || state
                .entities
                .iter()
                .enumerate()
                .any(|(i, e)| i != idx && e.unique_id == new_uid);
        if !collision || attempts >= 10_000 {
            break;
        }
    }
    state.entities[idx].unique_id = new_uid;

    if state.event_logging {
        println!("  Entity spawned with uid: {new_uid}");
    }
    state.game_log_arr.push(format!(
        "{{{}}}[{}#{}] spawned",
        state.hex_color_arr[spawn_team as usize],
        entity_type_string(state.entities[idx].entity_type),
        new_uid
    ));

    match state.entities[idx].entity_type {
        EntityType::Tank => {
            let ev = &state.config.entity_values;
            let (tank_hp, tank_damage, tank_range, tank_target_ticks) = (
                ev.tank_hp,
                ev.tank_damage,
                ev.tank_range,
                ev.tank_target_ticks,
            );

            let team = spawn_team;
            let col = state.color_arr[team as usize];
            let opp_col = opposite_color(col);
            let ang = state.rand_range(0..360) as f32;

            let e = &mut state.entities[idx];
            e.team = team;
            e.col = col;
            e.opp_col = opp_col;
            e.ia = true;
            e.pos = spawn_pos;
            e.rect.width = tex_width as f32;
            e.rect.height = tex_height as f32;
            e.rect.left = spawn_pos.x + e.rect.width / 2.0;
            e.rect.top = spawn_pos.y + e.rect.height / 2.0;
            e.tex_width = tex_width;
            e.tex_height = tex_height;
            e.hitpoints = tank_hp;
            e.max_hitpoints = tank_hp;
            e.dam = tank_damage;
            e.range = tank_range as f32;
            e.ang = ang;
            e.lock_target_timer = 0.0;
            e.max_lock_target_time = tank_target_ticks as f32;
            e.enemy_uid = None;
            e.enemy_index_hint = None;
            e.ammo_target_uid = None;
            e.ammo_target_index_hint = None;
            e.kill_count = 0;

            entity_lock_target(state, idx);

            if state.entities[idx].level == 1 {
                entity_upgrade_tank(state, idx);
            }
        }
        EntityType::Fort => {
            let ev = &state.config.entity_values;
            let (fort_hp, fort_range, fort_damage, fort_size) =
                (ev.fort_hp, ev.fort_range, ev.fort_damage, ev.fort_size);

            let team = spawn_team;
            let col = state.color_arr[team as usize];
            let opp_col = opposite_color(col);
            let ang = state.rand_range(0..360) as f32;

            let e = &mut state.entities[idx];
            e.ia = true;
            e.max_hitpoints = fort_hp;
            e.hitpoints = e.max_hitpoints;
            e.pos = spawn_pos;
            e.range = fort_range as f32;
            e.dam = fort_damage;
            e.team = team;
            e.col = col;
            e.opp_col = opp_col;
            e.size = fort_size as f32;
            e.rect = FloatRect::new(
                spawn_pos.x - e.size / 2.0,
                spawn_pos.y - e.size / 2.0,
                e.size,
                e.size,
            );
            e.ang = ang;
            e.tex_width = tex_width;
            e.tex_height = tex_height;
        }
        EntityType::Null => {}
    }

    if state.event_logging {
        println!("  SPAWN() executed");
    }
}

/// C++: void Entity::lock_target() (TankAttack.cpp ~L733-835 per the
/// original spec's extract). Sets `enemy_uid`/`target_pos`/`valid_target`.
pub fn entity_lock_target(state: &mut GameState, idx: usize) {
    let entity_type = state.entities[idx].entity_type;
    let self_team = state.entities[idx].team;
    let self_pos = state.entities[idx].pos;
    let self_level = state.entities[idx].level;
    let gang_up = state.gang_up;
    let victim_team = state.victim_team;

    match entity_type {
        EntityType::Tank => {
            let mut best: Option<(usize, f32)> = None;
            for i in 0..state.entities.len() {
                let (e_ia, e_team, e_type, e_pos) = {
                    let e = &state.entities[i];
                    (e.ia, e.team, e.entity_type, e.pos)
                };
                if i == idx || !e_ia {
                    continue;
                }
                if !enemies(self_team, e_team, gang_up, victim_team) {
                    continue;
                }
                let mut dis = crate::util::displacement(self_pos, e_pos);
                if self_level == 1 && e_type == EntityType::Fort {
                    dis /= 2.0;
                }
                if best.is_none() || dis < best.unwrap().1 {
                    best = Some((i, dis));
                }
            }

            match best {
                Some((i, _)) => {
                    let (uid, pos) = { (state.entities[i].unique_id, state.entities[i].pos) };
                    let e = &mut state.entities[idx];
                    e.enemy_uid = Some(uid);
                    e.enemy_index_hint = Some(i);
                    e.target_pos = pos;
                    e.valid_target = true;
                }
                None => {
                    let e = &mut state.entities[idx];
                    e.enemy_uid = None;
                    e.enemy_index_hint = None;
                    e.valid_target = false;
                }
            }
        }
        EntityType::Fort => {
            if state.config.rules.forts_neutered {
                return;
            }
            if state.entities[idx].tank_spawn_timer <= 0.0 {
                return;
            }
            let self_range = state.entities[idx].range;
            let forts_attack_forts = state.config.rules.forts_attack_forts;
            let fort_aimbot = state.config.rules.fort_aimbot;
            let fort_laser_speed = state.config.entity_values.fort_laser_speed;

            let mut best: Option<(usize, f32)> = None;
            for i in 0..state.entities.len() {
                let (e_ia, e_team, e_type, e_pos) = {
                    let e = &state.entities[i];
                    (e.ia, e.team, e.entity_type, e.pos)
                };
                if i == idx || !e_ia {
                    continue;
                }
                if !enemies(self_team, e_team, gang_up, victim_team) {
                    continue;
                }
                if !forts_attack_forts && e_type == EntityType::Fort {
                    continue;
                }
                let dis = crate::util::displacement(self_pos, e_pos);
                if dis > self_range {
                    continue;
                }
                if best.is_none() || dis < best.unwrap().1 {
                    best = Some((i, dis));
                }
            }

            match best {
                Some((i, _)) => {
                    let (e_uid, e_pos, e_vel) = {
                        let e = &state.entities[i];
                        (e.unique_id, e.pos, e.vel)
                    };
                    let target_pos = if fort_aimbot {
                        intercept_point(self_pos, e_pos, e_vel, fort_laser_speed)
                    } else {
                        e_pos
                    };
                    let e = &mut state.entities[idx];
                    e.enemy_uid = Some(e_uid);
                    e.enemy_index_hint = Some(i);
                    e.target_pos = target_pos;
                    e.valid_target = true;
                }
                None => {
                    let e = &mut state.entities[idx];
                    e.enemy_uid = None;
                    e.enemy_index_hint = None;
                    e.valid_target = false;
                }
            }
        }
        EntityType::Null => {}
    }
}

/// C++: void Entity::move() (TankAttack.cpp L2750-2795). Fort branch is
/// empty in the original source -- reproduced as a no-op, not an omission.
pub fn entity_move(state: &mut GameState, idx: usize) {
    if state.entities[idx].entity_type != EntityType::Tank {
        return;
    }

    let fps = state.game_fps_ratio;

    state.entities[idx].lock_target_timer += fps;
    if state.entities[idx].lock_target_timer >= state.entities[idx].max_lock_target_time {
        state.entities[idx].lock_target_timer = 0.0;
        if !state.entities[idx].no_ammo {
            entity_lock_target(state, idx);
        }
    }

    let needs_new_target = match state.entities[idx].enemy_uid {
        None => true,
        Some(uid) => match state.find_entity_index_by_uid(uid) {
            Some(i) => !state.entities[i].ia,
            None => true,
        },
    };
    if needs_new_target {
        entity_lock_target(state, idx);
    }

    let e = &mut state.entities[idx];
    // Tank rotation.
    e.ang_vel += e.ang_acc * fps;
    e.ang += e.ang_vel * fps;
    e.ang_vel *= 0.92f32.powf(fps);
    e.ang_acc = 0.0;

    // Motion.
    e.vel.x += e.acc.x * fps;
    e.vel.y += e.acc.y * fps;
    e.pos.x += e.vel.x * fps;
    e.pos.y += e.vel.y * fps;
    e.vel.x *= 0.9f32.powf(fps);
    e.vel.y *= 0.9f32.powf(fps);

    // Update chunks.
    e.chunk[0] = (e.pos.x / 16.0) as i32;
    e.chunk[1] = (e.pos.y / 16.0) as i32;
}

/// C++: void Entity::AI() (TankAttack.cpp L1676-2749). The single largest
/// function in the game; tank and fort branches are almost entirely
/// separate logic sharing only the entity_index self-realignment preamble.
///
/// DEVIATION: the C++ preamble (L1683-1710) re-derives `entity_index` by
/// scanning `EntityArr` for `unique_id` if the cached index looks stale --
/// a safety net for C++'s pointer/`this`-validity concerns after vector
/// reallocation. It's inapplicable here: this port always calls
/// `entity_ai(state, idx)` with `idx` freshly supplied by the caller's
/// `0..state.entities.len()` loop, so `state.entities[idx]` is always the
/// entity being processed by construction. Omitted entirely.
///
/// DEVIATION: several C++ blocks exist purely to keep a cached
/// `*_entity_index` in sync with a `*_unique_id` so a raw pointer stays
/// dereferenceable (the "If Invalid Pointer" / "misaligned" ceremony for
/// `enemy` and `ammo_target`). This port stores `Option<u32>` uids and
/// resolves them via `GameState::find_entity_index_by_uid` fresh at each
/// point of use instead of maintaining a cache that needs revalidating --
/// behaviorally identical (same "does a live, correctly-teamed entity with
/// this uid still exist?" question, answered on demand rather than
/// speculatively). These blocks are omitted at the top of each branch;
/// the equivalent check happens inline where the value is actually used.
/// None of the omitted blocks contain `rand()` calls, so this has no
/// effect on RNG draw order/count.
pub fn entity_ai(state: &mut GameState, idx: usize) {
    if state.event_logging {
        println!("Entity::AI() called");
    }

    let entity_type = state.entities[idx].entity_type;

    if entity_type == EntityType::Tank {
        entity_ai_tank(state, idx);
    } else if entity_type == EntityType::Fort {
        entity_ai_fort(state, idx);
    }
}

fn entity_ai_tank(state: &mut GameState, idx: usize) {
    use crate::spawn::{spawn_dead_tank, spawn_laser, spawn_missile, spawn_particle};

    if state.entities[idx].level == 1 {
        entity_upgrade_tank(state, idx);
    }

    if state.event_logging {
        println!(
            "\nTank.AI(); GameTick: {}\n  Tank uid: {}  Tank Pos: {},{}\n  lock_target Timer: {}; no_ammo: {}",
            state.game_ticks,
            state.entities[idx].unique_id,
            state.entities[idx].pos.x,
            state.entities[idx].pos.y,
            state.entities[idx].lock_target_timer,
            state.entities[idx].no_ammo
        );
    }

    state.entities[idx].age += state.game_fps_ratio;

    // Tanks with Kills get Perks.
    {
        let e = &state.entities[idx];
        let earned_levelup = ((e.kill_count >= e.kill_count_threshold
            && e.kill_count_threshold != -1)
            || (e.age >= e.old_age && e.old_age != -1.0))
            && e.level == 0;
        if earned_levelup && e.level == 0 && state.config.rules.special_tanks {
            entity_upgrade_tank(state, idx);
        }
    }

    // Special-tank (level 1) passive healing.
    if state.entities[idx].level != 0 && state.config.rules.special_tanks {
        let ev = &state.config.entity_values;
        match state.entities[idx].level {
            1 => {
                if !state.entities[idx].no_ammo {
                    let heal = state.entities[idx].max_hitpoints
                        * state.game_fps_ratio
                        * ev.special_tank_healing_factor
                        / 100.0
                        / 60.0;
                    state.entities[idx].hitpoints += heal;
                }
                if state.entities[idx].hitpoints > state.entities[idx].max_hitpoints {
                    state.entities[idx].hitpoints = state.entities[idx].max_hitpoints;
                }
            }
            _ => {}
        }
    }

    // Check Tank Ammo situation.
    {
        let e = &state.entities[idx];
        let critical = (e.ammo_supply <= 0
            || e.hitpoints <= e.max_hitpoints * state.config.entity_values.panic_health_perc)
            && state.config.rules.need_supplies;
        if critical {
            if !state.entities[idx].no_ammo {
                let (pos, col) = (state.entities[idx].pos, state.entities[idx].col);
                for _ in 0..10 {
                    spawn_particle(state, pos, 4.0, 2.0, 2, 0.5, col);
                }
            }
            state.entities[idx].no_ammo = true;
        }
    }

    // Tank Death Conditions.
    if state.entities[idx].hitpoints <= 0.0 {
        state.entities[idx].ia = false;

        let (pos, ang, team, col) = {
            let e = &state.entities[idx];
            (e.pos, e.ang, e.team, e.col)
        };
        spawn_dead_tank(state, pos, ang, team);
        for _ in 0..10 {
            spawn_particle(state, pos, 8.0, 0.5, 2, 1.5, col);
            for _ in 0..2 {
                spawn_particle(state, pos, 3.0, 1.2, 1, 0.8, col);
                spawn_particle(state, pos, 3.0, 2.0, 1, 0.8, opposite_color(col));
            }
        }

        // C++: coin-flip between tankdies01/02, gated on GameScreenRect.
        if state.game_screen_rect.contains(pos) {
            let pick = state.rand_range(0..2);
            state.sound_queue.push(if pick == 0 {
                crate::assets::SoundEffect::TankDies01
            } else {
                crate::assets::SoundEffect::TankDies02
            });
        }

        let last_bullet_team = state.entities[idx].last_bullet_team;
        state.tanks_lost[team as usize] += 1;
        if last_bullet_team >= 0 {
            state.tanks_killed[last_bullet_team as usize] += 1;
        }
    }

    if state.entities[idx].laser_cooldown > 0.0 {
        state.entities[idx].laser_cooldown -= state.game_fps_ratio;
    }

    // Calculate Tank Hitbox.
    {
        let e = &mut state.entities[idx];
        e.rect.left = e.pos.x - e.rect.width / 2.0;
        e.rect.top = e.pos.y - e.rect.height / 2.0;
    }

    // Set target coordinates for movement/aiming.
    if !state.entities[idx].no_ammo {
        let enemy_uid = state.entities[idx].enemy_uid;
        let enemy_idx = enemy_uid.and_then(|u| state.find_entity_index_by_uid(u));
        match enemy_idx {
            Some(i) => {
                let tank_aimbot = state.config.rules.tank_aimbot;
                let tank_laser_speed = state.config.entity_values.tank_laser_speed;
                let (e_pos, e_vel) = (state.entities[i].pos, state.entities[i].vel);
                let self_pos = state.entities[idx].pos;
                let target_pos = if tank_aimbot {
                    intercept_point(self_pos, e_pos, e_vel, tank_laser_speed)
                } else {
                    e_pos
                };
                state.entities[idx].target_pos = target_pos;
            }
            None => {
                state.entities[idx].target_pos = state.entities[idx].pos;
            }
        }
    } else {
        state.entities[idx].ammo_supply_timer -= state.game_fps_ratio;
    }

    // Look for Ammo Supply (nearest friendly, available fort).
    if state.entities[idx].no_ammo && state.entities[idx].ammo_supply_timer <= 0.0 {
        state.entities[idx].enemy_uid = None;
        state.entities[idx].enemy_index_hint = None;
        state.entities[idx].target_pos = state.entities[idx].pos;
        state.entities[idx].ammo_supply_timer = state.entities[idx].max_ammo_supply_time;

        let fort_validhealer_ticks = state.config.entity_values.fort_validhealer_ticks;
        let fort_assimilation_ticks = state.config.entity_values.fort_assimilation_ticks;
        let max_tank_heal_array = state.config.entity_values.max_tank_heal_array as usize;
        let (self_team, self_pos, self_uid) = {
            let e = &state.entities[idx];
            (e.team, e.pos, e.unique_id)
        };

        let mut lowest_dis = 100_000i32;
        let mut final_unit_index: Option<usize> = None;

        for i in 0..state.entities.len() {
            let (e_team, e_ia, e_type, e_spawn_timer) = {
                let e = &state.entities[i];
                (e.team, e.ia, e.entity_type, e.tank_spawn_timer)
            };
            if !(e_team == self_team
                && e_ia
                && e_type == EntityType::Fort
                && e_spawn_timer > (fort_validhealer_ticks - fort_assimilation_ticks) as f32)
            {
                continue;
            }

            if state.entities[i].heal_tank_array.len() >= max_tank_heal_array {
                // Fort's waitlist is full: only usable if we're already on it.
                if state.entities[i]
                    .heal_tank_array
                    .iter()
                    .any(|&u| u as u32 == self_uid)
                {
                    final_unit_index = Some(i);
                    break;
                }
            } else {
                let dx = (state.entities[i].pos.x - self_pos.x) as i32;
                let dy = (state.entities[i].pos.y - self_pos.y) as i32;
                let dis = ((dx * dx + dy * dy) as f64).sqrt() as i32;
                if dis < lowest_dis {
                    lowest_dis = dis;
                    final_unit_index = Some(i);
                }
            }
        }

        match final_unit_index {
            Some(i) => {
                let already_in_list = state.entities[i]
                    .heal_tank_array
                    .iter()
                    .any(|&u| u as u32 == self_uid);
                if !already_in_list {
                    state.entities[i].heal_tank_array.push(self_uid as i32);
                }
                let (uid, pos) = (state.entities[i].unique_id, state.entities[i].pos);
                let e = &mut state.entities[idx];
                e.ammo_target_uid = Some(uid);
                e.ammo_target_index_hint = Some(i);
                if state.event_logging {
                    println!(
                        "    Ammo Target found; uid: {uid}, pos: {},{}",
                        pos.x, pos.y
                    );
                }
            }
            None => {
                let e = &mut state.entities[idx];
                e.ammo_target_uid = None;
                e.ammo_target_index_hint = None;
            }
        }
    }

    // --------------- Calculate Enemy Tank Angle / handle retreat --------------
    let dis: f64;
    if state.entities[idx].no_ammo {
        let ammo_target_idx = state.entities[idx]
            .ammo_target_uid
            .and_then(|u| state.find_entity_index_by_uid(u));

        match ammo_target_idx {
            Some(i) => {
                let pos = state.entities[i].pos;
                state.entities[idx].target_pos = pos;
            }
            None => {
                // No (or no longer valid) ammo target: wander erratically.
                let self_pos = state.entities[idx].pos;
                let ox = state.rand_range(0..100) - 50;
                let oy = state.rand_range(0..100) - 50;
                state.entities[idx].target_pos =
                    Vector2f::new(self_pos.x + ox as f32, self_pos.y + oy as f32);
                entity_deccel(state, idx);
                state.entities[idx].ammo_target_uid = None;
                state.entities[idx].ammo_target_index_hint = None;
            }
        }
    }

    let (mut dx, mut dy);
    {
        let e = &state.entities[idx];
        dx = e.target_pos.x - e.pos.x;
        dy = e.target_pos.y - e.pos.y;
    }
    if dy.abs() < 0.0001 {
        dy = if dy >= 0.0 { 0.0001 } else { -0.0001 };
    }
    let t_ang = 360.0 - ((dx as f64 / dy as f64).atan() * 180.0 / std::f64::consts::PI);
    let mut targ_ang = if dy <= 0.0 {
        t_ang - 90.0
    } else {
        t_ang + 90.0
    };

    let mut ang = state.entities[idx].ang as f64;
    while ang < 0.0 {
        ang += 360.0;
    }
    while ang >= 360.0 {
        ang -= 360.0;
    }
    while targ_ang < 0.0 {
        targ_ang += 360.0;
    }
    while targ_ang >= 360.0 {
        targ_ang -= 360.0;
    }
    state.entities[idx].ang = ang as f32;
    state.entities[idx].targ_ang = targ_ang as f32;

    let d_ang = delta_angle(ang, targ_ang);
    if d_ang.abs() > 1.5 {
        let dirc = if d_ang > 0.0 { 1 } else { -1 };
        state.entities[idx].tilt(dirc);
    } else {
        let ratio = state.game_fps_ratio;
        state.entities[idx].deccel_ang(ratio);
    }

    // TANK MOVEMENT.
    {
        let e = &state.entities[idx];
        dx = e.target_pos.x - e.pos.x;
        dy = e.target_pos.y - e.pos.y;
    }
    if dy == 0.0 {
        dy = if state.rand_range(0..2) == 0 {
            -0.1
        } else {
            0.1
        };
    }
    dis = ((dx * dx + dy * dy) as f64).sqrt();

    let ev = &state.config.entity_values;
    let (shinys_move_fast, damaged_move_fast, tank_speed_factor, special_tank_speed_factor) = (
        ev.shinys_move_fast,
        ev.damaged_move_fast,
        ev.tank_speed_factor,
        ev.special_tank_speed_factor,
    );

    let mut tankspeed = {
        let e = &state.entities[idx];
        let ratio = e.ammo_supply as f32 / e.max_ammo_supply;
        if shinys_move_fast {
            1.0 + ratio
        } else {
            2.0 - ratio
        }
    };
    if state.config.rules.special_tanks && state.entities[idx].level != 0 {
        tankspeed *= special_tank_speed_factor;
    }

    let no_ammo = state.entities[idx].no_ammo;
    let range = state.entities[idx].range as f64;
    if dis > range && !no_ammo {
        let dirc = if d_ang.abs() > 180.0 {
            if dx > 0.0 {
                tankspeed
            } else {
                -tankspeed
            }
        } else {
            tankspeed
        };
        state.entities[idx].accel(dirc, tank_speed_factor);
    } else if !no_ammo {
        let ratio = state.game_fps_ratio;
        entity_deccel(state, idx);
        let _ = ratio;
    }

    if no_ammo {
        let hp_ratio = {
            let e = &state.entities[idx];
            e.hitpoints / e.max_hitpoints
        };
        tankspeed = if damaged_move_fast {
            2.0 - hp_ratio
        } else {
            1.0 + hp_ratio
        };
        let dirc = if d_ang.abs() > 180.0 {
            if dx > 0.0 {
                tankspeed
            } else {
                -tankspeed
            }
        } else {
            tankspeed
        };
        state.entities[idx].accel(dirc, tank_speed_factor);
    }

    entity_move(state, idx);

    // Reload ammo / heal tank.
    if state.entities[idx].no_ammo {
        let mut close_dis = 1_000_000.0f32;
        let ammo_target_idx = state.entities[idx]
            .ammo_target_uid
            .and_then(|u| state.find_entity_index_by_uid(u));

        match ammo_target_idx {
            Some(i) => {
                let target_pos = state.entities[i].pos;
                state.entities[idx].target_pos = target_pos;
                let self_pos = state.entities[idx].pos;
                let ddx = self_pos.x - target_pos.x;
                let ddy = self_pos.y - target_pos.y;
                close_dis = ((ddx * ddx + ddy * ddy) as f64).sqrt() as f32;
            }
            None => {
                entity_deccel(state, idx);
                state.entities[idx].ammo_target_uid = None;
                state.entities[idx].ammo_target_index_hint = None;
                let pos = state.entities[idx].pos;
                state.entities[idx].target_pos = pos;
            }
        }

        if let Some(i) = ammo_target_idx {
            let (fort_range, fort_ia) = {
                let f = &state.entities[i];
                (f.range, f.ia)
            };
            if (close_dis as f64) < (fort_range / 15.0) as f64 && fort_ia {
                let ratio = state.game_fps_ratio;
                entity_deccel(state, idx);
                let _ = ratio;

                if state.entities[idx].hitpoints < state.entities[idx].max_hitpoints {
                    let bump = state.entities[idx].max_hitpoints / 25.0;
                    state.entities[idx].hitpoints += bump;
                } else {
                    state.entities[i].fort_tanks_healed += 1;

                    let fort_healing = state.config.rules.fort_healing;
                    if state.entities[i].hitpoints < state.entities[i].max_hitpoints && fort_healing
                    {
                        let (fort_level, fort_max_hp) = {
                            let f = &state.entities[i];
                            (f.fort_level, f.max_hitpoints)
                        };
                        let tank_specialfort_heal_percent =
                            state.config.entity_values.tank_specialfort_heal_percent;
                        let tank_fort_heal_percent =
                            state.config.entity_values.tank_fort_heal_percent;
                        if fort_level == 1 {
                            state.entities[i].hitpoints +=
                                fort_max_hp * tank_specialfort_heal_percent;
                        } else if fort_level == 0 {
                            state.entities[i].hitpoints += fort_max_hp * tank_fort_heal_percent;
                        }
                        if state.entities[i].hitpoints > state.entities[i].max_hitpoints {
                            state.entities[i].hitpoints = state.entities[i].max_hitpoints;
                        }
                    }

                    state.entities[idx].no_ammo = false;
                    let max_ammo = state.entities[idx].max_ammo_supply;
                    state.entities[idx].ammo_supply = max_ammo as i32;
                    let max_hp = state.entities[idx].max_hitpoints;
                    state.entities[idx].hitpoints = max_hp;

                    let (pos, col) = {
                        let e = &state.entities[idx];
                        (e.pos, e.col)
                    };
                    if state.game_screen_rect.contains(pos) {
                        state
                            .sound_queue
                            .push(crate::assets::SoundEffect::TankReload);
                        for _ in 0..5 {
                            spawn_particle(state, pos, 3.0, 2.0, 2, 1.0, col);
                            spawn_particle(state, pos, 3.0, 2.0, 2, 1.0, opposite_color(col));
                        }
                    }
                }
            }
        }
    }

    // SHOOT.
    let should_shoot = {
        let e = &state.entities[idx];
        (targ_ang - ang).abs() <= 7.5
            && e.laser_cooldown <= 0.0
            && dis <= e.range as f64
            && !e.no_ammo
            && e.enemy_uid.is_some()
    };
    if should_shoot {
        let (pos, ang_f, rect_height, dam, team, entity_uid, col) = {
            let e = &state.entities[idx];
            (
                e.pos,
                e.ang,
                e.rect.height,
                e.dam,
                e.team,
                e.unique_id,
                e.col,
            )
        };
        let mut laser_pos = pos;
        let rad = (ang_f as f64) * std::f64::consts::PI / 180.0;
        laser_pos.x += (rad.cos() as f32) * rect_height / 2.0;
        laser_pos.y += (rad.sin() as f32) * rect_height / 2.0;

        let level = state.entities[idx].level;
        // DEVIATION: C++'s missile-branch condition is
        // `(ammo_supply <= Tank_MISSLE_NO && false) || level > 0` -- the
        // `&& false` permanently disables the first clause, so this is
        // exactly `level > 0`. Reproduced as such.
        if (level > 0)
            && state.entities[idx].ammo_supply
                < state.config.entity_values.tank_missle_projectile_no
        {
            let ev = &state.config.entity_values;
            let (tank_missle_velocity, tank_missle_damagefactor, tank_missle_projectile_no) = (
                ev.tank_missle_velocity,
                ev.tank_missle_damagefactor,
                ev.tank_missle_projectile_no,
            );
            let (tank_missle_splash_radius, tank_missle_terminal_velocity, tank_missle_power) = (
                ev.tank_missle_splash_radius,
                ev.tank_missle_terminal_velocity,
                ev.tank_missle_power,
            );
            let (tank_missle_size, tank_missle_trail_length) = (
                state.config.graphics.tank_missle_size,
                state.config.graphics.tank_missle_trail_length,
            );
            let enemy_uid = state.entities[idx].enemy_uid.unwrap_or(0);
            spawn_missile(
                state,
                laser_pos,
                tank_missle_velocity,
                ang_f,
                dam * tank_missle_damagefactor,
                tank_missle_projectile_no,
                team,
                entity_uid,
                enemy_uid,
                tank_missle_size,
                tank_missle_trail_length,
                tank_missle_splash_radius,
                tank_missle_terminal_velocity,
                tank_missle_power,
            );
            if state.event_logging {
                println!("  Missle shot");
            }
        } else {
            let tank_laser_speed = state.config.entity_values.tank_laser_speed;
            spawn_laser(
                state,
                laser_pos.x,
                laser_pos.y,
                tank_laser_speed,
                ang_f,
                dam,
                team,
                entity_uid,
                vec![4.0, 0.66],
            );
            if state.event_logging {
                println!("  Laser shot");
            }
        }

        state.entities[idx].ammo_supply -= 1;
        let max_cd = state.entities[idx].max_laser_cooldown;
        state.entities[idx].laser_cooldown = max_cd;

        for _ in 0..8 {
            spawn_particle(state, laser_pos, 4.0, 0.45, 2, 1.0, opposite_color(col));
        }

        // C++: coin-flip between tankshoot01/02, gated on GameScreenRect.
        if state.game_screen_rect.contains(pos) {
            let pick = state.rand_range(0..2);
            state.sound_queue.push(if pick == 0 {
                crate::assets::SoundEffect::TankShoot01
            } else {
                crate::assets::SoundEffect::TankShoot02
            });
        }
    }
}

/// C++'s `deccel()` needs `GameFPSRatio`, which isn't part of `Entity`
/// itself -- small free-function wrapper so call sites above don't need
/// to juggle `&mut Entity` + a separately-fetched ratio inline.
fn entity_deccel(state: &mut GameState, idx: usize) {
    let ratio = state.game_fps_ratio;
    state.entities[idx].deccel(ratio);
}

fn entity_ai_fort(state: &mut GameState, idx: usize) {
    use crate::spawn::{
        spawn_floating_text, spawn_laser, spawn_missile, spawn_particle, spawn_tank,
    };

    if state.event_logging {
        let heal_list: Vec<String> = state.entities[idx]
            .heal_tank_array
            .iter()
            .map(|u| u.to_string())
            .collect();
        println!(
            "\nFort.AI(); GameTick: {}\n  Fort uid: {}  Fort Pos: {},{}\n  Fort Angles; ang = {}; targ_ang = {}\n  Healing Tanks Array: [{}]",
            state.game_ticks,
            state.entities[idx].unique_id,
            state.entities[idx].pos.x,
            state.entities[idx].pos.y,
            state.entities[idx].ang,
            state.entities[idx].targ_ang,
            heal_list.join(",")
        );
    }

    // Clear the healing waitlist every 30 ticks.
    if state.game_ticks % 30 == 0 {
        state.entities[idx].heal_tank_array.clear();
    }

    // Upgrade to special fort.
    {
        let e = &state.entities[idx];
        let qualifies = ((e.fort_tanks_spawned >= e.fort_tanks_spawned_threshold
            && e.fort_tanks_spawned_threshold != -1)
            || (e.fort_tanks_healed >= e.fort_tanks_healed_threshold
                && e.fort_tanks_healed_threshold != -1))
            && state.config.rules.special_forts;
        if qualifies {
            if !state.entities[idx].special_fort {
                state.entities[idx].special_fort = true;
            }
            if state.entities[idx].fort_level == 0 {
                let ev = &state.config.entity_values;
                let (
                    special_fort_misslecooldown_factor,
                    special_fort_hitpoints_factor,
                    special_fort_tankspawn_factor,
                    special_fort_range_factor,
                ) = (
                    ev.special_fort_misslecooldown_factor,
                    ev.special_fort_hitpoints_factor,
                    ev.special_fort_tankspawn_factor,
                    ev.special_fort_range_factor,
                );
                let (fort_hp, fort_spawntank_time, fort_range_cfg) =
                    (ev.fort_hp, ev.fort_spawntank_time, ev.fort_range);

                let max_laser_cd = state.entities[idx].max_laser_cooldown;
                let e = &mut state.entities[idx];
                e.max_missle_cooldown = max_laser_cd * 33.0 / special_fort_misslecooldown_factor;
                e.max_hitpoints = fort_hp * special_fort_hitpoints_factor;
                e.hitpoints = e.max_hitpoints;
                e.max_tank_spawn_time = fort_spawntank_time as f32 / special_fort_tankspawn_factor;
                e.range = fort_range_cfg as f32 * special_fort_range_factor;
                e.fort_level = 1;

                let (pos, col, team) = (e.pos, e.col, e.team);
                for _ in 0..15 {
                    spawn_particle(state, pos, 8.0, 2.0, 2, 0.5, col);
                }
                if state.game_screen_rect.contains(pos) {
                    let col2 = state.color_arr[team as usize];
                    spawn_floating_text(
                        state,
                        "Upgraded".to_string(),
                        pos,
                        col2,
                        opposite_color(col2),
                        30,
                        3.0,
                        FontChoice::Font2,
                        0.33,
                    );
                }
            }
        }
    }

    // Level-based passive healing.
    if state.entities[idx].fort_level == 1 {
        let special_fort_healing_factor = state.config.entity_values.special_fort_healing_factor;
        let e = &mut state.entities[idx];
        if e.hitpoints < e.max_hitpoints {
            e.hitpoints += e.max_hitpoints * state.game_fps_ratio
                / ((100.0 / special_fort_healing_factor) * 60.0);
        }
        if e.hitpoints > e.max_hitpoints {
            e.hitpoints = e.max_hitpoints;
        }
    }

    // Fort destroyed / captured.
    if state.entities[idx].hitpoints <= 0.0 {
        if state.event_logging {
            println!("  Fort Captured");
        }

        if state.config.rules.convert_forts {
            let last_bullet_team = state.entities[idx].last_bullet_team;
            let newteam = last_bullet_team;

            let (pos, tank_spawn_timer, team, unique_id) = {
                let e = &state.entities[idx];
                (e.pos, e.tank_spawn_timer, e.team, e.unique_id)
            };
            if state.game_screen_rect.contains(pos) && tank_spawn_timer > 0.0 {
                let ncol = state.color_arr[newteam as usize];
                spawn_floating_text(
                    state,
                    "Captured".to_string(),
                    pos,
                    ncol,
                    opposite_color(ncol),
                    20,
                    2.0,
                    FontChoice::Font2,
                    0.33,
                );
            }
            state.game_log_arr.push(format!(
                "{{{}}}[fort#{unique_id}] converted to {{{}}}[fort#{unique_id}]",
                state.hex_color_arr[team as usize], state.hex_color_arr[newteam as usize]
            ));

            let ev = &state.config.entity_values;
            let (fort_missle_cooldown_factor, fort_hp, fort_spawntank_time, fort_range) = (
                ev.fort_missle_cooldown_factor,
                ev.fort_hp,
                ev.fort_spawntank_time,
                ev.fort_range,
            );

            let ncol = state.color_arr[newteam as usize];
            let e = &mut state.entities[idx];
            e.team = newteam;
            e.col = ncol;
            e.opp_col = opposite_color(ncol);
            // DEVIATION: `entity_sprite.setTexture(FortTextArr[team])`
            // dropped -- Entity no longer stores a Sprite (see Stage 0.5).
            e.tank_spawn_timer = -(fort_assimilation_ticks_f32(&state.config));
            e.fort_tanks_healed = 0;
            e.fort_tanks_spawned = 0;
            e.fort_level = 0;
            let max_laser_cd = e.max_laser_cooldown;
            e.max_missle_cooldown = max_laser_cd * 33.0 * fort_missle_cooldown_factor;
            e.max_hitpoints = fort_hp;
            e.hitpoints = e.max_hitpoints;
            e.max_tank_spawn_time = fort_spawntank_time as f32;
            e.range = fort_range as f32;
            e.special_fort = false;
            e.level = 0;

            let pos = e.pos;
            for _ in 0..40 {
                spawn_particle(state, pos, 15.0, 4.0, 2, 0.8, ncol);
            }

            if state.config.rules.gift_tanks {
                for _ in 0..2 {
                    let ox = state.rand_range(0..10) - 5;
                    let oy = state.rand_range(0..10) - 5;
                    let pos2 = Vector2f::new(pos.x + ox as f32, pos.y + oy as f32);
                    spawn_tank(state, pos2, newteam, 0, false);
                }
            }

            // C++: fortcaptured01.play(), gated on GameScreenRect.
            if state.game_screen_rect.contains(pos) {
                state
                    .sound_queue
                    .push(crate::assets::SoundEffect::FortCaptured);
            }

            state.forts_lost[team as usize] += 1;
            if newteam >= 0 {
                state.forts_destroyed[newteam as usize] += 1;
            }
            let max_hp = state.entities[idx].max_hitpoints;
            state.entities[idx].hitpoints = max_hp;
        } else {
            // CONVERT_FORTS disabled: C++ still runs the two stat-array
            // updates and the HP reset outside the `if` block.
            let (team, last_bullet_team) = {
                let e = &state.entities[idx];
                (e.team, e.last_bullet_team)
            };
            state.forts_lost[team as usize] += 1;
            if last_bullet_team >= 0 {
                state.forts_destroyed[last_bullet_team as usize] += 1;
            }
            let max_hp = state.entities[idx].max_hitpoints;
            state.entities[idx].hitpoints = max_hp;
        }
    }

    // Low-HP damage-smoke particles.
    {
        let (hitpoints, max_hitpoints, pos, col) = {
            let e = &state.entities[idx];
            (e.hitpoints, e.max_hitpoints, e.pos, e.col)
        };
        if hitpoints <= max_hitpoints * 0.5 {
            let factor = (0.5 - (hitpoints / max_hitpoints)) * 2.0 * 5.0;
            if state.rand_range(0..100) as f32 <= 10.0 * factor {
                spawn_particle(state, pos, 4.0, 1.5, 2, 0.5, opposite_color(col));
            }
            if state.rand_range(0..100) as f32 <= 2.0 * factor {
                for _ in 0..5 {
                    spawn_particle(state, pos, 3.0, 3.0, 1, 0.8, opposite_color(col));
                }
            }
        }
    }

    if state.entities[idx].laser_cooldown > 0.0 {
        state.entities[idx].laser_cooldown -= state.game_fps_ratio;
    }
    if state.entities[idx].missle_cooldown > 0.0 {
        state.entities[idx].missle_cooldown -= state.game_fps_ratio;
    }

    // Spawn tanks from fort.
    state.entities[idx].tank_spawn_timer += state.game_fps_ratio;
    if state.entities[idx].tank_spawn_timer >= state.entities[idx].max_tank_spawn_time {
        let team = state.entities[idx].team;
        let tank_population = get_tank_pop(state, team, 0);
        let cap = state.config.entity_values.tank_min_pop + state.extra_population;
        if tank_population < cap && state.config.rules.forts_spawn_tanks {
            let pos = state.entities[idx].pos;
            let ox = state.rand_range(0..10) - 5;
            let oy = state.rand_range(0..10) - 5;
            let spawn_pos = Vector2f::new(pos.x + ox as f32, pos.y + oy as f32);

            state.entities[idx].tank_spawn_timer = 0.0;
            state.entities[idx].fort_tanks_spawned += 1;

            let col = state.entities[idx].col;
            for _ in 0..20 {
                spawn_particle(state, pos, 7.5, 2.25, 2, 1.0 / 1.5, col);
            }
            if state.game_screen_rect.contains(pos) {
                state
                    .sound_queue
                    .push(crate::assets::SoundEffect::TankCreated);
            }
            spawn_tank(state, spawn_pos, team, 0, false);
        } else {
            state.entities[idx].tank_spawn_timer = state.entities[idx].max_tank_spawn_time;
            if state.rand_range(0..100) as f32 <= 10.0 * state.game_fps_ratio {
                let (pos, col) = (state.entities[idx].pos, state.entities[idx].col);
                spawn_particle(state, pos, 5.0, 0.75, 2, 2.0, col);
            }
        }
    }

    if state.event_logging {
        println!("  Fort Starts looking for enemies if idle....");
    }

    // DEVIATION: the C++ `this != nullptr` / "is this fort still in
    // EntityArr" self-check (L2544-2559) can't happen in this port --
    // `idx` always refers to a live element of `state.entities` for the
    // duration of this call. Omitted.

    if state.entities[idx].lock_target_timer <= state.entities[idx].max_lock_target_time {
        state.entities[idx].lock_target_timer += state.game_fps_ratio;
    } else {
        entity_lock_target(state, idx);
    }

    let (target_pos, pos) = {
        let e = &state.entities[idx];
        (e.target_pos, e.pos)
    };
    let mut dx = (target_pos.x - pos.x) as f64;
    let mut dy = (target_pos.y - pos.y) as f64;
    let _dis_unused = (dx * dx + dy * dy).sqrt();
    if state.event_logging {
        println!("  Fort Target Pos: {},{}", target_pos.x, target_pos.y);
    }

    dx = (target_pos.x - pos.x) as f64;
    dy = (target_pos.y - pos.y) as f64;
    if dy == 0.0 {
        dy = if state.rand_range(0..2) == 0 {
            -0.1
        } else {
            0.1
        };
    }
    if dx == 0.0 {
        dx = if state.rand_range(0..2) == 0 {
            -0.1
        } else {
            0.1
        };
    }

    if state.event_logging {
        println!("  Moving Fort Gun...");
    }
    let temp_ang = 360.0 - ((dx / dy).atan() * 180.0 / std::f64::consts::PI);
    let mut targ_ang = if dy <= 0.0 {
        temp_ang - 90.0
    } else {
        temp_ang + 90.0
    };

    let mut ang = state.entities[idx].ang as f64;
    if state.event_logging {
        println!("  Fort Angles; ang = {ang}; targ_ang = {targ_ang}");
    }
    // DEVIATION: C++ truncates to int before the modulo --
    // `ang = (int)ang % 360;` -- discarding the fractional part. Preserved
    // exactly (not "fixed" to `ang.rem_euclid(360.0)`), since it's a
    // deliberate-looking quirk that changes fort gun-angle precision.
    ang = ((ang as i64) % 360) as f64;
    targ_ang = ((targ_ang as i64) % 360) as f64;

    while ang > 360.0 {
        ang -= 360.0;
    }
    while ang < 0.0 {
        ang += 360.0;
    }
    while targ_ang > 360.0 {
        targ_ang -= 360.0;
    }
    while targ_ang < 0.0 {
        targ_ang += 360.0;
    }

    // Moving Fort Gun.
    if targ_ang > ang {
        if (targ_ang - ang).abs() < 180.0 {
            state.entities[idx].tilt(4);
        } else {
            state.entities[idx].tilt(-4);
        }
    } else if targ_ang < ang {
        if (targ_ang - ang).abs() < 180.0 {
            state.entities[idx].tilt(-4);
        } else {
            state.entities[idx].tilt(4);
        }
    }

    {
        let ratio = state.game_fps_ratio;
        let e = &mut state.entities[idx];
        e.ang_vel += e.ang_acc * ratio;
        e.ang += e.ang_vel * ratio;
        e.ang_vel *= 0.9f32.powf(ratio);
        e.ang_acc = 0.0;
        e.targ_ang = targ_ang as f32;
    }

    // SHOOT MISSILE.
    let missile_gate = {
        let e = &state.entities[idx];
        e.missle_cooldown <= 0.0
            && !state.config.rules.forts_neutered
            && e.tank_spawn_timer > 0.0
            && state.config.rules.fort_missles
    };
    if missile_gate {
        let (self_team, self_pos, self_ang, self_uid) = {
            let e = &state.entities[idx];
            (e.team, e.pos, e.ang, e.unique_id)
        };
        let range = state.entities[idx].range as f64;

        let mut candidates: Vec<usize> = Vec::new();
        for i in 0..state.entities.len() {
            let (e_team, e_type, e_pos) = {
                let e = &state.entities[i];
                (e.team, e.entity_type, e.pos)
            };
            if e_team != self_team && e_type != EntityType::Fort {
                let ddx = (e_pos.x - self_pos.x) as f64;
                let ddy = (e_pos.y - self_pos.y) as f64;
                let d = (ddx * ddx + ddy * ddy).sqrt();
                if d <= range * 0.9 {
                    candidates.push(i);
                }
            }
        }

        let ev = &state.config.entity_values;
        let (fort_missle_velocity, fort_dam_factor, fort_missle_proj_no) = (
            ev.fort_missle_velocity,
            ev.fort_missle_damagefactor,
            ev.fort_missle_projectile_no,
        );
        let (fort_missle_splash_radius, fort_missle_terminal_vel, fort_missle_power) = (
            ev.fort_missle_splash_radius,
            ev.fort_missle_terminal_velocity,
            ev.fort_missle_power,
        );
        let fort_damage = ev.fort_damage;
        let (fort_missle_size, fort_missle_trail_length) = (
            state.config.graphics.fort_missle_size,
            state.config.graphics.fort_missle_trail_length,
        );

        let target_uid = if !candidates.is_empty() {
            let pick = candidates[state.rand_range(0..candidates.len())];
            state.entities[pick].unique_id
        } else {
            // DEVIATION: C++ reads `EntityArr[enemy_entity_index]` directly
            // here with no null/bounds check even when `enemy == nullptr`
            // -- a latent out-of-bounds read. This port passes uid `0`
            // (guaranteed invalid), which `missile_spawn` safely rejects
            // (`ia = false`) instead of crashing.
            state.entities[idx].enemy_uid.unwrap_or(0)
        };

        spawn_missile(
            state,
            pos,
            fort_missle_velocity,
            self_ang,
            fort_damage * fort_dam_factor,
            fort_missle_proj_no,
            self_team,
            self_uid,
            target_uid,
            fort_missle_size,
            fort_missle_trail_length,
            fort_missle_splash_radius,
            fort_missle_terminal_vel,
            fort_missle_power,
        );
        let max_missle_cd = state.entities[idx].max_missle_cooldown;
        state.entities[idx].missle_cooldown = max_missle_cd;
    }

    // SHOOT LASER.
    let laser_gate = {
        let e = &state.entities[idx];
        (targ_ang - ang).abs() <= 15.0
            && e.laser_cooldown <= 0.0
            && !state.config.rules.forts_neutered
            && e.tank_spawn_timer > 0.0
            && e.enemy_uid.is_some()
    };
    if laser_gate {
        if state.event_logging {
            println!("  Fort Shoots Laser");
        }
        let (pos, ang_f, gun_len, dam, team, uid, col) = {
            let e = &state.entities[idx];
            (
                e.pos,
                e.ang,
                e.gun_barrel[0],
                e.dam,
                e.team,
                e.unique_id,
                e.col,
            )
        };
        let rad = (ang_f as f64) * std::f64::consts::PI / 180.0;
        let x = gun_len as f32 * (rad.cos() as f32) + pos.x;
        let y = gun_len as f32 * (rad.sin() as f32) + pos.y;

        let fort_laser_speed = state.config.entity_values.fort_laser_speed;
        spawn_laser(
            state,
            x,
            y,
            fort_laser_speed,
            ang_f,
            dam,
            team,
            uid,
            vec![2.0, 0.75],
        );
        let max_cd = state.entities[idx].max_laser_cooldown;
        state.entities[idx].laser_cooldown = max_cd;

        for _ in 0..5 {
            spawn_particle(
                state,
                Vector2f::new(x, y),
                4.0,
                3.0,
                2,
                0.25,
                opposite_color(col),
            );
        }
        if state.game_screen_rect.contains(pos) {
            state
                .sound_queue
                .push(crate::assets::SoundEffect::FortShoot);
        }
    }
}

/// C++: int get_tank_pop(int team, int mode = 0) (TankAttack.cpp
/// L5418-5437). Only mode 0 (count all live tanks on a team) is used by
/// `entity_ai_fort` so far; modes 1/2 (ammo/no-ammo split) are included
/// for completeness since Stage E's stats module will need them too.
pub fn get_tank_pop(state: &GameState, team: i32, mode: i32) -> i32 {
    let mut num = 0;
    for e in &state.entities {
        if e.entity_type == EntityType::Tank && e.team == team && e.ia {
            match mode {
                0 => num += 1,
                1 => {
                    if !e.no_ammo {
                        num += 1;
                    }
                }
                2 => {
                    if e.no_ammo {
                        num += 1;
                    }
                }
                _ => {}
            }
        }
    }
    num
}

fn fort_assimilation_ticks_f32(config: &crate::config::Config) -> f32 {
    config.entity_values.fort_assimilation_ticks as f32
}

// ---------------------------------------------------------------------
// Stage D.3: Entity::draw(), ported from TankAttack.cpp L2798-3038.
//
// Unlike every other Entity function so far, `draw()` has real gameplay
// side effects mixed into the render path in the original: it spawns
// particles (`spawn_particle`) and, for tanks, can flip `no_ammo = true`
// as a side effect of a purely visual check. This port preserves that
// exactly rather than "cleaning it up" into a separate update step --
// takes `&mut GameState` and mutates `state.entities[idx]` same as
// `entity_ai`. `col`/`opp_col`'s alpha channel is also genuinely mutated
// on the entity for the duration of the draw (used as working state for
// layered visual effects), and reset to 255 at the very end of the
// function in the original -- reproduced identically, so nothing leaks
// into the next frame's logic.
// ---------------------------------------------------------------------

use crate::assets::Assets;
use sfml::graphics::{RenderTarget, Sprite, Transformable};

/// C++: void Entity::draw() (TankAttack.cpp L2798-3038).
pub fn entity_draw<T: RenderTarget>(
    state: &mut GameState,
    assets: &Assets,
    target: &mut T,
    idx: usize,
) {
    use crate::render::{
        draw_arc, draw_circle, draw_filled_circle, draw_line, draw_rect, draw_thick_line,
        render_text,
    };
    use crate::spawn::spawn_particle;

    let entity_type = state.entities[idx].entity_type;

    if entity_type == EntityType::Tank {
        if state.event_logging {
            println!("Tank.draw()");
        }

        state.entities[idx].opp_col.a = 255;

        // Spawn particles for damaged/out-of-ammo tanks (and, as in the
        // original, this check can itself set no_ammo -- see module doc).
        {
            let (ammo_supply, max_hitpoints, hitpoints, pos, opp_col) = {
                let e = &state.entities[idx];
                (
                    e.ammo_supply,
                    e.max_hitpoints,
                    e.hitpoints,
                    e.pos,
                    e.opp_col,
                )
            };
            let panic_perc = state.config.entity_values.panic_health_perc;
            if (ammo_supply <= 0 || hitpoints <= max_hitpoints * panic_perc)
                && state.config.rules.need_supplies
            {
                if state.rand_range(0..100) as f32 <= 7.0 * state.game_fps_ratio {
                    spawn_particle(state, pos, 2.5, 0.5, 2, 2.0, opp_col);
                }
                state.entities[idx].no_ammo = true;
            }
        }

        let ratio = {
            let e = &state.entities[idx];
            e.hitpoints / e.max_hitpoints
        };

        // Spawn particles for damaged tanks II.
        if (state.rand_range(0..100) as f32 <= 25.0 * (1.0 - ratio)) && ratio < 0.67 {
            let (pos, opp_col) = {
                let e = &state.entities[idx];
                (e.pos, e.opp_col)
            };
            let n = state.rand_range(0..4);
            for _ in 0..n {
                spawn_particle(state, pos, 2.0, 2.0, 1, 0.8, opp_col);
            }
        }

        let g_pos = {
            let e = &state.entities[idx];
            Vector2f::new(e.pos.x - state.camera_pos[0], e.pos.y - state.camera_pos[1])
        };

        {
            let e = &mut state.entities[idx];
            e.col.a = (32.0 * (e.ammo_supply as f32 / e.max_ammo_supply)) as u8;
            e.opp_col.a = 32;
            if e.hitpoints < 0.0 {
                e.col.a = 0;
            }
        }

        let tank_bloom = state.config.graphics.tank_bloom_range_factor;
        let tank_range = state.config.entity_values.tank_range as f32;
        let tank_circle_radius = tank_bloom * tank_range;

        let col = state.entities[idx].col;
        draw_filled_circle(target, g_pos, tank_circle_radius * 0.75, col);

        let (team, tex_width, tex_height, ang) = {
            let e = &state.entities[idx];
            (e.team, e.tex_width, e.tex_height, e.ang)
        };
        let mut sprite = Sprite::with_texture(&assets.tank_textures[team as usize]);
        sprite.set_origin(Vector2f::new(
            tex_width as f32 / 2.0,
            tex_height as f32 / 2.0,
        ));
        sprite.set_position(g_pos);
        sprite.set_rotation(ang + 90.0);
        target.draw(&sprite);

        // Health bar/circle.
        state.entities[idx].tank_health_arc_angle += state.game_fps_ratio * 2.0;
        let (ang0, opp_col, hitpoints, max_hitpoints) = {
            let e = &state.entities[idx];
            (
                e.tank_health_arc_angle,
                e.opp_col,
                e.hitpoints,
                e.max_hitpoints,
            )
        };
        draw_arc(
            target,
            g_pos,
            tank_circle_radius * 0.8,
            opp_col,
            3.0,
            ang0,
            ang0 + hitpoints * 360.0 / max_hitpoints,
            20,
        );

        // Circle for special (leveled-up) tanks.
        let level = state.entities[idx].level;
        if level != 0 {
            let col = state.entities[idx].col;
            draw_circle(target, g_pos, tank_circle_radius, col, 3.0);
            if state.rand_range(0..100) as f32 <= 8.0 * state.game_fps_ratio {
                let pos = state.entities[idx].pos;
                spawn_particle(state, pos, 3.5, 0.4, 2, 2.25, col);
            }
        }

        if state.debug_features {
            let (mut col, target_pos, unique_id, kill_count) = {
                let e = &state.entities[idx];
                (e.col, e.target_pos, e.unique_id, e.kill_count)
            };
            col.a = 64;
            draw_line(
                target,
                g_pos,
                Vector2f::new(
                    target_pos.x - state.camera_pos[0],
                    target_pos.y - state.camera_pos[1],
                ),
                col,
            );

            let mut n_rect = state.entities[idx].collision_rect();
            n_rect.left -= state.camera_pos[0];
            n_rect.top -= state.camera_pos[1];

            let (hitpoints, max_hitpoints) = {
                let e = &state.entities[idx];
                (e.hitpoints, e.max_hitpoints)
            };
            col.a = (255.0 * (hitpoints / max_hitpoints)) as u8;
            draw_rect(target, n_rect, col, 1.0);

            render_text(
                target,
                Vector2f::new(g_pos.x + 10.0, g_pos.y - 15.0),
                &unique_id.to_string(),
                &assets.font_2,
                15,
                Color::WHITE,
                0.0,
                Color::BLACK,
                false,
            );
            render_text(
                target,
                Vector2f::new(g_pos.x + 10.0, g_pos.y + 5.0),
                &format!("Kills: {kill_count}"),
                &assets.font_2,
                15,
                Color::WHITE,
                0.0,
                Color::BLACK,
                false,
            );
        }

        if state.event_logging {
            println!("  Tank Drawn.");
        }
    }

    if entity_type == EntityType::Fort {
        if state.event_logging {
            println!("Fort.draw()");
        }

        state.entities[idx].opp_col.a = 255;

        // Fort assimilation particles.
        if state.entities[idx].tank_spawn_timer < 0.0 {
            let fort_assimilation_ticks = state.config.entity_values.fort_assimilation_ticks as f32;
            let (tank_spawn_timer, pos, opp_col) = {
                let e = &state.entities[idx];
                (e.tank_spawn_timer, e.pos, e.opp_col)
            };
            let mut ratio = -tank_spawn_timer / fort_assimilation_ticks;
            ratio = 1.0 - ratio;

            if state.rand_range(0..100) as f32 <= (14.0 * state.game_fps_ratio * ratio) + 1.0 {
                for _ in 0..2 {
                    spawn_particle(state, pos, 3.0, 0.5, 2, 1.0 + (3.0 * ratio), opp_col);
                }
            }
        }

        let (pos, fort_level, team) = {
            let e = &state.entities[idx];
            (e.pos, e.fort_level, e.team)
        };
        let x0 = (pos.x - state.camera_pos[0]).round();
        let y0 = (pos.y - state.camera_pos[1]).round();

        let fort_size_cfg = state.config.entity_values.fort_size as f32;
        let tmp_size = if fort_level != 0 {
            fort_size_cfg * 1.125
        } else {
            fort_size_cfg
        };
        let tmp_rect = FloatRect::new(x0 - tmp_size / 2.0, y0 - tmp_size / 2.0, tmp_size, tmp_size);
        let _ = tmp_rect; // matches C++'s tmp_rect, computed but only used by the commented-out "Draw Fort Body" block.

        let flicker_timer = state.entities[idx].flicker_timer;
        let max_flicker_time = state.entities[idx].max_flicker_time;
        let flicker_ratio = flicker_timer * 24.0 / max_flicker_time;
        state.entities[idx].col.a = (flicker_ratio + 8.0) as u8;

        let fort_bloom = state.config.graphics.fort_bloom_range_factor;
        let fort_range_cfg = state.config.entity_values.fort_range as f32;

        if state.draw_transp_fort && state.entities[idx].tank_spawn_timer > 0.0 {
            let (tank_spawn_timer, max_tank_spawn_time, col, opp_col) = {
                let e = &state.entities[idx];
                (e.tank_spawn_timer, e.max_tank_spawn_time, e.col, e.opp_col)
            };
            let ratio = 0.5 * (tank_spawn_timer / max_tank_spawn_time);
            let n_ratio = 0.5 - ratio;
            let temp_col = Color::rgba(
                (col.r as f32 * n_ratio + opp_col.r as f32 * ratio) as u8,
                (col.g as f32 * n_ratio + opp_col.g as f32 * ratio) as u8,
                (col.b as f32 * n_ratio + opp_col.b as f32 * ratio) as u8,
                col.a,
            );
            draw_filled_circle(
                target,
                Vector2f::new(x0, y0),
                fort_range_cfg * fort_bloom * 0.8,
                temp_col,
            );
            if fort_level != 0 {
                draw_circle(
                    target,
                    Vector2f::new(x0, y0),
                    fort_range_cfg * fort_bloom,
                    col,
                    4.0,
                );
            }
        }

        // Health bar.
        state.entities[idx].col.a = ((state.entities[idx].col.a as f32) * 1.5) as u8;
        state.entities[idx].fort_health_arc_angle += state.game_fps_ratio * 0.33;
        let (ang0, col, hitpoints, max_hitpoints) = {
            let e = &state.entities[idx];
            (e.fort_health_arc_angle, e.col, e.hitpoints, e.max_hitpoints)
        };
        draw_arc(
            target,
            Vector2f::new(x0, y0),
            fort_range_cfg * fort_bloom * 0.88 - 1.0,
            col,
            5.0,
            ang0,
            ang0 + hitpoints * 360.0 / max_hitpoints,
            25,
        );
        state.entities[idx].col.a = ((state.entities[idx].col.a as f32) / 1.5) as u8;

        // Assimilation ring.
        if state.entities[idx].tank_spawn_timer < 0.0 {
            state.entities[idx].opp_col.a = 24;
            let fort_assimilation_ticks = state.config.entity_values.fort_assimilation_ticks as f32;
            let tank_spawn_timer = state.entities[idx].tank_spawn_timer;
            let n_ratio = 1.0 + (tank_spawn_timer / fort_assimilation_ticks);

            state.entities[idx].fort_assimilation_arc_angle -= state.game_fps_ratio * 0.5;
            let (ang0, opp_col) = {
                let e = &state.entities[idx];
                (e.fort_assimilation_arc_angle, e.opp_col)
            };
            draw_arc(
                target,
                Vector2f::new(x0, y0),
                fort_range_cfg * fort_bloom * 0.75,
                opp_col,
                4.0,
                ang0,
                ang0 + 360.0 * n_ratio,
                25,
            );
        }

        // Fort gun.
        if !state.config.rules.forts_neutered {
            if fort_level != 0 {
                state.entities[idx].gun_barrel[0] = 45;
                state.entities[idx].gun_barrel[1] = 3;
            }

            let (gun_len, ang_deg) = {
                let e = &state.entities[idx];
                (e.gun_barrel[0], e.ang)
            };
            let rad = (ang_deg as f64) * std::f64::consts::PI / 180.0;
            let mut x1 = gun_len as f32 * (rad.cos() as f32) + pos.x;
            let mut y1 = gun_len as f32 * (rad.sin() as f32) + pos.y;
            x1 -= state.camera_pos[0];
            y1 -= state.camera_pos[1];

            state.entities[idx].opp_col.a = 255;
            let (opp_col, gun_width) = {
                let e = &state.entities[idx];
                (e.opp_col, e.gun_barrel[1])
            };
            draw_thick_line(
                target,
                Vector2f::new(x0, y0),
                Vector2f::new(x1, y1),
                gun_width as f32,
                opp_col,
            );
        }

        // Debug features: aim line to target + healing-waitlist-full ring.
        let (target_pos, pos_now) = {
            let e = &state.entities[idx];
            (e.target_pos, e.pos)
        };
        let dx = target_pos.x - pos_now.x;
        let dy = target_pos.y - pos_now.y;
        let dis = ((dx * dx + dy * dy) as f64).sqrt() as f32;

        if state.debug_features {
            let range = state.entities[idx].range;
            let (mut x2, mut y2) = (x0, y0);
            if dis <= range {
                x2 = target_pos.x - state.camera_pos[0];
                y2 = target_pos.y - state.camera_pos[1];
            }

            let (flicker_timer, max_flicker_time) = {
                let e = &state.entities[idx];
                (e.flicker_timer, e.max_flicker_time)
            };
            state.entities[idx].opp_col.a = (128.0 * flicker_timer / max_flicker_time) as u8;
            let opp_col = state.entities[idx].opp_col;
            draw_thick_line(
                target,
                Vector2f::new(x0, y0),
                Vector2f::new(x2, y2),
                4.0,
                opp_col,
            );

            let max_tank_heal_array = state.config.entity_values.max_tank_heal_array as usize;
            if state.entities[idx].heal_tank_array.len() >= max_tank_heal_array {
                state.entities[idx].col.a = 64;
                let col = state.entities[idx].col;
                draw_circle(target, Vector2f::new(x0, y0), 50.0, col, 5.0);
            }
        }

        // Fort sprite.
        let (tex_width, tex_height, ang_deg, tank_spawn_timer) = {
            let e = &state.entities[idx];
            (e.tex_width, e.tex_height, e.ang, e.tank_spawn_timer)
        };
        let mut sprite = Sprite::with_texture(&assets.fort_textures[team as usize]);
        sprite.set_origin(Vector2f::new(
            tex_width as f32 / 2.0,
            tex_height as f32 / 2.0,
        ));
        sprite.set_position(Vector2f::new(x0, y0));
        if tank_spawn_timer < 0.0 {
            state.entities[idx].fort_ang += state.entities[idx].fort_ang_vel;
        }
        sprite.set_rotation(ang_deg);
        target.draw(&sprite);

        // Flicker timer update (drives the health-bar alpha above, next frame).
        {
            let e = &mut state.entities[idx];
            e.flicker_timer += e.flicker_vel;
            if e.flicker_timer >= e.max_flicker_time {
                e.flicker_vel = -1.0;
                // DEVIATION: the C++ here is `flicker_timer - max_flicker_time;`
                // -- a statement with no assignment (`-` instead of `-=`),
                // so it computes a value and discards it; `flicker_timer`
                // itself is left unchanged. Reproduced as a genuine no-op
                // (not "fixed" to `-=`), matching the observable behavior.
            }
            if e.flicker_timer <= 0.0 {
                e.flicker_vel = 1.0;
                e.flicker_timer = 0.0;
            }
        }

        // C++'s "Draw Fort Info" block is gated by `draw_fort_stats &&
        // Tank_spawn_timer > 0 && false` -- permanently disabled by the
        // `&& false`, so it never executes. Omitted entirely as dead code.

        // DEBUG_FEATURES text block is empty in the original
        // (`//---- Draw text ----//`, no actual code). Nothing to port.

        if state.event_logging {
            println!("  Fort Drawn.");
        }
    }

    state.entities[idx].col.a = 255;
    state.entities[idx].opp_col.a = 255;
}
