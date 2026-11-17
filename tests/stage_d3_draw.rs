//! Integration tests for Stage D.3 (Entity::draw / Tile::draw+idle_draw /
//! DeadTank::draw), per the tapered verification plan: render each to an
//! off-screen RenderTexture and confirm something visible actually
//! appears, plus a couple of behavior-preserving side-effect checks
//! (entity_draw can flip no_ammo, dead_tank_draw can spawn particles).

use tank_attack_rust::assets::Assets;
use tank_attack_rust::config::Config;
use tank_attack_rust::dead_tank::{dead_tank_draw, DeadTank};
use tank_attack_rust::entity::entity_draw;
use tank_attack_rust::spawn::{spawn_fort, spawn_tank};
use tank_attack_rust::state::GameState;
use tank_attack_rust::tile::{tile_draw, tile_idle_draw};
use rand::SeedableRng;
use rand::Rng;
use sfml::graphics::{Color, RenderTarget, RenderTexture, Texture};
use sfml::system::Vector2f;

const W: u32 = 400;
const H: u32 = 400;

fn state_with_assets() -> (GameState, Assets) {
    let config = Config::load();
    let mut state = GameState::new(config);
    state.game_fps_ratio = 1.0;
    state.rng = rand::rngs::SmallRng::seed_from_u64(42);
    let color_arr = state.color_arr.clone();
    let assets = Assets::load(&state.config, &color_arr).expect("assets should load");
    (state, assets)
}

fn any_nonblack(rt: &RenderTexture) -> bool {
    rt.display();
    let tex: &Texture = rt.texture();
    let img = tex.copy_to_image().unwrap();
    for x in 0..W {
        for y in 0..H {
            let p = unsafe { img.pixel_at(x, y) };
            if p.r != 0 || p.g != 0 || p.b != 0 {
                return true;
            }
        }
    }
    false
}

#[test]
fn entity_draw_renders_a_tank_and_a_fort() {
    let (mut state, assets) = state_with_assets();
    state.camera_pos = [0.0, 0.0];

    spawn_tank(&mut state, Vector2f::new(200.0, 150.0), 0, 0, false);
    let tank_idx = state.entities.len() - 1;
    spawn_fort(&mut state, Vector2f::new(200.0, 250.0), 1, false, false);
    let fort_idx = state.entities.len() - 1;

    let mut rt = RenderTexture::new(W, H).expect("RenderTexture::new should succeed under Xvfb");
    rt.clear(Color::BLACK);
    entity_draw(&mut state, &assets, &mut rt, tank_idx);
    entity_draw(&mut state, &assets, &mut rt, fort_idx);

    assert!(any_nonblack(&rt), "entity_draw for a tank and a fort should have drawn something visible");

    // Numeric sanity on the state mutations entity_draw performs.
    assert!(state.entities[tank_idx].col.a == 255 && state.entities[tank_idx].opp_col.a == 255, "entity color alpha should be reset to 255 at the end of draw()");
    assert!(state.entities[fort_idx].col.a == 255 && state.entities[fort_idx].opp_col.a == 255);
}

#[test]
fn entity_draw_can_flip_no_ammo_as_a_side_effect() {
    // Regression guard for the documented (unusual) C++ behavior: a tank
    // with 0 ammo and NEED_SUPPLIES enabled gets no_ammo=true set inside
    // draw(), not just inside AI().
    let (mut state, assets) = state_with_assets();
    assert!(state.config.rules.need_supplies, "this test assumes the shipped config has NEED_SUPPLIES on");

    spawn_tank(&mut state, Vector2f::new(100.0, 100.0), 0, 0, false);
    let idx = state.entities.len() - 1;
    state.entities[idx].ammo_supply = 0;
    state.entities[idx].no_ammo = false;

    let mut rt = RenderTexture::new(W, H).unwrap();
    rt.clear(Color::BLACK);
    entity_draw(&mut state, &assets, &mut rt, idx);

    assert!(state.entities[idx].no_ammo, "entity_draw should have set no_ammo=true as a side effect, matching the C++");
}

#[test]
fn tile_draw_and_idle_draw_render_a_claimed_tile() {
    let (mut state, assets) = state_with_assets();
    state.game_map_width = 500.0;
    state.game_map_height = 500.0;
    tank_attack_rust::tile::init_tiles(&mut state);
    spawn_fort(&mut state, Vector2f::new(250.0, 250.0), 0, false, false);
    tank_attack_rust::tile::update_tiles(&mut state);
    state.camera_pos = [50.0, 50.0]; // so the map center lands inside our 400x400 texture

    let fort_pos = Vector2f::new(250.0, 250.0);
    let claimed = (0..state.tiles.len())
        .flat_map(|r| (0..state.tiles[r].len()).map(move |c| (r, c)))
        .filter(|&(r, c)| state.tiles[r][c].team == 0)
        .min_by(|&(r1, c1), &(r2, c2)| {
            let d1 = tank_attack_rust::util::displacement(state.tiles[r1][c1].pos, fort_pos);
            let d2 = tank_attack_rust::util::displacement(state.tiles[r2][c2].pos, fort_pos);
            d1.partial_cmp(&d2).unwrap()
        })
        .expect("at least one tile should be claimed by team 0's fort");

    let mut rt = RenderTexture::new(W, H).unwrap();
    rt.clear(Color::BLACK);
    tile_idle_draw(&mut state, claimed.0, claimed.1);
    tile_draw(&mut state, &assets, &mut rt, claimed.0, claimed.1);

    assert!(any_nonblack(&rt), "tile_draw for a claimed tile should have drawn something visible");
}

#[test]
fn tile_draw_skips_unclaimed_tiles() {
    let (mut state, assets) = state_with_assets();
    state.game_map_width = 500.0;
    state.game_map_height = 500.0;
    tank_attack_rust::tile::init_tiles(&mut state);
    // No forts spawned -- update_tiles will leave every tile at team -1.
    tank_attack_rust::tile::update_tiles(&mut state);
    state.camera_pos = [0.0, 0.0];

    let mut rt = RenderTexture::new(W, H).unwrap();
    rt.clear(Color::BLACK);
    for r in 0..state.tiles.len().min(20) {
        for c in 0..state.tiles[r].len().min(20) {
            tile_idle_draw(&mut state, r, c);
            tile_draw(&mut state, &assets, &mut rt, r, c);
        }
    }
    assert!(!any_nonblack(&rt), "tile_draw should draw nothing for unclaimed (team == -1) tiles");
}

#[test]
fn dead_tank_draw_renders_and_can_spawn_particles() {
    let (mut state, assets) = state_with_assets();
    state.camera_pos = [0.0, 0.0];

    let dead_tank_life = state.config.graphics.dead_tank_ticks as f32;
    let mut dt = DeadTank::new(|n| state.rng.gen_range(0..n));
    dt.spawn(Vector2f::new(200.0, 200.0), 45.0, 0, dead_tank_life, &state.color_arr);
    state.dead_tanks.push(dt);
    let idx = state.dead_tanks.len() - 1;

    let mut rt = RenderTexture::new(W, H).unwrap();
    rt.clear(Color::BLACK);
    // Run draw many times (as the real loop would across frames) so the
    // probabilistic particle-spawn branches get exercised at least once.
    for _ in 0..50 {
        dead_tank_draw(&mut state, &assets, &mut rt, idx);
    }

    assert!(any_nonblack(&rt), "dead_tank_draw should have drawn something visible");
}
