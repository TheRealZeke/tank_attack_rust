//! DeadTank: ported from `class DeadTank` (TankAttack.cpp L3480-3529).
//! `sf::Sprite sprite` is not stored (avoids a self-referential borrow on
//! the texture atlas); it's constructed at draw time from
//! `DeadTankTextArr[team][kind]`, per the deviation noted in tile.rs/entity.rs.

use sfml::graphics::Color;
use sfml::system::Vector2f;

pub struct DeadTank {
    pub life: f32,
    pub pos: Vector2f,
    pub ang: f32,
    /// C++ `type` (0 or 1, selects DeadTank01/02 texture). Renamed: keyword.
    pub kind: i32,
    pub ia: bool,
    pub team: i32,
    pub col: Color,
}

impl DeadTank {
    /// C++: DeadTank()
    pub fn new(mut rand_fn: impl FnMut(u32) -> u32) -> Self {
        DeadTank {
            life: 0.0,
            pos: Vector2f::new(0.0, 0.0),
            ang: 0.0,
            kind: (rand_fn(2)) as i32,
            ia: false,
            team: 0,
            col: Color::rgba(0, 0, 0, 0),
        }
    }

    /// C++: void spawn(sf::Vector2f s_pos, float s_ang, int s_team)
    pub fn spawn(
        &mut self,
        s_pos: Vector2f,
        s_ang: f32,
        s_team: i32,
        dead_tank_life: f32,
        col_arr: &[Color],
    ) {
        self.pos = s_pos;
        self.ang = s_ang;
        self.team = s_team;
        self.life = dead_tank_life;
        self.ia = true;
        self.col = crate::util::opposite_color(col_arr[s_team as usize]);
    }

    /// C++: void AI()
    pub fn tick(&mut self, game_fps_ratio: f32) {
        self.life -= game_fps_ratio;
        if self.life <= 0.0 {
            self.ia = false;
        }
    }

    /// C++: void draw(sf::RenderWindow& surface) -- returns the alpha ratio
    /// used for the sprite tint; actual sprite construction/draw happens in
    /// the free function `dead_tank_draw` below (needs the texture atlas +
    /// RenderTarget, consistent with Entity/Tile's Stage D.3 pattern).
    pub fn draw_alpha(&self, dead_tank_life: f32) -> u8 {
        (160.0 * (self.life / dead_tank_life)) as u8
    }
}

// ---------------------------------------------------------------------
// Stage D.3: DeadTank::draw(), ported from TankAttack.cpp L3511-3529.
// Like Entity::draw(), this has real side effects (spawn_particle) mixed
// into the render path in the original -- preserved, not split out.
// ---------------------------------------------------------------------

use crate::assets::Assets;
use crate::state::GameState;
use sfml::graphics::{RenderTarget, Sprite, Transformable};

/// C++: void DeadTank::draw(sf::RenderWindow& surface) (TankAttack.cpp
/// L3511-3529).
pub fn dead_tank_draw<T: RenderTarget>(
    state: &mut GameState,
    assets: &Assets,
    target: &mut T,
    idx: usize,
) {
    let dead_tank_life = state.config.graphics.dead_tank_ticks as f32;
    let (life, ang, pos, team, kind, col) = {
        let dt = &state.dead_tanks[idx];
        (dt.life, dt.ang, dt.pos, dt.team, dt.kind, dt.col)
    };

    let ratio = 160.0 * (life / dead_tank_life);

    let mut sprite = Sprite::with_texture(&assets.dead_tank_textures[team as usize][kind as usize]);
    let b = sprite.local_bounds();
    sprite.set_origin(Vector2f::new(b.width / 2.0, b.height / 2.0));
    sprite.set_rotation(ang + 90.0);
    sprite.set_position(pos);
    sprite.move_(Vector2f::new(-state.camera_pos[0], -state.camera_pos[1]));

    let mut n_col = sprite.color();
    n_col.a = ratio as u8;
    sprite.set_color(n_col);

    target.draw(&sprite);

    let ratio2 = life / dead_tank_life;
    if state.rand_range(0..1000) as f32 <= 50.0 * state.game_fps_ratio * ratio2 {
        crate::spawn::spawn_particle(state, pos, 3.0, 1.0, 2, 1.0, col);
    }
    if state.rand_range(0..1000) as f32 <= 25.0 * state.game_fps_ratio * ratio2 {
        crate::spawn::spawn_particle(state, pos, 3.0, 1.0, 1, 2.0, col);
    }
}
