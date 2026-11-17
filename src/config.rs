//! Typed representations of the five JSON config files, loaded at runtime
//! from the same relative paths the C++ build used (GameRules.json,
//! EntityValues.json, Graphics.json, TeamSettings.json, Audio.json).
//!
//! Field names/types are taken directly from every `_jData_GameRules[...]`,
//! `_jsonData01[...]`, `_jsonGraphicsData[...]`, `_jData_Teams[...]`, and
//! `_jsonAudioData[...]` access in TankAttack.cpp.

use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize, Clone)]
pub struct GameRules {
    #[serde(rename = "NEED_SUPPLIES")]
    pub need_supplies: bool,
    #[serde(rename = "CONVERT_FORTS")]
    pub convert_forts: bool,
    #[serde(rename = "CONVERT_TANKS")]
    pub convert_tanks: bool,
    #[serde(rename = "GIFT_TANKS")]
    pub gift_tanks: bool,
    #[serde(rename = "FORTS_SPAWN_TANKS")]
    pub forts_spawn_tanks: bool,
    #[serde(rename = "FORTS_ATTACK_FORTS")]
    pub forts_attack_forts: bool,
    #[serde(rename = "FORTS_NEUTERED")]
    pub forts_neutered: bool,
    #[serde(rename = "FORT_MISSLES")]
    pub fort_missles: bool,
    #[serde(rename = "FORT_HEALING")]
    pub fort_healing: bool,
    #[serde(rename = "SPECIAL_TANKS")]
    pub special_tanks: bool,
    #[serde(rename = "SPECIAL_FORTS")]
    pub special_forts: bool,
    #[serde(rename = "SMART_MISSLES")]
    pub smart_missles: bool,
    // NOTE: C++ local var is `TANK_AIMBOT` but json key is `TANKS_HAVE_AIMBOT`
    #[serde(rename = "TANKS_HAVE_AIMBOT")]
    pub tank_aimbot: bool,
    // C++ local var is `FORT_AIMBOT`, json key is `FORTS_HAVE_AIMBOT`
    #[serde(rename = "FORTS_HAVE_AIMBOT")]
    pub fort_aimbot: bool,
}

impl GameRules {
    pub fn load() -> Self {
        load_json("GameRules.json")
    }
}

/// EntityValues.json — mixed camelCase / snake_case / SCREAMING_SNAKE_CASE,
/// matched exactly to the C++ member names since several are renamed on
/// load (e.g. json `Fort_SPAWNTANK_TIME` -> local `Fort_SPAWNTANK_TIME`,
/// kept as-is; only truly mismatched names get an explicit #[serde(rename)]).
#[derive(Debug, Deserialize, Clone)]
pub struct EntityValues {
    #[serde(rename = "Tank_HP")]
    pub tank_hp: f32,
    #[serde(rename = "Tank_RANGE")]
    pub tank_range: i32,
    #[serde(rename = "Tank_DAMAGE")]
    pub tank_damage: f32,

    #[serde(rename = "Tank_MAX_AMMO")]
    pub tank_max_ammo: i32,
    #[serde(rename = "Tank_MISSLE_NO")]
    pub tank_missle_no: i32,
    #[serde(rename = "Tank_MIN_POP")]
    pub tank_min_pop: i32,
    #[serde(rename = "Tank_SPEED_FACTOR")]
    pub tank_speed_factor: f32,
    #[serde(rename = "Tank_LASER_COOLDOWN")]
    pub tank_laser_cooldown: i32,
    #[serde(rename = "Tank_LASER_SPEED")]
    pub tank_laser_speed: f32,

    #[serde(rename = "Fort_HP")]
    pub fort_hp: f32,
    #[serde(rename = "Fort_RANGE")]
    pub fort_range: i32,
    #[serde(rename = "Fort_DAMAGE")]
    pub fort_damage: f32,

    #[serde(rename = "Fort_SIZE")]
    pub fort_size: i32,
    #[serde(rename = "Fort_SPAWNTANK_TIME")]
    pub fort_spawntank_time: i32,
    #[serde(rename = "Fort_LASER_COOLDOWN")]
    pub fort_laser_cooldown: i32,
    #[serde(rename = "Fort_LASER_SPEED")]
    pub fort_laser_speed: f32,

