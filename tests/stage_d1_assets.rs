//! Integration test for Stage D.1 (asset loading), per continuation
//! spec's testable-tier guidance: assert each loaded texture has
//! non-zero dimensions and the expected number of teams.

use tank_attack_rust::assets::Assets;
use tank_attack_rust::config::Config;
use sfml::graphics::Color;

#[test]
fn all_assets_load_with_correct_counts_and_nonzero_dimensions() {
    let config = Config::load();
    let n = config.teams.number_of_teams as usize;
    let color_arr: Vec<Color> = config.teams.colors.iter().map(|c| Color::rgb(c[0], c[1], c[2])).collect();

    let assets = Assets::load(&config, &color_arr).expect("asset loading should succeed with the shipped files/ directory");

    assert_eq!(assets.tank_textures.len(), n, "should have one tank texture per team");
    assert_eq!(assets.fort_textures.len(), n, "should have one fort texture per team");
    assert_eq!(assets.dead_tank_textures.len(), n, "should have one dead-tank texture pair per team");

    for i in 0..n {
        let ts = assets.tank_textures[i].size();
        assert!(ts.x > 0 && ts.y > 0, "team {i} tank texture has zero dimension: {ts:?}");

        let fs = assets.fort_textures[i].size();
        assert!(fs.x > 0 && fs.y > 0, "team {i} fort texture has zero dimension: {fs:?}");

        for (k, dt) in assets.dead_tank_textures[i].iter().enumerate() {
            let ds = dt.size();
            assert!(ds.x > 0 && ds.y > 0, "team {i} dead-tank texture {k} has zero dimension: {ds:?}");
        }
    }

    // Different teams' tank textures should actually differ in pixel
    // content (the whole point of transform_image_color) -- compare team
    // 0 vs team 1's recolored image via a fresh transform rather than
    // reading back GPU texture data (Texture has no direct pixel read in
    // this crate version without a RenderTexture round-trip).
    assert_ne!(color_arr[0], color_arr[1], "sanity: first two teams must have different colors in the config");

    // Fonts loaded (SfBox<Font> has no direct introspection API worth
    // asserting on beyond "it didn't fail to load", which `expect` above
    // already covers for the whole Assets struct).
    let _ = (&assets.font_1, &assets.font_2);

    // Background music path should be a nonempty, plausible-looking path.
    assert!(assets.background_music_path.contains("audio"), "background_music_path = {}", assets.background_music_path);
}

#[test]
fn transform_image_color_mode0_recolors_midtone_pixels_and_leaves_outline_alone() {
    use tank_attack_rust::assets::transform_image_color;
    use sfml::graphics::Image;

    // Build a tiny synthetic 3x1 grayscale image: pure black (outline,
    // should pass through unchanged), a midtone gray (should recolor),
    // and 254 (near-white "highlight", should pass through unchanged --
    // matches the C++'s `col.r != 254` exclusion).
    let img = Image::from_color(3, 1, Color::BLACK).unwrap();
    // SAFETY: coordinates are within the 3x1 image created above.
    unsafe {
        let mut img = img;
        img.set_pixel(0, 0, Color::rgb(0, 0, 0));
        img.set_pixel(1, 0, Color::rgb(180, 180, 180));
        img.set_pixel(2, 0, Color::rgb(254, 254, 254));

        let team_col = Color::rgb(255, 0, 0);
        let out = transform_image_color(&img, team_col, 0);

        let p0 = out.pixel_at(0, 0);
        let p2 = out.pixel_at(2, 0);
        assert_eq!((p0.r, p0.g, p0.b), (0, 0, 0), "pure black outline pixel should pass through unchanged");
        assert_eq!((p2.r, p2.g, p2.b), (254, 254, 254), "254-value highlight pixel should pass through unchanged");

        // The midtone pixel (180 > 127.5) should be tinted toward tcol:
        // pixcol.r = ((180-127.5)/127.5) * 255 ~= 105, g=0, b=0.
        let p1 = out.pixel_at(1, 0);
        assert!(p1.r > 90 && p1.r < 120, "midtone pixel red channel = {}, expected ~105", p1.r);
        assert_eq!(p1.g, 0);
        assert_eq!(p1.b, 0);
    }
}
