//! FloatingText: ported from `class FloatingText` (TankAttack.cpp L3619-3651).
//! `sf::Font font` member becomes a `FontChoice` enum per spec §6.4 --
//! rust-sfml's `Font` isn't cheaply cloneable, so the struct stores which
//! font to use rather than owning one.

use sfml::graphics::Color;
use sfml::system::Vector2f;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FontChoice {
    Font1,
    Font2,
}

pub struct FloatingText {
    pub font_size: i32,
    pub ia: bool,
    pub text: String,
    pub timer: f32,
    pub max_timer: f32,
    pub vel: f32,
    pub pos: Vector2f,
    pub col: Color,
    pub out_col: Color,
    pub font: FontChoice,
}

impl FloatingText {
    /// C++: FloatingText(sp_text, sp_pos, sp_col, sp_outcol, sp_size, sp_dur, sp_font, sp_vel = 1)
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        text: String,
        pos: Vector2f,
        col: Color,
        out_col: Color,
        size: i32,
        dur: f32,
        font: FontChoice,
        vel: f32,
    ) -> Self {
        FloatingText {
            font_size: size,
            ia: true,
            text,
            timer: dur * 60.0,
            max_timer: dur * 60.0,
            vel,
            pos,
            col,
            out_col,
            font,
        }
    }

    /// C++: void draw(sf::RenderWindow& surface)
    /// Ticks the timer/position and returns the color to render with;
    /// actual text draw happens in render.rs (owns Font_1/Font_2 + window).
    pub fn tick(&mut self, game_fps_ratio: f32) -> Option<Color> {
        self.timer -= game_fps_ratio;
        if self.timer <= 0.0 {
            self.ia = false;
            return None;
        }
        let mut a = 192.0 * 2.0 * (self.timer / self.max_timer);
        if a >= 192.0 {
            a = 192.0;
        }
        self.col.a = a as u8;
        self.pos.y -= game_fps_ratio * self.vel;
        Some(self.col)
    }
}

// ---------------------------------------------------------------------
// Stage D.4: FloatingText::draw(), ported from TankAttack.cpp
// L3641-3651.
// ---------------------------------------------------------------------

use crate::render::render_text;
use crate::state::GameState;
use sfml::graphics::{Font, RenderTarget};

/// C++: void FloatingText::draw(sf::RenderWindow& surface) (TankAttack.cpp
/// L3641-3651). `font_1`/`font_2` are both passed in; the correct one is
/// selected via the entry's `FontChoice`.
pub fn floating_text_draw<T: RenderTarget>(
    state: &mut GameState,
    font_1: &Font,
    font_2: &Font,
    target: &mut T,
    idx: usize,
) {
    state.floating_texts[idx].timer -= state.game_fps_ratio;
    if state.floating_texts[idx].timer <= 0.0 {
        state.floating_texts[idx].ia = false;
        return;
    }

    let mut a =
        192.0 * 2.0 * (state.floating_texts[idx].timer / state.floating_texts[idx].max_timer);
    if a >= 192.0 {
        a = 192.0;
    }
    state.floating_texts[idx].col.a = a as u8;

    let (pos, text, font_size, col, out_col, font_choice, vel) = {
        let ft = &state.floating_texts[idx];
        (
            ft.pos,
            ft.text.clone(),
            ft.font_size,
            ft.col,
            ft.out_col,
            ft.font,
            ft.vel,
        )
    };
    let font = if font_choice == FontChoice::Font1 {
        font_1
    } else {
        font_2
    };
    let g_pos = Vector2f::new(pos.x - state.camera_pos[0], pos.y - state.camera_pos[1]);

    render_text(
        target,
        g_pos,
        &text,
        font,
        font_size as u32,
        col,
        1.0,
        Color::rgba(out_col.r, out_col.g, out_col.b, col.a),
        true,
    );

    state.floating_texts[idx].pos.y -= state.game_fps_ratio * vel;
}
