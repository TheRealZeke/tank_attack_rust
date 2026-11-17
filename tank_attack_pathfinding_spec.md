# Spec: Impassable Terrain + Hex-Grid Pathfinding for Tank Attack Rust Port
*(rev 2 — performance-oriented updates integrated; changes are marked **[UPDATED]**)*

## 1. Goal

Add two related features to the existing Rust/SFML port:

1. **Impassable map tiles** — some `Tile`s are water/rock/void and cannot be entered by tanks.
2. **Pathfinding** — tanks use A* over the existing hex-like `Tile` grid to navigate around impassable tiles instead of steering directly through them.

The implementation must preserve existing C++-ported behavior as much as possible when terrain is disabled or when `GameState::tiles` is empty. It must not alter RNG call order/count in `entity_ai` / `laser_move` / `missile_move`, because existing tests assert exact RNG draw counts.

**Performance target:** adding this feature should cost under ~2% of frame time in typical play and under ~5% in heavy combat (100 tanks, terrain on, large map). Every design decision below is chosen with that budget in mind.

---

## 2. Non-goals

- Do not rewrite the entire movement/AI system.
- Do not make forts move.
- Do not require pathfinding for forts.
- Do not change laser/missile damage rules unless you explicitly add optional LOS blocking.
- Do not use RNG inside pathfinding itself.
- Do not break headless tests that never call `init_tiles()` or `generate_random_map()`.
- **[UPDATED]** Do not introduce per-frame heap allocations inside hot paths (`entity_move`, `entity_ai_tank`, A*). Reuse scratch buffers on `GameState`.

---

## 3. Existing architecture facts to use

- Hex grid lives in `GameState::tiles: Vec<Vec<Tile>>`.
- `Tile` already stores:
  - `pos: Vector2f` — world position of tile center.
  - `grid_x`, `grid_y` — grid coordinates.
  - `neighbors: Vec<(usize, usize)>` — valid neighboring tile indices.
  - `team`, `fort_uid`, `border_tile`, `soft_border_tile`, colors, etc.
- `Tile::neighbors` is already populated by `init_tiles()`.
- Tanks are `Entity` with `entity_type == EntityType::Tank`.
- Tank AI is in `entity.rs::entity_ai_tank`.
- Movement integration is in `entity.rs::entity_move`.
- Map generation is in `mapgen.rs::generate_random_map`, which ends by calling `post_setup()`.
- `post_setup()` currently calls `init_tiles(state)`, then `update_tiles(state)`, then `load_game_buttons(state)`.
- Rendering of tiles is in `tile.rs::tile_draw` / `tile_idle_draw` and `render.rs::render_minimap`.

---

## 4. New config

Add a new optional terrain config to `src/config.rs`.

### 4.1 Add `TerrainConfig` **[UPDATED]**

```rust
#[derive(Debug, Deserialize, Clone)]
pub struct TerrainConfig {
    #[serde(default = "default_terrain_enabled")]
    pub enabled: bool,
    #[serde(default = "default_terrain_density")]
    pub density: f32,
    #[serde(default = "default_terrain_cluster_chance")]
    pub cluster_chance: f32,
    #[serde(default = "default_terrain_cluster_size")]
    pub cluster_size: i32,
    #[serde(default = "default_pathfinding_enabled")]
    pub pathfinding_enabled: bool,
    #[serde(default = "default_path_recalc_ticks")]
    pub path_recalc_ticks: i32,
    /// Upper bound on A* nodes explored before giving up. Default 8192
    /// (up from 2048) so corner-to-corner paths on the largest shipped
    /// map (12000x12000) don't silently fail and fall back to direct
    /// steering. See `path_max_nodes_auto` below.
    #[serde(default = "default_path_max_nodes")]
    pub path_max_nodes: usize,
    /// If true, `path_max_nodes` is ignored and instead computed as
    /// `(map_width / tiles_size) * (map_height / tiles_size) / 20`,
    /// clamped to [512, 16384]. Recommended for games that pick a random
    /// map size per match. Default: true.
    #[serde(default = "default_path_max_nodes_auto")]
    pub path_max_nodes_auto: bool,
    /// Heuristic weight for weighted A*. 1.0 = optimal A*, 1.5–2.0 =
    /// faster but slightly suboptimal paths (fine for tank steering).
    /// Default 1.5.
    #[serde(default = "default_path_heuristic_weight")]
    pub path_heuristic_weight: f32,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            density: 0.12,
            cluster_chance: 0.35,
            cluster_size: 6,
            pathfinding_enabled: true,
            path_recalc_ticks: 30,
            path_max_nodes: 8192,
            path_max_nodes_auto: true,
            path_heuristic_weight: 1.5,
        }
    }
}
```

