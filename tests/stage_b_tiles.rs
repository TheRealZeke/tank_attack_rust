//! Integration tests for Stage B (tile.rs: update_tile, update_tile_borders,
//! update_tiles, init_tiles), per continuation_spec.md's acceptance
//! criteria (§B.6).

use tank_attack_rust::config::Config;
use tank_attack_rust::spawn::spawn_fort;
use tank_attack_rust::state::GameState;
use tank_attack_rust::tile::{get_tile_neighbours, init_tiles, update_tiles};
use sfml::system::Vector2f;

fn fresh_state() -> GameState {
    let cfg = Config::load();
    let mut state = GameState::new(cfg);
    state.game_fps_ratio = 1.0;
    // Use a small map so these tests run fast and grid math is easy to
    // reason about; GameState::new's defaults (2500x2500) work fine too,
    // but a smaller one keeps init_tiles/update_tiles snappy in CI.
    state.game_map_width = 500.0;
    state.game_map_height = 500.0;
    state
}

#[test]
fn empty_map_has_no_claimed_tiles() {
    let mut state = fresh_state();
    init_tiles(&mut state);
    assert!(!state.tiles.is_empty(), "init_tiles should have produced a nonempty grid");

    update_tiles(&mut state);

    for i in 0..state.number_of_teams as usize {
        assert_eq!(state.team_tile_no[i], 0, "team {i} should own 0 tiles on an empty map");
        assert_eq!(state.team_tile_perc[i], 0.0, "team {i} percentage should be 0 (guarded against div-by-zero)");
    }
    assert_eq!(state.total_claimed_tiles, 0.0);

    // No tile should think it has a fort, and none should be a border tile
    // (no team boundaries exist when every tile is team -1).
    for row in &state.tiles {
        for t in row {
            assert_eq!(t.team, -1);
            assert!(t.fort_uid.is_none());
            assert!(!t.border_tile);
        }
    }
}

#[test]
fn single_fort_at_center_claims_a_nonzero_area_matching_pi_r_squared() {
    let mut state = fresh_state();
    init_tiles(&mut state);

    let cx = state.game_map_width / 2.0;
    let cy = state.game_map_height / 2.0;
    spawn_fort(&mut state, Vector2f::new(cx, cy), 0, false, false);

    update_tiles(&mut state);

    assert!(state.total_claimed_tiles > 0.0, "a fort at map center should claim at least some tiles");
    assert_eq!(state.team_tile_no[0] as f32, state.total_claimed_tiles, "team 0's fort should own every claimed tile (no other forts)");
    for i in 1..state.number_of_teams as usize {
        assert_eq!(state.team_tile_no[i], 0);
    }
    assert!((state.team_tile_perc[0] - 100.0).abs() < 0.01, "team 0 should own 100% of claimed tiles");

    // Sanity-check the claimed area is in the right ballpark of
    // pi * range^2 / tile_area (generous tolerance -- the fort-territory
    // distance metric divides by get_fort_territory_power and hex tiles
    // aren't a perfect circle packing, so this is a coarse check).
    let range = state.entities[state.entities.len() - 1].range as f64;
    let tile_size = state.config.graphics.tiles_size as f64;
    // Hex tile area (regular hexagon with "radius" tile_size/2, matching
    // Tile::new's `radius = Tiles_Size / 2`): (3*sqrt(3)/2) * r^2.
    let tile_area = 1.5 * 3.0f64.sqrt() * (tile_size / 2.0).powi(2);
    let expected_tiles = std::f64::consts::PI * range * range / tile_area;
    let actual = state.total_claimed_tiles as f64;
    assert!(
        actual > expected_tiles * 0.2 && actual < expected_tiles * 3.0,
        "claimed tile count {actual} is wildly outside the expected pi*r^2/tile_area ballpark of {expected_tiles} (range={range}, tile_area={tile_area})"
    );
}

#[test]
fn neighbor_offsets_match_c_plus_plus_odd_even_tables() {
    let mut state = fresh_state();
    init_tiles(&mut state);

    // Find an odd-row tile away from the map edges so all 6 neighbors exist.
    let rows = state.tiles.len();
    assert!(rows > 10, "test map too small to find an interior odd row");
    let mut odd_row_idx = None;
    for r in 0..rows {
        if state.tiles[r][0].grid_y % 2 != 0 && r > 2 && r < rows - 2 {
            odd_row_idx = Some(r);
            break;
        }
    }
    let r = odd_row_idx.expect("no interior odd row found");
    let cols = state.tiles[r].len();
    assert!(cols > 10, "test map too small to find an interior column");
    let c = cols / 2;

    let (gx, gy) = (state.tiles[r][c].grid_x, state.tiles[r][c].grid_y);
    assert_eq!(gy % 2, 1, "sanity: chosen tile should be on an odd grid row");

    let neighbors = get_tile_neighbours(&state.tiles, gx, gy);
    assert_eq!(neighbors.len(), 6, "an interior tile should have all 6 hex neighbors");

    // C++ odd_offsets: {0,-1},{1,-1},{1,0},{1,1},{0,1},{-1,0} (dx, dy).
    let expected_offsets = [(0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 0)];
    let mut expected_coords: Vec<(i32, i32)> = expected_offsets.iter().map(|&(ox, oy)| (gx + ox, gy + oy)).collect();
    let mut actual_coords: Vec<(i32, i32)> = neighbors
        .iter()
        .map(|&(nr, nc)| (state.tiles[nr][nc].grid_x, state.tiles[nr][nc].grid_y))
        .collect();
    expected_coords.sort();
    actual_coords.sort();
    assert_eq!(actual_coords, expected_coords, "neighbor grid coordinates don't match the C++ odd_offsets table");

    // Matches the field's stored neighbor list too (set during init_tiles).
    let stored: Vec<(usize, usize)> = {
        let mut v = state.tiles[r][c].neighbors.clone();
        v.sort();
        v
    };
    let mut fresh = neighbors.clone();
    fresh.sort();
    assert_eq!(stored, fresh);
}

#[test]
fn two_forts_produce_a_border() {
    let mut state = fresh_state();
    init_tiles(&mut state);

    spawn_fort(&mut state, Vector2f::new(150.0, 250.0), 0, false, false);
    spawn_fort(&mut state, Vector2f::new(350.0, 250.0), 1, false, false);

    update_tiles(&mut state);

    assert!(state.team_tile_no[0] > 0);
    assert!(state.team_tile_no[1] > 0);

    let any_border = state.tiles.iter().flatten().any(|t| t.border_tile);
    assert!(any_border, "two adjacent forts of different teams should produce at least one border tile");

    // No NaN/garbage colors anywhere.
    for t in state.tiles.iter().flatten() {
        assert!(t.distance_from_fort.is_finite());
    }
}
