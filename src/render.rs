//! Stage D.2: low-level draw helpers, ported from TankAttack.cpp
//! L5263-5411 (draw_line through draw_arc), L5606-5630 (draw_thick_line),
//! L4175-4204 (render_text), L6557-6650 (render_formatted_text), and
//! L4219-4222 (dim_color/opposite_color already live in util.rs; this
//! file adds `stage_color`, L4222-4247).
//!
//! Every function here is generic over `T: RenderTarget` (the trait both
//! `RenderWindow` and `RenderTexture` implement) rather than hardcoded to
//! `sf::RenderWindow` like the C++, specifically so Stage D's headless
//! smoke test can render to an off-screen `RenderTexture` instead of
//! needing a live window.
//!
//! DEVIATION: `draw_line`, `draw_filled_circle`, `draw_regular_polygon`,
//! and `render_text` each used a C++ `static` object reused across calls
//! (a `sf::VertexArray`/`sf::CircleShape`/`sf::Text` recreated fresh only
//! when a cached field changed). None of that caching affects the drawn
//! output -- every call still explicitly sets every visible property
//! (color, position, string, ...) regardless of cache hit, and
//! `render_text`'s `cachedBounds` is write-only dead state (computed, never
//! read). These are implemented as plain stateless functions instead;
//! behaviorally identical, and avoids Rust `static mut`/`thread_local`
//! ceremony for a caching layer that had no observable effect anyway.

use crate::util::hex_to_rgb;
use sfml::graphics::{
    CircleShape, Color, FloatRect, Font, PrimitiveType, RectangleShape, RenderStates, RenderTarget,
    Shape, Text, Transformable, Vertex,
};
use sfml::system::Vector2f;

/// C++: void draw_line(sf::RenderWindow& wind, sf::Vector2f pos1, sf::Vector2f pos2, sf::Color col)
pub fn draw_line<T: RenderTarget>(target: &mut T, pos1: Vector2f, pos2: Vector2f, col: Color) {
    let verts = [
        Vertex::new(pos1, col, Vector2f::new(0.0, 0.0)),
        Vertex::new(pos2, col, Vector2f::new(0.0, 0.0)),
    ];
    target.draw_primitives(&verts, PrimitiveType::LINES, &RenderStates::DEFAULT);
}

/// C++: void draw_thick_line(sf::RenderWindow& wind, sf::Vector2f pos1, sf::Vector2f pos2, float thickness, sf::Color color)
pub fn draw_thick_line<T: RenderTarget>(
    target: &mut T,
    pos1: Vector2f,
    pos2: Vector2f,
    thickness: f32,
    color: Color,
) {
    let dir = pos2 - pos1;
    let sq_len = dir.x * dir.x + dir.y * dir.y;
    if sq_len < 1e-12 {
        return;
    }
    let inv_len = 1.0 / sq_len.sqrt();
    let dir = Vector2f::new(dir.x * inv_len, dir.y * inv_len);
    let perp = Vector2f::new(-dir.y, dir.x);
    let offset = perp * (thickness * 0.5);

    let verts = [
        Vertex::new(pos1 - offset, color, Vector2f::new(0.0, 0.0)),
        Vertex::new(pos1 + offset, color, Vector2f::new(0.0, 0.0)),
        Vertex::new(pos2 - offset, color, Vector2f::new(0.0, 0.0)),
        Vertex::new(pos2 + offset, color, Vector2f::new(0.0, 0.0)),
    ];
    target.draw_primitives(
        &verts,
        PrimitiveType::TRIANGLE_STRIP,
        &RenderStates::DEFAULT,
    );
}

/// C++: void draw_filled_circle(sf::RenderWindow& myWindow, sf::Vector2f center, float radius, sf::Color col)
pub fn draw_filled_circle<T: RenderTarget>(
    target: &mut T,
    center: Vector2f,
    radius: f32,
    col: Color,
) {
    let mut circle = CircleShape::new(radius, 30);
    let bounds = circle.global_bounds();
    circle.set_origin(Vector2f::new(bounds.width / 2.0, bounds.height / 2.0));
    circle.set_fill_color(col);
    circle.set_position(center);
    target.draw(&circle);
}

/// C++: void draw_circle(sf::RenderWindow& myWindow, sf::Vector2f center, float radius, sf::Color col, int thickness)
pub fn draw_circle<T: RenderTarget>(
    target: &mut T,
    center: Vector2f,
    radius: f32,
    col: Color,
    thickness: f32,
) {
    let mut circle = CircleShape::new(radius, 30);
    let bounds = circle.global_bounds();
    circle.set_origin(Vector2f::new(bounds.width / 2.0, bounds.height / 2.0));
    circle.set_outline_color(col);
    circle.set_fill_color(Color::rgba(0, 0, 0, 0));
    circle.set_outline_thickness(thickness);
    circle.set_position(center);
    target.draw(&circle);
}

