//! Missile: ported from `class Missle` (TankAttack.cpp L1306-1673; C++ spelling
//! kept as "Missle" in comments, renamed to Missile per spec §4.2).

use crate::state::GameState;
use crate::util::check_line_collision;
use sfml::graphics::{CircleShape, Color, Shape, Transformable};
use sfml::system::Vector2f;

pub struct Missile {
    pub ia: bool,
    pub pos: Vector2f,
    pub lpos: Vector2f,
    pub vel: Vector2f,
    pub acc: Vector2f,
    pub id: i32,
    pub dam: f32,
    pub no_of_proj: i32,
    pub life: f32,
    pub last_pos: Vec<Vector2f>,
    pub team: i32,
    pub col: Color,
    pub size: f32,
    pub trail_length: i32,
    pub max_life: f32,
    pub ang: f32,
    pub splash_radius: f32,
    pub terminal_vel: f32,
    pub power: f32,
    pub stray: bool,

    // C++: Entity* target / sender
    pub sender_uid: Option<u32>,
    pub target_uid: Option<u32>,
    pub sender_entity_index_hint: Option<usize>,
    pub target_entity_index_hint: Option<usize>,

    /// C++: `sf::CircleShape projectile` -- persistent per-missile shape
    /// (a 3-point "triangle" CircleShape), reused across `spawn()`/
    /// `move()`/`draw()` the same way Tile's `circle` is. `'static`
    /// because it never has a texture set.
    pub projectile: CircleShape<'static>,
}

impl Missile {
    pub fn new() -> Self {
        Missile {
            ia: false,
            pos: Vector2f::new(0.0, 0.0),
            lpos: Vector2f::new(0.0, 0.0),
            vel: Vector2f::new(0.0, 0.0),
            acc: Vector2f::new(0.0, 0.0),
            id: -1,
            dam: 0.0,
            no_of_proj: 1,
            life: 0.0,
            last_pos: Vec::new(),
            team: -1,
            col: Color::WHITE,
            size: 0.0,
            trail_length: 10,
            max_life: 180.0,
            ang: 0.0,
            splash_radius: 0.0,
            terminal_vel: 0.0,
            power: 0.0,
            stray: false,
            sender_uid: None,
            target_uid: None,
            sender_entity_index_hint: None,
            target_entity_index_hint: None,
            projectile: CircleShape::new(0.0, 30),
        }
    }
}

impl Default for Missile {
    fn default() -> Self {
        Self::new()
    }
}

/// C++: void Missle::spawn(...)
#[allow(clippy::too_many_arguments)]
pub fn missile_spawn(
    state: &mut GameState,
    idx: usize,
    sp_pos: Vector2f,
    sp_vel: Vector2f,
    sp_dam: f32,
    sp_no_of_proj: i32,
    sp_team: i32,
    sender_uid: u32,
    target_uid: u32,
    sp_size: f32,
    sp_trail_length: i32,
    sp_splash_radius: f32,
    sp_terminal_vel: f32,
    sp_power: f32,
) {
    if sp_team < 0 || sp_team >= state.number_of_teams {
        return;
    }

    let sender_idx = state.find_entity_index_by_uid(sender_uid);
    let target_idx = state.find_entity_index_by_uid(target_uid);

    let (sender_idx, target_idx) = match (sender_idx, target_idx) {
        (Some(s), Some(t)) => (s, t),
        _ => {
            state.missiles[idx].ia = false;
            return;
        }
    };

    let m = &mut state.missiles[idx];
    m.sender_uid = Some(sender_uid);
    m.target_uid = Some(target_uid);
    m.sender_entity_index_hint = Some(sender_idx);
    m.target_entity_index_hint = Some(target_idx);
    m.ia = true;
    m.dam = sp_dam;
    m.no_of_proj = sp_no_of_proj;
    m.pos = sp_pos;
    m.lpos = m.pos;
    m.vel = sp_vel;
    m.team = sp_team;
    m.col = state.color_arr[sp_team as usize];
    m.life = m.max_life;
    m.trail_length = sp_trail_length;
    m.splash_radius = sp_splash_radius;
    m.size = sp_size;
    m.terminal_vel = sp_terminal_vel;
    m.power = sp_power;

    // C++: projectile.setRadius(size); setOrigin({size,size});
    // setPointCount(3); setOutlineThickness(1); (Missle::spawn L1410-1413)
    m.projectile.set_radius(sp_size);
    m.projectile.set_origin(Vector2f::new(sp_size, sp_size));
    m.projectile.set_point_count(3);
    m.projectile.set_outline_thickness(1.0);
}

