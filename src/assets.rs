//! Stage D.1: asset loading. Ported from the "Load Unit Textures" / "Load
//! Fonts" / sound-buffer-loading block in `main()` (TankAttack.cpp
//! L3730-3792) and `transform_image_color` (L4111-4169).
//!
//! Kept as a separate struct rather than folded into `GameState`: assets
//! are render-only resources (SFML `Texture`/`Font`/`SoundBuffer` handles)
//! that have nothing to do with game logic, and keeping them out of
//! `GameState` is what lets `tests/stage_a*.rs` through `stage_c*.rs` run
//! headless without ever touching a texture or font file. Render code
//! (Stage D.2+) takes `&GameState` and `&Assets` as separate parameters.
//!
//! DEVIATION: the C++ source's file paths use a mix of cases
//! (`"files/images/comptank.png"`, `"files/images/deadtank01.bmp"`) that
//! only resolve on a case-insensitive filesystem (Windows). The actual
//! files shipped in this project's `files/images/` directory are
//! `CompTank.png`, `DeadTank01.bmp`, `DeadTank02.bmp` (differently
//! cased). This port uses the on-disk casing so loading actually
//! succeeds on Linux/macOS's case-sensitive filesystems; `Fort.bmp`
//! already matched exactly. Audio/font paths are unchanged since their
//! casing already matches on disk.

use crate::config::Config;
use crate::util::opposite_color;
use sfml::audio::{Sound, SoundBuffer, SoundSource};
use sfml::graphics::{Color, Font, Image, IntRect, Texture};
use sfml::SfBox;

pub struct SoundAssets {
    pub tankshoot01: SfBox<SoundBuffer>,
    pub tankshoot02: SfBox<SoundBuffer>,
    pub fortshoot01: SfBox<SoundBuffer>,
    pub fortcaptured01: SfBox<SoundBuffer>,
    pub laserhit01: SfBox<SoundBuffer>,
    pub tankreload01: SfBox<SoundBuffer>,
    pub tankdies01: SfBox<SoundBuffer>,
    pub tankdies02: SfBox<SoundBuffer>,
    pub tankcreated01: SfBox<SoundBuffer>,
}

/// One variant per persistent `sf::Sound` object in the C++ globals
/// (TankAttack.cpp L178-224: `tankshoot01`, `tankshoot02`, `fortshoot01`,
/// `fortcaptured01`, `laserhit01`, `tankreload01`, `tankdies01`,
/// `tankdies02`, `tankcreated01`). Simulation code (`entity.rs`,
/// `laser.rs`) pushes these onto `GameState::sound_queue` instead of
/// calling `.play()` directly -- `Sound<'a>` borrows its `SoundBuffer`
/// with a lifetime tied to `Assets`, so actually playing one needs a
/// `SoundPlayers<'a>` (see below), which simulation code doesn't have
/// access to (and shouldn't need to, to stay testable without audio
/// hardware). `crate::game_loop::tick` drains the queue once per frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundEffect {
    TankShoot01,
    TankShoot02,
    FortShoot,
    FortCaptured,
    LaserHit,
    TankReload,
    TankDies01,
    TankDies02,
    TankCreated,
}

pub struct Assets {
    /// Per-team colorized tank texture (`CompTankTextArr[team]`).
    pub tank_textures: Vec<SfBox<Texture>>,
    /// Per-team colorized fort texture (`FortTextArr[team]`).
    pub fort_textures: Vec<SfBox<Texture>>,
    /// Per-team, 2 dead-tank variants (`DeadTankTextArr[team][0|1]`).
    pub dead_tank_textures: Vec<[SfBox<Texture>; 2]>,
    pub font_1: SfBox<Font>,
    pub font_2: SfBox<Font>,
    pub sounds: SoundAssets,
    /// C++ plays this via `sf::Music` (streamed, not a `SoundBuffer`); Stage
    /// D just records the path here, since opening/looping the stream is a
    /// main-loop concern (Stage E), not an asset-loading one.
    pub background_music_path: String,
}

fn load_image(path: &str) -> Result<Image, String> {
    Image::from_file(path).ok_or_else(|| format!("failed to load image: {path}"))
}