Add helper `fn default_*() -> T { ... }` functions.

### 4.2 Add to `Graphics` (unchanged from rev 1)

```rust
#[derive(Debug, Deserialize, Clone)]
pub struct Graphics {
    // existing fields...
    #[serde(default, rename = "Terrain")]
    pub terrain: TerrainConfig,
}
```

If `Graphics.json` has no `Terrain` key, `TerrainConfig::default()` is used.

Example optional JSON block:

```json
"Terrain": {
  "enabled": true,
  "density": 0.12,
  "cluster_chance": 0.35,
  "cluster_size": 6,
  "pathfinding_enabled": true,
  "path_recalc_ticks": 30,
  "path_max_nodes": 8192,
  "path_max_nodes_auto": true,
  "path_heuristic_weight": 1.5
}
```

---

## 5. Tile data model changes

Edit `src/tile.rs`.

### 5.1 Add terrain enum (unchanged)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainType {
    Ground,
    Water,
    Rock,
}

impl TerrainType {
    pub fn is_passable(self) -> bool {
        matches!(self, TerrainType::Ground)
    }

    pub fn color(self) -> Color {
        match self {
            TerrainType::Ground => Color::rgba(0, 0, 0, 0), // unused; normal tile color
            TerrainType::Water => Color::rgb(20, 80, 160),
            TerrainType::Rock => Color::rgb(80, 80, 80),
        }
    }
}
```

### 5.2 Add fields to `Tile` (unchanged)

```rust
pub terrain: TerrainType,
pub move_cost: f32,
```

### 5.3 Initialize in `Tile::new`

```rust
terrain: TerrainType::Ground,
move_cost: 1.0,
```

### 5.4 Add helper

```rust
impl Tile {
    pub fn is_passable(&self) -> bool {
        self.terrain.is_passable()
    }
}
```

---

## 6. `GameState` changes **[UPDATED]**

Edit `src/state.rs`.

Add two categories of fields:

### 6.1 Terrain fast-path flag

```rust
/// True iff the current map contains at least one blocked tile.
/// Set by `generate_terrain` (mapgen.rs) and reset by
/// `reset_for_map_gen` (mapgen.rs). When false, `is_world_pos_passable`
/// returns `true` immediately, making this whole feature free on
/// terrain-disabled maps (and on all the existing test suites that
/// never call `generate_terrain`).
pub has_blocked_tiles: bool,
```

Initialize to `false` in `GameState::new`.

### 6.2 A* scratch buffers

These are reused across every `find_path` call, so A* never allocates. Cleared via an epoch counter, not via `fill()`, so reset is O(1) per call.

```rust
/// Scratch buffers for `pathfinding::find_path`, sized to
/// (tiles.len(), tiles[0].len()). Reused across every A* call.
/// `pathfind_epoch` is incremented once per A* call; any cell whose
/// `pathfind_visited_epoch` value is not equal to the current epoch is
/// treated as "unvisited" without needing a memset.
pub pathfind_g_score: Vec<Vec<f32>>,
pub pathfind_came_from_row: Vec<Vec<i32>>,
pub pathfind_came_from_col: Vec<Vec<i32>>,
pub pathfind_visited_epoch: Vec<Vec<u32>>,
pub pathfind_epoch: u32,
/// Binary heap reused across calls (clear() instead of drop+realloc).
/// Uses the std BinaryHeap; item ordering is by f-score in a private
/// tuple struct that implements Ord.
pub pathfind_open: std::collections::BinaryHeap<crate::pathfinding::OpenNode>,
```

Initialize all to empty in `GameState::new`.

Add a helper to (re)size them after `init_tiles`:

```rust
impl GameState {
    /// Called by `init_tiles` after `self.tiles` is fully built.
    pub fn resize_pathfind_buffers(&mut self) {
        let rows = self.tiles.len();
        let cols = if rows == 0 { 0 } else { self.tiles[0].len() };
        self.pathfind_g_score = vec![vec![f32::INFINITY; cols]; rows];
        self.pathfind_came_from_row = vec![vec![-1; cols]; rows];
        self.pathfind_came_from_col = vec![vec![-1; cols]; rows];
        self.pathfind_visited_epoch = vec![vec![0; cols]; rows];
        self.pathfind_epoch = 0;
        self.pathfind_open.clear();
    }
}
```

---

## 7. New module: `src/pathfinding.rs`

Add `pub mod pathfinding;` to `src/lib.rs`.

### 7.1 Required public API

```rust
use crate::state::GameState;
use sfml::system::Vector2f;

