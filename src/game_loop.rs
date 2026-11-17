//! Stage E: the main game loop, ported from `int main()` (TankAttack.cpp
//! L3684-4110).
//!
//! Split for testability, same reasoning as Stage D.5's `input.rs`: the
//! per-frame body (`tick`) takes an already-open `&mut RenderWindow` and
//! does everything except the outer `while` condition, so a test can
//! create a real (headless, under Xvfb) `RenderWindow` and run `tick` a
//! bounded number of times, exercising the actual production code path
//! rather than a parallel test-only reimplementation.

use crate::assets::Assets;
use crate::config::Config;
use crate::entity::EntityType;
use crate::input::{check_keyboard_inputs, handle_events};
use crate::mapgen::{generate_random_map, load_game_buttons};
use crate::render::{
    draw_filled_rect, draw_tiles, render_buttons, render_dead_tanks, render_entities,
    render_floating_texts, render_game_texts, render_lasers, render_minimap, render_missiles,
    render_particles, render_text, stage_color,
};
use crate::spawn::spawn_game_text;
use crate::state::GameState;
use crate::stats::{get_average_tank_age, get_team_territory, save_json_data, update_stats};
use crate::tile::{init_tiles, update_tiles};
use sfml::graphics::{Color, RenderTarget, RenderWindow};
use sfml::system::Vector2f;

/// C++: `const int FPS = 120;` (TankAttack.cpp L262).
const FPS: i32 = 120;

/// C++'s per-`main()`-call local variables that persist across loop
/// iterations (`max_pop_reached`, `last_man_standing`, `autoplay_time`,
/// `teams_left`, `frames`, `frames2`) -- kept as locals in C++, not
/// `GameState` fields, since they're purely main-loop bookkeeping.
/// Grouped into a struct here so `tick` can take them by `&mut` without a
/// long parameter list.
pub struct LoopCounters {
    pub max_pop_reached: bool,
    pub last_man_standing: bool,
    /// C++: `int autoplay_time = -1;` -- `-1` means "not scheduled".
    pub autoplay_time: i64,
    pub teams_left: i32,
    pub frames: u32,
    pub frames2: u32,
    pub clock: std::time::Instant,
    pub fps_clock: std::time::Instant,
}

impl LoopCounters {
    pub fn new() -> Self {
        let now = std::time::Instant::now();
        LoopCounters {
            max_pop_reached: false,
            last_man_standing: false,
            autoplay_time: -1,
            teams_left: -1,
            frames: 0,
            frames2: 0,
            clock: now,
            fps_clock: now,
        }
    }
}

impl Default for LoopCounters {
    fn default() -> Self {
        Self::new()
    }
}

/// C++'s setup before the main loop (TankAttack.cpp L3684-3826), minus
/// window creation (the caller does that, since it needs the real
/// `VideoMode`/`Style`/fullscreen-vs-dev-window choice) and minus music
/// (opened by the caller too, since `Music` isn't stored on `GameState`
/// or `Assets` -- see the module doc comment on why).
pub fn setup(config: Config) -> Result<(GameState, Assets), String> {
    let mut state = GameState::new(config);
    let color_arr = state.color_arr.clone();
    let assets = Assets::load(&state.config, &color_arr)?;

    // Real texture dimensions, replacing the placeholder `state.
    // tank_texture_size`/`fort_texture_size` GameState::new sets by
    // default -- every team's texture is the same size (all recolors of
    // the same source image), so team 0's is representative.
    let tank_size = assets.tank_textures[0].size();
    let fort_size = assets.fort_textures[0].size();
    state.tank_texture_size = (tank_size.x as i32, tank_size.y as i32);
    state.fort_texture_size = (fort_size.x as i32, fort_size.y as i32);

    state.last_fps = FPS as f32;
    state.game_fps_ratio = 1.0;

    init_tiles(&mut state);

    let (sw, sh) = (
        state.config.graphics.screen_width,
        state.config.graphics.screen_height,
    );
    spawn_game_text(
        &mut state,
        "GAME STARTED".to_string(),
        Vector2f::new(sw / 2.0, sh / 2.0),
        Color::rgb(255, 255, 255),
        30,
        10.0,
        2.0,
        true,
    );

    load_game_buttons(&mut state);

    Ok((state, assets))
}