fn load_font(path: &str) -> Result<SfBox<Font>, String> {
    Font::from_file(path).ok_or_else(|| format!("failed to load font: {path}"))
}

fn load_sound(path: &str) -> Result<SfBox<SoundBuffer>, String> {
    SoundBuffer::from_file(path).map_err(|e| format!("failed to load sound {path}: {e:?}"))
}

/// Builds a GPU texture from a CPU-side `Image`, matching the C++'s
/// `Texture::loadFromImage(image)` (no sub-rect -- the whole image).
fn texture_from_image(image: &Image) -> Result<SfBox<Texture>, String> {
    let mut tex =
        Texture::new().ok_or_else(|| "failed to allocate an empty Texture".to_string())?;
    let size = image.size();
    tex.load_from_image(image, IntRect::new(0, 0, size.x as i32, size.y as i32))
        .map_err(|e| format!("failed to load texture from image: {e:?}"))?;
    Ok(tex)
}

/// C++: sf::Image transform_image_color(sf::Image img, sf::Color tcol, int mode)
/// (TankAttack.cpp L4111-4169).
///
/// `mode == 0` recolors a grayscale unit sprite (used for tanks and dead
/// tanks): pixels above the 127.5 midpoint are tinted toward `tcol`,
/// pixels below are tinted toward `tcol`'s complement (`255 - tcol`), and
/// pure black (0) or near-white (254) pixels are left untouched (they're
/// the sprite's outline/highlight, not shading). `mode == 1` recolors the
/// fort sprite, which encodes "use main color" (blue channel zero) vs "use
/// opposite color" (red channel zero) per-pixel, with the green channel as
/// a brightness/blend factor.
pub fn transform_image_color(img: &Image, tcol: Color, mode: i32) -> Image {
    let size = img.size();
    let mut new_img = Image::from_color(size.x, size.y, Color::TRANSPARENT)
        .expect("Image::from_color should not fail for a nonzero size");
    let opp_col = opposite_color(tcol);

    if mode == 0 {
        new_img.create_mask_from_color(Color::rgb(0, 128, 128), 0);

        for x in 0..size.x {
            for y in 0..size.y {
                // SAFETY: x, y are in [0, size) by construction of the loop bounds.
                let col = unsafe { img.pixel_at(x, y) };

                if col.r == col.g && col.g == col.b && col.r != 0 && col.r != 254 {
                    let pixcol = if (col.r as f32) > 127.5 {
                        Color::rgb(
                            (((col.r as f32 - 127.5) / 127.5) * tcol.r as f32) as u8,
                            (((col.r as f32 - 127.5) / 127.5) * tcol.g as f32) as u8,
                            (((col.r as f32 - 127.5) / 127.5) * tcol.b as f32) as u8,
                        )
                    } else {
                        Color::rgb(
                            ((-(col.r as f32 - 127.5) / 127.5) * (255 - tcol.r) as f32) as u8,
                            ((-(col.r as f32 - 127.5) / 127.5) * (255 - tcol.g) as f32) as u8,
                            ((-(col.r as f32 - 127.5) / 127.5) * (255 - tcol.b) as f32) as u8,
                        )
                    };
                    // SAFETY: x, y in bounds as above.
                    unsafe { new_img.set_pixel(x, y, pixcol) };
                } else {
                    // SAFETY: x, y in bounds as above.
                    unsafe { new_img.set_pixel(x, y, col) };
                }
            }
        }
    } else if mode == 1 {
        for x in 0..size.x {
            for y in 0..size.y {
                // SAFETY: x, y in bounds as above.
                let col = unsafe { img.pixel_at(x, y) };

                if (col.r == 0 && col.b != 0) || (col.b == 0 && col.r != 0) {
                    let brightness_factor = col.g as f32 / 255.0;
                    let mut pixcol = if col.b == 0 {
                        // Using main color.
                        Color::rgb(
                            (col.r as f32 * (tcol.r as f32 / 255.0)) as u8,
                            (col.r as f32 * (tcol.g as f32 / 255.0)) as u8,
                            (col.r as f32 * (tcol.b as f32 / 255.0)) as u8,
                        )
                    } else {
                        // col.r == 0: using opposite color.
                        Color::rgb(
                            (col.b as f32 * (opp_col.r as f32 / 255.0)) as u8,
                            (col.b as f32 * (opp_col.g as f32 / 255.0)) as u8,
                            (col.b as f32 * (opp_col.b as f32 / 255.0)) as u8,
                        )
                    };
                    pixcol.r =
                        (brightness_factor * (255 - pixcol.r) as f32 + pixcol.r as f32) as u8;
                    pixcol.g =
                        (brightness_factor * (255 - pixcol.g) as f32 + pixcol.g as f32) as u8;
                    pixcol.b =
                        (brightness_factor * (255 - pixcol.b) as f32 + pixcol.b as f32) as u8;
                    // SAFETY: x, y in bounds as above.
                    unsafe { new_img.set_pixel(x, y, pixcol) };
                } else {
                    // SAFETY: x, y in bounds as above.
                    unsafe { new_img.set_pixel(x, y, col) };
                }
            }
        }
        new_img.create_mask_from_color(Color::rgb(255, 255, 255), 0);
    }

    new_img
}

