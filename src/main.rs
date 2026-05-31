#![allow(unused)]

use core::panic;
use serde_json::Value;
use sfml::graphics::{
    CircleShape, Color, FloatRect, Font, Image, RenderTarget, RenderWindow, Shape, Sprite, Text,
    Texture, Transformable,
};
use sfml::system::{Vector2f, Vector2i, Vector2u};
use sfml::window::{Event, Style, VideoMode};
use sfml::{audio, window};
use std::fs::File;
use std::io::BufReader;

const EVENT_LOGGING: bool = false;
const DEV_WINDOW: bool = false;
const FPS: i32 = 120;
const SCREEN_WIDTH: u32 = 1920;
const SCREEN_HEIGHT: u32 = 1080;

// Load the tank image at runtime instead of a const global.

fn main() {
    // Initialize classes(structs)
    struct Entity;
    struct Laser;
    struct Missle;
    struct Particle;
    struct Tile;
    struct GameDisplayText;
    struct DeadTank;
    struct Button;
    struct FloatingText;

    // Initialize images
    let tank_image = Image::from_file("assets/images/tank.png").unwrap();
    let fort_image = Image::from_file("assets/images/fort.bmp").unwrap();
    let dead_tank_image = [
        Image::from_file("assets/images/dead_tank_01.png").unwrap(),
        Image::from_file("assets/images/dead_tank_02.png").unwrap(),
    ];

    let mut window = RenderWindow::new(
        VideoMode::new(SCREEN_WIDTH, SCREEN_HEIGHT, 64),
        "Tank Attack",
        Style::CLOSE,
        &Default::default(),
    );

    let mut quit_game: bool = false;
    let mut game_fps_ratio = 0;
    let mut summon_team: u8 = 0;

    // Initialize audio volumes from JSON data
    let audio_file = File::open("audio.json").expect("Unable to open Audio.json");
    let audio_file_reader = BufReader::new(audio_file);
    let audio_data: Value =
        serde_json::from_reader(audio_file_reader).expect("Unable to parse Audio.json");
    let background_music_volume = audio_data["background"].clone();
    let tank_shoot_volume = audio_data["tank_shoot"].clone();
    let fort_shoot_volume = audio_data["fort_shoot"].clone();
    let tank_dies_volume = audio_data["tank_dies"].clone();
    let fort_captured_volume = audio_data["fort_captured"].clone();
    let laser_hit_volume = audio_data["laser_hit"].clone();
    let tank_reload_volume = audio_data["tank_reload"].clone();
    let tank_created_volume = audio_data["tank_created"].clone();

    // Initialize fonts
    let font_01 = Font::from_file("assets/fonts/VerminVibesV-Zlg3.ttf").unwrap();
    let font_02 = Font::from_file("assets/fonts/JustMyType-KePl.ttf").unwrap();

    // Initialize game map and minimap dimensions
    let mut number_of_tiles = 0;
    let mut game_map_width = 2500;
    let mut game_map_height = 2500;
    let mut minimap_width = 250;
    let mut minimap_height = 250;

    // Initialize teams
    let team_settings_file =
        File::open("team_settings.json").expect("Unable to open team_settings.json");
    let team_settings_file_reader = BufReader::new(team_settings_file);
    let team_settings_data: Value = serde_json::from_reader(team_settings_file_reader)
        .expect("Unable to parse team_settings.json");
    let number_of_teams = team_settings_data["number_of_teams"].clone();
    let team_colors = team_settings_data["team_colors"].clone();

    let mut team_tile_no: Vec<i32> = Vec::new();
    let mut team_tile_perc: Vec<f64> = Vec::new();

    let recording = false;

    // Initialize data collection arrays
    let mut tank_pop_array: Vec<Vec<i32>> = Vec::new();
    let mut fort_pop_array: Vec<Vec<i32>> = Vec::new();
    let mut special_tank_pop_array: Vec<Vec<i32>> = Vec::new();
    let mut special_fort_pop_array: Vec<Vec<i32>> = Vec::new();
    let mut tanks_killed_array: Vec<Vec<i32>> = Vec::new();
    let mut tanks_lost_array: Vec<Vec<i32>> = Vec::new();
    let mut average_tank_age_array: Vec<i32> = Vec::new();
    let mut team_territory_array: Vec<Vec<i32>> = Vec::new();
    let mut captured_fort_pop_array: Vec<Vec<i32>> = Vec::new();
    let mut dead_tank_pop_array: Vec<Vec<i32>> = Vec::new();
    let mut special_tank_ratio_array: Vec<f64> = Vec::new();
    let mut special_fort_ratio_array: Vec<f64> = Vec::new();
    let mut captured_fort_ratio_array: Vec<f64> = Vec::new();

    let mut temp_tank_pop: Vec<i32> = Vec::new();
    let mut temp_fort_pop: Vec<i32> = Vec::new();
    let mut temp_captured_fort_pop: Vec<i32> = Vec::new();
    let mut temp_special_tank_pop: Vec<i32> = Vec::new();
    let mut temp_special_fort_pop: Vec<i32> = Vec::new();

    let mut tanks_killed: Vec<i32> = Vec::new();
    let mut forts_captured: Vec<i32> = Vec::new();
    let mut tanks_lost: Vec<i32> = Vec::new();
    let mut forts_lost: Vec<i32> = Vec::new();
    let mut tanks_killed_by_tanks: Vec<i32> = Vec::new();
    let mut tanks_lost_to_tanks: Vec<i32> = Vec::new();

    let mut tank_age_team_time_array: Vec<Vec<[f32; 2]>> = Vec::new();
    let mut tank_kills_team_time_array: Vec<Vec<[f32; 2]>> = Vec::new();

    // Initialize image and texture arrays
    let mut tank_image_array: Vec<Image> = Vec::new();
    let mut tank_texture_array: Vec<Texture> = Vec::new();
    let mut fort_image_array: Vec<Image> = Vec::new();
    let mut fort_texture_array: Vec<Texture> = Vec::new();
    let mut dead_tank_image_array: Vec<[Image; 2]> = Vec::new();
    let mut dead_tank_texture_array: Vec<[Texture; 2]> = Vec::new();

    // Initialize game object arrays
    let mut entity_array: [Entity; 10000];
    let mut laser_array: Vec<Laser> = Vec::new();
    let mut missle_array: Vec<Missle> = Vec::new();
    let mut particle_array: Vec<Particle> = Vec::new();
    let mut tile_array: Vec<Vec<Tile>> = Vec::new();
    let mut game_display_text_array: Vec<GameDisplayText> = Vec::new();
    let mut dead_tank_array: Vec<DeadTank> = Vec::new();
    let mut button_array: Vec<Button> = Vec::new();
    let mut floating_text_array: Vec<FloatingText> = Vec::new();

    let mut tank_followed_uid: i32 = -1;
    let mut game_screen_rect: FloatRect;
    let mut stage_color: Color = Color::rgba(0, 0, 0, 255);
    let mut game_logs: Vec<String> = Vec::new();

   

    // Initialize Game Rules
    let game_rules_file = File::open("game_rules.json").expect("Unable to open game_rules.json");
    let game_rules_file_reader = BufReader::new(game_rules_file);
    let game_rules_data: Value =
        serde_json::from_reader(game_rules_file_reader).expect("Unable to parse game_rules.json");

    let tanks_need_supplies = game_rules_data["need_supplies"].clone();
    let convert_forts = game_rules_data["convert_forts"].clone();
    let convert_tanks = game_rules_data["convert_tanks"].clone();
    let gift_tanks = game_rules_data["gift_tanks"].clone();
    let forts_spawn_tanks = game_rules_data["forts_spawn_tanks"].clone();
    let fort_attack_forts = game_rules_data["fort_attack_forts"].clone();
    let neutered_forts = game_rules_data["neutered_forts"].clone();
    let fort_missles = game_rules_data["fort_missiles"].clone();
    let fort_healing = game_rules_data["fort_healing"].clone();
    let special_tanks = game_rules_data["special_tanks"].clone();
    let special_forts = game_rules_data["special_forts"].clone();
    let smart_missles = game_rules_data["smart_missiles"].clone();
    let tank_aimbot = game_rules_data["tank_aimbot"].clone();
    let fort_aimbot = game_rules_data["fort_aimbot"].clone();

    // Initialize Entity Values
    let entity_values_file =
        File::open("entity_values.json").expect("Unable to open entity_values.json");
    let entity_values_file_reader = BufReader::new(entity_values_file);
    let entity_values_data: Value = serde_json::from_reader(entity_values_file_reader)
        .expect("Unable to parse entity_values.json");

    let assimilating_fort_terrirory_power =
        entity_values_data["assimilating_fort_territory_power"].clone();
    let damaged_tanks_move_fast = entity_values_data["damaged_tanks_move_fast"].clone();
    let extra_population_interval = entity_values_data["extra_population_interval"].clone();
    let extra_population_quanta = entity_values_data["extra_population_quanta"].clone();
    //
    let fort_assimilation_ticks = entity_values_data["fort_assimilation_ticks"].clone();
    let fort_damage = entity_values_data["fort_damage"].clone();
    let fort_gun_length = entity_values_data["fort_gun_length"].clone();
    let fort_gun_width = entity_values_data["fort_gun_width"].clone();
    let fort_healing_lobby_size = entity_values_data["fort_healing_lobby_size"].clone();
    let fort_hitpoints = entity_values_data["fort_hitpoints"].clone();
    let fort_laser_cooldown = entity_values_data["fort_laser_cooldown"].clone();
    let fort_laser_speed = entity_values_data["fort_laser_speed"].clone();
    let fort_range = entity_values_data["fort_range"].clone();
    let fort_size = entity_values_data["fort_size"].clone();
    let fort_spawn_tank_time = entity_values_data["fort_spawn_tank_time"].clone();
    let fort_target_time = entity_values_data["fort_target_time"].clone();
    let fort_valid_healer_ticks = entity_values_data["fort_valid_healer_ticks"].clone();
    let fort_wander_time = entity_values_data["fort_wander_time"].clone();
    //
    let fort_missle_cooldown_factor = entity_values_data["fort_missle_cooldown_factor"].clone();
    let fort_missle_damage_factor = entity_values_data["fort_missle_damage_factor"].clone();
    let fort_missle_power = entity_values_data["fort_missle_power"].clone();
    let fort_missle_projectile_count = entity_values_data["fort_missle_projectile_count"].clone();
    let fort_missle_splash_radius = entity_values_data["fort_missle_splash_radius"].clone();
    let fort_missle_terminal_velocity = entity_values_data["fort_missle_terminal_velocity"].clone();
    let fort_missle_velocity = entity_values_data["fort_missle_velocity"].clone();
    //
    let new_tanks_move_fast = entity_values_data["new_tanks_move_fast"].clone();
    let panic_health_percent = entity_values_data["panic_health_percent"].clone();
    //
    let special_fort_damage_factor = entity_values_data["special_fort_damage_factor"].clone();
    let special_fort_healing_factor = entity_values_data["special_fort_healing_factor"].clone();
    let special_fort_hitpoints_factor = entity_values_data["special_fort_hitpoints_factor"].clone();
    let special_fort_missle_cooldown_factor =
        entity_values_data["special_fort_missle_cooldown_factor"].clone();
    let special_fort_missle_cooldown_factor =
        entity_values_data["special_fort_missle_cooldown_factor"].clone();
    let special_fort_range_factor = entity_values_data["special_fort_range_factor"].clone();
    let special_fort_tank_spawn_factor =
        entity_values_data["special_fort_tank_spawn_factor"].clone();
    let special_fort_tanks_healed = entity_values_data["special_fort_tanks_healed"].clone();
    let special_fort_tanks_spawned = entity_values_data["special_fort_tanks_spawned"].clone();
    let special_fort_territory_power = entity_values_data["special_fort_territory_power"].clone();
    //
    let special_tank_age = entity_values_data["special_tank_age"].clone();
    let special_tank_ammo_factor = entity_values_data["special_tank_ammo_factor"].clone();
    let special_tank_damage_factor = entity_values_data["special_tank_damage_factor"].clone();
    let special_tank_healing_factor = entity_values_data["special_tank_healing_factor"].clone();
    let special_tank_hitpoint_factor = entity_values_data["special_tank_hitpoint_factor"].clone();
    let special_tank_killcount = entity_values_data["special_tank_killcount"].clone();
    let special_tank_range_factor = entity_values_data["special_tank_range_factor"].clone();
    let special_tank_speed_factor = entity_values_data["special_tank_speed_factor"].clone();
    //
    let tank_damage = entity_values_data["tank_damage"].clone();
    let tank_fort_heal_percent = entity_values_data["tank_fort_heal_percent"].clone();
    let tank_hitpoints = entity_values_data["tank_hitpoints"].clone();
    let tank_laser_cooldown = entity_values_data["tank_laser_cooldown"].clone();
    let tank_laser_speed = entity_values_data["tank_laser_speed"].clone();
    let tank_los_range_factor = entity_values_data["tank_los_range_factor"].clone();
    let tank_max_ammo = entity_values_data["tank_max_ammo"].clone();
    let tank_min_pop = entity_values_data["tank_min_pop"].clone();
    let tank_range = entity_values_data["tank_range"].clone();
    let tank_special_fort_heal_percent =
        entity_values_data["tank_special_fort_heal_percent"].clone();
    let tank_speed_factor = entity_values_data["tank_speed_factor"].clone();
    let tank_supply_ammo_ticks = entity_values_data["tank_supply_ammo_ticks"].clone();
    let tank_target_ticks = entity_values_data["tank_target_ticks"].clone();
    let tanks_per_fort = entity_values_data["tanks_per_fort"].clone();
    //
    let tank_missle_count = entity_values_data["tank_missle_count"].clone();
    let tank_missle_damage_factor = entity_values_data["tank_missle_damage_factor"].clone();
    let tank_missle_power = entity_values_data["tank_missle_power"].clone();
    let tank_missle_projectile_count = entity_values_data["tank_missle_projectile_count"].clone();
    let tank_missle_splash_radius = entity_values_data["tank_missle_splash_radius"].clone();
    let tank_missle_terminal_velocity = entity_values_data["tank_missle_terminal_velocity"].clone();
    let tank_missle_velocity = entity_values_data["tank_missle_velocity"].clone();

    // Initialize Graphics variables
    let graphics_options_file =
        File::open("graphics_options.json").expect("Unable to open graphics_options.json");
    let graphics_options_file_reader = BufReader::new(graphics_options_file);
    let graphics_options_data: Value = serde_json::from_reader(graphics_options_file_reader)
        .expect("Unable to parse graphics_options.json");

    let screen_height = graphics_options_data["screen_height"].clone();
    let screen_width = graphics_options_data["screen_width"].clone();
    let wind_physics = graphics_options_data["wind_physics"].clone();
    let wind_max_speed_factor = graphics_options_data["wind_max_speed_factor"].clone();
    let wind_strength_factor = graphics_options_data["wind_strength_factor"].clone();
    let dead_tank_lifetime_ticks = graphics_options_data["dead_tank_lifetime_ticks"].clone();
    // 
    let fort_bloom_range_factor = graphics_options_data["fort_bloom_range_factor"].clone();
    let fort_missle_size = graphics_options_data["fort_missle_size"].clone();
    let fort_missle_trail_length = graphics_options_data["fort_missle_trail_length"].clone();
    // 
    let game_text_lifetime_ticks = graphics_options_data["game_text_lifetime_ticks"].clone();
    let particle_lifetime_ticks = graphics_options_data["particle_lifetime_ticks"].clone();
    // 
    let tank_bloom_range_factor = graphics_options_data["tank_bloom_range_factor"].clone();
    let tank_trail_length = graphics_options_data["tank_trail_length"].clone();
    let tank_missle_size = graphics_options_data["tank_missle_size"].clone();
    let tank_missle_trail_length = graphics_options_data["tank_missle_trail_length"].clone();
    // 
    let tile_size = graphics_options_data["tile_size"].clone();
    // 
    let mut debug_features = false;
    let mut draw_particles = true;
    let mut draw_minimap = true;
    let mut draw_user_interface = true;
    let mut draw_tiles = true;
    let mut draw_soft_borders = true;
    let mut draw_stats = true;

    // More Graphical Variables
    let mut camera_pos = Vector2f::new(0.0, 0.0);
    let mut camera_follow_tank = false;
    let mut follow_random_tank = false;
    let mut tank_followed_index: i32 = -1;
    let mut game_text = String::new();

    // Initialize other variables
    let mut mouse_pos = Vector2f::new(0.0, 0.0);
    let mut game_ticks = 0;
    let mut winner: i32 = -1;
    let mut last_fps = 0;
    let mut wind_vel = Vector2f::new(0.0, 0.0);
    let mut minimap_rect = FloatRect::new(0.0, 0.0, 0.0, 0.0);
    let mut scroll_sensitivity = 15;
    let mut paused = false;
    let mut game_volume = 100;
    let mut autoplay = true;
    let mut current_game_timer = 0;
    let mut extra_population = 0;
    let mut tank_death_positions: Vec<[f32; 2]> = Vec::new();
    let mut total_claimed_tiles = 0;
    let mut total_tank_pop = 0;
    let mut team_gang_up = false;
    let mut gang_up_victim_team: i32 = -1;

}
