//! Tile: ported from `class Tile` (TankAttack.cpp L3143-3402) and the free
//! function `get_tile_neighbours` (L6497-6521).
//!
//! `Entity* fort` becomes `fort_uid: Option<u32>`. `std::vector<Tile*>
//! neighbors` becomes `Vec<(usize, usize)>` grid-index pairs (spec §4.3 /
//! §5.5), resolved through `state.tiles[row][col]` rather than stored
//! pointers.

use sfml::graphics::{CircleShape, Color, Shape, Transformable};
use sfml::system::Vector2f;

pub struct Tile {
    pub pos: Vector2f,
    pub grid_x: i32,
    pub grid_y: i32,
    pub col: Color,
    pub opp_col: Color,
    pub team: i32,
    pub fort_uid: Option<u32>,
    pub radius: f32,

    pub flash_state: bool,
    pub flash_timer: f32,
    pub max_flash_time: f32,
    pub flash_vel: f32,
    pub flash_col: Color,
    pub flash_intensity: f32,
    pub border_tile: bool,
    pub soft_border_tile: bool,

    pub flicker_timer: f32,
    pub flicker_timer_vel: f32,
    pub flicker_timer_acc: f32,

    /// (row, col) indices into `GameState::tiles`, replacing `Vec<Tile*>`.
    pub neighbors: Vec<(usize, usize)>,

    pub tank_deaths: i32,

    pub fort_is_assimilating: bool,
    pub fort_is_special: bool,
    pub distance_from_fort: f32,

    /// C++: `sf::CircleShape circle` -- persistent per-tile shape, reused
    /// (not recreated) across `update()`/`draw()`/`idle_draw()` calls, per
    /// the original. `'static` because a tile's circle never has a
    /// texture set (SFML shapes only need a lifetime when textured).
    pub circle: CircleShape<'static>,
}

impl Tile {
    /// C++: Tile(sf::Vector2f sp_pos, int gx, int gy)
    pub fn new(
        sp_pos: Vector2f,
        gx: i32,
        gy: i32,
        tiles_size: f32,
        mut rand_fn: impl FnMut(u32) -> u32,
    ) -> Self {
        Tile {
            pos: sp_pos,
            grid_x: gx,
            grid_y: gy,
            col: Color::rgba(0, 0, 0, 0),
            opp_col: Color::rgba(0, 0, 0, 0),
            team: -1,
            fort_uid: None,
            radius: tiles_size / 2.0,
            flash_state: false,
            flash_timer: 0.0,
            max_flash_time: 60.0,
            flash_vel: 1.0,
            flash_col: Color::rgba(0, 0, 0, 0),
            flash_intensity: 255.0,
            border_tile: false,
            soft_border_tile: false,
            flicker_timer: (rand_fn(99) + 1) as f32,
            flicker_timer_vel: 1.0,
            flicker_timer_acc: 1.0 + (rand_fn(10) as f32).sqrt(),
            neighbors: Vec::new(),
            tank_deaths: 0,
            fort_is_assimilating: false,
            fort_is_special: false,
            distance_from_fort: 0.0,
            circle: {
                let mut c = CircleShape::new(tiles_size / 2.0 * 1.15, 6);
                c.set_outline_thickness((tiles_size / 50.0) as i32 as f32 + 1.0);
                c.set_origin(Vector2f::new(tiles_size / 2.0, tiles_size / 2.0));
                c.set_fill_color(Color::rgba(0, 0, 0, 0));
                c
            },
        }
    }

    /// C++: void flash(float vel, sf::Color t_col, float t_intensity, int type = 0)
    fn flash_fields(&mut self, vel: f32, t_col: Color, t_intensity: f32) {
        self.flash_state = true;
        self.flash_timer = 1.0;
        self.max_flash_time = 60.0;
        self.flash_vel = vel;
        self.flash_col = t_col;
        self.flash_intensity = t_intensity;
    }
}