/// Heap entry used by A*. Public so it can be stored on `GameState`
/// without pulling a private type across the module boundary.
#[derive(Copy, Clone, PartialEq)]
pub struct OpenNode {
    pub f: f32,
    pub row: usize,
    pub col: usize,
}
impl Eq for OpenNode {}
impl PartialOrd for OpenNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for OpenNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // BinaryHeap is a max-heap; reverse so lowest f pops first.
        other.f.partial_cmp(&self.f).unwrap_or(std::cmp::Ordering::Equal)
    }
}

pub fn world_to_tile(state: &GameState, pos: Vector2f) -> Option<(usize, usize)>;
pub fn tile_center(state: &GameState, row: usize, col: usize) -> Vector2f;
pub fn is_tile_passable(state: &GameState, row: usize, col: usize) -> bool;
pub fn is_world_pos_passable(state: &GameState, pos: Vector2f) -> bool;
pub fn find_nearest_passable_tile(
    state: &GameState,
    start: (usize, usize),
    max_radius: i32,
) -> Option<(usize, usize)>;
pub fn find_path(
    state: &mut GameState,
    start: (usize, usize),
    goal: (usize, usize),
    max_nodes: usize,
    heuristic_weight: f32,
) -> Option<Vec<(usize, usize)>>;
pub fn line_of_sight(state: &GameState, from: Vector2f, to: Vector2f) -> bool;
pub fn ensure_spawn_pos(state: &GameState, pos: Vector2f) -> Option<Vector2f>;
pub fn update_entity_path(state: &mut GameState, idx: usize, target_pos: Vector2f) -> Vector2f;
```

Note: `find_path` now takes `&mut GameState` (to borrow the scratch buffers) plus two new parameters. All call sites are inside `update_entity_path`, so this is contained.

### 7.2 `world_to_tile` — O(1) arithmetic **[UPDATED]**

The tile layout in `init_tiles` is deterministic, so we can invert it algebraically instead of doing a 5×5 guess-and-verify search (which would cost ~25 distance computations per call, and this is called ~4× per tank per frame in `entity_move` plus every `line_of_sight` sample).

```rust
pub fn world_to_tile(state: &GameState, pos: Vector2f) -> Option<(usize, usize)> {
    if state.tiles.is_empty() { return None; }
    let size = state.config.graphics.tiles_size as f32;
    if size <= 0.0 { return None; }
    let x_fix = 2.0 / 3.0f32.sqrt();

    // Inverse of init_tiles:
    //   y = -size + row * size                    => row = (y + size) / size
    //   even row: x offset = -size * x_fix / 4,  col base = -size * x_fix
    //   odd  row: x offset = +size * x_fix / 4
    //   x = col_base + offset + col * size * x_fix
    let row_f = (pos.y + size) / size;
    let row_guess = row_f.round() as i32;

    // Because row parity affects x offset, allow ±1 row correction.
    let mut best: Option<(usize, usize, f32)> = None;
    for dr in -1..=1 {
        let row = row_guess + dr;
        if row < 0 || row as usize >= state.tiles.len() { continue; }
        let row_usize = row as usize;
        let offset = if row % 2 == 0 { -size * x_fix / 4.0 } else { size * x_fix / 4.0 };
        let col_f = (pos.x - offset + size * x_fix) / (size * x_fix);
        let col_guess = col_f.round() as i32;

        for dc in -1..=1 {
            let col = col_guess + dc;
            if col < 0 || col as usize >= state.tiles[row_usize].len() { continue; }
            let col_usize = col as usize;
            let d = crate::util::displacement(state.tiles[row_usize][col_usize].pos, pos);
            if best.map_or(true, |(_, _, bd)| d < bd) {
                best = Some((row_usize, col_usize, d));
            }
        }
    }

    // Reject if nearest candidate is farther than a tile diameter --
    // this happens only for out-of-map positions.
    let (r, c, d) = best?;
    if d > size * 1.5 { return None; }
    Some((r, c))
}
```

This explores at most 9 candidates instead of 25, and the column guess is direct rather than searched. Roughly 4–5× faster than the rev-1 strategy, and still robust to off-by-one at hex boundaries.

### 7.3 `is_world_pos_passable` — fast-path **[UPDATED]**

```rust
pub fn is_world_pos_passable(state: &GameState, pos: Vector2f) -> bool {
    if state.tiles.is_empty() || !state.has_blocked_tiles {
        return true; // tests / pre-map state / terrain-disabled maps
    }
    match world_to_tile(state, pos) {
        Some((r, c)) => is_tile_passable(state, r, c),
        None => false,
    }
}
```

The `has_blocked_tiles` check is the single most important optimization: it makes the whole feature cost literally nothing on maps that don't have any blocked tiles (including every existing test that doesn't explicitly opt into terrain).

### 7.4 `find_path` — A* with reusable buffers **[UPDATED]**

```rust
pub fn find_path(
    state: &mut GameState,
    start: (usize, usize),
    goal: (usize, usize),
    max_nodes: usize,
    heuristic_weight: f32,
) -> Option<Vec<(usize, usize)>> {
    let rows = state.tiles.len();
    if rows == 0 { return None; }
    let cols = state.tiles[start.0].len();
    if state.pathfind_g_score.len() != rows
        || state.pathfind_g_score.get(0).map_or(0, |r| r.len()) != cols
    {
        state.resize_pathfind_buffers();
    }

    // Bump epoch — O(1) logical clear of g_score / came_from / visited.
    state.pathfind_epoch = state.pathfind_epoch.wrapping_add(1);
    let epoch = state.pathfind_epoch;

    state.pathfind_open.clear();
    let size = state.config.graphics.tiles_size as f32;
    let x_fix = 2.0 / 3.0f32.sqrt();
    let inv_step = 1.0 / (size * x_fix);

    let h = |r: usize, c: usize| -> f32 {
        crate::util::displacement(state.tiles[r][c].pos, state.tiles[goal.0][goal.1].pos) * inv_step
    };

    state.pathfind_g_score[start.0][start.1] = 0.0;
    state.pathfind_visited_epoch[start.0][start.1] = epoch;
    state.pathfind_open.push(OpenNode { f: h(start.0, start.1) * heuristic_weight, row: start.0, col: start.1 });

    let mut visited = 0usize;
    while let Some(OpenNode { f: _, row, col }) = state.pathfind_open.pop() {
        if (row, col) == goal {
            return Some(reconstruct_path(state, epoch, start, goal));
        }
        visited += 1;
        if visited > max_nodes { return None; }

        let g_here = state.pathfind_g_score[row][col];
        // Neighbors: copy out so we don't hold a borrow of state.tiles.
        let neighbors = state.tiles[row][col].neighbors.clone(); // small Vec, ≤6 entries

        for (nr, nc) in neighbors {
            if !state.tiles[nr][nc].is_passable() { continue; }
            let step_cost = state.tiles[nr][nc].move_cost;
            let tentative = g_here + step_cost;

            let already_seen = state.pathfind_visited_epoch[nr][nc] == epoch;
            if already_seen && tentative >= state.pathfind_g_score[nr][nc] { continue; }

            state.pathfind_g_score[nr][nc] = tentative;
            state.pathfind_came_from_row[nr][nc] = row as i32;
            state.pathfind_came_from_col[nr][nc] = col as i32;
            state.pathfind_visited_epoch[nr][nc] = epoch;

            let f = tentative + h(nr, nc) * heuristic_weight;
            state.pathfind_open.push(OpenNode { f, row: nr, col: nc });
        }
    }
    None
}
```

Key performance properties:

- No `HashMap` lookups.
- No per-call heap allocations except the returned `Vec` (which is short — typically 3–30 entries).
- `state.pathfind_open` is reused; `BinaryHeap::clear()` keeps its capacity.
- `state.tiles[row][col].neighbors.clone()` is a `Vec` of ≤6 `(usize, usize)` — a small one-time alloc per popped node. If this shows up in profiles, replace `neighbors: Vec<(usize,usize)>` on `Tile` with a fixed-size `[(usize, usize); 6]` + `neighbor_count: u8`, but that's a bigger refactor and shouldn't be necessary.
- Weighted A* via `heuristic_weight > 1.0` typically explores 3–10× fewer nodes than unweighted A* on open maps, and the resulting paths are nearly indistinguishable to a tank.

Helper:

```rust
fn reconstruct_path(
    state: &GameState,
    epoch: u32,
    start: (usize, usize),
    goal: (usize, usize),
) -> Vec<(usize, usize)> {
    let mut path = Vec::new();
    let mut cur = goal;
    loop {
        path.push(cur);
        if cur == start { break; }
        if state.pathfind_visited_epoch[cur.0][cur.1] != epoch { break; }
        let pr = state.pathfind_came_from_row[cur.0][cur.1];
        let pc = state.pathfind_came_from_col[cur.0][cur.1];
        if pr < 0 || pc < 0 { break; }
        cur = (pr as usize, pc as usize);
    }
    path.reverse();
    path
}
```

### 7.5 `find_nearest_passable_tile`

Unchanged from rev 1, except: it should also short-circuit on `!state.has_blocked_tiles` (return `Some(start)`).

### 7.6 `line_of_sight` — unchanged from rev 1

```rust
pub fn line_of_sight(state: &GameState, from: Vector2f, to: Vector2f) -> bool {
    if state.tiles.is_empty() || !state.has_blocked_tiles {
        return true;
    }
    let dist = crate::util::displacement(from, to);
    let step = (state.config.graphics.tiles_size as f32 * 0.5).max(1.0);
    let steps = (dist / step).ceil() as i32;
    for i in 0..=steps {
        let t = i as f32 / steps.max(1) as f32;
        let p = from + (to - from) * t;
        if !is_world_pos_passable(state, p) {
            return false;
        }
    }
    true
}
```

**Call-order discipline [UPDATED]:** in `entity_ai_tank`'s shooting gate, place `line_of_sight` **last** in the `&&` chain. Every prior condition (`delta_angle`, `laser_cooldown`, `dis_to_target <= range`, `!no_ammo`, `enemy_uid.is_some()`) is cheap and gates most frames. Only when a tank is already aimed at its target and off cooldown does `line_of_sight` run — a handful of times per frame, not once per tank per frame.

### 7.7 `ensure_spawn_pos`

Unchanged from rev 1, plus the fast path:

```rust
pub fn ensure_spawn_pos(state: &GameState, pos: Vector2f) -> Option<Vector2f> {
    if state.tiles.is_empty() || !state.has_blocked_tiles {
        return Some(pos);
    }
    // ...rev-1 logic...
}
```

### 7.8 `update_entity_path` — updated signature **[UPDATED]**

Same logic as rev 1, but:

- Call `find_path(state, start_tile, goal_tile, effective_max_nodes, terrain.path_heuristic_weight)`.
- Compute `effective_max_nodes`:

```rust
let effective_max_nodes = if terrain.path_max_nodes_auto {
    let rows = state.tiles.len() as f32;
    let cols = state.tiles.get(0).map_or(0, |r| r.len()) as f32;
    ((rows * cols / 20.0) as usize).clamp(512, 16384)
} else {
    terrain.path_max_nodes
};
```

- Add a fast-path early return:

```rust
if !state.has_blocked_tiles {
    return target_pos;
}
```

- **Do not** pathfind when the goal tile equals the start tile (adjacent or already on the target):

```rust
if goal_tile == start_tile {
    return target_pos;
}
```

- **Cache the last-successful path's goal** to skip A* when the enemy has barely moved (see §7.9 below — this is optional but cheap and worth doing given how often `target_pos` changes by a pixel).

### 7.9 Enemy-jitter cache **[NEW, optional but recommended]**

In a live battle, an enemy tank's `pos` changes every frame by a few pixels, and `target_pos` follows. Naively, that changes `goal_tile` only when the enemy crosses a tile boundary — so this is usually already handled by the "goal_tile changed" check. But when a tank is *circling* an enemy, or the enemy is near a tile boundary, `goal_tile` can oscillate between two adjacent tiles every frame.

Mitigation: keep `path_recalc_timer` from rev 1, and only trigger a full recalc if:

- `recalc_timer <= 0`, **or**
- The new goal tile is not equal to `path_goal_tile`, **and** the new goal tile is not a direct neighbor of `path_goal_tile`.

If the goal tile is a direct neighbor of the previous goal tile, keep the existing path and just retarget the last waypoint. This eliminates most of the "recalc every frame" cases without any explicit throttling.

---

## 8. Entity changes

Edit `src/entity.rs`.

### 8.1 Add fields to `Entity` (unchanged from rev 1)

```rust
pub path: Vec<(usize, usize)>,
pub path_index: usize,
pub path_goal_tile: Option<(usize, usize)>,
pub path_recalc_timer: f32,
```

### 8.2 Initialize in `Entity::new`

Same as rev 1.

### 8.3 Modify `entity_move` to respect impassable tiles (unchanged from rev 1)

Full block:

```rust
let old_pos = e.pos;
let candidate = Vector2f::new(
    e.pos.x + e.vel.x * fps,
    e.pos.y + e.vel.y * fps,
);

