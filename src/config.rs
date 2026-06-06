use serde_json::Value;
use sfml::graphics::Color;
use std::fs::File;
use std::io::BufReader;
use std::sync::LazyLock;

pub static TEAM_COLORS: LazyLock<Vec<Color>> = LazyLock::new(|| {
    let file = match File::open("config/team_settings.json") {
        Ok(file) => {
            if super::EVENT_LOGGING {
                println!("Successfully opened team_settings.json");
            }
            file
        }
        Err(e) => {
            panic!("Unable to open team_settings.json: {}", e);
        }
    };
    let reader = BufReader::new(file);
    let data: Value = serde_json::from_reader(reader).expect("Unable to parse team_settings.json");

    let team_colors: Vec<[u8; 3]> = serde_json::from_value(data["team_colors"].clone())
        .expect("Unable to parse team_colors from team_settings.json");

    team_colors
        .iter()
        .map(|&[r, g, b]| Color::rgb(r, g, b))
        .collect()
});
pub static NUMBER_OF_TEAMS: LazyLock<i32> = LazyLock::new(|| {
    // let file = File::open("config/team_settings.json").expect("Unable to open team_settings.json");
    let file = match File::open("config/team_settings.json") {
        Ok(file) => {
            if super::EVENT_LOGGING {
                println!("Successfully opened team_settings.json");
            }
            file
        }
        Err(e) => {
            panic!("Unable to open team_settings.json: {}", e);
        }
    };
    let reader = BufReader::new(file);
    let data: Value = serde_json::from_reader(reader).expect("Unable to parse team_settings.json");

    data["number_of_teams"].as_i64().unwrap() as i32
});

pub static SCREEN_WIDTH: u32 = 1920;
pub static SCREEN_HEIGHT: u32 = 1080;