/// C++: void draw_regular_polygon(...)
#[allow(clippy::too_many_arguments)]
pub fn draw_regular_polygon<T: RenderTarget>(
    target: &mut T,
    center: Vector2f,
    radius: f32,
    col: Color,
    thickness: f32,
    sides: usize,
    rotation: f32,
    fill: bool,
) {
    let mut circle = CircleShape::new(radius, 30);
    let bounds = circle.global_bounds();
    circle.set_origin(Vector2f::new(bounds.width / 2.0, bounds.height / 2.0));
    circle.set_outline_color(col);
    circle.set_outline_thickness(thickness);
    circle.set_fill_color(Color::rgba(0, 0, 0, 0));
    if fill {
        circle.set_fill_color(col);
    }
    circle.set_position(center);
    circle.set_point_count(sides);
    circle.set_rotation(rotation);
    target.draw(&circle);
}

/// C++: void draw_rect(sf::RenderWindow& myWindow, const sf::FloatRect rect, sf::Color col, int thickness)
pub fn draw_rect<T: RenderTarget>(target: &mut T, rect: FloatRect, col: Color, thickness: f32) {
    let mut r = RectangleShape::with_size(Vector2f::new(rect.width, rect.height));
    r.set_position(Vector2f::new(rect.left, rect.top));
    r.set_outline_thickness(thickness);
    r.set_fill_color(Color::rgba(0, 0, 0, 0));
    r.set_outline_color(col);
    target.draw(&r);
}

/// C++: void draw_filled_rect(sf::RenderWindow& myWindow, const sf::FloatRect& rect, sf::Color col)
pub fn draw_filled_rect<T: RenderTarget>(target: &mut T, rect: FloatRect, col: Color) {
    let mut r = RectangleShape::with_size(Vector2f::new(rect.width, rect.height));
    r.set_position(Vector2f::new(rect.left, rect.top));
    r.set_fill_color(col);
    target.draw(&r);
}

/// C++: void draw_arc(...) (TankAttack.cpp L5331-5360). Builds a
/// triangle-strip ring segment by hand, matching the original's manual
/// vertex generation exactly (not approximated via `draw_regular_polygon`
/// or similar).
#[allow(clippy::too_many_arguments)]
pub fn draw_arc<T: RenderTarget>(
    target: &mut T,
    center: Vector2f,
    radius: f32,
    col: Color,
    thickness: f32,
    start_angle: f32,
    end_angle: f32,
    point_count: i32,
) {
    let start_rad = start_angle * (std::f32::consts::PI / 180.0);
    let end_rad = end_angle * (std::f32::consts::PI / 180.0);
    let angle_step = (end_rad - start_rad) / point_count as f32;

    let mut verts: Vec<Vertex> = Vec::with_capacity(((point_count + 1) * 2) as usize);
    for i in 0..=point_count {
        let angle = start_rad + i as f32 * angle_step;

        let outer_x = center.x + (radius + thickness / 2.0) * angle.cos();
        let outer_y = center.y + (radius + thickness / 2.0) * angle.sin();
        verts.push(Vertex::new(
            Vector2f::new(outer_x, outer_y),
            col,
            Vector2f::new(0.0, 0.0),
        ));

        let inner_x = center.x + (radius - thickness / 2.0) * angle.cos();
        let inner_y = center.y + (radius - thickness / 2.0) * angle.sin();
        verts.push(Vertex::new(
            Vector2f::new(inner_x, inner_y),
            col,
            Vector2f::new(0.0, 0.0),
        ));
    }
    target.draw_primitives(
        &verts,
        PrimitiveType::TRIANGLE_STRIP,
        &RenderStates::DEFAULT,
    );
}

/// C++: void render_text(...) (TankAttack.cpp L4175-4204). See the module
/// doc comment for why the C++'s `static` caching is dropped.
#[allow(clippy::too_many_arguments)]
pub fn render_text<T: RenderTarget>(
    target: &mut T,
    pos: Vector2f,
    text: &str,
    font: &Font,
    size: u32,
    fill_col: Color,
    outline_thickness: f32,
    outline_col: Color,
    centered: bool,
) {
    let mut t = Text::new(text, font, size);
    t.set_fill_color(fill_col);
    t.set_outline_color(outline_col);
    t.set_outline_thickness(outline_thickness);

    if centered {
        let b = t.local_bounds();
        t.set_origin(Vector2f::new(b.width / 2.0, b.height / 2.0));
    } else {
        t.set_origin(Vector2f::new(0.0, 0.0));
    }
    t.set_position(pos);

    target.draw(&t);
}