    #[serde(rename = "PanicHealthPerc")]
    pub panic_health_perc: f32,

    #[serde(rename = "special_tank_age")]
    pub special_tank_age: i32,
    #[serde(rename = "special_tank_killcount")]
    pub special_tank_killcount: i32,

    #[serde(rename = "special_tank_damage_factor")]
    pub special_tank_damage_factor: f32,
    #[serde(rename = "special_tank_ammo_factor")]
    pub special_tank_ammo_factor: f32,
    #[serde(rename = "special_tank_hitpoints_factor")]
    pub special_tank_hitpoints_factor: f32,
    #[serde(rename = "special_tank_healing_factor")]
    pub special_tank_healing_factor: f32,
    #[serde(rename = "special_tank_speed_factor")]
    pub special_tank_speed_factor: f32,
    #[serde(rename = "special_tank_range_factor")]
    pub special_tank_range_factor: f32,

    #[serde(rename = "special_fort_tanks_healed")]
    pub special_fort_tanks_healed: i32,
    #[serde(rename = "special_fort_tanks_spawned")]
    pub special_fort_tanks_spawned: i32,

    #[serde(rename = "special_fort_hitpoints_factor")]
    pub special_fort_hitpoints_factor: f32,
    #[serde(rename = "special_fort_healing_factor")]
    pub special_fort_healing_factor: f32,
    #[serde(rename = "special_fort_tankspawn_factor")]
    pub special_fort_tankspawn_factor: f32,
    #[serde(rename = "special_fort_misslecooldown_factor")]
    pub special_fort_misslecooldown_factor: f32,
    #[serde(rename = "special_fort_damage_factor")]
    pub special_fort_damage_factor: f32,
    #[serde(rename = "special_fort_range_factor")]
    pub special_fort_range_factor: f32,
    #[serde(rename = "special_fort_territory_power")]
    pub special_fort_territory_power: f32,

    #[serde(rename = "assimilating_fort_territory_power")]
    pub assimilating_fort_territory_power: f32,

    #[serde(rename = "fort_missle_cooldown_factor")]
    pub fort_missle_cooldown_factor: f32,
    #[serde(rename = "fort_target_time")]
    pub fort_target_time: i32,
    #[serde(rename = "fort_wander_time")]
    pub fort_wander_time: i32,
    #[serde(rename = "fort_gun_length")]
    pub fort_gun_length: i32,
    #[serde(rename = "fort_gun_width")]
    pub fort_gun_width: i32,
    #[serde(rename = "tank_supplyammo_ticks")]
    pub tank_supplyammo_ticks: i32,
    #[serde(rename = "tank_target_ticks")]
    pub tank_target_ticks: i32,
    #[serde(rename = "tank_LOSrange_factor")]
    pub tank_losrange_factor: f32,

    #[serde(rename = "tank_fort_heal_percent")]
    pub tank_fort_heal_percent: f32,
    #[serde(rename = "tank_specialfort_heal_percent")]
    pub tank_specialfort_heal_percent: f32,
    #[serde(rename = "tanks_per_fort")]
    pub tanks_per_fort: f32,

    #[serde(rename = "fort_assimilation_ticks")]
    pub fort_assimilation_ticks: i32,
    #[serde(rename = "fort_validhealer_ticks")]
    pub fort_validhealer_ticks: i32,

    // C++ local name `maxTankHealArray` <- json key `fort_healing_lobby_size`
    #[serde(rename = "fort_healing_lobby_size")]
    pub max_tank_heal_array: i32,

    #[serde(rename = "tank_missle_velocity")]
    pub tank_missle_velocity: f32,
    #[serde(rename = "tank_missle_damagefactor")]
    pub tank_missle_damagefactor: f32,
    #[serde(rename = "tank_missle_projectile_no")]
    pub tank_missle_projectile_no: i32,
    #[serde(rename = "tank_missle_splash_radius")]
    pub tank_missle_splash_radius: f32,
    #[serde(rename = "tank_missle_terminal_velocity")]
    pub tank_missle_terminal_velocity: f32,
    #[serde(rename = "tank_missle_power")]
    pub tank_missle_power: f32,

