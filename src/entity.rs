#![allow(unused)]
use crate::EVENT_LOGGING;
use sfml::graphics::{Color, FloatRect, Sprite, Texture};
use sfml::system::{Vector2f, Vector2i};
// mod crate::config;
use crate::config::{NUMBER_OF_TEAMS, TEAM_COLORS};
use rand::Rng;
use std::sync::atomic::{AtomicU32, Ordering};

static NEXT_ENTITY_UID: AtomicU32 = AtomicU32::new(1);

pub struct Common<'s> {
    acc: Vector2f,
    age: f32,
    ang: f32,
    ang_acc: f32,
    ang_vel: f32,
    color: Color,
    damage: f32,
    enemy_index: i32,
    enemy_type: Option<EntityType>,
    enemy_uid: i32,
    entity_sprite: Sprite<'s>,
    entity_type: EntityType,
    entity_uid: i32,
    health_arc_angle: f32,
    hit_points: f32,
    hit_points_max: f32,
    idle: bool,
    in_action: bool,
    kill_count: i32,
    laser_cooldown: f32,
    laser_cooldown_max: f32,
    last_bullet_entity_type: Option<EntityType>,
    last_bullet_team: i32,
    last_bullet_uid: i32,
    level: i32,
    lock_target_timer: f32,
    lock_target_timer_max: f32,
    opposite_color: Color,
    pos: Vector2f,
    range: f32,
    rect: FloatRect,
    special: bool,
    target_ang: f32,
    target_pos: Vector2f,
    team: i32,
    texture_height: u32,
    texture_width: u32,
    vel: Vector2f,
    wander_timer: f32,
    wander_timer_max: f32,
}
pub struct TankData {
    wander_pos: Vector2f,
    ammo_supply_max: i32,
    ammo_supply: i32,
    no_ammo: bool,
    ammo_supply_timer_max: f32,
    ammo_supply_timer: f32,
    ammo_target_id: i32,
    tilt_preference: i32,
    speed_factor: f32,
}

pub struct FortData {
    gun_barrel_length: f32,
    gun_barrel_width: f32,
    flicker_timer: f32,
    flicker_timer_max: f32,
    flicker_timer_vel: f32,
    tank_spawn_timer: f32,
    tank_spawn_timer_max: f32,
    size: f32,
    missle_cooldown: f32,
    missle_cooldown_max: f32,
    tanks_healed: i32,
    tanks_spawned: i32,
    healed_tanks: [i32; 10],
    assimilation_arc_angle: f32,
}

pub enum Entity<'s> {
    Tank { common: Common<'s>, tank: TankData },
    Fort { common: Common<'s>, fort: FortData },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EntityType {
    Tank,
    Fort,
}

impl<'s> Entity<'s> {
    pub fn new_tank(team: i32, pos: Vector2f, special: bool, texture_array: &'s Vec<Texture>) -> Self {
        if EVENT_LOGGING {
            println!("New Tank Created!");
        }
        Entity::Tank {
            common: Common{
                acc: Vector2f::new(0.0, 0.0),
                age: 0.0,
                ang: 0.0,
                ang_acc: 0.0,
                ang_vel: 0.0,
                color: TEAM_COLORS[team as usize],
                damage: 0.0,
                enemy_index: -1,
                enemy_type: None,
                enemy_uid: -1,
                entity_sprite: Sprite::with_texture(&texture_array[team as usize]),
                entity_type: EntityType::Tank,
                entity_uid: NEXT_ENTITY_UID.fetch_add(1, Ordering::SeqCst) as i32,
                health_arc_angle: 360.0,
                hit_points: 100.0,
                hit_points_max: 100.0,
                idle: false,
                in_action: true,
                kill_count: 0,
                laser_cooldown: 0.0,
                laser_cooldown_max: 1.0,
                last_bullet_entity_type: None,
                last_bullet_team: -1,
                last_bullet_uid: -1,
                level: 1,
                lock_target_timer: 0.0,
                lock_target_timer_max: 1.0,
                opposite_color: Color::rgb(
                    255 - TEAM_COLORS[team as usize].r,
                    255 - TEAM_COLORS[team as usize].g,
                    255 - TEAM_COLORS[team as usize].b,
                ),
                pos: pos,
                range: 150.0,
                rect: FloatRect::new(pos.x - 7.5, pos.y - 15.0, 15.0, 30.0),
                special,
                target_ang: 0.0,
                target_pos: Vector2f::new(0.0, 0.0),
                team: team,
                texture_height: 30,
                texture_width: 15,
                vel: Vector2f::new(0.0, 0.0),
                wander_timer: 0.0,
                wander_timer_max: 5.0,
            },
            tank: TankData {
                wander_pos: Vector2f::new(0.0, 0.0),
                ammo_supply_max: 5,
                ammo_supply: 5,
                no_ammo: false,
                ammo_supply_timer_max: 10.0,
                ammo_supply_timer: 0.0,
                ammo_target_id: -1,
                tilt_preference: if rand::random() { 1 } else { -1 },
                speed_factor: 1.0,
            },
        }
    }
}