/// C++: void render_formatted_text(...) (TankAttack.cpp L6557-6650).
///
/// Markup: `{#rrggbb}[segment text]` switches the fill color for that
/// segment only (falling back to `fill_col` if the hex spec fails to
/// parse); everything else is plain text drawn in `fill_col`. Segments
/// are laid out left-to-right, each advancing by its own measured
/// on-screen width (`local_bounds().left + .width`, clamped to >= 0).
#[allow(clippy::too_many_arguments)]
pub fn render_formatted_text<T: RenderTarget>(
    target: &mut T,
    pos: Vector2f,
    text: &str,
    font: &Font,
    size: u32,
    fill_col: Color,
    outline_thickness: f32,
    outline_col: Color,
    centered: bool,
) {
    // 1) Parse Text -> segments of (text, color).
    let mut segments: Vec<(String, Color)> = Vec::new();
    let mut buffer = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0usize;

    while i < chars.len() {
        if chars[i] == '{' {
            if let Some(brace_close) = (i + 1..chars.len()).find(|&j| chars[j] == '}') {
                if brace_close + 1 < chars.len() && chars[brace_close + 1] == '[' {
                    if let Some(bracket_close) =
                        (brace_close + 2..chars.len()).find(|&j| chars[j] == ']')
                    {
                        let spec: String = chars[i + 1..brace_close].iter().collect();
                        let content: String =
                            chars[brace_close + 2..bracket_close].iter().collect();

                        if !buffer.is_empty() {
                            segments.push((std::mem::take(&mut buffer), fill_col));
                        }
                        let col = hex_to_rgb(&spec, fill_col, fill_col.a);
                        segments.push((content, col));
                        i = bracket_close + 1;
                        continue;
                    }
                }
            }
        }
        buffer.push(chars[i]);
        i += 1;
    }
    if !buffer.is_empty() {
        segments.push((buffer, fill_col));
    }

    // 2) Build sf::Text per segment for measurement and drawing.
    let mut texts: Vec<Text> = Vec::with_capacity(segments.len());
    let mut total_width = 0.0f32;
    let mut max_height = 0.0f32;

    for (seg_text, seg_col) in &segments {
        let mut t = Text::new(seg_text.as_str(), font, size);
        t.set_fill_color(*seg_col);
        t.set_outline_color(outline_col);
        t.set_outline_thickness(outline_thickness);

        let b = t.local_bounds();
        let seg_width = (b.left + b.width).max(0.0);
        let seg_height = (b.top + b.height).max(0.0);

        texts.push(t);
        total_width += seg_width;
        if seg_height > max_height {
            max_height = seg_height;
        }
    }

    // 3) Starting position (account for centering).
    let (mut start_x, start_y) = if centered {
        (pos.x - total_width * 0.5, pos.y - max_height * 0.5)
    } else {
        (pos.x, pos.y)
    };

    // 4) Draw segments left-to-right.
    for t in &mut texts {
        let b = t.local_bounds();
        let seg_width = (b.left + b.width).max(0.0);

        t.set_position(Vector2f::new(
            (start_x - b.left).round(),
            (start_y - b.top).round(),
        ));
        target.draw(t);
        start_x += seg_width;
    }
}

/// C++: sf::Color stage_color() (TankAttack.cpp L4222-4247). Averages the
/// colors of every team with a live tank or fort, dimmed to 1/8 brightness;
/// falls back to a dark gray if no team is active. `tank_pop` and
/// `fort_pop` are injected per team (via `get_tank_pop`/`get_fort_pop`)
/// rather than read from globals.
pub fn stage_color(
    number_of_teams: i32,
    color_arr: &[Color],
    tank_pop: impl Fn(i32) -> i32,
    fort_pop: impl Fn(i32) -> i32,
) -> Color {
    let mut r = 0.0f32;
    let mut g = 0.0f32;
    let mut b = 0.0f32;
    let mut active = 0;

    for team in 0..number_of_teams {
        if tank_pop(team) > 0 || fort_pop(team) > 0 {
            let c = color_arr[team as usize];
            r += c.r as f32;
            g += c.g as f32;
            b += c.b as f32;
            active += 1;
        }
    }

    if active == 0 {
        return Color::rgba(8, 8, 8, 255);
    }

    r = (r / active as f32) / 8.0;
    g = (g / active as f32) / 8.0;
    b = (b / active as f32) / 8.0;

    Color::rgb(r as u8, g as u8, b as u8)
}

