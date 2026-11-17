//! Integration tests for Stage D.2 (render.rs draw helpers), per
//! continuation spec's tapered verification: render to an off-screen
//! `RenderTexture` and check specific pixels, since these are visual
//! primitives with no other meaningful headless assertion.

use tank_attack_rust::assets::Assets;
use tank_attack_rust::config::Config;
use tank_attack_rust::render::*;
use sfml::graphics::{Color, FloatRect, RenderTarget, RenderTexture, Texture};
use sfml::system::Vector2f;

const W: u32 = 200;
const H: u32 = 200;

fn blank_texture() -> RenderTexture {
    let mut rt = RenderTexture::new(W, H).expect("RenderTexture::new should succeed under Xvfb");
    rt.clear(Color::BLACK);
    rt
}

fn pixel_at(rt: &RenderTexture, x: u32, y: u32) -> Color {
    rt.display();
    let tex: &Texture = rt.texture();
    let img = tex.copy_to_image().expect("copy_to_image should succeed");
    unsafe { img.pixel_at(x, y) }
}

#[test]
fn draw_line_colors_the_pixels_it_passes_through() {
    let mut rt = blank_texture();
    draw_line(&mut rt, Vector2f::new(10.0, 100.0), Vector2f::new(190.0, 100.0), Color::RED);
    let p = pixel_at(&rt, 100, 100);
    assert_eq!((p.r, p.g, p.b), (255, 0, 0), "midpoint of a horizontal red line should be red, got {p:?}");
    // A point far from the line should still be background black.
    let bg = pixel_at(&rt, 10, 10);
    assert_eq!((bg.r, bg.g, bg.b), (0, 0, 0));
}

#[test]
fn draw_filled_circle_fills_center_and_leaves_corners_blank() {
    let mut rt = blank_texture();
    draw_filled_circle(&mut rt, Vector2f::new(100.0, 100.0), 50.0, Color::GREEN);
    let center = pixel_at(&rt, 100, 100);
    assert_eq!((center.r, center.g, center.b), (0, 255, 0), "circle center should be filled green, got {center:?}");
    let corner = pixel_at(&rt, 5, 5);
    assert_eq!((corner.r, corner.g, corner.b), (0, 0, 0), "far corner should remain background black, got {corner:?}");
}

#[test]
fn draw_circle_outline_leaves_center_hollow() {
    let mut rt = blank_texture();
    draw_circle(&mut rt, Vector2f::new(100.0, 100.0), 50.0, Color::BLUE, 4.0);
    let center = pixel_at(&rt, 100, 100);
    assert_eq!((center.r, center.g, center.b), (0, 0, 0), "outline-only circle should leave its center unfilled, got {center:?}");
    // The ring itself: sample directly on the circle's edge (radius=50 to
    // the right of center).
    let edge = pixel_at(&rt, 150, 100);
    assert_eq!((edge.r, edge.g, edge.b), (0, 0, 255), "circle outline should be blue at the edge, got {edge:?}");
}

#[test]
fn draw_filled_rect_fills_exact_bounds() {
    let mut rt = blank_texture();
    draw_filled_rect(&mut rt, FloatRect::new(50.0, 50.0, 100.0, 100.0), Color::rgb(255, 255, 0));
    let inside = pixel_at(&rt, 100, 100);
    assert_eq!((inside.r, inside.g, inside.b), (255, 255, 0), "inside the rect should be filled, got {inside:?}");
    let outside = pixel_at(&rt, 10, 10);
    assert_eq!((outside.r, outside.g, outside.b), (0, 0, 0), "outside the rect should stay background, got {outside:?}");
}

#[test]
fn draw_rect_outline_leaves_interior_hollow() {
    let mut rt = blank_texture();
    draw_rect(&mut rt, FloatRect::new(50.0, 50.0, 100.0, 100.0), Color::rgb(255, 0, 255), 5.0);
    let interior = pixel_at(&rt, 100, 100);
    assert_eq!((interior.r, interior.g, interior.b), (0, 0, 0), "outline-only rect should leave its interior unfilled, got {interior:?}");
    // SFML's rectangle outline is drawn *outward* from the shape's fill
    // boundary by `thickness` px, not centered on it -- so for a rect at
    // x=50 with a 5px outline, the colored band is x in [45, 50), not
    // around x=50 itself. Sample inside that outward band.
    let edge = pixel_at(&rt, 47, 100);
    assert_eq!((edge.r, edge.g, edge.b), (255, 0, 255), "rect outline should be visible in its outward-drawn band, got {edge:?}");
}

