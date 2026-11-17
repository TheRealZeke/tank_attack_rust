//! GameDisplayText: ported from `class GameDisplayText` (TankAttack.cpp
//! L3403-3479). The `sf::Text TextObj` member is not stored for the same
//! reason as FloatingText -- built at draw time from `Font_1`.

use sfml::graphics::Color;
use sfml::system::Vector2f;

pub struct GameDisplayText {
    pub ia: bool,
    pub pos: Vector2f,
    pub max_life: f32,
    pub life: f32,
    pub shadow: bool,
    pub col: Color,
    pub size: i32,
    pub thickness: f32,
    pub text: String,
}

impl GameDisplayText {
    /// C++: GameDisplayText(sp_Text, sp_pos, sp_col, sp_size, sp_dur, sp_thickness, sp_shadow = false)
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        text: String,
        pos: Vector2f,
        col: Color,
        size: i32,
        dur: f32,
        thickness: f32,
        shadow: bool,
    ) -> Self {
        GameDisplayText {
            ia: true,
            pos,
            size,
            max_life: dur * 60.0,
            life: dur * 60.0,
            shadow,
            thickness,
            col,
            text,
        }
    }

    /// C++: void draw(sf::RenderWindow& surface)
    /// Ticks life and returns the render color; actual sf::Text
    /// construction/draw happens in render.rs.
    pub fn tick(&mut self) -> Option<Color> {
        self.life -= 1.0;
        if self.life <= 0.0 {
            self.ia = false;
            return None;
        }
        let mut a = 192.0 * 2.0 * (self.life / self.max_life);
        if a >= 192.0 {
            a = 192.0;
        }
        self.col.a = a as u8;
        Some(self.col)
    }
}

// ---------------------------------------------------------------------
// Stage D.4: GameDisplayText::draw(), ported from TankAttack.cpp
// L3441-3473.
//
// DEVIATION: the C++ `sf::Text TextObj` member (with its manual
// `.move()`/position juggling for the shadow effect) is not stored;
// built fresh each draw call from `text`/`size`, per the Stage 0.5
// pattern. The shadow offset is applied via position arithmetic instead
// of relative `Text::move`, same visual result.
// ---------------------------------------------------------------------

use crate::state::GameState;
use crate::util::dim_color;
use sfml::graphics::{Font, RenderTarget, Text, Transformable};

/// C++: void GameDisplayText::draw(sf::RenderWindow& surface)
/// (TankAttack.cpp L3441-3473).
pub fn game_display_text_draw<T: RenderTarget>(
    state: &mut GameState,
    font: &Font,
    target: &mut T,
    idx: usize,
) {
    state.game_texts[idx].life -= 1.0;
    if state.game_texts[idx].life <= 0.0 {
        state.game_texts[idx].ia = false;
        return;
    }

    let mut a = 192.0 * 2.0 * (state.game_texts[idx].life / state.game_texts[idx].max_life);
    if a >= 192.0 {
        a = 192.0;
    }
    state.game_texts[idx].col.a = a as u8;

    let (pos, text, size, col, shadow, thickness) = {
        let gt = &state.game_texts[idx];
        (
            gt.pos,
            gt.text.clone(),
            gt.size,
            gt.col,
            gt.shadow,
            gt.thickness,
        )
    };

    let mut t = Text::new(&text, font, size as u32);
    // C++'s TextObj is centered via `setOrigin` computed once at
    // construction from `getGlobalBounds()`; matched here per-frame
    // since the text never changes after construction anyway.
    let b = t.local_bounds();
    t.set_origin(Vector2f::new(
        b.left + b.width / 2.0,
        b.top + b.height / 2.0,
    ));

    if shadow {
        let mut shadow_col = dim_color(col, 25.0);
        shadow_col.a = col.a;
        t.set_position(Vector2f::new(pos.x + 2.0, pos.y + 2.0));
        t.set_fill_color(shadow_col);
        target.draw(&t);

        t.set_position(Vector2f::new(pos.x, pos.y));
        t.set_fill_color(col);
        t.set_outline_color(Color::rgba(0, 0, 0, col.a));
        t.set_outline_thickness(thickness);
        target.draw(&t);
    } else {
        t.set_position(pos);
        t.set_outline_color(Color::rgba(0, 0, 0, col.a));
        t.set_fill_color(col);
        t.set_outline_thickness(thickness);
        target.draw(&t);
    }
}