/// C++'s main-loop body (TankAttack.cpp L3866-4073), one iteration. The
/// caller's `while window.is_open() && !state.quit_game` loop (or a
/// bounded test loop) calls this once per frame.
pub fn tick(
    state: &mut GameState,
    assets: &Assets,
    sound_players: &mut crate::assets::SoundPlayers,
    window: &mut RenderWindow,
    counters: &mut LoopCounters,
) {
    state.game_ticks += 1;
    counters.frames += 1;
    counters.frames2 += 1;
    state.current_game_timer += 1;

    if state.game_ticks % 25 == 0 {
        update_stats(state);
    }

    if state.game_ticks % 60 == 0 && counters.autoplay_time == -1 && state.autoplay {
        counters.last_man_standing = false;
        counters.teams_left = 0;
        for i in 0..state.number_of_teams {
            if state.get_fort_pop(i, false) > 0 {
                counters.teams_left += 1;
            }
        }
        if counters.teams_left == 1 {
            counters.last_man_standing = true;
        }
        if counters.last_man_standing {
            counters.autoplay_time = state.game_ticks as i64 + 600;
        }
    }

    if !state.autoplay && counters.autoplay_time != -1 {
        counters.autoplay_time = -1;
    }

    if state.autoplay && state.game_ticks as i64 == counters.autoplay_time {
        let _ = save_json_data(state);
        generate_random_map(state, None);
        counters.autoplay_time = -1;
    }

    if state.game_ticks % 20 == 0 && !counters.max_pop_reached {
        let mut teams_maxpop = 0;
        for i in 0..state.number_of_teams {
            if crate::entity::get_tank_pop(state, i, 0)
                >= state.extra_population + state.config.entity_values.tank_min_pop
            {
                teams_maxpop += 1;
            }
        }
        counters.max_pop_reached = teams_maxpop >= 2;
    }

    if state.config.entity_values.extra_population_interval > 0
        && state.game_ticks % state.config.entity_values.extra_population_interval as u64 == 0
        && counters.max_pop_reached
    {
        state.extra_population += state.config.entity_values.extra_population_quanta;
        counters.max_pop_reached = false;
    }

    handle_events(state, window);
    check_keyboard_inputs(state);

    window.clear(Color::BLACK);

    state.game_screen_rect = sfml::graphics::FloatRect::new(
        state.camera_pos[0] - 25.0,
        state.camera_pos[1] - 25.0,
        state.config.graphics.screen_width + 50.0,
        state.config.graphics.screen_height + 50.0,
    );
    draw_filled_rect(
        window,
        sfml::graphics::FloatRect::new(
            -state.camera_pos[0],
            -state.camera_pos[1],
            state.game_map_width,
            state.game_map_height,
        ),
        state.stage_colour,
    );
    if state.draw_tiles {
        draw_tiles(state, assets, window);
    }
    render_dead_tanks(state, assets, window);
    render_entities(state, assets, window);
    render_lasers(state, window);
    render_missiles(state, window);
    if !state.draw_particles {
        state.particles.clear();
    }
    render_particles(state, window);
    render_floating_texts(state, assets, window);
    if state.draw_ui {
        crate::render::draw_ui(state, assets, window, state.draw_stats);
    } else {
        render_game_texts(state, assets, window);
        render_buttons(state, assets, window);
    }
    if state.draw_minimap && state.game_ticks != 0 {
        render_minimap(state, window);
    }

    update_wind(state);
    if state.draw_tiles && state.game_ticks % 45 == 0 {
        update_tiles(state);
    }

    if state.autoplay {
        let (sw, _sh) = (
            state.config.graphics.screen_width,
            state.config.graphics.screen_height,
        );
        let text = if counters.autoplay_time != -1 {
            format!(
                "Autoplay in: {} seconds",
                (counters.autoplay_time - state.game_ticks as i64) / 60
            )
        } else {
            "Autoplay Enabled".to_string()
        };
        render_text(
            window,
            Vector2f::new(sw / 2.0, 20.0),
            &text,
            &assets.font_1,
            20,
            Color::rgb(255, 255, 255),
            1.0,
            Color::BLACK,
            false,
        );
    }

    if state.game_ticks % 300 == 0 {
        capture_stats_history(state);
    }

    // C++'s "Demo Game Autoplay" block (regenerate the map when only one
    // team remains) is gated by `game_ticks % 1800 == 0 && false` --
    // permanently disabled by the literal `&& false` in the original.
    // Omitted entirely, matching that dead code rather than the
    // behavior its comment describes (see README Deviations).

    window.display();

    // C++'s Y/H-key volume-rescale block only re-applies on keypress
    // (TankAttack.cpp L4575-4585); this port re-applies every frame
    // instead (cheap and idempotent) so `SoundPlayers` doesn't need
    // threading into `input.rs`'s keyboard handling. Actually playing
    // queued one-shot effects does still need to happen here, since
    // simulation code (entity.rs/laser.rs) only queues them.
    sound_players.set_volumes(&state.config.audio.volumes, state.game_volume);
    sound_players.play_queued(&mut state.sound_queue);

    if counters.clock.elapsed().as_secs_f32() >= 1.0 {
        state.stage_colour = stage_color(
            state.number_of_teams,
            &state.color_arr,
            |t| crate::entity::get_tank_pop(state, t, 0),
            |t| state.get_fort_pop(t, false),
        );
        state.last_fps = counters.frames as f32;
        counters.frames = 0;
        counters.clock = std::time::Instant::now();
    }

    if counters.fps_clock.elapsed().as_secs_f32() >= 0.5 {
        if counters.frames2 > 0 {
            state.game_fps_ratio = (60.0 / counters.frames2 as f32) * 0.5;
        }
        if state.game_fps_ratio > 6.0 {
            state.game_fps_ratio = 6.0;
        }
        counters.frames2 = 0;
        counters.fps_clock = std::time::Instant::now();
    }
}