// ---------------------------------------------------------------------
// Stage D.4: render_entities / render_lasers / render_particles /
// render_missiles / render_dead_tanks / render_floating_texts /
// render_game_texts / render_buttons / render_minimap / draw_ui /
// draw_logs. Ported from TankAttack.cpp L4655-5023, L4814-4965
// (minimap), L5023-5119 (draw_ui), L6516-6526 (draw_logs).
//
// These are the functions that tie the per-entity `*_draw` functions
// (entity.rs/laser.rs/missile.rs/particle.rs/dead_tank.rs/
// game_display_text.rs/button.rs/floating_text.rs) to their arrays, and
// -- matching the original -- also drive simulation (`entity.AI()`,
// `laser.move()`, ...) and cull dead entries. Kept in `render.rs` rather
// than a separate module since they're purely orchestration over the
// draw/update functions that already live in their own files.
// ---------------------------------------------------------------------

use crate::assets::Assets;
use crate::button::{button_draw, Button};
use crate::dead_tank::dead_tank_draw;
use crate::entity::{entity_ai, entity_draw, EntityType};
use crate::floating_text::floating_text_draw;
use crate::game_display_text::game_display_text_draw;
use crate::laser::{laser_draw, laser_move};
use crate::missile::{missile_draw, missile_move};
use crate::particle::particle_draw;
use crate::state::GameState;
use crate::tile::{tile_draw, tile_idle_draw};
use crate::util::{dim_color, enemies, opposite_color};

/// C++: void render_entities() (TankAttack.cpp L4655-4732).
pub fn render_entities<T: RenderTarget>(state: &mut GameState, assets: &Assets, target: &mut T) {
    if state.event_logging {
        println!("/RENDERENTITIES()/");
    }

    // Draw tanks first, then forts. A dead (!ia) tank/fort still gets its
    // outgoing lasers orphaned here, same as C++.
    for i in 0..state.entities.len() {
        if state.entities[i].entity_type != EntityType::Tank {
            continue;
        }
        if state.entities[i].ia {
            if !state.paused {
                entity_ai(state, i);
            }
            if state.game_screen_rect.contains(state.entities[i].pos) {
                entity_draw(state, assets, target, i);
            }
        } else {
            let uid = state.entities[i].unique_id;
            for l in state.lasers.iter_mut() {
                if l.sender_uid == Some(uid) {
                    l.sender_uid = None;
                    l.ia = false;
                }
            }
        }
    }

    let size = state.entities.len();
    for i in 0..size {
        if state.entities[i].entity_type == EntityType::Fort && state.entities[i].ia {
            if !state.paused {
                entity_ai(state, i);
            }
            if state.game_screen_rect.contains(state.entities[i].pos) {
                entity_draw(state, assets, target, i);
            }
        }
    }

    for i in 0..size {
        if !state.entities[i].ia {
            let pos = state.entities[i].pos;
            state.tank_death_positions.push([pos.x, pos.y]);
        }
    }

    // Erase inactive entities every 667 game ticks; null out every
    // laser's sender first (matches the C++ comment: prevents lasers
    // pointing at the wrong tank after indices shift).
    if state.game_ticks % 667 == 0 {
        for l in state.lasers.iter_mut() {
            l.sender_uid = None;
        }
        state.entities.retain(|e| e.ia);
        for l in state.lasers.iter_mut() {
            l.sender_uid = None;
        }
    }

    state.total_tank_pop = 0.0;
    for e in &state.entities {
        if e.entity_type == EntityType::Tank && e.ia {
            state.total_tank_pop += 1.0;
        }
    }

    // C++'s "shuffle EntityArr every 1259 ticks" block is gated by
    // `game_ticks % 1259 == 0 && false` -- permanently disabled by the
    // `&& false`. Omitted entirely as dead code.

    if state.event_logging {
        println!("  RENDERENTITIES() executed");
    }
}

/// C++: void render_lasers() (TankAttack.cpp L4754-4772).
pub fn render_lasers<T: RenderTarget>(state: &mut GameState, target: &mut T) {
    if state.event_logging {
        println!("/RENDERLASERS()/");
    }

    let (gang_up, victim_team) = (state.gang_up, state.victim_team);
    for i in 0..state.lasers.len() {
        if !state.paused {
            laser_move(state, i, |a, b| enemies(a, b, gang_up, victim_team));
        }
        if state.lasers[i].ia && state.game_screen_rect.contains(state.lasers[i].pos) {
            laser_draw(state, target, i);
        }
    }

    state.lasers.retain(|l| l.ia);

    if state.event_logging {
        println!("  RENDERLASERS() executed");
    }
}

/// C++: void render_particles() (TankAttack.cpp L4773-4793).
pub fn render_particles<T: RenderTarget>(state: &mut GameState, target: &mut T) {
    if state.event_logging {
        println!("RENDERPARTICLES() called...");
    }

    let fps = state.game_fps_ratio;
    let (wind_vel, wind_physics) = (state.wind_vel, state.wind_physics);
    for i in 0..state.particles.len() {
        if !state.paused {
            state.particles[i].tick(fps, wind_vel, wind_physics);
        }
        particle_draw(state, target, i);
    }

    state.particles.retain(|p| p.ia);

    if state.event_logging {
        println!("RENDERPARTICLES() executed.");
    }
}

