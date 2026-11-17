# TankAttack — Rust/SFML Port

A complete port of `TankAttack.cpp` (C++/SFML) to Rust, using the
`rust-sfml` crate, following `tank_attack_rust_spec.md` and
`continuation_spec.md`'s staged plan (Stages A through F, with Stage D
further split into D.1-D.5).

## Status: all stages complete

**Compiles cleanly** (zero warnings across the whole lib) with Rust 1.75
+ `sfml = "=0.21.0"` (pinned — newer `sfml` versions require rustc ≥1.81)
against the system's CSFML 2.6.1 / SFML 2.6.1 native libraries.

**Test status: 58/58 passing** (`cargo test`), across 13 test files.
Anything touching SFML rendering needs a display to create textures/
render targets, even off-screen ones — **run tests with
`xvfb-run -a cargo test`** if no real display is available. Stages A/B/C
(pure simulation logic, no SFML resources) pass with a plain
`cargo test` too.

**The strongest evidence this actually works**: `tests/stage_e_game_loop.rs`
runs the real, unmodified `game_loop::run()` — the same function
`main.rs` calls — through 700 real frames with a real (headless) SFML
window: `setup()` → 200 ticks of the empty just-started state → a real
generated map → 500 more ticks of full simulation, AI, rendering, input
polling, and stats bookkeeping. It finishes with 265 live entities, 19
in-flight lasers, 7 dead tanks, no panics, and no NaN/Inf anywhere in the
entity positions or hitpoints.

## What's implemented

Every stage from the spec, in dependency order:

- **Config** (`config.rs`) — all five JSON config files, loaded and
  verified against your actual data.
- **Stage A** (`entity.rs`, `laser.rs`, `missile.rs`, `spawn.rs`,
  `util.rs`, `state.rs`) — `Entity::spawn`/`lock_target`/`move`/`AI()`
  (the largest single piece of the original, ~700 lines), laser/missile
  spawn-move-collision, all `spawn_*` free functions, and the central
  `GameState` that replaces the C++'s file-scope globals.
- **Stage B** (`tile.rs`) — territory claiming (`update_tile`), border
  detection, hex-grid neighbor math, tile flashing.
- **Stage C** (`mapgen.rs`) — `generate_random_map`, all 4 map types
  (Random, 1v1, Roundtable, Chaos), with the C++'s retry-via-recursion
  converted to a bounded retry loop (documented deviation).