/// C++: std::vector<Tile*> get_tile_neighbours(Tile& tile)
/// Even/odd row parity offset tables, ported exactly.
pub fn get_tile_neighbours(tiles: &[Vec<Tile>], grid_x: i32, grid_y: i32) -> Vec<(usize, usize)> {
    const EVEN_OFFSETS: [(i32, i32); 6] = [(-1, -1), (0, -1), (1, 0), (0, 1), (-1, 1), (-1, 0)];
    const ODD_OFFSETS: [(i32, i32); 6] = [(0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 0)];

    let mut neighbors = Vec::new();
    let offsets = if grid_y % 2 == 0 {
        &EVEN_OFFSETS
    } else {
        &ODD_OFFSETS
    };

    let max_rows = tiles.len() as i32;
    if max_rows == 0 {
        return neighbors;
    }

    for &(ox, oy) in offsets.iter() {
        let nx = grid_x + ox;
        let ny = grid_y + oy;
        if ny >= 0 && ny < max_rows {
            let max_cols = tiles[ny as usize].len() as i32;
            if nx >= 0 && nx < max_cols {
                neighbors.push((ny as usize, nx as usize));
            }
        }
    }
    neighbors
}

// ---------------------------------------------------------------------
// Stage B: update_tile / update_tile_borders / update_tiles / init_tiles /
// tile_flash, ported from TankAttack.cpp L3187-3271, L5489-5584, L6430,
// and Tile::flash (L3373-3395). `tile_draw`/`tile_idle_draw` remain
// unported -- SFML-only, deferred to Stage D (render.rs).
// ---------------------------------------------------------------------

use crate::entity::EntityType;
use crate::state::GameState;
use crate::util::opposite_color;
use rand::Rng;

/// C++: `Tile::flash(...)` with its particle-spawn side effect.
///
/// CORRECTION: an earlier version of this port's doc comment claimed
/// `type == 1`'s 2-particle variant was unused dead code -- that was
/// wrong. It's used by Stage D.5's `handle_mouse_button_pressed` when a
/// fort is clicked (`tile.flash(5, EntityArr[i].col, 64, 1)`,
/// TankAttack.cpp's mouse-click handler). Both variants are implemented.
pub fn tile_flash(
    state: &mut GameState,
    row: usize,
    col: usize,
    vel: f32,
    t_col: Color,
    t_intensity: f32,
    kind: i32,
) {
    let pos = state.tiles[row][col].pos;
    state.tiles[row][col].flash_fields(vel, t_col, t_intensity);
    match kind {
        0 => {
            for _ in 0..3 {
                crate::spawn::spawn_particle(state, pos, 4.0, 1.25, 2, 2.0, t_col);
            }
        }
        1 => {
            for _ in 0..2 {
                crate::spawn::spawn_particle(state, pos, 3.0, 1.0, 2, 1.5, t_col);
            }
        }
        _ => {}
    }
}

/// C++: void Tile::update() (TankAttack.cpp L3190-3236).
pub fn update_tile(state: &mut GameState, row: usize, col: usize) {
    if state.event_logging {
        let (gx, gy) = (state.tiles[row][col].grid_x, state.tiles[row][col].grid_y);
        println!("Tile.update() called on Tile({gx},{gy})");
    }

    let fort_range_cfg = state.config.entity_values.fort_range as f32;
    let mut smallest_dis = fort_range_cfg * 2.0;
    let last_col = state.tiles[row][col].col;
    let pos = state.tiles[row][col].pos;

    state.tiles[row][col].distance_from_fort = -1.0;
    state.tiles[row][col].fort_uid = None;

    let assim_power = state.config.entity_values.assimilating_fort_territory_power;
    let special_power = state.config.entity_values.special_fort_territory_power;

    let mut found: Option<(usize, f32)> = None; // (entity idx, raw distance)
    for i in 0..state.entities.len() {
        let e = &state.entities[i];
        if e.entity_type != EntityType::Fort {
            continue;
        }
        let dx = pos.x - e.pos.x;
        let dy = pos.y - e.pos.y;
        let dis = ((dx * dx + dy * dy) as f64).sqrt() as f32;
        let power = e.get_fort_territory_power(assim_power, special_power);
        let scaled = dis / power;
        if scaled < smallest_dis && scaled < e.range * 2.0 {
            smallest_dis = scaled;
            found = Some((i, dis));
        }
    }

    match found {
        Some((i, raw_dis)) => {
            let (team, fort_uid, fort_col, tank_spawn_timer, special_fort, range) = {
                let e = &state.entities[i];
                (
                    e.team,
                    e.unique_id,
                    e.col,
                    e.tank_spawn_timer,
                    e.special_fort,
                    e.range,
                )
            };

            let mut new_col = state.color_arr[team as usize];
            let opp_col = opposite_color(new_col);
            new_col.a = 64;
            let fort_is_assimilating = tank_spawn_timer < 0.0;
            if smallest_dis > range {
                new_col.a = 16;
            }

            let changed = new_col != last_col;

            state.tiles[row][col].team = team;
            state.tiles[row][col].col = new_col;
            state.tiles[row][col].opp_col = opp_col;
            state.tiles[row][col].fort_is_assimilating = fort_is_assimilating;
            state.tiles[row][col].fort_is_special = special_fort;
            state.tiles[row][col].fort_uid = Some(fort_uid);
            state.tiles[row][col].distance_from_fort = raw_dis;

            if changed && state.game_screen_rect.contains(pos) {
                tile_flash(
                    state,
                    row,
                    col,
                    fort_range_cfg * 2.0 / smallest_dis,
                    fort_col,
                    128.0,
                    0,
                );
                // C++'s `if (col.r > 255) std::cout << "Tile Update Error"`
                // check is meaningless in C++ too (sf::Uint8 can't exceed
                // 255) and is omitted as genuine dead code.
            }
        }
        None => {
            state.tiles[row][col].team = -1;
        }
    }
}