impl Assets {
    /// C++'s asset-loading block in `main()`. `color_arr` must have one
    /// entry per team (matches `GameState::color_arr`).
    pub fn load(config: &Config, color_arr: &[Color]) -> Result<Assets, String> {
        let tank_img = load_image("files/images/CompTank.png")?;
        let fort_img = load_image("files/images/Fort.bmp")?;
        let dead_tank_img = [
            load_image("files/images/DeadTank01.bmp")?,
            load_image("files/images/DeadTank02.bmp")?,
        ];

        let mut tank_textures = Vec::with_capacity(color_arr.len());
        let mut fort_textures = Vec::with_capacity(color_arr.len());
        let mut dead_tank_textures = Vec::with_capacity(color_arr.len());

        for &team_col in color_arr {
            let tank_recolored = transform_image_color(&tank_img, team_col, 0);
            tank_textures.push(texture_from_image(&tank_recolored)?);

            let fort_recolored = transform_image_color(&fort_img, team_col, 1);
            fort_textures.push(texture_from_image(&fort_recolored)?);

            let dt0 = transform_image_color(&dead_tank_img[0], team_col, 0);
            let dt1 = transform_image_color(&dead_tank_img[1], team_col, 0);
            dt0.create_mask_from_color(Color::rgb(0, 128, 128), 0);
            dt1.create_mask_from_color(Color::rgb(0, 128, 128), 0);
            dead_tank_textures.push([texture_from_image(&dt0)?, texture_from_image(&dt1)?]);
        }

        let font_1 = load_font("files/fonts/VerminVibesV-Zlg3.ttf")?;
        let font_2 = load_font("files/fonts/JustMyType-KePl.ttf")?;

        let sounds = SoundAssets {
            tankshoot01: load_sound("files/audio/tankshoot.wav")?,
            tankshoot02: load_sound("files/audio/tankshoot2.wav")?,
            fortshoot01: load_sound("files/audio/fortshoot.wav")?,
            fortcaptured01: load_sound("files/audio/fortcaptured.wav")?,
            laserhit01: load_sound("files/audio/laserhit.wav")?,
            tankreload01: load_sound("files/audio/tankreload.wav")?,
            tankdies01: load_sound("files/audio/tankdies.wav")?,
            tankdies02: load_sound("files/audio/tankdies2.wav")?,
            tankcreated01: load_sound("files/audio/tankcreated.wav")?,
        };

        Ok(Assets {
            tank_textures,
            fort_textures,
            dead_tank_textures,
            font_1,
            font_2,
            sounds,
            background_music_path: config.audio.background_music_file.clone(),
        })
    }
}