    #[serde(rename = "fort_missle_velocity")]
    pub fort_missle_velocity: f32,
    #[serde(rename = "fort_missle_damagefactor")]
    pub fort_missle_damagefactor: f32,
    #[serde(rename = "fort_missle_projectile_no")]
    pub fort_missle_projectile_no: i32,
    #[serde(rename = "fort_missle_splash_radius")]
    pub fort_missle_splash_radius: f32,
    #[serde(rename = "fort_missle_terminal_velocity")]
    pub fort_missle_terminal_velocity: f32,
    #[serde(rename = "fort_missle_power")]
    pub fort_missle_power: f32,

    #[serde(rename = "SHINYS_MOVE_FAST")]
    pub shinys_move_fast: bool,
    #[serde(rename = "DAMAGED_MOVE_FAST")]
    pub damaged_move_fast: bool,

    #[serde(rename = "extra_population_interval")]
    pub extra_population_interval: i32,
    #[serde(rename = "extra_population_quanta")]
    pub extra_population_quanta: i32,
}

impl EntityValues {
    pub fn load() -> Self {
        load_json("EntityValues.json")
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Graphics {
    #[serde(rename = "SCREEN_WIDTH")]
    pub screen_width: f32,
    #[serde(rename = "SCREEN_HEIGHT")]
    pub screen_height: f32,

    #[serde(rename = "Default_Particle_Ticks")]
    pub default_particle_ticks: i32,
    #[serde(rename = "Tiles_Size")]
    pub tiles_size: i32,
    #[serde(rename = "DeadTank_Ticks")]
    pub dead_tank_ticks: i32,
    #[serde(rename = "GameText_Ticks")]
    pub game_text_ticks: i32,
    #[serde(rename = "Tank_BloomRangeFactor")]
    pub tank_bloom_range_factor: f32,
    #[serde(rename = "Fort_BloomRangeFactor")]
    pub fort_bloom_range_factor: f32,

    #[serde(rename = "Tank_Missle_Size")]
    pub tank_missle_size: f32,
    #[serde(rename = "Tank_Missle_TrailLength")]
    pub tank_missle_trail_length: i32,

    #[serde(rename = "Fort_Missle_Size")]
    pub fort_missle_size: f32,
    #[serde(rename = "Fort_Missle_TrailLength")]
    pub fort_missle_trail_length: i32,

    #[serde(rename = "WIND")]
    pub wind: bool,
    #[serde(rename = "max_wind_speed_factor")]
    pub max_wind_speed_factor: f32,
    #[serde(rename = "wind_strength_factor")]
    pub wind_strength_factor: f32,

    #[serde(rename = "tank_trail_length")]
    pub tank_trail_length: i32,
    #[serde(rename = "flag_node_length")]
    pub flag_node_length: i32,
    #[serde(rename = "flag_node_number")]
    pub flag_node_number: i32,
}

impl Graphics {
    pub fn load() -> Self {
        load_json("Graphics.json")
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct TeamSettings {
    #[serde(rename = "numberOfTeams")]
    pub number_of_teams: i32,
    pub colors: Vec<[u8; 3]>,
}

impl TeamSettings {
    pub fn load() -> Self {
        load_json("TeamSettings.json")
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct AudioVolumes {
    pub tankshoot: i32,
    pub fortshoot: i32,
    pub laserhit: i32,
    pub tankdies: i32,
    pub tankreload: i32,
    pub tankcreated: i32,
    pub fortcaptured: i32,
    pub background: i32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AudioConfig {
    pub volumes: AudioVolumes,
    pub background_music_file: String,
}

impl AudioConfig {
    pub fn load() -> Self {
        load_json("Audio.json")
    }
}

/// Bundles all five config files, mirroring the file-scope const globals
/// loaded at the top of TankAttack.cpp before `main()`.
pub struct Config {
    pub rules: GameRules,
    pub entity_values: EntityValues,
    pub graphics: Graphics,
    pub teams: TeamSettings,
    pub audio: AudioConfig,
}

impl Config {
    pub fn load() -> Self {
        Config {
            rules: GameRules::load(),
            entity_values: EntityValues::load(),
            graphics: Graphics::load(),
            teams: TeamSettings::load(),
            audio: AudioConfig::load(),
        }
    }
}

fn load_json<T: for<'de> Deserialize<'de>>(path: &str) -> T {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read config file {path}: {e}"));
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("failed to parse config file {path}: {e}"))
}
