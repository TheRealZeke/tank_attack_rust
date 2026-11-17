//! GameState: the single owned struct threaded through the game, replacing
//! the file-scope globals at the top of TankAttack.cpp (EntityArr, LaserArr,
//! TileArr, Camera_Pos, colorArr, game_ticks, ...). Per spec §4.1, functions
//! take `&mut GameState` rather than reaching for statics/mutexes.

use crate::button::Button;
use crate::config::Config;
use crate::dead_tank::DeadTank;
use crate::entity::Entity;
use crate::floating_text::FloatingText;
use crate::game_display_text::GameDisplayText;
use crate::laser::Laser;
use crate::missile::Missile;
use crate::particle::Particle;
use crate::tile::Tile;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use sfml::graphics::{Color, FloatRect};
use sfml::system::Vector2f;

pub struct GameState {
    pub config: Config,

    // --- entity/projectile/fx arrays (C++: EntityArr, LaserArr, MissleArr, ParticleArr) ---
    pub entities: Vec<Entity>,
    pub lasers: Vec<Laser>,
    pub missiles: Vec<Missile>,
    pub particles: Vec<Particle>,
    pub tiles: Vec<Vec<Tile>>,
    pub dead_tanks: Vec<DeadTank>,
    pub game_texts: Vec<GameDisplayText>,
    pub floating_texts: Vec<FloatingText>,
    /// C++: ButtonArr. Added per continuation spec §0.2/§0.4 (missing from
    /// the initial Stage 1-6 scaffold).
    pub buttons: Vec<Button>,

    // --- team/color data (C++: colorArr, hexcolorArr, numberOfTeams) ---
    pub color_arr: Vec<Color>,
    pub hex_color_arr: Vec<String>,
    pub number_of_teams: i32,

    // --- camera / screen (C++: Camera_Pos, GameScreenRect, MINIMAP_*) ---
    pub camera_pos: [f32; 2],
    pub game_screen_rect: FloatRect,
    pub game_map_width: f32,
    pub game_map_height: f32,
    pub minimap_width: f32,
    pub minimap_height: f32,

    // --- timing (C++: game_ticks, GameFPSRatio) ---
    pub game_ticks: u64,
    pub game_fps_ratio: f32,

    // --- wind (C++: wind_vel, WIND_Physics) ---
    pub wind_vel: Vector2f,
    pub wind_physics: bool,

    // --- ganging-up rule state (C++: gangUp, victimTeam) ---
    pub gang_up: bool,
    pub victim_team: i32,

    // --- misc toggles (C++: eventLogging, devWindow, paused, DEBUG_FEATURES, draw_softborders) ---
    pub event_logging: bool,
    pub dev_window: bool,
    pub paused: bool,
    pub debug_features: bool,
    pub draw_softborders: bool,
    /// C++: `draw_transp_fort` (referenced by `Entity::draw`'s fort branch,
    /// Stage D.3). Not part of the Stage 0.4 field list since Stage D
    /// hadn't been scoped yet when that list was written.
    pub draw_transp_fort: bool,
    /// C++ toggle globals, all set to their C++ defaults. Grouped here
    /// per Stage D.5 (input handling), which is what reads/toggles them.
    pub draw_particles: bool,
    pub draw_tiles: bool,
    pub draw_ui: bool,
    pub draw_minimap: bool,
    pub draw_stats: bool,
    /// C++: `quitGame`. Only `sf::Event::Closed` sets this true in the
    /// original (Escape's `quitGame = true;` is commented out there).
    pub quit_game: bool,
    /// C++: `static bool MiniMapHighlight` inside `handle_events` -- a
    /// function-local `static` that persists across calls. Rust has no
    /// direct equivalent without `unsafe`/`thread_local!`, so it's a
    /// regular `GameState` field instead; same persistence, no unsafe.
    pub minimap_highlight: bool,
    /// Queued sound-effect triggers, drained and actually played once per
    /// frame by `SoundPlayers::play_queued` (see `assets.rs`'s doc
    /// comment for why simulation code can't call `.play()` directly).
    pub sound_queue: Vec<crate::assets::SoundEffect>,
    /// C++: `MiniMapRect`, computed once in `main()` from
    /// SCREEN_WIDTH/HEIGHT and MINIMAP_WIDTH/HEIGHT. Computed here in
    /// `GameState::new` instead, since those config values are already
    /// available then -- fixes the "defaults to origin" placeholder
    /// `render_minimap` used before Stage D.5 existed.
    pub minimap_rect: FloatRect,
    /// Real on-disk tank/fort texture dimensions (`CompTank.png` is
    /// 15x30, `Fort.bmp` is 33x33 in the shipped assets -- every team's
    /// texture is the same size, since they're all recolors of the same
    /// source image via `transform_image_color`, matching the C++'s
    /// `CompTankImgArr[team].getSize()` being identical across teams).
    /// Defaults to a placeholder until `game_loop::setup` overwrites it
    /// with the real value right after `Assets::load` -- so
    /// `entity_spawn`'s tank/fort branches (which read this) don't need
    /// an `&Assets` parameter threaded through every `spawn_tank`/
    /// `spawn_fort` call site (mapgen.rs, input.rs, entity.rs's fort
    /// tank-spawning, ...). Tests that never call `game_loop::setup`
    /// keep using the placeholder, which is fine since none of them
    /// assert on sprite bounds/collision-rect size.
    pub tank_texture_size: (i32, i32),
    pub fort_texture_size: (i32, i32),

