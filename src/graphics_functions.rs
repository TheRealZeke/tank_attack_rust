use sfml::graphics::{
    CircleShape, Color, FloatRect, Font, Image, PrimitiveType, RcFont, RcText, RectangleShape,
    RenderStates, RenderTarget, RenderWindow, Shape, Text, Transformable, Vertex
};
use sfml::cpp::FBox;
use sfml::system::Vector2f;
use std::cell::RefCell;
use std::time::Instant;

thread_local! {
    // Reusable buffer for two vertices (no heap allocation, just stack memory)
    static LINE_VERTICES: RefCell<[Vertex; 2]> = RefCell::new([Vertex::default(); 2]);
    static CACHED_CIRCLE: RefCell<CircleShape<'static>> = RefCell::new(CircleShape::new(0., 30));
    static CACHED_RECTANGLE: RefCell<RectangleShape<'static>> = RefCell::new(RectangleShape::new());
    static CACHED_FILLED_RECT: RefCell<RectangleShape<'static>> = RefCell::new(RectangleShape::new());
    static CACHED_ARC_VERTICES: RefCell<Vec<Vertex>> = RefCell::new(Vec::new());
    static CACHED_TEXT: RefCell<Option<RcText>> = RefCell::new(None);
    static FPS: RefCell<FpsState> = RefCell::new(FpsState {
        last_time: None,
        smoothed: 0.0,
    });
}

pub fn draw_line(window: &mut RenderWindow, pos_1: Vector2f, pos_2: Vector2f, col: Color) {
    LINE_VERTICES.with(|verts| {
        let mut verts = verts.borrow_mut();
        verts[0].position = pos_1;
        verts[0].color = col;
        verts[1].position = pos_2;
        verts[1].color = col;

        window.draw_primitives(
            &*verts,                  // the two vertices
            PrimitiveType::LINES,     // draw as a single line
            &RenderStates::default(), // no special states needed
        );
    });
}

pub fn draw_circle(
    window: &mut RenderWindow,
    center: Vector2f,
    radius: f32,
    col: Color,
    thickness: f32, // set_outline_thickness takes f32
) {
    let mut circle = CircleShape::new(radius, 30); // 30 points for smoothness
    let bounds = circle.global_bounds();
    circle.set_origin((bounds.width / 2.0, bounds.height / 2.0));
    circle.set_outline_color(col);
    circle.set_fill_color(Color::rgba(0, 0, 0, 0)); // transparent
    circle.set_outline_thickness(thickness);
    circle.set_position(center);
    window.draw(&circle);
}

pub fn draw_reg_polygon(
    window: &mut RenderWindow,
    center: Vector2f,
    radius: f32,
    col: Color,
    thickness: f32,
    sides: u32,
    rotation: f32,
    fill: bool,
) {
    // Create a circle shape with default 30 points; we'll set the actual point count next
    let mut circle = CircleShape::new(radius, 30);
    circle.set_point_count(sides as usize);

    let bounds = circle.global_bounds();
    circle.set_origin((bounds.width / 2.0, bounds.height / 2.0));
    circle.set_outline_color(col);
    circle.set_outline_thickness(thickness);
    circle.set_fill_color(if fill { col } else { Color::rgba(0, 0, 0, 0) });
    circle.set_position(center);
    circle.set_rotation(rotation);

    window.draw(&circle);
}

pub fn draw_reg_polygon_cached(
    window: &mut RenderWindow,
    center: Vector2f,
    radius: f32,
    col: Color,
    thickness: f32,
    sides: u32,
    rotation: f32,
    fill: bool,
) {
    CACHED_CIRCLE.with(|cell| {
        let mut circle = cell.borrow_mut();
        circle.set_radius(radius); // now the radius is always correct
        circle.set_point_count(sides as usize);
        let bounds = circle.global_bounds();
        circle.set_origin((bounds.width / 2.0, bounds.height / 2.0));
        circle.set_outline_color(col);
        circle.set_outline_thickness(thickness);
        circle.set_fill_color(if fill { col } else { Color::rgba(0, 0, 0, 0) });
        circle.set_position(center);
        circle.set_rotation(rotation);
        window.draw(&*circle);
    });
}

pub fn draw_rect(window: &mut RenderWindow, rect: &FloatRect, col: Color, thickness: f32) {
    let mut rectangle = RectangleShape::new();
    rectangle.set_position((rect.left, rect.top));
    rectangle.set_size((rect.width, rect.height));
    rectangle.set_outline_thickness(thickness);
    rectangle.set_fill_color(Color::rgba(0, 0, 0, 0));
    rectangle.set_outline_color(col);
    window.draw(&rectangle);
}

pub fn draw_rect_cached(window: &mut RenderWindow, rect: &FloatRect, col: Color, thickness: f32) {
    CACHED_RECTANGLE.with(|cell| {
        let mut rectangle = cell.borrow_mut();
        rectangle.set_position((rect.left, rect.top));
        rectangle.set_size((rect.width, rect.height));
        rectangle.set_outline_thickness(thickness);
        rectangle.set_fill_color(Color::rgba(0, 0, 0, 0));
        rectangle.set_outline_color(col);
        window.draw(&*rectangle);
    });
}