/// C++: void Missle::move()
/// Homing acceleration + trail update + collision check, ported 1:1 minus
/// particle-spawn/sound calls (see laser.rs note -- same TODO applies).
/// Returns true if the missile exploded this tick (caller should call
/// `missile_explode`).
pub fn missile_move(state: &mut GameState) -> Vec<usize> {
    let mut exploded = Vec::new();
    let ratio = state.game_fps_ratio;
    let forts_attack_forts = state.config.rules.forts_attack_forts;

    for idx in 0..state.missiles.len() {
        {
            let m = &mut state.missiles[idx];
            m.life -= ratio;
            if m.life <= 0.0 {
                m.ia = false;
            }
        }
        if !state.missiles[idx].ia {
            exploded.push(idx);
            continue;
        }

        // Re-validate sender.
        let sender_uid = match state.missiles[idx].sender_uid {
            Some(u) if (u as i64) >= 0 => u,
            _ => {
                state.missiles[idx].ia = false;
                continue;
            }
        };
        let sender_idx = match state.missiles[idx]
            .sender_entity_index_hint
            .filter(|&i| i < state.entities.len() && state.entities[i].unique_id == sender_uid)
        {
            Some(i) => i,
            None => match state.find_entity_index_by_uid(sender_uid) {
                Some(i) => i,
                None => {
                    state.missiles[idx].ia = false;
                    continue;
                }
            },
        };
        state.missiles[idx].sender_entity_index_hint = Some(sender_idx);

        // Re-validate target (target on same team or dead => stray, keeps flying).
        let team = state.missiles[idx].team;
        let target_uid = state.missiles[idx].target_uid;
        let mut target_idx: Option<usize> = None;
        if let Some(tuid) = target_uid {
            target_idx = state.missiles[idx].target_entity_index_hint.filter(|&i| {
                i < state.entities.len()
                    && state.entities[i].unique_id == tuid
                    && state.entities[i].team != team
                    && state.entities[i].ia
            });
            if target_idx.is_none() {
                target_idx = state
                    .entities
                    .iter()
                    .position(|e| e.unique_id == tuid && e.team != team && e.ia);
                if target_idx.is_some() {
                    state.missiles[idx].stray = false;
                } else {
                    state.missiles[idx].target_uid = None;
                    state.missiles[idx].target_entity_index_hint = None;
                    state.missiles[idx].stray = true;
                }
            }
        } else {
            state.missiles[idx].stray = true;
        }
        state.missiles[idx].target_entity_index_hint = target_idx;

        // Angle + speed clamp.
        {
            let m = &mut state.missiles[idx];
            let dx = m.vel.x;
            let mut dy = m.vel.y;
            if dy.abs() < 0.0001 {
                dy = if dy >= 0.0 { 0.0001 } else { -0.0001 };
            }
            let t_ang = 360.0 - ((dx / dy).atan() * 180.0 / std::f32::consts::PI);
            m.ang = if dy <= 0.0 {
                t_ang - 90.0
            } else {
                t_ang + 90.0
            };

            let mut t_vel = (m.vel.x * m.vel.x + m.vel.y * m.vel.y).sqrt();
            if t_vel == 0.0 {
                t_vel = 0.0001;
            }
            if t_vel > m.terminal_vel {
                let r = m.terminal_vel / t_vel;
                m.vel.x *= r;
                m.vel.y *= r;
            }

            if m.life <= 0.0 {
                m.ia = false;
            }

            // Trail update.
            if (m.last_pos.len() as i32) < m.trail_length {
                m.last_pos.push(m.pos);
            } else if !m.last_pos.is_empty() {
                let n = m.last_pos.len();
                for i in 0..n - 1 {
                    m.last_pos[i] = m.last_pos[i + 1];
                }
                let last = n - 1;
                m.last_pos[last] = m.pos;
            }
        }

        // Homing acceleration toward target.
        if let Some(t_idx) = target_idx {
            let t_pos = state.entities[t_idx].pos;
            let m = &mut state.missiles[idx];
            let disx = (m.pos.x - t_pos.x).abs();
            let disy = (m.pos.y - t_pos.y).abs();
            if m.pos.x < t_pos.x {
                m.acc.x = 0.05 * disx.sqrt() * m.power;
            } else if m.pos.x > t_pos.x {
                m.acc.x = -0.05 * disx.sqrt() * m.power;
            }
            if m.pos.y < t_pos.y {
                m.acc.y = 0.05 * disy.sqrt() * m.power;
            } else if m.pos.y > t_pos.y {
                m.acc.y = -0.05 * disy.sqrt() * m.power;
            }
        }

        {
            let m = &mut state.missiles[idx];
            m.lpos = m.pos;
            m.vel.x += m.acc.x * ratio;
            m.vel.y += m.acc.y * ratio;
            m.pos.x += m.vel.x * ratio;
            m.pos.y += m.vel.y * ratio;
            // C++: projectile.setPosition(pos); (Missle::move, L1623)
            m.projectile.set_position(m.pos);
        }

        // C++: `if (rand() % 100 <= 25 * GameFPSRatio) { spawn_particle(pos,
        // 3, 1, 2, 1, col); spawn_particle(pos, 3, 1, 2, 1,
        // opposite_color(col)); }` (Missle::move, L1624-1627).
        if state.rand_range(0..100) as f32 <= 25.0 * ratio {
            let (pos, col) = {
                let m = &state.missiles[idx];
                (m.pos, m.col)
            };
            crate::spawn::spawn_particle(state, pos, 3.0, 1.0, 2, 1.0, col);
            crate::spawn::spawn_particle(
                state,
                pos,
                3.0,
                1.0,
                2,
                1.0,
                crate::util::opposite_color(col),
            );
        }

        // Collision check against all enemy entities.
        let (pos, lpos, mteam) = {
            let m = &state.missiles[idx];
            (m.pos, m.lpos, m.team)
        };
        let sender_type = state.entities[sender_idx].entity_type;
        let mut hit = false;
        for i in 0..state.entities.len() {
            let (e_ia, e_team, e_type) = {
                let e = &state.entities[i];
                (e.ia, e.team, e.entity_type)
            };
            if e_ia && e_team != mteam {
                if forts_attack_forts
                    || sender_type != crate::entity::EntityType::Fort
                    || e_type == crate::entity::EntityType::Tank
                {
                    let rect = state.entities[i].collision_rect();
                    if check_line_collision(rect, pos, lpos) {
                        hit = true;
                        break;
                    }
                }
            }
        }
        if hit {
            state.missiles[idx].ia = false;
            exploded.push(idx);
        }
    }

    exploded
}

