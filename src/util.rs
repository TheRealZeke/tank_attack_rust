//! Standalone math / color / collision helpers.
//! Ported 1:1 from the free functions in TankAttack.cpp (lines ~4200-6740).

use sfml::graphics::{Color, FloatRect};
use sfml::system::Vector2f;
use std::f64::consts::PI as PI_F64;

/// C++: bool compare_team(const std::vector<int>& team1, const std::vector<int> team2)
/// Used as a sort comparator: team1[1] > team2[1] (descending by 2nd element).
pub fn compare_team(team1: &[i32], team2: &[i32]) -> std::cmp::Ordering {
    team2[1].cmp(&team1[1])
}

/// C++: sf::Color dim_color(sf::Color col, float perc)
pub fn dim_color(col: Color, perc: f32) -> Color {
    let p = perc / 100.0;
    Color::rgba(
        (col.r as f32 * p) as u8,
        (col.g as f32 * p) as u8,
        (col.b as f32 * p) as u8,
        255,
    )
}

/// C++: sf::Color opposite_color(const sf::Color& col)
pub fn opposite_color(col: Color) -> Color {
    Color::rgba(255 - col.r, 255 - col.g, 255 - col.b, 255)
}

/// C++: float displacement(const sf::Vector2f& vec_1, const sf::Vector2f& vec_2)
pub fn displacement(vec_1: Vector2f, vec_2: Vector2f) -> f32 {
    let diff = vec_2 - vec_1;
    (diff.x * diff.x + diff.y * diff.y).sqrt()
}

/// C++: bool line_segment_intersection(p1, p2, q1, q2)
pub fn line_segment_intersection(p1: Vector2f, p2: Vector2f, q1: Vector2f, q2: Vector2f) -> bool {
    fn cross(v1: Vector2f, v2: Vector2f) -> f32 {
        v1.x * v2.y - v1.y * v2.x
    }
    let r = p2 - p1;
    let s = q2 - q1;

    let rxs = cross(r, s);
    let qpxr = cross(q1 - p1, r);

    if rxs == 0.0 && qpxr == 0.0 {
        return false; // Collinear
    }
    if rxs == 0.0 && qpxr != 0.0 {
        return false; // Parallel
    }

    let t = cross(q1 - p1, s) / rxs;
    let u = qpxr / rxs;

    (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)
}

/// C++: bool check_line_collision(sf::FloatRect rect, sf::Vector2f pos1, sf::Vector2f pos2)
pub fn check_line_collision(rect: FloatRect, pos1: Vector2f, pos2: Vector2f) -> bool {
    if rect.contains(pos1) || rect.contains(pos2) {
        return true;
    }

    let top_left = Vector2f::new(rect.left, rect.top);
    let top_right = Vector2f::new(rect.left + rect.width, rect.top);
    let bottom_left = Vector2f::new(rect.left, rect.top + rect.height);
    let bottom_right = Vector2f::new(rect.left + rect.width, rect.top + rect.height);

    line_segment_intersection(pos1, pos2, top_left, top_right)
        || line_segment_intersection(pos1, pos2, top_right, bottom_right)
        || line_segment_intersection(pos1, pos2, bottom_right, bottom_left)
        || line_segment_intersection(pos1, pos2, bottom_left, top_left)
}

/// C++: sf::Vector2f intercept_point(shooterPos, targetPos, targetVel, projectileSpeed)
pub fn intercept_point(
    shooter_pos: Vector2f,
    target_pos: Vector2f,
    target_vel: Vector2f,
    projectile_speed: f32,
) -> Vector2f {
    let relative_pos = target_pos - shooter_pos;

    let a = target_vel.x * target_vel.x + target_vel.y * target_vel.y
        - projectile_speed * projectile_speed;
    let b = 2.0 * (relative_pos.x * target_vel.x + relative_pos.y * target_vel.y);
    let c = relative_pos.x * relative_pos.x + relative_pos.y * relative_pos.y;

    let discriminant = b * b - 4.0 * a * c;

    if discriminant < 0.0 {
        return target_pos + target_vel;
    }

    let sqrt_discriminant = discriminant.sqrt();

    let t1 = (-b + sqrt_discriminant) / (2.0 * a);
    let t2 = (-b - sqrt_discriminant) / (2.0 * a);

    let t = if t1 >= 0.0 && t2 >= 0.0 {
        t1.min(t2)
    } else if t2 >= 0.0 {
        t2
    } else if t1 < 0.0 {
        return target_pos + target_vel;
    } else {
        t1.max(t2)
    };

    target_pos + target_vel * t
}

/// C++: sf::Vector2f scale_vector(sf::Vector2f& vector, float magnitude)
/// (not shown in the excerpted source but referenced in the forward
/// declarations; standard normalize-then-scale, matching displacement()'s
/// distance formula.)
pub fn scale_vector(vector: Vector2f, magnitude: f32) -> Vector2f {
    let len = (vector.x * vector.x + vector.y * vector.y).sqrt();
    if len == 0.0 {
        return Vector2f::new(0.0, 0.0);
    }
    Vector2f::new(vector.x / len * magnitude, vector.y / len * magnitude)
}

/// C++: double delta_angle(double a, double b)
pub fn delta_angle(a: f64, b: f64) -> f64 {
    let mut delta = (b - a) % 360.0;
    if delta < -180.0 {
        delta += 360.0;
    } else if delta > 180.0 {
        delta -= 360.0;
    }
    delta
}

/// C++: std::string rgb_to_hex(const sf::Color& color)
pub fn rgb_to_hex(color: Color) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b)
}

/// C++: static sf::Color hex_to_rgb(const std::string& spec, const sf::Color& fallback, int alpha)
/// Parses a "#rrggbb" string; falls back on parse failure.
pub fn hex_to_rgb(spec: &str, fallback: Color, alpha: u8) -> Color {
    let s = spec.trim_start_matches('#');
    if s.len() != 6 {
        return fallback;
    }
    let r = u8::from_str_radix(&s[0..2], 16);
    let g = u8::from_str_radix(&s[2..4], 16);
    let b = u8::from_str_radix(&s[4..6], 16);
    match (r, g, b) {
        (Ok(r), Ok(g), Ok(b)) => Color::rgba(r, g, b, alpha),
        _ => fallback,
    }
}

/// C++: std::string round_to_decimal_places(double value, int n)
/// Rust's format! already round-trips this behavior for the ranges the
/// game uses it for (display-only stat formatting).
pub fn round_to_decimal_places(value: f64, n: i32) -> String {
    if !value.is_finite() {
        return value.to_string();
    }
    let n = n.max(0) as usize;
    format!("{:.*}", n, value)
}

/// C++: bool enemies(int my_team, int their_team)
/// `gang_up` / `victim_team` are read from GameState; passed in explicitly
/// here since this is a free function with no global access in Rust.
pub fn enemies(my_team: i32, their_team: i32, gang_up: bool, victim_team: i32) -> bool {
    (!gang_up && their_team != my_team)
        || (gang_up
            && ((my_team != victim_team && their_team == victim_team)
                || (my_team == victim_team && their_team != my_team)))
}

/// Degrees -> radians, matching the repeated `M_PI * x / 180.0` pattern.
pub fn deg_to_rad(deg: f64) -> f64 {
    deg * PI_F64 / 180.0
}