pub fn draw_filled_rect_cached(window: &mut RenderWindow, rect: &FloatRect, col: Color) {
    CACHED_FILLED_RECT.with(|cell| {
        let mut rect_shape = cell.borrow_mut();
        rect_shape.set_position((rect.left, rect.top));
        rect_shape.set_size((rect.width, rect.height));
        rect_shape.set_fill_color(col);
        // Ensure no outline interferes
        rect_shape.set_outline_thickness(0.0);
        rect_shape.set_outline_color(Color::TRANSPARENT);
        window.draw(&*rect_shape);
    });
}

pub fn draw_arc(
    window: &mut RenderWindow,
    center: Vector2f,
    radius: f32,
    col: Color,
    thickness: f32,
    start_angle: f32, // degrees
    end_angle: f32,   // degrees
    point_count: u32,
) {
    let start_rad = start_angle.to_radians();
    let end_rad = end_angle.to_radians();
    let angle_step = (end_rad - start_rad) / point_count as f32;
    let num_vertices = ((point_count + 1) * 2) as usize;

    let mut vertices = Vec::with_capacity(num_vertices);

    for i in 0..=point_count {
        let angle = start_rad + i as f32 * angle_step;
        let (sin, cos) = angle.sin_cos();

        // Outer edge
        let outer_x = center.x + (radius + thickness * 0.5) * cos;
        let outer_y = center.y + (radius + thickness * 0.5) * sin;
        vertices.push(Vertex::new(
            Vector2f::new(outer_x, outer_y),
            col,
            Vector2f::default(),
        ));

        // Inner edge
        let inner_x = center.x + (radius - thickness * 0.5) * cos;
        let inner_y = center.y + (radius - thickness * 0.5) * sin;
        vertices.push(Vertex::new(
            Vector2f::new(inner_x, inner_y),
            col,
            Vector2f::default(),
        ));
    }

    window.draw_primitives(
        &vertices,
        PrimitiveType::TRIANGLE_STRIP,
        &RenderStates::default(),
    );
}

pub fn draw_arc_cached(
    window: &mut RenderWindow,
    center: Vector2f,
    radius: f32,
    col: Color,
    thickness: f32,
    start_angle: f32,
    end_angle: f32,
    point_count: u32,
) {
    let start_rad = start_angle.to_radians();
    let end_rad = end_angle.to_radians();
    let angle_step = (end_rad - start_rad) / point_count as f32;
    let num_vertices = ((point_count + 1) * 2) as usize;

    CACHED_ARC_VERTICES.with(|cell| {
        let mut verts = cell.borrow_mut();
        verts.clear();
        verts.reserve_exact(num_vertices); // Avoid reallocation each call

        for i in 0..=point_count {
            let angle = start_rad + i as f32 * angle_step;
            let (sin, cos) = angle.sin_cos();

            // Outer
            let outer_x = center.x + (radius + thickness * 0.5) * cos;
            let outer_y = center.y + (radius + thickness * 0.5) * sin;
            verts.push(Vertex::new(
                Vector2f::new(outer_x, outer_y),
                col,
                Vector2f::default(),
            ));

            // Inner
            let inner_x = center.x + (radius - thickness * 0.5) * cos;
            let inner_y = center.y + (radius - thickness * 0.5) * sin;
            verts.push(Vertex::new(
                Vector2f::new(inner_x, inner_y),
                col,
                Vector2f::default(),
            ));
        }

        window.draw_primitives(
            &verts,
            PrimitiveType::TRIANGLE_STRIP,
            &RenderStates::default(),
        );
    });
}

pub fn draw_thick_line(
    window: &mut RenderWindow,
    pos1: Vector2f,
    pos2: Vector2f,
    thickness: f32,
    color: Color,
) {
    let mut dir = pos2 - pos1;
    let sq_len = dir.x * dir.x + dir.y * dir.y;

    // Zero-length line – nothing to draw
    if sq_len < 1e-12 {
        return;
    }

    let inv_len = 1.0 / sq_len.sqrt();
    dir.x *= inv_len;
    dir.y *= inv_len;

    let perp = Vector2f::new(-dir.y, dir.x);
    let offset = perp * (thickness * 0.5);

    let vertices = [
        Vertex::new(pos1 - offset, color, Vector2f::default()),
        Vertex::new(pos1 + offset, color, Vector2f::default()),
        Vertex::new(pos2 - offset, color, Vector2f::default()),
        Vertex::new(pos2 + offset, color, Vector2f::default()),
    ];

    window.draw_primitives(
        &vertices,
        PrimitiveType::TRIANGLE_STRIP,
        &RenderStates::default(),
    );
}