- **Stage D** (`assets.rs`, `render.rs`, `entity.rs`/`tile.rs`/
  `dead_tank.rs`/etc.'s `*_draw` functions, `input.rs`) — asset loading
  and per-team recoloring, every SFML draw primitive, every render pass
  (entities, lasers, missiles, particles, dead tanks, floating text, UI,
  minimap with its quad-stitch tile mosaic), and all mouse/keyboard input
  handling.
- **Stage E** (`game_loop.rs`, `main.rs`) — the actual main loop: setup,
  per-frame simulation/render/input/stats ordering, autoplay,
  extra-population scaling, wind, FPS tracking.
- **Stage F** (`stats.rs`) — population statistics, the 15-file JSON
  export (`J` key), and full game-state save/load shape (`S` key).

## Building and running

Requires SFML 2.6 + CSFML 2.6 native libraries:
```
sudo apt install libsfml-dev libcsfml-dev
```
Then:
```
cargo build               # builds both the lib and the `tank_attack_rust` binary
xvfb-run -a cargo test     # runs all test files (58/58 passing)
cargo run                  # opens a real fullscreen window and plays
cargo build --release      # optimized build
```
`sfml` is pinned to `=0.21.0` in `Cargo.toml` because newer versions
require rustc ≥1.81; relax the pin if you have a newer toolchain.

`cargo run` opens a real fullscreen SFML window, loads assets, and plays
music, same as the original executable. Controls match the original's
key bindings (`KeyBinds.txt`): left-click to spawn a tank for the
selected team (Shift = special, Ctrl = no-ammo), right-click for a fort
(Shift = special, Ctrl = captured), number keys to pick a team, `R` to
follow a random tank, `D` for debug overlays, `J`/`S` to export stats/
save state, and the on-screen buttons for quit/regenerate/delete/null-
team/gang-up.

## Layout

- `src/lib.rs` re-exports every module (`pub mod ...`) so `tests/` (and
  `main.rs`) can use them.
- `src/main.rs` is the real entry point: window/music setup, then hands
  off to `game_loop::run`.
- `src/game_loop.rs` holds the actual frame loop (`setup`, `tick`,
  `run`), split so `tick` is callable from a test with a real headless
  window.
- Config JSON, `KeyBinds.txt`, and `files/` (audio/fonts/images) sit in
  the project root, loaded via the same relative paths the C++ build used.

## Post-completion bug fixes

Reported after real-machine test runs (`cargo run --release` on a
different Linux system than this was developed on):

1. **Sound effects were completely silent** (background music worked,
   one-shot effects didn't). Root cause: `assets.rs` only ever loaded
   `SoundBuffer`s; nothing ever constructed a playable `sf::Sound` from
   them or called `.play()` — every trigger site in `entity.rs`/
   `laser.rs` was still a `// TODO(Stage D audio)` comment. Fixed with a
   `SoundEffect` enum + `GameState::sound_queue` (simulation code pushes
   onto it, staying audio-hardware-free and testable) and a new
   `SoundPlayers<'a>` in `assets.rs` holding one persistent `Sound<'a>`
   per effect — matching the C++'s exact model of one global `sf::Sound`
   object per effect, retriggered with `.play()` (a single channel per
   effect, not a pool: two overlapping tank shots cut each other off,
   same as the original). Drained once per frame in `game_loop::tick`.
2. **Tank/fort sprites weren't centered**, and it went deeper than the
   sprite: `spawn_tank`/`spawn_fort` were passing **hardcoded placeholder
   texture dimensions** (32×32 / 48×48) left over from Stage D.1, before
   real assets were loaded — the real textures are 15×30 (tank) and
   33×33 (fort). Since `Entity::tex_width`/`tex_height` also drive
   `collision_rect()` for tanks, this wasn't just a rendering bug — tank
   **hitboxes were the wrong size** too. Fixed at the source:
   `GameState` now has `tank_texture_size`/`fort_texture_size`, populated
   from the real loaded textures in `game_loop::setup`, and `spawn_tank`/
   `spawn_fort` read from there instead of hardcoding a number. A
   narrower fix (re-deriving the sprite origin from the texture at draw
   time, leaving the wrong stored dimensions everywhere else that reads
   them) was proposed and confirmed to visually work, but wouldn't have
   fixed the hitbox bug, so this port fixes the root cause instead.
3. **"Entitities" typo** in the stats panel — turned out to be in the
   original C++ too (`TankAttack.cpp` L5034, `"Entitities: "`), not
   introduced by the port. Fixed to "Entities" anyway per explicit
   request, documented as an intentional deviation from the source.
4. **FPS counter and stats panel ran off the right edge of the screen**.
   Both are left-anchored, non-centered text growing *rightward* from
   their position — anchoring that close to the screen edge means any
   string longer than a couple characters clips off-screen (e.g. "Dead
   Tanks:12" needs ~130px of room but only had 75px). Shifted both
   further left with real margin.
5. **Missiles dealt zero damage.** `render_missiles` called
   `missile_move`, which returns the list of missiles that exploded that
   tick, but **discarded the return value** — so
   `missile_explode_params` (which computes the splash-damage lasers an
   explosion should spawn) was never actually invoked. Missiles flew,
   visually "exploded" (despawned), and dealt nothing. Fixed by wiring
   the returned indices into a new `explode_missile` that spawns the
   real lasers, matching `Missle::explode()` exactly (TankAttack.cpp
   L1649-1664: `no_of_proj` lasers, splash-radius-scaled velocity,
   random angle, fixed size `{5, 1}`). Verified with a test that fires a
   missile at a stationary tank and confirms its HP actually drops
   through the real `render_missiles`/`render_lasers` passes, not a
   reimplementation.

**Also reported, not yet acted on — needs your input:** non-special
tanks apparently never fire missiles regardless of `TANK_MISSLE_NO` in
`EntityValues.json`, only special (leveled-up) tanks do. I re-verified
the exact source line before changing anything:
`TankAttack.cpp` L2288 reads
`if ((ammo_supply <= Tank_MISSLE_NO && false) || level > 0)` — the
`&& false` genuinely is in the shipped C++, permanently disabling the
`Tank_MISSLE_NO` clause. So this port's behavior matches the original
exactly (already flagged as a documented deviation in `entity.rs`); it
isn't a porting bug. Whether to keep matching that dead code or make
`TANK_MISSLE_NO` actually functional (a real gameplay behavior change
beyond what the original ships) is a design call, not something to
silently change — flagging for a decision rather than guessing.

## Known deviations from the C++

Every deviation is flagged inline with a `// DEVIATION:` comment at its
exact location, per the spec's requirement. The recurring categories:

**Omitted C++ dead code** (preserved as absence, not "cleaned up" —
these genuinely never execute in the original):
- Several `if (... && false)` guarded blocks (a disabled AI missile
  condition, a disabled "shuffle EntityArr" block, a disabled "Draw Fort
  Info" block, a disabled autoplay map-regen experiment).
- A few dead local variables/statements that compute a value and never
  use it (a shadowed `col` local in `Entity::spawn`, a fort's
  `flicker_timer - max_flicker_time;` no-op statement, a `Tile::draw`
  debug block gated by a literal `if (false)`).

**Structural changes required by Rust's ownership model** (behaviorally
identical, verified by test):
- All C++ pointer members (`Entity*`, `Laser::sender`, etc.) become
  `Option<u32>` unique-id handles, resolved fresh at point of use via
  `GameState::find_entity_index_by_uid` instead of cached and
  periodically revalidated. This makes an entire class of C++
  dangling-pointer safety-net code (the `entity_index` self-realignment
  ceremony throughout `Entity::AI()`) structurally unnecessary; it's
  omitted rather than translated, since none of it contains `rand()`
  calls (no effect on RNG draw order).
- `sf::Text`/`sf::Sprite`/most `sf::Shape` objects the C++ stores as
  class members and reuses across calls are rebuilt fresh each call
  instead (they get fully overwritten every use anyway, so this is
  behaviorally identical and avoids Rust `static mut` ceremony). The two
  *genuinely* persistent ones — `Tile::circle` and `Missile::projectile`,
  which read partial state across separate calls — **are** kept as real
  struct fields, matching the original.
- `generate_random_map`'s recursion-on-retry-failure becomes a bounded
  8-attempt loop plus a deterministic fallback, since unbounded recursion
  on a hostile RNG seed risks a stack overflow in Rust in a way it
  wouldn't necessarily in C++.

**Genuine bugs/gaps found in the C++ and handled safely rather than
reproduced as crashes**: a fort's missile-fire fallback that reads an
enemy index with no null/bounds check (this port validates via uid
instead); asset file paths that only resolve on a case-insensitive
filesystem (this port uses the actual on-disk casing).

**Preserved quirks that could look like bugs but aren't** (verified
against the source, not assumed): a fort's gun-angle truncates to an
integer before its modulo every tick, losing precision — kept exactly;
a captured-fort-ratio calculation divides by zero into `NaN` for a
team with no forts while a sibling calculation guards against the same
case — both kept as-is, not made consistent; `forts_per_team` can
legitimately compute to zero for small maps with high team counts,
producing a map with no forts at all — confirmed by tracing the actual
arithmetic, not just observed as a test failure.

**Real crate/environment constraints worth knowing for future work**:
`rust-sfml`'s `Key::is_pressed()` and `RenderWindow::new()` both require
all windowing calls in a process to happen from one consistent OS
thread — relevant to any future multi-window or multi-threaded tooling
built on this crate, not just this port's tests (which route around it
by keeping thread-sensitive test scenarios in one `#[test]` function).
