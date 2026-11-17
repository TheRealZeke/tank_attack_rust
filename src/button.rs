//! Button: ported from `class Button` (TankAttack.cpp L3535-3617).
//! `fire()`'s side effects (quit, regenerate map, delete entities, toggle
//! gang-up, select summon team) are represented as a `ButtonAction` enum
//! returned to the caller, rather than mutating globals directly -- the
//! caller (input.rs, not yet ported) applies it to `GameState`.

use sfml::graphics::{Color, FloatRect};
use sfml::system::Vector2f;

pub struct Button {
    pub pos: Vector2f,
    pub rect: FloatRect,
    pub colour: Color,
    pub text: String,
    pub ia: bool,
    pub action: i32,
    pub text_size: i32,
}

impl Button {
    /// C++: Button(sp_pos, sp_size, sp_text, sp_colour, sp_action, sp_textsize)
    pub fn new(
        pos: Vector2f,
        size: Vector2f,
        text: String,
        colour: Color,
        action: i32,
        text_size: i32,
    ) -> Self {
        Button {
            pos,
            rect: FloatRect::new(pos.x, pos.y, size.x, size.y),
            colour,
            text,
            ia: true,
            action,
            text_size,
        }
    }
}

pub enum ButtonAction {
    QuitGame,
    RegenerateMap,
    DeleteEntities,
    SetSummonTeamRandom,
    ToggleGangUp,
    SetSummonTeam(i32),
    None,
}

/// C++: void Button::fire()
pub fn button_fire(action: i32, number_of_teams: i32) -> ButtonAction {
    if action == 0 {
        ButtonAction::QuitGame
    } else if action == 1 {
        ButtonAction::RegenerateMap
    } else if action == number_of_teams + 2 {
        ButtonAction::DeleteEntities
    } else if action == number_of_teams + 3 {
        ButtonAction::SetSummonTeamRandom
    } else if action == number_of_teams + 4 {
        ButtonAction::ToggleGangUp
    } else if action > 1 && action < number_of_teams + 2 {
        ButtonAction::SetSummonTeam(action - 2)
    } else {
        ButtonAction::None
    }
}

// ---------------------------------------------------------------------
// Stage D.4: Button::draw(), ported from TankAttack.cpp L3564-3572.
// DEVIATION: `TextObj` not stored (Stage 0.5 pattern); built fresh each
// draw call and centered the same way the C++ constructor computed its
// one-time position (via `getGlobalBounds()`).
// ---------------------------------------------------------------------

use crate::render::{draw_filled_rect, draw_rect};
use sfml::graphics::{Font, RenderTarget, Text, Transformable};

/// C++: void Button::draw() (TankAttack.cpp L3564-3572).
pub fn button_draw<T: RenderTarget>(button: &Button, font: &Font, target: &mut T) {
    let mut n_colour = button.colour;
    n_colour.a = 85;
    draw_filled_rect(target, button.rect, n_colour);
    draw_rect(target, button.rect, button.colour, 2.0);

    let mut t = Text::new(&button.text, font, button.text_size as u32);
    let b = t.global_bounds();
    let x = button.pos.x + (button.rect.width / 2.0) - (b.width / 2.0);
    let y = button.pos.y + (button.rect.height / 2.0) - (b.height / 2.0);
    t.set_fill_color(button.colour);
    t.set_position(Vector2f::new(x, y));
    target.draw(&t);
}