    // --- logging (C++: GameLogArr) ---
    pub game_log_arr: Vec<String>,

    // --- additions per continuation spec §0.4 ---
    /// Seeded RNG replacing C++'s global `rand()` (seeded via `srand(time(0))`).
    pub rng: SmallRng,
    /// Total `rand()`-equivalent draws made so far. Not part of the C++
    /// source -- added per continuation spec §A.5's "RNG-order test
    /// (soft)" to give future refactors a number to diff against. See
    /// `GameState::rand_range`/`rand_u32`, and the manual counting in
    /// `spawn.rs`'s closure-based calls (`Entity::new`, `Particle::spawn`,
    /// `DeadTank::new`), for how draws that don't go through those two
    /// helpers still get counted.
    pub rng_calls: u64,
    pub last_fps: f32,
    pub stage_colour: Color,
    pub extra_population: i32,
    pub current_game_timer: i32,
    pub total_claimed_tiles: f32,
    pub total_tank_pop: f32,
    /// C++: TankDeathPositions (renamed to match spec's field list).
    pub tank_death_positions: Vec<[f32; 2]>,
    pub camera_follow_tank: bool,
    pub tank_followed_uid: Option<u32>,
    pub mouse_pos: Vector2f,
    pub game_volume: f32,
    pub autoplay: bool,

    // --- stats (C++: TankPopArr, FortPopArr, ... -- one Vec<i32> series per team-history) ---
    pub tank_pop_arr: Vec<Vec<i32>>,
    pub fort_pop_arr: Vec<Vec<i32>>,
    pub special_tank_pop_arr: Vec<Vec<i32>>,
    pub special_fort_pop_arr: Vec<Vec<i32>>,
    pub tanks_killed_arr: Vec<Vec<i32>>,
    pub tanks_lost_arr: Vec<Vec<i32>>,
    pub average_tank_age_arr: Vec<Vec<i32>>,
    pub team_territory_arr: Vec<Vec<i32>>,
    pub captured_fort_pop_arr: Vec<Vec<i32>>,
    pub dead_tank_pop_arr: Vec<Vec<i32>>,
    pub captured_fort_ratio_arr: Vec<Vec<f32>>,
    pub special_tank_ratio_arr: Vec<Vec<f32>>,
    pub special_fort_ratio_arr: Vec<Vec<f32>>,
    /// C++: Tank_Age_Team_Time_Array / Tank_Kills_Team_Time_Array -- each
    /// inner Vec entry is a `[time, value]` pair (C++: `std::array<float,2>`).
    pub tank_age_team_time_arr: Vec<Vec<[f32; 2]>>,
    pub tank_kills_team_time_arr: Vec<Vec<[f32; 2]>>,