/// C++: void render_missles() (TankAttack.cpp L4794-4813).
pub fn render_missiles<T: RenderTarget>(state: &mut GameState, target: &mut T) {
    if state.event_logging {
        println!("/RENDERMISSLES()/");
    }

    if !state.paused {
        let exploded = missile_move(state);
        for idx in exploded {
            explode_missile(state, idx);
        }
    }
    for i in 0..state.missiles.len() {
        if state.missiles[i].ia && state.game_screen_rect.contains(state.missiles[i].pos) {
            missile_draw(state, target, i);
        }
    }

    state.missiles.retain(|m| m.ia);

    if state.event_logging {
        println!("  RENDERMISSLES() executed");
    }
}

/// Actually spawns the splash-damage lasers `missile_explode_params`
/// computes. This wiring was missing entirely before: `missile_move`'s
/// returned exploded-index list was discarded at the call site, so
/// `missile_explode_params` was never invoked and missiles dealt zero
/// damage. C++: `Missle::explode()` (TankAttack.cpp L1649-1664) --
/// `spawn_laser(pos.x, pos.y, ..., dam, team, {sender_entity_index,
/// sender_unique_id}, {5, 1})`, called `no_of_proj` times.
fn explode_missile(state: &mut GameState, idx: usize) {
    use rand::Rng;
    let mut rng = state.rng.clone();
    let mut rng_calls = 0u64;
    let params = crate::missile::missile_explode_params(state, idx, |n| {
        rng_calls += 1;
        rng.gen_range(0..n)
    });
    state.rng = rng;
    state.rng_calls += rng_calls;

    if let Some(lasers) = params {
        for (posx, posy, vel, ang, dam, team, sender_uid) in lasers {
            crate::spawn::spawn_laser(
                state,
                posx,
                posy,
                vel,
                ang,
                dam,
                team,
                sender_uid,
                vec![5.0, 1.0],
            );
        }
    }
}

/// C++: void render_dead_tanks() (TankAttack.cpp L4978-4991).
pub fn render_dead_tanks<T: RenderTarget>(state: &mut GameState, assets: &Assets, target: &mut T) {
    let fps = state.game_fps_ratio;
    for i in 0..state.dead_tanks.len() {
        if !state.paused {
            state.dead_tanks[i].tick(fps);
        }
        if state.dead_tanks[i].ia && state.game_screen_rect.contains(state.dead_tanks[i].pos) {
            dead_tank_draw(state, assets, target, i);
        }
    }
    state.dead_tanks.retain(|dt| dt.ia);
}

/// C++: void render_floating_texts() (TankAttack.cpp L5004-5017).
pub fn render_floating_texts<T: RenderTarget>(
    state: &mut GameState,
    assets: &Assets,
    target: &mut T,
) {
    for i in 0..state.floating_texts.len() {
        if state.game_screen_rect.contains(state.floating_texts[i].pos) {
            floating_text_draw(state, &assets.font_1, &assets.font_2, target, i);
        } else {
            state.floating_texts[i].ia = false;
        }
    }
    state.floating_texts.retain(|ft| ft.ia);
}

/// C++: void render_game_texts(sf::RenderWindow& surface) (TankAttack.cpp
/// L4965-4976).
pub fn render_game_texts<T: RenderTarget>(state: &mut GameState, assets: &Assets, target: &mut T) {
    for i in 0..state.game_texts.len() {
        game_display_text_draw(state, &assets.font_1, target, i);
    }
    state.game_texts.retain(|gt| gt.ia);
}

/// C++: void render_buttons() (TankAttack.cpp L4992-5002).
pub fn render_buttons<T: RenderTarget>(state: &mut GameState, assets: &Assets, target: &mut T) {
    let buttons: Vec<Button> = state
        .buttons
        .iter()
        .map(|b| Button {
            pos: b.pos,
            rect: b.rect,
            colour: b.colour,
            text: b.text.clone(),
            ia: b.ia,
            action: b.action,
            text_size: b.text_size,
        })
        .collect();
    for b in &buttons {
        button_draw(b, &assets.font_2, target);
    }
    state.buttons.retain(|b| b.ia);
}