/// C++: the 9 global `sf::Sound` objects, each bound once to its buffer
/// (`sf::Sound tankshoot01(b_tankshoot01);` etc., TankAttack.cpp
/// L178-224) and retriggered with `.play()` (which restarts playback if
/// already playing -- a single channel per effect, not a pool; two
/// overlapping tank shots share one channel and the second cuts the
/// first off, matching the original exactly).
///
/// Owns one `Sound<'a>` per effect, borrowing its buffer from `Assets`.
/// Constructed once (in `main.rs`, alongside `Assets::load`) and reused
/// for the life of the program, mirroring the C++ globals' lifetime.
pub struct SoundPlayers<'a> {
    tankshoot01: Sound<'a>,
    tankshoot02: Sound<'a>,
    fortshoot01: Sound<'a>,
    fortcaptured01: Sound<'a>,
    laserhit01: Sound<'a>,
    tankreload01: Sound<'a>,
    tankdies01: Sound<'a>,
    tankdies02: Sound<'a>,
    tankcreated01: Sound<'a>,
}

impl<'a> SoundPlayers<'a> {
    /// C++'s initial `X01.setVolume(X01_Volume);` block (L3744-3752).
    pub fn new(assets: &'a Assets, volumes: &crate::config::AudioVolumes) -> Self {
        let mut p = SoundPlayers {
            tankshoot01: Sound::with_buffer(&assets.sounds.tankshoot01),
            tankshoot02: Sound::with_buffer(&assets.sounds.tankshoot02),
            fortshoot01: Sound::with_buffer(&assets.sounds.fortshoot01),
            fortcaptured01: Sound::with_buffer(&assets.sounds.fortcaptured01),
            laserhit01: Sound::with_buffer(&assets.sounds.laserhit01),
            tankreload01: Sound::with_buffer(&assets.sounds.tankreload01),
            tankdies01: Sound::with_buffer(&assets.sounds.tankdies01),
            tankdies02: Sound::with_buffer(&assets.sounds.tankdies02),
            tankcreated01: Sound::with_buffer(&assets.sounds.tankcreated01),
        };
        p.set_volumes(volumes, 100.0);
        p
    }

    /// C++'s Y/H-key volume-rescale block (`X.setVolume(X_Volume *
    /// game_volume / 100);`, TankAttack.cpp L4576-4585).
    pub fn set_volumes(&mut self, volumes: &crate::config::AudioVolumes, game_volume: f32) {
        self.tankshoot01
            .set_volume(volumes.tankshoot as f32 * game_volume / 100.0);
        self.tankshoot02
            .set_volume(volumes.tankshoot as f32 * game_volume / 100.0);
        self.fortshoot01
            .set_volume(volumes.fortshoot as f32 * game_volume / 100.0);
        self.fortcaptured01
            .set_volume(volumes.fortcaptured as f32 * game_volume / 100.0);
        self.laserhit01
            .set_volume(volumes.laserhit as f32 * game_volume / 100.0);
        self.tankreload01
            .set_volume(volumes.tankreload as f32 * game_volume / 100.0);
        self.tankdies01
            .set_volume(volumes.tankdies as f32 * game_volume / 100.0);
        self.tankdies02
            .set_volume(volumes.tankdies as f32 * game_volume / 100.0);
        self.tankcreated01
            .set_volume(volumes.tankcreated as f32 * game_volume / 100.0);
    }

    /// Drains `state.sound_queue`, playing (retriggering) the matching
    /// `Sound` for each queued effect. Called once per frame from
    /// `game_loop::tick`.
    pub fn play_queued(&mut self, queue: &mut Vec<SoundEffect>) {
        for effect in queue.drain(..) {
            let sound = match effect {
                SoundEffect::TankShoot01 => &mut self.tankshoot01,
                SoundEffect::TankShoot02 => &mut self.tankshoot02,
                SoundEffect::FortShoot => &mut self.fortshoot01,
                SoundEffect::FortCaptured => &mut self.fortcaptured01,
                SoundEffect::LaserHit => &mut self.laserhit01,
                SoundEffect::TankReload => &mut self.tankreload01,
                SoundEffect::TankDies01 => &mut self.tankdies01,
                SoundEffect::TankDies02 => &mut self.tankdies02,
                SoundEffect::TankCreated => &mut self.tankcreated01,
            };
            sound.play();
        }
    }
}