/// C++: void Tile::updateBorders() (TankAttack.cpp L3247-3271).
pub fn update_tile_borders(state: &mut GameState, row: usize, col: usize) {
    state.tiles[row][col].border_tile = false;
    state.tiles[row][col].soft_border_tile = false;

    let self_team = state.tiles[row][col].team;
    let gang_up = state.gang_up;
    let victim_team = state.victim_team;
    let neighbors = state.tiles[row][col].neighbors.clone();

    let mut border = false;
    for &(nr, nc) in &neighbors {
        let n_team = state.tiles[nr][nc].team;
        if (!gang_up && n_team != self_team)
            || (gang_up
                && ((self_team != victim_team && n_team == victim_team)
                    || (self_team == victim_team && n_team != self_team)))
        {
            border = true;
            break;
        }
    }
    state.tiles[row][col].border_tile = border;

    if state.draw_softborders && !border {
        let fort_uid = state.tiles[row][col].fort_uid;
        if let Some(fuid) = fort_uid {
            let fort_bloom = state.config.graphics.fort_bloom_range_factor;
            let fort_range_cfg = state.config.entity_values.fort_range as f32;
            let (dis_from_fort, is_special) = {
                let t = &state.tiles[row][col];
                (t.distance_from_fort, t.fort_is_special)
            };
            let qualifies = (!is_special && dis_from_fort > fort_range_cfg * fort_bloom * 1.5)
                || (is_special && dis_from_fort > fort_range_cfg * 5.0 * fort_bloom);
            if qualifies {
                let mut soft = false;
                for &(nr, nc) in &neighbors {
                    let n_fort_uid = state.tiles[nr][nc].fort_uid;
                    let n_team = state.tiles[nr][nc].team;
                    if n_fort_uid != Some(fuid) && n_team == self_team {
                        soft = true;
                        break;
                    }
                }
                state.tiles[row][col].soft_border_tile = soft;
            }
        }
    }
}

/// C++: void update_tiles() (TankAttack.cpp L5489-5540).
pub fn update_tiles(state: &mut GameState) {
    if state.event_logging {
        println!(
            "###== Updating Tiles...; GameTick: {}  ==###",
            state.game_ticks
        );
    }

    let n = state.number_of_teams as usize;
    state.team_tile_no = vec![0; n];
    state.team_tile_perc = vec![0.0; n];
    state.total_claimed_tiles = 0.0;

    let rows = state.tiles.len();
    for r in 0..rows {
        let cols = state.tiles[r].len();
        for c in 0..cols {
            update_tile(state, r, c);
            let team = state.tiles[r][c].team;
            if team != -1 {
                state.team_tile_no[team as usize] += 1;
                state.total_claimed_tiles += 1.0;
            }
        }
    }

    // C++ QUIRK: a block assigning tank deaths to the nearest claimed tile
    // (incrementing `Tile::tank_deaths`) is commented out in the original
    // source (L5507-5527) and was never active. Preserved as disabled here
    // too -- not reproduced, since dead code that indexes
    // `TankDeathPositions[j]` with the *tile-column* loop variable `j`
    // (rather than the death-index variable, also confusingly named `j`)
    // would be a bug to faithfully port even if it were live.

    for r in 0..rows {
        let cols = state.tiles[r].len();
        for c in 0..cols {
            update_tile_borders(state, r, c);
        }
    }

    for i in 0..n {
        state.team_tile_perc[i] = if state.total_claimed_tiles != 0.0 {
            state.team_tile_no[i] as f32 * 100.0 / state.total_claimed_tiles
        } else {
            0.0
        };
    }

    if state.event_logging {
        println!("  Tiles Updated");
    }

    state.tank_death_positions.clear();
}