/// C++: void render_minimap() (TankAttack.cpp L4814-4962). The
/// quad-stitch math (gap-filling between adjacent hex tiles so the
/// minimap reads as a solid mosaic rather than a grid of separated
/// diamonds) is ported exactly, including its row/column-relative
/// "previous tile" bookkeeping.
pub fn render_minimap<T: RenderTarget>(state: &mut GameState, target: &mut T) {
    if state.event_logging {
        println!("/RENDERMINIMAP()/");
    }

    let mut t_col = state.stage_colour;
    t_col.a = 192;

    let inv_map_w = 1.0 / state.game_map_width;
    let inv_map_h = 1.0 / state.game_map_height;
    let width_scale = state.minimap_width * inv_map_w;
    let height_scale = state.minimap_height * inv_map_h;

    let minimap_rect = state.minimap_rect;
    draw_filled_rect(target, minimap_rect, t_col);
    draw_rect(
        target,
        minimap_rect,
        opposite_color(state.stage_colour),
        1.0,
    );

    // Draw tiles (assumes a "draw_Tiles" toggle defaulting to on; not yet
    // an exposed GameState field since nothing else reads it in this
    // port -- always drawn).
    {
        let tile_w = (state.config.graphics.tiles_size as f32 * width_scale) as i32;
        let tile_h = (state.config.graphics.tiles_size as f32 * height_scale) as i32;

        let mut verts: Vec<Vertex> = Vec::new();

        let rows = state.tiles.len();
        for i in 0..rows {
            let cols = state.tiles[i].len();
            let mut prev_t_pos;
            let mut prev_row_t_pos_y = 0.0f32;
            for j in 0..cols {
                if state.tiles[i][j].team == -1 {
                    continue;
                }
                let t_pos = state.tiles[i][j].pos;
                if j > 0 {
                    prev_t_pos = state.tiles[i][j - 1].pos;
                } else {
                    prev_t_pos = t_pos;
                }
                if i > 0 {
                    prev_row_t_pos_y = state.tiles[i - 1][j].pos.y;
                }

                let g_x = (t_pos.x * width_scale + minimap_rect.left).round() as i32;
                let g_y = (t_pos.y * height_scale + minimap_rect.top).round() as i32;
                let p_g_x = (prev_t_pos.x * width_scale + minimap_rect.left).round() as i32;
                let p_r_g_y = (prev_row_t_pos_y * height_scale + minimap_rect.top).round() as i32;

                let mut x = g_x;
                let mut y = g_y;
                let mut tile_width = tile_w;
                let mut tile_height = tile_h;

                let diff = x - (p_g_x + tile_w);
                x -= diff;
                tile_width += diff;

                if i > 0 {
                    let diff2 = y - (p_r_g_y + tile_h);
                    y -= diff2;
                    tile_height += diff2;
                }

                let mut c = state.tiles[i][j].col;
                if state.tiles[i][j].border_tile {
                    c.a = ((state.tiles[i][j].col.a as u32) * 2).min(255) as u8;
                } else {
                    c.a = (state.tiles[i][j].col.a / 2)
                        + (((state.tiles[i][j].col.a as f32 / 3.0)
                            * state.tiles[i][j].flicker_timer
                            / 50.0) as u8);
                }
                if state.tiles[i][j].soft_border_tile {
                    c.a = ((c.a as f32) * 0.5) as u8;
                }

                if state.tiles[i][j].fort_is_assimilating {
                    let (grid_x, grid_y) = (state.tiles[i][j].grid_x, state.tiles[i][j].grid_y);
                    let adjusted_x = grid_x - if grid_y % 2 == 1 { 1 } else { 0 };
                    if ((adjusted_x + grid_y) % 3 == 0 || (adjusted_x - grid_y) % 3 == 0)
                        && !state.tiles[i][j].border_tile
                    {
                        let tmp = c.a;
                        c = opposite_color(c);
                        c.a = tmp;
                    }
                }

                let (xf, yf, wf, hf) = (x as f32, y as f32, tile_width as f32, tile_height as f32);
                verts.push(Vertex::new(
                    Vector2f::new(xf, yf),
                    c,
                    Vector2f::new(0.0, 0.0),
                ));
                verts.push(Vertex::new(
                    Vector2f::new(xf + wf, yf),
                    c,
                    Vector2f::new(0.0, 0.0),
                ));
                verts.push(Vertex::new(
                    Vector2f::new(xf + wf, yf + hf),
                    c,
                    Vector2f::new(0.0, 0.0),
                ));
                verts.push(Vertex::new(
                    Vector2f::new(xf, yf + hf),
                    c,
                    Vector2f::new(0.0, 0.0),
                ));
            }
        }

        if !verts.is_empty() {
            target.draw_primitives(&verts, PrimitiveType::QUADS, &RenderStates::DEFAULT);
        }
    }

    // Draw tanks.
    for e in &state.entities {
        if e.ia && e.entity_type == EntityType::Tank {
            let g_x = (e.pos.x / state.game_map_width) * state.minimap_width + minimap_rect.left;
            let g_y = (e.pos.y / state.game_map_height) * state.minimap_height + minimap_rect.top;
            let mut col = e.col;
            if state.game_ticks % 180 < 10 && e.level > 0 {
                col = opposite_color(col);
            }
            draw_filled_rect(target, FloatRect::new(g_x - 1.0, g_y - 1.0, 2.0, 2.0), col);
        }
    }

    // Draw forts.
    for e in &state.entities {
        if e.ia && e.entity_type == EntityType::Fort {
            let g_x = (e.pos.x / state.game_map_width) * state.minimap_width + minimap_rect.left;
            let g_y = (e.pos.y / state.game_map_height) * state.minimap_height + minimap_rect.top;
            let mut col = e.col;
            if e.tank_spawn_timer < 0.0 {
                col = dim_color(col, 60.0);
            }
            if e.special_fort {
                draw_filled_rect(target, FloatRect::new(g_x - 3.0, g_y - 3.0, 6.0, 6.0), col);
                draw_rect(
                    target,
                    FloatRect::new(g_x - 3.0, g_y - 3.0, 5.0, 5.0),
                    dim_color(col, 25.0),
                    1.0,
                );
            } else {
                draw_filled_rect(target, FloatRect::new(g_x - 2.0, g_y - 2.0, 4.0, 4.0), col);
                draw_rect(
                    target,
                    FloatRect::new(g_x - 2.0, g_y - 2.0, 4.0, 4.0),
                    dim_color(e.col, 25.0),
                    1.0,
                );
            }
        }
    }

    let g_x =
        (state.camera_pos[0] * state.minimap_width / state.game_map_width) + minimap_rect.left;
    let g_y =
        (state.camera_pos[1] * state.minimap_height / state.game_map_height) + minimap_rect.top;
    let t_x = state.config.graphics.screen_width * state.minimap_width / state.game_map_width;
    let t_y = state.config.graphics.screen_height * state.minimap_height / state.game_map_height;
    draw_rect(
        target,
        FloatRect::new(g_x, g_y, t_x, t_y),
        opposite_color(state.stage_colour),
        1.0,
    );

    if state.event_logging {
        println!("  RENDERMINIMAP() executed");
    }
}