if crate::pathfinding::is_world_pos_passable(state, candidate) {
    e.pos = candidate;
} else {
    let cand_x = Vector2f::new(candidate.x, old_pos.y);
    let cand_y = Vector2f::new(old_pos.x, candidate.y);

    if crate::pathfinding::is_world_pos_passable(state, cand_x) {
        e.pos = cand_x;
        e.vel.y = 0.0;
    } else if crate::pathfinding::is_world_pos_passable(state, cand_y) {
        e.pos = cand_y;
        e.vel.x = 0.0;
    } else {
        e.vel = Vector2f::new(0.0, 0.0);
    }
}
```

**Note:** because of the `has_blocked_tiles` fast path in `is_world_pos_passable`, this whole block is effectively free when terrain is off. On terrain-on maps it's the equivalent of one `world_to_tile` call per move in the common case (candidate is passable).

### 8.4 Modify `entity_ai_tank` (unchanged from rev 1, with notes)

Split aiming vs movement as described in rev 1:

- `target_pos` — actual objective.
- `move_target` — waypoint from `update_entity_path`.

**Shooting gate ordering [UPDATED]:** keep the current `&&` order and append `line_of_sight` at the *end*:

```rust
let should_shoot = {
    let e = &state.entities[idx];
    crate::util::delta_angle(ang, aim_ang).abs() <= 7.5
        && e.laser_cooldown <= 0.0
        && dis_to_target <= e.range as f64
        && !e.no_ammo
        && e.enemy_uid.is_some()
        && crate::pathfinding::line_of_sight(state, e.pos, e.target_pos)
};
```

Rust short-circuits `&&` left-to-right, so the expensive LOS check only runs when everything else already passed.

---

## 9. Terrain generation

Add `generate_terrain(state: &mut GameState)` in `src/mapgen.rs`.

Call it in `post_setup` **after** `init_tiles(state)` and **before** `update_tiles(state)`:

```rust
crate::tile::init_tiles(state);
generate_terrain(state);
crate::tile::update_tiles(state);
crate::mapgen::load_game_buttons(state);
```

### 9.1 Algorithm (unchanged from rev 1)

1. If `!state.config.graphics.terrain.enabled`, set `state.has_blocked_tiles = false; return;`
2. Compute target blocked tile count: `(rows * cols) as f32 * density`.
3. Randomly place clusters until `blocked_count >= target`.
4. Clear blocked tiles around every fort.
5. Ensure connectivity by BFS + carving.

At the end, set `state.has_blocked_tiles`:

```rust
state.has_blocked_tiles = state.tiles.iter().flatten().any(|t| !t.is_passable());
```

Also: in `reset_for_map_gen`, add `state.has_blocked_tiles = false;`.

### 9.2 Helpers

Same as rev 1 (`set_tile_terrain`, `clear_terrain_radius`).

Random walk uses `state.tiles[r][c].neighbors` and `state.rand_range`.

---

## 10. Tile update / rendering changes

### 10.1 `update_tile` (unchanged from rev 1)

Early return for blocked tiles, using `terrain.color()` for the tile color.

### 10.2 `update_tile_borders` (unchanged from rev 1)

Early return for blocked tiles.

### 10.3 `tile_draw` (unchanged from rev 1)

Draw blocked tiles as a filled circle using the terrain color. Since we draw these for every blocked tile every frame, and the block is a solid circle, this is 1 draw call per visible blocked tile — same cost as a normal tile.

### 10.4 `tile_idle_draw` (unchanged from rev 1)

Early return for blocked tiles (set fill color once, don't animate flicker).

### 10.5 `render_minimap` (unchanged from rev 1)

Use terrain color for blocked tiles, skip border/assimilation logic.

---

## 11. Spawning changes

### 11.1 `spawn_tank` and `spawn_fort`

At the beginning of both functions, after logging:

```rust
let pos = match crate::pathfinding::ensure_spawn_pos(state, pos) {
    Some(p) => p,
    None => return,
};
```

Because `ensure_spawn_pos` returns `Some(pos)` when `state.tiles.is_empty() || !state.has_blocked_tiles`, existing tests that spawn before `init_tiles()` are unaffected.

### 11.2 Input spawning

No extra change required — the spawn functions handle it.

---

## 12. Tests

Add `tests/stage_g_pathfinding.rs`.

Required tests (unchanged from rev 1):

1. `world_to_tile_roundtrips_tile_centers`
2. `astar_finds_path_around_blocked_tiles`
3. `astar_returns_none_when_goal_isolated`
4. `entity_move_does_not_enter_blocked_tile`
5. `tank_ai_navigates_around_wall_to_reach_enemy`
6. `terrain_generation_clears_around_forts_and_keeps_connectivity`
7. `pathfinding_does_not_use_rng` (soft)

**New tests [UPDATED]:**

8. `has_blocked_tiles_fast_path_returns_true_for_empty_and_terrainless_maps`

   - Fresh `GameState` with empty `tiles` → `is_world_pos_passable(any_pos) == true`.
   - After `init_tiles` with no `generate_terrain` → `has_blocked_tiles == false`; `is_world_pos_passable` still returns `true` everywhere.

9. `world_to_tile_is_constant_time_and_exact_at_tile_centers`

   - For 500 sampled interior tiles, `world_to_tile(tile.pos) == Some((r,c))`.
   - For 500 sampled points offset by `size * 0.4` in each of 6 neighbor directions from a tile center, `world_to_tile` returns that neighbor.

10. `find_path_does_not_allocate_unboundedly`

    - Run 500 back-to-back `find_path` calls on the same `GameState`.
    - Capture the process's `pathfind_open.capacity()` before and after; assert it doesn't grow beyond a small constant (proves buffer reuse).

11. `astar_node_budget_scales_with_map_size`

    - Assert `effective_max_nodes` (as computed in `update_entity_path`) is ≥ 512 for the smallest map and ≤ 16384 for the largest.

12. Regression: re-run `stage_a3_ai::rng_call_count_over_1000_ticks_is_documented`. It should still pass unchanged.

---

## 13. Acceptance criteria

All items from rev 1, plus:

- [ ] **[UPDATED]** `TerrainConfig` default `path_max_nodes` is 8192, and `path_max_nodes_auto` is `true`.
- [ ] **[UPDATED]** `GameState` has `has_blocked_tiles`, `pathfind_g_score`, `pathfind_came_from_row`, `pathfind_came_from_col`, `pathfind_visited_epoch`, `pathfind_epoch`, `pathfind_open`.
- [ ] **[UPDATED]** `is_world_pos_passable` short-circuits on `!state.has_blocked_tiles`.
- [ ] **[UPDATED]** `world_to_tile` uses direct arithmetic (≤ 9 candidates), not a 5×5 search.
- [ ] **[UPDATED]** `find_path` takes `&mut GameState` and reuses scratch buffers (no `HashMap`, no per-call allocation of score/visited structures).
- [ ] **[UPDATED]** `find_path` supports weighted A* via `heuristic_weight`.
- [ ] **[UPDATED]** `line_of_sight` is called **last** in the shooting `&&` chain in `entity_ai_tank`.
- [ ] **[UPDATED]** A new regression test asserts 500 sequential `find_path` calls do not grow the process's A* scratch state beyond a constant.
- [ ] **Existing tests still pass**, especially:
  - `stage_a3_ai::rng_call_count_over_1000_ticks_is_documented` (RNG count must be unchanged: `10850`).
  - `stage_c_mapgen::all_four_types_across_20_seeds_produce_valid_maps` (mapgen still terminates quickly).
  - `stage_d4_render_passes::full_frame_render_pass_draws_something_and_does_not_panic` (rendering unchanged for passable tiles).
- [ ] **Performance sanity**: `stage_e_game_loop::game_loop_runs_many_frames_with_no_panic` runs 700 ticks in comparable wall time to a pre-change baseline. If it regresses by more than 30% wall time on terrain-on maps, investigate `find_path` first.
- [ ] New `stage_g_pathfinding` tests pass.

---

## 14. Suggested implementation order **[UPDATED]**

1. Add `TerrainConfig` to `config.rs` with the rev-2 defaults.
2. Add `TerrainType`, `terrain`, `move_cost`, `is_passable` to `Tile`.
3. Add `has_blocked_tiles` and the `pathfind_*` scratch fields to `GameState`, plus `resize_pathfind_buffers`.
4. Wire `resize_pathfind_buffers` into `init_tiles` (call it at the end of the existing function).
5. Create `pathfinding.rs` with `world_to_tile` (O(1)), `is_world_pos_passable` (with fast path), `find_path` (buffer-reusing weighted A*), `line_of_sight`, `ensure_spawn_pos`, `update_entity_path`.
6. Add `path` fields to `Entity` and initialize them.
7. Modify `entity_move` to respect passability.
8. Modify `entity_ai_tank`: split `target_pos` (aim) from `move_target` (steer), with `line_of_sight` last in the shooting gate.
9. Add `generate_terrain` to `mapgen.rs`; call it in `post_setup` between `init_tiles` and `update_tiles`; set `state.has_blocked_tiles` at the end; reset the flag in `reset_for_map_gen`.
10. Update `tile.rs` update/draw functions for blocked tiles.
11. Update `render.rs::render_minimap` for blocked tiles.
12. Update `spawn_tank` / `spawn_fort` to use `ensure_spawn_pos`.
13. Add `tests/stage_g_pathfinding.rs`.
14. Run `cargo test` and confirm `stage_a3_ai`'s RNG count is still `10850`.

---

## 15. Pitfalls / notes for the implementing chatbot

All items from rev 1, plus:

- **[UPDATED] Buffer reuse is mandatory, not optional.** `pathfind_g_score` and friends must be stored on `GameState` and reused; do not allocate them inside `find_path`.
- **[UPDATED] Epoch, not `fill`.** Clear the visited state by bumping `pathfind_epoch`, never by calling `fill()` on any grid.
- **[UPDATED] No `clone()` of `state.tiles[row][col].neighbors` unless it's genuinely a small Vec (≤6 entries).** It is, but the point stands: don't take a slice into `state.tiles` while also needing `&mut state`.
- **[UPDATED] If A* on the largest map is still too slow**, increase `path_heuristic_weight` first (2.0 is fine), then look at *reducing `path_recalc_ticks` frequency* for tanks that already have a valid path (the enemy-jitter cache in §7.9 covers most of this).
- **[UPDATED] `has_blocked_tiles` must be maintained on both sides**: set true at the end of `generate_terrain`; reset false in `reset_for_map_gen`. If it drifts, the fast path silently disables pathfinding everywhere.
- **[UPDATED] On the largest maps, `world_to_tile` will be called ~4× per tank per frame (~100 tanks × 4 = 400 calls/frame).** At 200 ns/call that's 80 µs — 1% of the 8.3 ms budget. Keep the O(1) arithmetic version; do not "simplify" it back to a search.
- **[UPDATED] Recalc interval for no-ammo tanks:** the spec's `path_recalc_ticks = 30` applies uniformly. If profiling shows no-ammo retreat to forts causing A* churn, add a second timer field for no-ammo state and set it to 60 or 90.
- **[UPDATED] Rust's `&&` short-circuit is load-bearing** in the shooting gate — keep `line_of_sight` after the angle check, not before.

---

## 16. Performance budget — reference **[NEW]**

Target: under **2% added frame time** in typical play, under **5%** at 100 tanks on the largest map.

| Component | Cost per call | Calls per frame (100 tanks, large map) | Total |
|---|---|---|---|
| `world_to_tile` (O(1)) | ~50 ns | ~400 (4× per tank) + ~400 (LOS samples, only when shooting) | ~40 µs |
| `entity_move` passability check | ~150 ns (1–3 calls to `world_to_tile`) | ~200 | ~30 µs |
| `line_of_sight` (only when aiming + off cooldown) | ~2 µs (60 samples) | ~20 | ~40 µs |
| `find_path` (weighted A*, ~100 nodes typical, ≤ 8192 cap) | ~100 ns/node | 100 tanks / 30 recalc interval ≈ 3.3/frame | ~35 µs (typical) / ~270 µs (worst case at 8192 nodes × 3.3) |
| Terrain generation | — | one-time per map | 50–200 ms, once |

**Total typical: ~145 µs ≈ 1.7% of the 8.3 ms budget.**
**Total worst case (all tanks hit the node cap simultaneously): ~380 µs ≈ 4.6%.**

If the worst case ever fires, it means A* is failing to find the goal within `max_nodes` — most likely because the goal is unreachable. The `find_nearest_passable_tile` fallback and terrain connectivity pass in `generate_terrain` should prevent that; if it still happens, investigate map generation first, not `find_path`.
