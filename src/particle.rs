//! Particle: ported from `class Particle` (TankAttack.cpp L3040-3132).

use sfml::graphics::{Color, RectangleShape, Shape, Transformable};
use sfml::system::Vector2f;

pub struct Particle {
    pub ia: bool,
    pub timer: f32,
    pub max_timer: f32,
    pub pos: Vector2f,
    pub vel: Vector2f,
    pub size: f32,
    pub range: f32,
    pub col: Color,
    /// C++ `type` (1 = line particle, 2 = rect particle). Renamed: `type` is
    /// a Rust keyword.
    pub kind: i32,
    pub windforce: f32,
}

impl Particle {
    /// C++: Particle()
    pub fn new() -> Self {
        Particle {
            ia: true,
            timer: 0.0,
            max_timer: 0.0,
            pos: Vector2f::new(0.0, 0.0),
            vel: Vector2f::new(0.0, 0.0),
            size: 1.0,
            range: 1.0,
            col: Color::rgba(255, 255, 255, 255),
            kind: 1,
            windforce: 0.0,
        }
    }

    /// C++: void spawn(sp_pos, sp_size, sp_range, sp_type, sp_durRate, sp_col)
    pub fn spawn(
        &mut self,
        sp_pos: Vector2f,
        sp_size: f32,
        sp_range: f32,
        sp_type: i32,
        sp_dur_rate: f32,
        sp_col: Color,
        default_part_timer: f32,
        mut rand_fn: impl FnMut(u32) -> u32,
    ) {
        self.pos = sp_pos;
        self.size = sp_size;
        self.range = sp_range;
        self.kind = sp_type;
        self.col = sp_col;

        let ang = (rand_fn(360) as f64) * std::f64::consts::PI / 180.0;
        let sp_vel = sp_range * (rand_fn(100) as f32) / 100.0;
        self.vel = Vector2f::new((ang.cos() as f32) * sp_vel, (ang.sin() as f32) * sp_vel);

        self.max_timer = default_part_timer * sp_dur_rate;
        self.timer = self.max_timer;
        self.windforce = 0.0;
    }

    /// C++: void move()
    pub fn tick(&mut self, game_fps_ratio: f32, wind_vel: Vector2f, wind_physics: bool) {
        if wind_physics {
            if self.kind == 2 {
                self.vel.x += wind_vel.x * game_fps_ratio;
                self.vel.y += wind_vel.y * game_fps_ratio;
            }
            let n_wind = Vector2f::new(wind_vel.x * 0.5, wind_vel.y * 0.5);
            if self.kind == 1 {
                self.vel.x += n_wind.x * game_fps_ratio;
                self.vel.y += n_wind.y * game_fps_ratio;
            }
        }

        self.pos.x += self.vel.x * game_fps_ratio;
        self.pos.y += self.vel.y * game_fps_ratio;

        self.timer -= game_fps_ratio;
        if self.timer <= 0.0 {
            self.ia = false;
        }
    }

    /// C++: void draw() -- returns render-ready data; actual SFML draw call
    /// (draw_line / RectangleShape) happens in render.rs, which owns the
    /// RenderWindow.
    pub fn draw_color(&self) -> Color {
        let mut ratio = self.timer * 255.0 / self.max_timer;
        if ratio < 0.0 {
            ratio = 0.0;
        }
        Color::rgba(self.col.r, self.col.g, self.col.b, ratio as u8)
    }
}

impl Default for Particle {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------
// Stage D.4: Particle::draw(), ported from TankAttack.cpp L3105-3132.
// DEVIATION: the C++ `myRect` member (a persistent `sf::RectangleShape`,
// used only for `kind == 2`) is built fresh each draw call instead of
// stored on the struct -- its size/position/color are fully overwritten
// on every draw regardless, so there's no behavioral difference, and it
// sidesteps giving every `Particle` an SFML shape object it may never
// use (most particles are `kind == 1`, the line variant).
// ---------------------------------------------------------------------

use crate::state::GameState;
use sfml::graphics::RenderTarget;

/// C++: void Particle::draw() (TankAttack.cpp L3105-3132).
pub fn particle_draw<T: RenderTarget>(state: &mut GameState, target: &mut T, idx: usize) {
    let camera = Vector2f::new(state.camera_pos[0], state.camera_pos[1]);

    let (pos, vel, size, kind, timer, max_timer, mut col) = {
        let p = &state.particles[idx];
        (p.pos, p.vel, p.size, p.kind, p.timer, p.max_timer, p.col)
    };

    let x1 = pos.x;
    let y1 = pos.y;
    let x2 = x1 - vel.x * 2.0 * size;
    let y2 = y1 - vel.y * 2.0 * size;

    let mut ratio = timer * 255.0 / max_timer;
    if ratio < 0.0 {
        ratio = 0.0;
    }
    col.a = ratio as u8;
    state.particles[idx].col = col;

    match kind {
        1 => {
            let p1 = Vector2f::new(x1, y1) - camera;
            let p2 = Vector2f::new(x2, y2) - camera;
            crate::render::draw_line(target, p1, p2, col);
        }
        2 => {
            let mut rect = RectangleShape::with_size(Vector2f::new(size, size));
            rect.set_position(Vector2f::new(pos.x - size / 2.0, pos.y - size / 2.0));
            rect.set_fill_color(col);
            rect.move_(-camera);
            target.draw(&rect);
        }
        _ => {}
    }
}