/// C++: void draw_tiles() (TankAttack.cpp L5595-5602). Not part of the
/// `render_*` naming family in the original, but belongs alongside them
/// -- iterates every tile, always running `idle_draw` (the flicker
/// animation update) but only issuing an actual draw call for tiles
/// inside `GameScreenRect`.
pub fn draw_tiles<T: RenderTarget>(state: &mut GameState, assets: &Assets, target: &mut T) {
    let rows = state.tiles.len();
    for r in 0..rows {
        let cols = state.tiles[r].len();
        for c in 0..cols {
            tile_idle_draw(state, r, c);
            if !state.game_screen_rect.contains(state.tiles[r][c].pos) {
                continue;
            }
            tile_draw(state, assets, target, r, c);
        }
    }
}

/// C++: void draw_logs() (TankAttack.cpp L6516-6526).
pub fn draw_logs<T: RenderTarget>(state: &mut GameState, font: &Font, target: &mut T) {
    const MAX_LOG_SIZE: usize = 33;
    if state.game_log_arr.len() > MAX_LOG_SIZE {
        let excess = state.game_log_arr.len() - MAX_LOG_SIZE;
        state.game_log_arr.drain(0..excess);
    }
    for (i, line) in state.game_log_arr.clone().iter().enumerate() {
        render_formatted_text(
            target,
            Vector2f::new(1750.0, i as f32 * 15.0 + 275.0),
            line,
            font,
            12,
            Color::rgba(255, 255, 255, 128),
            0.0,
            Color::rgba(0, 0, 0, 0),
            false,
        );
    }
}

