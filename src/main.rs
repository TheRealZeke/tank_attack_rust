use sfml::audio::{Music, SoundSource};
use sfml::graphics::RenderWindow;
use sfml::window::{ContextSettings, Style};
use tank_attack_rust::assets::SoundPlayers;
use tank_attack_rust::config::Config;
use tank_attack_rust::game_loop::{run, setup};

fn main() {
    let cfg = Config::load();
    let (mut state, assets) = match setup(cfg) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to load assets: {e}");
            std::process::exit(1);
        }
    };

    let (sw, sh) = (
        state.config.graphics.screen_width as u32,
        state.config.graphics.screen_height as u32,
    );

    // C++: `if (!devWindow) { window.create(..., sf::Style::Fullscreen); }`
    // `devWindow` defaults to false in the original and isn't wired up to
    // any config/CLI flag in this port -- always fullscreen, matching the
    // default.
    let mut window = RenderWindow::new(
        (sw, sh),
        "Tank Attack",
        Style::FULLSCREEN | Style::CLOSE,
        &ContextSettings::default(),
    );
    window.set_framerate_limit(120);

    // C++: `background_Music.openFromFile(...); background_Music.play();
    // background_Music.setVolume(...); background_Music.setLoop(true);`
    // `Music` isn't stored on `GameState`/`Assets` (see game_loop.rs's
    // module doc comment) since it needs to keep playing/streaming for
    // the whole process lifetime and doesn't fit either struct's
    // ownership model cleanly; kept as a local here instead, same
    // effective lifetime as the C++ global.
    let mut music = Music::from_file(&assets.background_music_path);
    if let Some(m) = music.as_mut() {
        m.set_volume(state.config.audio.volumes.background as f32);
        m.set_looping(true);
        m.play();
    } else {
        eprintln!(
            "Warning: failed to load background music from {}",
            assets.background_music_path
        );
    }

    let mut sound_players = SoundPlayers::new(&assets, &state.config.audio.volumes);

    run(&mut state, &assets, &mut sound_players, &mut window, None);
}