/// C++: void update_tile_pop() (TankAttack.cpp L6430-6438).
pub fn update_tile_pop(state: &mut GameState) {
    let mut x = 0;
    for row in &state.tiles {
        x += row.len();
    }
    state.number_of_tiles = x as i32;
}

/// C++: void init_tiles() (TankAttack.cpp L5553-5584).
pub fn init_tiles(state: &mut GameState) {
    state.tiles.clear();

    let xx = state.game_map_width as f64;
    let yy = state.game_map_height as f64;
    let size = state.config.graphics.tiles_size as f64;
    let mut cy: i32 = 0;

    let x_fix = 2.0 / 3.0f64.sqrt();

    let mut y = -size;
    while y < yy + size {
        let mut cx: i32 = 0;
        let gy = y as f32;

        let mut row_of_tiles: Vec<Tile> = Vec::new();

        let mut x = -size * x_fix;
        while x < xx + (size * x_fix) {
            let gx = if cy % 2 == 0 {
                x - (size * x_fix / 4.0)
            } else {
                x + (size * x_fix / 4.0)
            };
            row_of_tiles.push(Tile::new(
                Vector2f::new(gx as f32, gy),
                cx,
                cy,
                state.config.graphics.tiles_size as f32,
                |n| state.rng.gen_range(0..n),
            ));
            cx += 1;
            x += size * x_fix;
        }
        state.tiles.push(row_of_tiles);
        cy += 1;
        y += size;
    }

    update_tile_pop(state);

    // Set neighbors for each tile.
    let rows = state.tiles.len();
    for i in 0..rows {
        let cols = state.tiles[i].len();
        for j in 0..cols {
            let (gx, gy) = (state.tiles[i][j].grid_x, state.tiles[i][j].grid_y);
            state.tiles[i][j].neighbors = get_tile_neighbours(&state.tiles, gx, gy);
        }
    }
}

// ---------------------------------------------------------------------
// Stage D.3: Tile::draw() / Tile::idle_draw(), ported from
// TankAttack.cpp L3269-3376.
// ---------------------------------------------------------------------

use crate::assets::Assets;
use sfml::graphics::RenderTarget;