/// C++: void Missle::explode()
/// Returns the laser-spawn parameters for `no_of_proj` lasers; the caller
/// (which owns `state.lasers`) is responsible for actually pushing them,
/// since spawning needs a free slot / new Laser::new() + laser_spawn call.
pub fn missile_explode_params(
    state: &GameState,
    idx: usize,
    mut rand_fn: impl FnMut(u32) -> u32,
) -> Option<Vec<(f32, f32, f32, f32, f32, i32, u32)>> {
    // (posx, posy, vel, ang_deg, dam, team, sender_uid) per laser
    let m = &state.missiles[idx];
    let sender_uid = m.sender_uid?;
    let mut out = Vec::with_capacity(m.no_of_proj as usize);
    for _ in 0..m.no_of_proj {
        let vel = (m.splash_radius * 0.75) + ((rand_fn(25) as f32) * m.splash_radius / 100.0);
        let ang = rand_fn(360) as f32;
        out.push((m.pos.x, m.pos.y, vel, ang, m.dam, m.team, sender_uid));
    }
    Some(out)
}

// ---------------------------------------------------------------------
// Stage D.4: Missle::draw(), ported from TankAttack.cpp L1420-1463.
// ---------------------------------------------------------------------

use crate::render::draw_thick_line;
use sfml::graphics::RenderTarget;

/// C++: void Missle::draw() (TankAttack.cpp L1420-1463).
pub fn missile_draw<T: RenderTarget>(state: &mut GameState, target: &mut T, idx: usize) {
    let camera = Vector2f::new(state.camera_pos[0], state.camera_pos[1]);

    // Draw trail.
    let last_pos = state.missiles[idx].last_pos.clone();
    let trail_length = state.missiles[idx].trail_length as f32;
    let size = state.missiles[idx].size;
    let col = state.missiles[idx].col;

    if last_pos.len() > 1 {
        for i in 0..last_pos.len() - 1 {
            let t_pos = last_pos[i] - camera;
            let t_pos2 = last_pos[i + 1] - camera;
            let ratio = (i as f32 / trail_length) * 192.0;
            let mut t_col = col;
            t_col.a = (ratio + 63.0) as u8;
            draw_thick_line(target, t_pos, t_pos2, size / 2.0, t_col);
        }
    }

    // Draw projectile.
    let (life, max_life, ang) = {
        let m = &state.missiles[idx];
        (m.life, m.max_life, m.ang)
    };
    let mut ratio = life * 2.0 * 255.0 / max_life;
    ratio = ratio.clamp(0.0, 255.0);

    let mut t_col = col;
    t_col.a = ratio as u8;

    let m = &mut state.missiles[idx];
    m.projectile.set_fill_color(t_col);
    let outline_col = {
        let mut c = crate::util::dim_color(t_col, 25.0);
        c.a = ratio as u8;
        c
    };
    m.projectile.set_outline_color(outline_col);
    m.projectile.move_(-camera);
    m.projectile.set_rotation(ang - 30.0);

    target.draw(&m.projectile);

    m.projectile.move_(camera);
}