pub fn render_text(
    window: &mut RenderWindow,
    pos: Vector2f,
    text_str: &str,
    font: &Font,
    size: u32,
    fill_col: Color,
    outline_thickness: f32,
    outline_col: Color,
    centered: bool,
) {
    let mut text = Text::new(text_str, font, size);
    text.set_fill_color(fill_col);
    text.set_outline_color(outline_col);
    text.set_outline_thickness(outline_thickness);

    if centered {
        let bounds = text.local_bounds();
        text.set_origin((bounds.width / 2.0, bounds.height / 2.0));
    } else {
        text.set_origin((0.0, 0.0));
    }

    text.set_position(pos);
    window.draw(&text);
}

fn opp_col(c: Color) -> Color {
    Color::rgb(255 - c.r, 255 - c.g, 255 - c.b)
}

pub fn transform_img_col(img: &Image, tcol: Color, mode: i32) -> FBox<Image> {
    let size = img.size();

    // Safe creation of a transparent image — no unsafe needed
    let mut new_img = Image::new_solid(size.x, size.y, Color::TRANSPARENT)
        .expect("Failed to create image");

    let opp_col = opp_col(tcol);

    match mode {
        0 => {
            new_img.create_mask_from_color(Color::rgb(0, 128, 128), 0);
            for x in 0..size.x {
                for y in 0..size.y {
                    if let Some(col) = img.pixel_at(x, y) {
                        if col.r == col.g && col.g == col.b && col.r != 0 && col.r != 254 {
                            let r = col.r as f32;
                            let pixcol = if r > 127.5 {
                                let factor = (r - 127.5) / 127.5;
                                Color::rgb(
                                    (factor * tcol.r as f32) as u8,
                                    (factor * tcol.g as f32) as u8,
                                    (factor * tcol.b as f32) as u8,
                                )
                            } else {
                                let factor = (127.5 - r) / 127.5;
                                Color::rgb(
                                    (factor * (255 - tcol.r) as f32) as u8,
                                    (factor * (255 - tcol.g) as f32) as u8,
                                    (factor * (255 - tcol.b) as f32) as u8,
                                )
                            };
                            let _ = new_img.set_pixel(x, y, pixcol);
                        } else {
                            let _ = new_img.set_pixel(x, y, col);
                        }
                    }
                }
            }
        }
        1 => {
            for x in 0..size.x {
                for y in 0..size.y {
                    if let Some(col) = img.pixel_at(x, y) {
                        if (col.r == 0 && col.b != 0) || (col.b == 0 && col.r != 0) {
                            let brightness_factor = col.g as f32 / 255.0;
                            let (r, g, b) = if col.b == 0 {
                                (
                                    col.r as f32 * (tcol.r as f32 / 255.0),
                                    col.r as f32 * (tcol.g as f32 / 255.0),
                                    col.r as f32 * (tcol.b as f32 / 255.0),
                                )
                            } else {
                                // col.r == 0
                                (
                                    col.b as f32 * (opp_col.r as f32 / 255.0),
                                    col.b as f32 * (opp_col.g as f32 / 255.0),
                                    col.b as f32 * (opp_col.b as f32 / 255.0),
                                )
                            };
                            let r = (brightness_factor * (255.0 - r) + r) as u8;
                            let g = (brightness_factor * (255.0 - g) + g) as u8;
                            let b = (brightness_factor * (255.0 - b) + b) as u8;
                            let _ = new_img.set_pixel(x, y, Color::rgb(r, g, b));
                        } else {
                            let _ = new_img.set_pixel(x, y, col);
                        }
                    }
                }
            }
            new_img.create_mask_from_color(Color::rgb(255, 255, 255), 0);
        }
        _ => {}
    }

    new_img
}

struct FpsState {
    last_time: Option<Instant>,
    smoothed: f32,
}
pub fn draw_fps(
    window: &mut RenderWindow,
    font: &Font,
    font_size: u32,
    col: Color,
    outline_thickness: f32,
    outline_col: Color,
) {
    FPS.with(|cell| {
        let mut state = cell.borrow_mut();
        let now = Instant::now();

        // First frame: we can't calculate FPS yet – just remember the time.
        if let Some(last) = state.last_time {
            let delta = now - last;
            let instant_fps = 1.0 / delta.as_secs_f32();

            // Exponential moving average (smoothing factor = 0.1)
            if state.smoothed == 0.0 {
                state.smoothed = instant_fps;
            } else {
                state.smoothed = 0.9 * state.smoothed + 0.1 * instant_fps;
            }
        } else {
            state.last_time = Some(now);
            return; // skip drawing on the very first call
        }
        state.last_time = Some(now);

        // Format and position the FPS string
        let fps_str = format!("FPS: {:.0}", state.smoothed);

        // Measure width using a temporary Text (same font & size as we're drawing)
        let mut measure = Text::new(&fps_str, font, font_size);
        let width = measure.local_bounds().width;

        let window_size = window.size();
        let pos_x = window_size.x as f32 - width - 10.0;
        let pos_y = 10.0;

        // Draw using your existing render_text function (no centering)
        render_text(
            window,
            Vector2f::new(pos_x, pos_y),
            &fps_str,
            font,
            font_size,
            col,
            outline_thickness,
            outline_col,
            false,
        );
    });
}