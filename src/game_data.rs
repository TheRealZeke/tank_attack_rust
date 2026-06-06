use std::fs::File;
use std::io::BufReader;
use std::time::{Duration, Instant};

use serde_json::Value;

use sfml::cpp::FBox;
use sfml::graphics::{Font, Image, RenderWindow};
use sfml::window::{Style, VideoMode};

use crate::config::{NUMBER_OF_TEAMS, SCREEN_HEIGHT, SCREEN_WIDTH, TEAM_COLORS};
use crate::graphics_functions::{transform_img_col};

pub struct AudioResources {
    // audio_file: File,
    // audio_file_reader: BufReader<File>,
    // audio_data: Value,
    background_music_volume: u64,
    tank_shoot_volume: u64,
    fort_shoot_volume: u64,
    tank_dies_volume: u64,
    fort_captured_volume: u64,
    laser_hit_volume: u64,
    tank_reload_volume: u64,
    tank_created_volume: u64,
}

pub struct GraphicsResources {
    tank_image: FBox<Image>,
    fort_image: FBox<Image>,
    dead_tank_image: [FBox<Image>; 2],
    font_01: FBox<Font>,
    font_02: FBox<Font>,
    tank_image_array: Vec<FBox<Image>>,
}

pub struct WindowResources {
    window: FBox<RenderWindow>,
    quit_game: bool,
    game_fps_ratio: f32,
    summon_team: usize,
}

pub struct GameVariables {
    number_of_tiles: u64,
    game_map_width: u64,
    game_map_height: u64,
    minimap_width: u64,
    minimap_height: u64,

    team_tile_no: Vec<i64>,
    team_tile_perc: Vec<f32>,
    recording: bool,

    tank_pop_array: Vec<Vec<i32>>,
    fort_pop_array: Vec<Vec<i32>>,
    special_tank_pop_array: Vec<Vec<i32>>,
    special_fort_pop_array: Vec<Vec<i32>>,
    tanks_killed_array: Vec<Vec<i32>>,
    tanks_lost_array: Vec<Vec<i32>>,
    average_tank_age_array: Vec<i32>,
    team_territory_array: Vec<Vec<i32>>,
    captured_fort_pop_array: Vec<Vec<i32>>,
    dead_tank_pop_array: Vec<Vec<i32>>,
    special_tank_ratio_array: Vec<f64>,
    special_fort_ratio_array: Vec<f64>,
    captured_fort_ratio_array: Vec<f64>,

    temp_tank_pop: Vec<i32>,
    temp_fort_pop: Vec<i32>,
    temp_captured_fort_pop: Vec<i32>,
    temp_special_tank_pop: Vec<i32>,
    temp_special_fort_pop: Vec<i32>,

    tanks_killed: Vec<i32>,
    forts_captured: Vec<i32>,
    tanks_lost: Vec<i32>,
    forts_lost: Vec<i32>,
    tanks_killed_by_tanks: Vec<i32>,
    tanks_lost_to_tanks: Vec<i32>,

    tank_age_team_time_array: Vec<Vec<[f32; 2]>>,
    tank_kills_team_time_array: Vec<Vec<[f32; 2]>>,
}

pub struct GameData {
    pub audio: AudioResources,
    pub graphics: GraphicsResources,
    pub window: WindowResources,
    pub game_variables: GameVariables,
}

impl GameData {
    pub fn new() -> Self {
        let audio_file = File::open("config/audio.json").expect("Unable to open Audio.json");
        let audio_file_reader = BufReader::new(audio_file);
        let audio_data: Value =
            serde_json::from_reader(audio_file_reader).expect("Unable to parse Audio.json");
        
        let default_tank = Image::from_file("assets/images/tank.png").unwrap();
        let mut tank_image_array: Vec<FBox<Image>> = Vec::new();
        for i in 0..(*NUMBER_OF_TEAMS as usize){
            tank_image_array.push(transform_img_col(&default_tank, TEAM_COLORS[i], 0));
        }
        
        
        
        
        GameData {
            audio: AudioResources {
                background_music_volume: audio_data["background"].as_u64().unwrap(),
                fort_shoot_volume: audio_data["fort_shoot"].as_u64().unwrap(),
                tank_shoot_volume: audio_data["tank_shoot"].as_u64().unwrap(),
                tank_dies_volume: audio_data["tank_dies"].as_u64().unwrap(),
                fort_captured_volume: audio_data["fort_captured"].as_u64().unwrap(),
                laser_hit_volume: audio_data["laser_hit"].as_u64().unwrap(),
                tank_reload_volume: audio_data["tank_reload"].as_u64().unwrap(),
                tank_created_volume: audio_data["tank_created"].as_u64().unwrap(),
            },
            graphics: GraphicsResources {
                tank_image: Image::from_file("assets/images/tank.png").unwrap(),
                fort_image: Image::from_file("assets/images/fort.bmp").unwrap(),
                dead_tank_image: [
                    Image::from_file("assets/images/dead_tank_01.png").unwrap(),
                    Image::from_file("assets/images/dead_tank_02.png").unwrap(),
                ],
                font_01: Font::from_file("assets/fonts/VerminVibesV-Zlg3.ttf").unwrap(),
                font_02: Font::from_file("assets/fonts/JustMyType-KePl.ttf").unwrap(),
                tank_image_array,
            },
            window: WindowResources {
                window: RenderWindow::new(
                    VideoMode::new(SCREEN_WIDTH, SCREEN_HEIGHT, 64),
                    "Tank Attack",
                    Style::CLOSE,
                    &Default::default(),
                )
                .expect("Failed to create window"),
                quit_game: false,
                game_fps_ratio: 0.0,
                summon_team: 0,
            },
            game_variables: GameVariables {
                number_of_tiles: 0,
                game_map_width: 2500,
                game_map_height: 2500,
                minimap_width: 250,
                minimap_height: 250,

                team_tile_no: Vec::new(),
                team_tile_perc: Vec::new(),
                recording: false,

                tank_pop_array: Vec::new(),
                fort_pop_array: Vec::new(),
                special_tank_pop_array: Vec::new(),
                special_fort_pop_array: Vec::new(),
                tanks_killed_array: Vec::new(),
                tanks_lost_array: Vec::new(),
                average_tank_age_array: Vec::new(),
                team_territory_array: Vec::new(),
                captured_fort_pop_array: Vec::new(),
                dead_tank_pop_array: Vec::new(),
                special_tank_ratio_array: Vec::new(),
                special_fort_ratio_array: Vec::new(),
                captured_fort_ratio_array: Vec::new(),

                temp_tank_pop: Vec::new(),
                temp_fort_pop: Vec::new(),
                temp_captured_fort_pop: Vec::new(),
                temp_special_tank_pop: Vec::new(),
                temp_special_fort_pop: Vec::new(),

                tanks_killed: Vec::new(),
                forts_captured: Vec::new(),
                tanks_lost: Vec::new(),
                forts_lost: Vec::new(),
                tanks_killed_by_tanks: Vec::new(),
                tanks_lost_to_tanks: Vec::new(),

                tank_age_team_time_array: Vec::new(),
                tank_kills_team_time_array: Vec::new(),
            },
        }
    }
}