    // --- per-tick scratch pop counts (C++: temp_Tank_Pop etc., refreshed by update_stats) ---
    pub temp_tank_pop: Vec<i32>,
    pub temp_fort_pop: Vec<i32>,
    pub temp_captured_fort_pop: Vec<i32>,
    pub temp_special_tank_pop: Vec<i32>,
    pub temp_special_fort_pop: Vec<i32>,

    // --- territory tallies (C++: TeamTileNo, TeamTilePerc) ---
    pub team_tile_no: Vec<i32>,
    pub team_tile_perc: Vec<f32>,

    pub tanks_killed: Vec<i32>,
    pub forts_destroyed: Vec<i32>,
    pub tanks_lost: Vec<i32>,
    pub forts_lost: Vec<i32>,
    pub pvp_tanks_killed: Vec<i32>,
    pub pvp_tanks_lost: Vec<i32>,

    /// simple counter-based RNG seed source; the real port should use
    /// `rand::rngs::SmallRng` seeded once at startup (spec §6.7).
    pub number_of_tiles: i32,
    pub summon_team: i32,
}

impl GameState {
    pub fn new(config: Config) -> Self {
        let number_of_teams = config.teams.number_of_teams;
        let color_arr: Vec<Color> = config
            .teams
            .colors
            .iter()
            .map(|c| Color::rgb(c[0], c[1], c[2]))
            .collect();
        let hex_color_arr: Vec<String> = color_arr
            .iter()
            .map(|c| crate::util::rgb_to_hex(*c))
            .collect();

        // C++: srand(time(0)). SmallRng isn't cryptographic; that's fine --
        // matches the C++'s non-cryptographic rand() usage.
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);