#[test]
fn draw_arc_draws_a_ring_segment_not_a_full_circle() {
    let mut rt = blank_texture();
    // Arc from 0 to 90 degrees only (SFML's y-axis points down, so this
    // sweeps from "right" to "down").
    draw_arc(&mut rt, Vector2f::new(100.0, 100.0), 50.0, Color::rgb(0, 255, 255), 6.0, 0.0, 90.0, 16);

    // A point on the arc (angle ~45 degrees into the swept range) should be colored.
    let on_arc_x = 100.0 + 50.0 * (std::f32::consts::PI / 4.0).cos();
    let on_arc_y = 100.0 + 50.0 * (std::f32::consts::PI / 4.0).sin();
    let on_arc = pixel_at(&rt, on_arc_x as u32, on_arc_y as u32);
    assert_eq!((on_arc.r, on_arc.g, on_arc.b), (0, 255, 255), "point on the swept arc should be colored, got {on_arc:?}");

    // A point on the opposite side of the circle (180 degrees away, well
    // outside the 0-90 sweep) should remain background.
    let off_arc = pixel_at(&rt, 50, 100);
    assert_eq!((off_arc.r, off_arc.g, off_arc.b), (0, 0, 0), "point outside the swept range should stay background, got {off_arc:?}");
}

#[test]
fn render_text_draws_something_nonblank() {
    let mut rt = blank_texture();
    let config = Config::load();
    let color_arr: Vec<Color> = config.teams.colors.iter().map(|c| Color::rgb(c[0], c[1], c[2])).collect();
    let assets = Assets::load(&config, &color_arr).expect("assets should load");

    render_text(&mut rt, Vector2f::new(100.0, 100.0), "HELLO", &assets.font_1, 40, Color::WHITE, 0.0, Color::BLACK, true);

    rt.display();
    let tex = rt.texture();
    let img = tex.copy_to_image().expect("copy_to_image should succeed");
    let mut any_nonblack = false;
    for x in 0..W {
        for y in 0..H {
            let p = unsafe { img.pixel_at(x, y) };
            if p.r != 0 || p.g != 0 || p.b != 0 {
                any_nonblack = true;
                break;
            }
        }
        if any_nonblack {
            break;
        }
    }
    assert!(any_nonblack, "render_text drew nothing visible onto the texture");
}

#[test]
fn render_formatted_text_applies_embedded_color_tags() {
    let mut rt = blank_texture();
    let config = Config::load();
    let color_arr: Vec<Color> = config.teams.colors.iter().map(|c| Color::rgb(c[0], c[1], c[2])).collect();
    let assets = Assets::load(&config, &color_arr).expect("assets should load");

    // "{#00ff00}[X]" should render the letter X in green, distinct from
    // the surrounding white default-color text.
    render_formatted_text(&mut rt, Vector2f::new(100.0, 100.0), "A{#00ff00}[X]B", &assets.font_1, 60, Color::WHITE, 0.0, Color::BLACK, true);

    rt.display();
    let tex = rt.texture();
    let img = tex.copy_to_image().expect("copy_to_image should succeed");
    let mut found_green = false;
    let mut found_white = false;
    for x in 0..W {
        for y in 0..H {
            let p = unsafe { img.pixel_at(x, y) };
            if p.g > 150 && p.r < 100 && p.b < 100 {
                found_green = true;
            }
            if p.r > 200 && p.g > 200 && p.b > 200 {
                found_white = true;
            }
        }
    }
    assert!(found_green, "expected at least one green pixel from the {{#00ff00}}[X] segment");
    assert!(found_white, "expected at least one white pixel from the default-color 'A'/'B' segments");
}

#[test]
fn stage_color_falls_back_when_no_teams_active_and_averages_when_active() {
    use tank_attack_rust::render::stage_color;
    let colors = vec![Color::rgb(255, 0, 0), Color::rgb(0, 0, 255)];

    let none_active = stage_color(2, &colors, |_| 0, |_| 0);
    assert_eq!((none_active.r, none_active.g, none_active.b), (8, 8, 8));

    let one_active = stage_color(2, &colors, |t| if t == 0 { 5 } else { 0 }, |_| 0);
    // team 0's red (255,0,0) / 8 = 31 (integer division/truncation via `as u8`).
    assert_eq!((one_active.r, one_active.g, one_active.b), (31, 0, 0));
}