/// C++: void Tile::draw() (TankAttack.cpp L3269-3338). Returns early
/// (draws nothing) for an unclaimed tile (`team == -1`), same as C++.
pub fn tile_draw<T: RenderTarget>(
    state: &mut GameState,
    assets: &Assets,
    target: &mut T,
    row: usize,
    col: usize,
) {
    if state.tiles[row][col].team == -1 {
        return;
    }

    let (pos, tile_col) = {
        let t = &state.tiles[row][col];
        (t.pos, t.col)
    };
    let g_pos = Vector2f::new(pos.x - state.camera_pos[0], pos.y - state.camera_pos[1]);
    let opp_col = opposite_color(tile_col);
    state.tiles[row][col].opp_col = opp_col;

    let tiles_size = state.config.graphics.tiles_size as f32;
    let (soft_border_tile, border_tile) = {
        let t = &state.tiles[row][col];
        (t.soft_border_tile, t.border_tile)
    };

    {
        let t = &mut state.tiles[row][col];
        t.circle.set_position(g_pos);
        t.circle.set_outline_color(tile_col);
        if soft_border_tile && !border_tile {
            t.circle.set_outline_thickness(0.0);
        } else {
            t.circle
                .set_outline_thickness((tiles_size / 50.0) as i32 as f32 + 1.0);
        }
    }

    let fort_is_assimilating = state.tiles[row][col].fort_is_assimilating;
    if fort_is_assimilating {
        let (grid_x, grid_y) = {
            let t = &state.tiles[row][col];
            (t.grid_x, t.grid_y)
        };
        let adjusted_x = grid_x - if grid_y % 2 == 1 { 1 } else { 0 };
        if (adjusted_x + grid_y) % 3 == 0 || (adjusted_x - grid_y) % 3 == 0 {
            let mut temp_col = opp_col;
            temp_col.a = (tile_col.a as f32 * 0.67) as u8;
            state.tiles[row][col].circle.set_outline_color(temp_col);
        }
    }

    // C++: `if (flash_timer < 0) { flash_state = false; }` then, if still
    // flashing, animate and override the circle's fill color.
    if state.tiles[row][col].flash_timer < 0.0 {
        state.tiles[row][col].flash_state = false;
    }
    if state.tiles[row][col].flash_state {
        let fps = state.game_fps_ratio;
        let t = &mut state.tiles[row][col];
        t.flash_timer += t.flash_vel * fps;
        if t.flash_timer > t.max_flash_time {
            t.flash_vel = -t.flash_vel;
            t.flash_timer = t.max_flash_time;
        }

        let mut ratio = t.flash_intensity * t.flash_timer / t.max_flash_time;
        ratio = ratio.clamp(0.0, 255.0);
        t.flash_col.a = ratio as u8;
        let flash_col = t.flash_col;
        t.circle.set_fill_color(flash_col);
    }

    target.draw(&state.tiles[row][col].circle);

    if state.debug_features {
        let (tmp_a, mut dbg_col, mut dbg_opp_col) = {
            let t = &state.tiles[row][col];
            (t.col.a, t.col, t.opp_col)
        };
        dbg_opp_col.a = ((dbg_col.a as f32) * 1.5) as u8;
        dbg_col.a = ((dbg_col.a as f32) * 1.5) as u8;
        let _ = dbg_opp_col; // matches C++: opp_col.a is set but never read again in this block.

        let bounds = state.tiles[row][col].circle.global_bounds();
        if bounds.contains(state.mouse_pos) {
            let (grid_x, grid_y, fort_uid) = {
                let t = &state.tiles[row][col];
                (t.grid_x, t.grid_y, t.fort_uid)
            };
            crate::render::render_text(
                target,
                g_pos,
                &format!("{grid_x},{grid_y}"),
                &assets.font_2,
                20,
                dbg_col,
                1.0,
                Color::BLACK,
                false,
            );
            let fort_uid_display = fort_uid.map(|u| u as i64).unwrap_or(-1);
            crate::render::render_text(
                target,
                Vector2f::new(g_pos.x, g_pos.y + 20.0),
                &format!("Fort uid: {fort_uid_display}"),
                &assets.font_2,
                16,
                dbg_col,
                0.0,
                Color::BLACK,
                false,
            );
        }
        // The "draw lines to neighbors" block is gated by a literal
        // `if (false)` in the original -- permanently disabled debug code,
        // never reached. Omitted entirely.

        state.tiles[row][col].col.a = tmp_a;
        state.tiles[row][col].opp_col.a = tmp_a;
    }
}

/// C++: void Tile::idle_draw() (TankAttack.cpp L3339-3364). Updates the
/// flicker animation and sets the circle's *fill* color (border/border-
/// adjacent tiles get progressively dimmer fills); does not itself issue
/// a draw call -- `tile_draw` (which sets outline + position and does
/// `window.draw(circle)`) is what actually puts it on screen.
pub fn tile_idle_draw(state: &mut GameState, row: usize, col: usize) {
    let fps = state.game_fps_ratio;
    {
        let t = &mut state.tiles[row][col];
        t.flicker_timer += t.flicker_timer_vel * t.flicker_timer_acc * fps * 0.67;
        if t.flicker_timer <= 0.0 || t.flicker_timer >= 100.0 {
            t.flicker_timer_vel = -t.flicker_timer_vel;
        }
        t.flicker_timer = t.flicker_timer.clamp(0.0, 100.0);
    }

    let (col_val, flicker_timer, soft_border_tile, border_tile, fort_is_assimilating) = {
        let t = &state.tiles[row][col];
        (
            t.col,
            t.flicker_timer,
            t.soft_border_tile,
            t.border_tile,
            t.fort_is_assimilating,
        )
    };

    let temp = col_val.a;
    let mut new_col = col_val;
    new_col.a = (((col_val.a as f32) / 6.0) * flicker_timer / 100.0) as u8;
    if soft_border_tile && !border_tile {
        new_col.a = ((new_col.a as f32) * 1.2) as u8;
    }
    if fort_is_assimilating {
        new_col.a = ((new_col.a as f32) / 2.0) as u8;
    }
    state.tiles[row][col].circle.set_fill_color(new_col);
    state.tiles[row][col].col.a = temp;

    if border_tile {
        let temp2 = state.tiles[row][col].col.a;
        let mut border_col = state.tiles[row][col].col;
        border_col.a = ((border_col.a as f32) / 3.0) as u8;
        state.tiles[row][col].circle.set_fill_color(border_col);
        state.tiles[row][col].col.a = temp2;
    }
}