        GameState {
            entities: Vec::new(),
            lasers: Vec::new(),
            missiles: Vec::new(),
            particles: Vec::new(),
            tiles: Vec::new(),
            dead_tanks: Vec::new(),
            game_texts: Vec::new(),
            floating_texts: Vec::new(),
            buttons: Vec::new(),
            color_arr,
            hex_color_arr,
            number_of_teams,
            camera_pos: [0.0, 0.0],
            game_screen_rect: FloatRect::new(
                0.0,
                0.0,
                config.graphics.screen_width,
                config.graphics.screen_height,
            ),
            game_map_width: 2500.0,
            game_map_height: 2500.0,
            minimap_width: 250.0,
            minimap_height: 250.0,
            minimap_rect: FloatRect::new(
                config.graphics.screen_width - 250.0 - 50.0,
                config.graphics.screen_height - 250.0 - 50.0,
                250.0,
                250.0,
            ),
            game_ticks: 0,
            game_fps_ratio: 1.0,
            wind_vel: Vector2f::new(0.0, 0.0),
            wind_physics: config.graphics.wind,
            gang_up: false,
            victim_team: -1,
            event_logging: false,
            dev_window: false,
            paused: false,
            debug_features: false,
            draw_transp_fort: true,
            draw_particles: true,
            draw_tiles: true,
            draw_ui: true,
            draw_minimap: true,
            draw_stats: true,
            quit_game: false,
            minimap_highlight: false,
            sound_queue: Vec::new(),
            tank_texture_size: (32, 32),
            fort_texture_size: (48, 48),
            draw_softborders: false,
            game_log_arr: Vec::new(),
            rng: SmallRng::seed_from_u64(seed),
            rng_calls: 0,
            last_fps: 0.0,
            stage_colour: Color::rgba(8, 8, 8, 255),
            extra_population: 0,
            current_game_timer: 0,
            total_claimed_tiles: 0.0,
            total_tank_pop: 0.0,
            tank_death_positions: Vec::new(),
            camera_follow_tank: false,
            tank_followed_uid: None,
            mouse_pos: Vector2f::new(0.0, 0.0),
            game_volume: 100.0,
            autoplay: false,
            tank_pop_arr: Vec::new(),
            fort_pop_arr: Vec::new(),
            special_tank_pop_arr: Vec::new(),
            special_fort_pop_arr: Vec::new(),
            tanks_killed_arr: Vec::new(),
            tanks_lost_arr: Vec::new(),
            average_tank_age_arr: Vec::new(),
            team_territory_arr: Vec::new(),
            captured_fort_pop_arr: Vec::new(),
            dead_tank_pop_arr: Vec::new(),
            captured_fort_ratio_arr: Vec::new(),
            special_tank_ratio_arr: Vec::new(),
            special_fort_ratio_arr: Vec::new(),
            tank_age_team_time_arr: Vec::new(),
            tank_kills_team_time_arr: Vec::new(),
            temp_tank_pop: vec![0; number_of_teams as usize],
            temp_fort_pop: vec![0; number_of_teams as usize],
            temp_captured_fort_pop: vec![0; number_of_teams as usize],
            temp_special_tank_pop: vec![0; number_of_teams as usize],
            temp_special_fort_pop: vec![0; number_of_teams as usize],
            team_tile_no: vec![0; number_of_teams as usize],
            team_tile_perc: vec![0.0; number_of_teams as usize],
            tanks_killed: vec![0; number_of_teams as usize],
            forts_destroyed: vec![0; number_of_teams as usize],
            tanks_lost: vec![0; number_of_teams as usize],
            forts_lost: vec![0; number_of_teams as usize],
            pvp_tanks_killed: vec![0; number_of_teams as usize],
            pvp_tanks_lost: vec![0; number_of_teams as usize],
            number_of_tiles: 0,
            summon_team: 0,
            config,
        }
    }

    /// C++'s repeated "self-healing" lookup pattern: search EntityArr for a
    /// given unique_id, returning its current index. Used everywhere a
    /// pointer member needs re-validating (Laser::sender, Missile::target,
    /// Entity::enemy, Tile::fort, ...). See spec §4.3.
    pub fn find_entity_index_by_uid(&self, uid: u32) -> Option<usize> {
        self.entities.iter().position(|e| e.unique_id == uid)
    }

    /// Counted wrapper around `self.rng.gen_range`, replacing C++'s
    /// `rand() % n` at most call sites. See the `rng_calls` field doc.
    pub fn rand_range<T>(&mut self, range: std::ops::Range<T>) -> T
    where
        T: rand::distributions::uniform::SampleUniform + PartialOrd,
    {
        self.rng_calls += 1;
        self.rng.gen_range(range)
    }

    /// Counted wrapper around a raw `u32` draw (used for unique-id
    /// generation, which needs the full range rather than a bounded one).
    pub fn rand_u32(&mut self) -> u32 {
        self.rng_calls += 1;
        self.rng.gen::<u32>()
    }

    // -------------------------------------------------------------
    // Small population-stat getters used by draw_ui (Stage D.4). These
    // read from the `temp_*` arrays that `update_stats()` (stats.rs,
    // not yet ported) is responsible for populating each frame; until
    // that lands, they'll simply read as 0 -- expected, not a bug.
    // -------------------------------------------------------------

    /// C++: int get_fort_pop(int team, bool captured = false)
    /// (TankAttack.cpp L5441-5449).
    pub fn get_fort_pop(&self, team: i32, captured: bool) -> i32 {
        if captured {
            self.temp_captured_fort_pop[team as usize]
        } else {
            self.temp_fort_pop[team as usize]
        }
    }

    /// C++: int get_special_tank_pop(int team) (TankAttack.cpp L5466-5469).
    pub fn get_special_tank_pop(&self, team: i32) -> i32 {
        self.temp_special_tank_pop[team as usize]
    }

    /// C++: int get_special_fort_pop(int team) (TankAttack.cpp L5471-5474).
    pub fn get_special_fort_pop(&self, team: i32) -> i32 {
        self.temp_special_fort_pop[team as usize]
    }

    /// C++: int active_entity_pop() (TankAttack.cpp L6725-6731).
    pub fn active_entity_pop(&self) -> i32 {
        self.entities.iter().filter(|e| e.ia).count() as i32
    }
}