/// C++: void draw_ui(sf::RenderWindow& surface) (TankAttack.cpp
/// L5023-5119).
pub fn draw_ui<T: RenderTarget>(
    state: &mut GameState,
    assets: &Assets,
    target: &mut T,
    draw_stats: bool,
) {
    let (sw, _sh) = (
        state.config.graphics.screen_width,
        state.config.graphics.screen_height,
    );

    // DEVIATION: shifted further left than the C++'s `SCREEN_WIDTH - 70`
    // (FPS) / `SCREEN_WIDTH - 75` (stats). Those anchor the *left* edge of
    // left-to-right-growing text that close to the screen's right edge,
    // so longer strings ("120", "Dead Tanks:12") clip off-screen -- likely
    // present in the original too (same numbers), but the user asked for
    // it fixed here regardless of source fidelity.
    render_text(
        target,
        Vector2f::new(5.0 + sw - 150.0, 30.0 + 5.0),
        &(state.last_fps as i32).to_string(),
        &assets.font_1,
        40,
        Color::rgba(32, 32, 32, 192),
        2.0,
        Color::rgba(0, 0, 0, 192),
        false,
    );
    render_text(
        target,
        Vector2f::new(sw - 150.0, 30.0),
        &(state.last_fps as i32).to_string(),
        &assets.font_1,
        40,
        Color::rgba(255, 255, 255, 192),
        2.0,
        Color::rgba(0, 0, 0, 192),
        false,
    );

    if draw_stats {
        // DEVIATION: "Entitities" -> "Entities" -- a genuine typo present
        // in the C++ source too (TankAttack.cpp L5034, `"Entitities: "`),
        // fixed here at the user's explicit request rather than preserved.
        let stats = [
            ("Entities: ", state.active_entity_pop()),
            ("Particles: ", state.particles.len() as i32),
            ("Lasers: ", state.lasers.len() as i32),
            ("Missles: ", state.missiles.len() as i32),
            ("Dead Tanks:", state.dead_tanks.len() as i32),
        ];
        for (i, (label, val)) in stats.iter().enumerate() {
            let text = format!("{label}{val}");
            render_text(
                target,
                Vector2f::new(sw - 220.0, 125.0 + i as f32 * 25.0),
                &text,
                &assets.font_2,
                20,
                Color::rgba(255, 255, 255, 192),
                1.0,
                Color::rgba(0, 0, 0, 192),
                false,
            );
        }
        draw_logs(state, &assets.font_2, target);
    }

    // Team fort stats.
    let mut team_list: Vec<[i32; 2]> = Vec::new();
    for i in 0..state.number_of_teams {
        let pop = state.get_fort_pop(i, false);
        if pop > 0 {
            team_list.push([i, pop]);
        }
    }
    team_list.sort_by(|a, b| crate::util::compare_team(a, b));

    let font_size = 16.0f32;
    for (i, entry) in team_list.iter().enumerate() {
        let (j, k) = (entry[0], entry[1]);
        let mut n_col = state.color_arr[j as usize];
        n_col.a = 128;

        let text = if !state.debug_features {
            let fort_count = state.get_fort_pop(j, false) - state.get_fort_pop(j, true);
            let captured = state.get_fort_pop(j, true);
            let percent = state
                .team_tile_perc
                .get(j as usize)
                .copied()
                .unwrap_or(0.0)
                .round() as i32;
            let _ = captured;
            format!("Forts: {k}[{fort_count}]   ({percent}%)")
        } else {
            let normal_forts = state.get_fort_pop(j, false)
                - state.get_fort_pop(j, true)
                - state.get_special_fort_pop(j);
            let special_forts = state.get_special_fort_pop(j);
            let captured = state.get_fort_pop(j, true);
            let perc = state.team_tile_perc.get(j as usize).copied().unwrap_or(0.0);
            format!(
                "Forts: [{normal_forts}][{special_forts}][{captured}]  ({}%)",
                crate::util::round_to_decimal_places(perc as f64, 1)
            )
        };

        render_text(
            target,
            Vector2f::new(5.0, 25.0 + font_size * 1.25 * i as f32),
            &text,
            &assets.font_2,
            font_size as u32,
            n_col,
            1.0,
            Color::rgba(0, 0, 0, n_col.a),
            false,
        );
    }

    // Team tank stats.
    let last_y = (font_size * 3.0 * 1.25) + (team_list.len() as f32 * font_size * 1.25);
    let mut tank_team_list: Vec<[i32; 2]> = Vec::new();
    for i in 0..state.number_of_teams {
        let pop = crate::entity::get_tank_pop(state, i, 0);
        if pop > 0 {
            tank_team_list.push([i, pop]);
        }
    }
    tank_team_list.sort_by(|a, b| crate::util::compare_team(a, b));

    for (i, entry) in tank_team_list.iter().enumerate() {
        let (j, k) = (entry[0], entry[1]);
        let mut n_col = state.color_arr[j as usize];
        n_col.a = 128;

        let text = if state.debug_features {
            let with_ammo = crate::entity::get_tank_pop(state, j, 1);
            let without_ammo = crate::entity::get_tank_pop(state, j, 2);
            let percent_val = if state.total_tank_pop > 0.0 {
                (crate::entity::get_tank_pop(state, j, 0) as f64) * 100.0
                    / (state.total_tank_pop as f64)
            } else {
                0.0
            };
            format!(
                "Tanks: [{with_ammo}][{without_ammo}] ({}%)",
                crate::util::round_to_decimal_places(percent_val, 1)
            )
        } else {
            format!("Tanks: {k}[{}]", state.get_special_tank_pop(j))
        };

        render_text(
            target,
            Vector2f::new(5.0, font_size * 1.25 * i as f32 + last_y),
            &text,
            &assets.font_2,
            font_size as u32,
            n_col,
            1.0,
            Color::rgba(0, 0, 0, n_col.a),
            false,
        );
    }

    render_game_texts(state, assets, target);
    render_buttons(state, assets, target);
}