/// C++: void update_wind() (TankAttack.cpp L6057-6076).
fn update_wind(state: &mut GameState) {
    if !state.wind_physics {
        return;
    }
    let max_wind_vel = 0.05 * state.config.graphics.max_wind_speed_factor;
    let wind_strength = state.config.graphics.wind_strength_factor;

    let vel = (((state.rand_range(0..100) + 1) as f32 / 10000.0) - 0.005) * wind_strength;
    let ang = state.rand_range(0..360) as f32;
    state.wind_vel.x += (ang * std::f32::consts::PI / 180.0).cos() * vel;
    state.wind_vel.y += (ang * std::f32::consts::PI / 180.0).sin() * vel;

    let (dx, dy) = (state.wind_vel.x, state.wind_vel.y);
    let scalar_wind = (dx * dx + dy * dy).sqrt();
    if scalar_wind > max_wind_vel {
        let factor = max_wind_vel / scalar_wind;
        state.wind_vel.x *= factor;
        state.wind_vel.y *= factor;
    }
}

/// C++'s "Update Population Data" block inside the main loop, run every
/// 300 ticks (TankAttack.cpp L3917-4042). See continuation_spec.md §E.2
/// point 11.
fn capture_stats_history(state: &mut GameState) {
    let n = state.number_of_teams;

    let tank_pops: Vec<i32> = (0..n)
        .map(|i| crate::entity::get_tank_pop(state, i, 0))
        .collect();
    state.tank_pop_arr.push(tank_pops);

    let fort_pops: Vec<i32> = (0..n).map(|i| state.get_fort_pop(i, false)).collect();
    state.fort_pop_arr.push(fort_pops);

    let captured_pops: Vec<i32> = (0..n).map(|i| state.get_fort_pop(i, true)).collect();
    state.captured_fort_pop_arr.push(captured_pops);

    let dead_tank_pops: Vec<i32> = (0..n)
        .map(|i| state.dead_tanks.iter().filter(|dt| dt.team == i).count() as i32)
        .collect();
    state.dead_tank_pop_arr.push(dead_tank_pops);

    let captured_ratios: Vec<f32> = (0..n)
        .map(|i| {
            let total = state.get_fort_pop(i, false);
            if total != 0 {
                state.get_fort_pop(i, true) as f32 / total as f32
            } else {
                0.0
            }
        })
        .collect();
    state.captured_fort_ratio_arr.push(captured_ratios);

    let special_tank_pops: Vec<i32> = (0..n).map(|i| state.get_special_tank_pop(i)).collect();
    state.special_tank_pop_arr.push(special_tank_pops);

    // DEVIATION: C++ divides `get_special_tank_pop(i) / (float)get_tank_pop(i)`
    // with no zero-guard -- a team with 0 tanks produces `0.0/0.0 = NaN`
    // (IEEE-754 float division, not a crash) rather than the `0.0`
    // fallback the sibling "captured ratio" computation two blocks above
    // uses. Reproduced exactly (including the NaN), not "fixed" to match
    // the other block's guard.
    let special_tank_ratios: Vec<f32> = (0..n)
        .map(|i| {
            state.get_special_tank_pop(i) as f32 / crate::entity::get_tank_pop(state, i, 0) as f32
        })
        .collect();
    state.special_tank_ratio_arr.push(special_tank_ratios);

    let special_fort_pops: Vec<i32> = (0..n).map(|i| state.get_special_fort_pop(i)).collect();
    state.special_fort_pop_arr.push(special_fort_pops);

    // Same NaN-on-zero deviation as special_tank_ratios above.
    let special_fort_ratios: Vec<f32> = (0..n)
        .map(|i| state.get_special_fort_pop(i) as f32 / state.get_fort_pop(i, false) as f32)
        .collect();
    state.special_fort_ratio_arr.push(special_fort_ratios);

    let tanks_killed: Vec<i32> = (0..n as usize).map(|i| state.pvp_tanks_killed[i]).collect();
    state.tanks_killed_arr.push(tanks_killed);

    let tanks_lost: Vec<i32> = (0..n as usize).map(|i| state.pvp_tanks_lost[i]).collect();
    state.tanks_lost_arr.push(tanks_lost);

    let avg_ages: Vec<i32> = (0..n)
        .map(|i| get_average_tank_age(state, i, FPS))
        .collect();
    state.average_tank_age_arr.push(avg_ages);

    let territories: Vec<i32> = (0..n).map(|i| get_team_territory(state, i)).collect();
    state.team_territory_arr.push(territories);

    let mut tank_age_snapshot: Vec<[f32; 2]> = Vec::new();
    let mut tank_kills_snapshot: Vec<[f32; 2]> = Vec::new();
    for e in &state.entities {
        if e.entity_type == EntityType::Tank && e.ia {
            tank_age_snapshot.push([e.team as f32, e.age]);
            tank_kills_snapshot.push([e.team as f32, e.kill_count as f32]);
        }
    }
    state.tank_age_team_time_arr.push(tank_age_snapshot);
    state.tank_kills_team_time_arr.push(tank_kills_snapshot);
}

/// C++'s `while (!quitGame)` main loop (TankAttack.cpp L3826-4073).
/// `max_frames`: if `Some(n)`, stop after `n` frames even if the window
/// is still open and `quit_game` is still false -- used by tests and by
/// any embedder that wants a bounded run rather than "until closed".
pub fn run(
    state: &mut GameState,
    assets: &Assets,
    sound_players: &mut crate::assets::SoundPlayers,
    window: &mut RenderWindow,
    max_frames: Option<u64>,
) {
    let mut counters = LoopCounters::new();
    let mut frame_count = 0u64;

    while window.is_open() && !state.quit_game {
        tick(state, assets, sound_players, window, &mut counters);
        frame_count += 1;
        if let Some(max) = max_frames {
            if frame_count >= max {
                break;
            }
        }
    }
}
