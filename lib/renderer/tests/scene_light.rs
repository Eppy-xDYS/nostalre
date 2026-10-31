use nostalre_renderer::LightUniform;
use nostalre_renderer::scene_light::{self, SceneOptions};

const LUMA: [f32; 3] = [0.2126, 0.7152, 0.0722];

fn luma(c: [f32; 3]) -> f32 {
    c[0] * LUMA[0] + c[1] * LUMA[1] + c[2] * LUMA[2]
}

fn spread(c: [f32; 3]) -> f32 {
    c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2])
}

fn close(a: [f32; 3], b: [f32; 3]) -> bool {
    a.iter().zip(&b).all(|(x, y)| (x - y).abs() < 1e-4)
}

fn day() -> LightUniform {
    LightUniform {
        light_dir: [0.3, -0.8, 0.5, 0.0],
        diffuse_color: [1.0, 0.9, 0.8, 1.0],
        ambient_color: [0.4, 0.4, 0.5, 1.0],
        shadow_strength: 1.0,
        glow: [0.6, 0.5, 0.3],
        ..LightUniform::default()
    }
}

fn hd_options() -> SceneOptions {
    SceneOptions {
        on: true,
        night_look: true,
        night_darkness: scene_light::NIGHT_DARKNESS,
        player_light: true,
        lamp_light: 0.5,
        clouds: 0.6,
        light_cap: 16,
        day_cycle: true,
    }
}

#[test]
fn treat_keeps_grey_and_dark() {
    assert_eq!(scene_light::lightmap_treat([0.0; 3]), [0.0; 3]);
    let grey = scene_light::lightmap_treat([0.1; 3]);
    assert!(spread(grey) < 1e-6, "grey stays grey");
    assert!(grey[0] < 0.1 && grey[0] > 0.07, "dark grey barely rolls off: {grey:?}");
}

#[test]
fn treat_pulls_colour_toward_white() {
    let lm = [0.3, 0.15, 0.0];
    let out = scene_light::lightmap_treat(lm);
    assert!(spread(out) < spread(lm), "less saturated: {out:?}");
    assert!(out[0] > out[1] && out[1] > out[2], "hue order kept: {out:?}");
}

#[test]
fn treat_rolls_off_bright() {
    let bright = scene_light::lightmap_treat([0.9; 3]);
    assert!(bright[0] < 0.9 * 0.5, "bright bake rolled off: {bright:?}");
    let mut last = 0.0;
    for i in 0..=20 {
        let v = i as f32 / 20.0;
        let y = luma(scene_light::lightmap_treat([v; 3]));
        assert!(y >= last, "monotonic at {v}");
        last = y;
    }
}

#[test]
fn baked_shadow_floors_only() {
    assert_eq!(scene_light::baked_shadow(0.0, 0.0), 1.0, "a wall keeps its light");
    assert_eq!(scene_light::baked_shadow(0.2, 0.3), 1.0);
    assert!((scene_light::baked_shadow(0.0, 1.0) - 0.4).abs() < 1e-5, "full floor shadow");
}

#[test]
fn baked_shadow_keeps_lit_level() {
    for alpha in [0.65, 0.8, 1.0] {
        assert_eq!(scene_light::baked_shadow(alpha, 1.0), 1.0, "{alpha}");
    }
    let mid = scene_light::baked_shadow(0.4, 1.0);
    assert!(mid > 0.4 && mid < 1.0, "{mid}");
}

#[test]
fn atten_full_at_zero_zero_at_range() {
    assert!((scene_light::point_atten(0.0, 50.0) - 1.0).abs() < 1e-6);
    assert_eq!(scene_light::point_atten(50.0, 50.0), 0.0);
    assert_eq!(scene_light::point_atten(80.0, 50.0), 0.0);
    assert_eq!(scene_light::point_atten(1.0, 0.0), 0.0, "no range, no light");
}

#[test]
fn atten_monotonic() {
    let mut last = f32::MAX;
    for i in 0..=100 {
        let a = scene_light::point_atten(i as f32 * 0.5, 50.0);
        assert!(a <= last, "rises at {i}");
        assert!(a >= 0.0);
        last = a;
    }
}

#[test]
fn knee_leaves_dim_alone() {
    let out = scene_light::light_knee([0.5, 0.3, 0.1]);
    assert!(close(out, [0.4, 0.24, 0.08]), "only × 0.8: {out:?}");
}

#[test]
fn knee_monotonic_and_capped() {
    let mut last = 0.0;
    for i in 0..=100 {
        let v = i as f32 * 0.1;
        let out = scene_light::light_knee([v, v * 0.5, 0.0]);
        assert!(out[0] >= last - 1e-6, "monotonic at {v}");
        assert!(out[0] <= 1.15 * 0.8 + 1e-5, "capped at {v}: {out:?}");
        assert!((out[1] - out[0] * 0.5).abs() < 1e-5, "hue kept at {v}");
        last = out[0];
    }
}

#[test]
fn night_lift_zero_by_day() {
    let lit = [0.2, 0.2, 0.3];
    let surface = [0.5, 0.4, 0.3];
    let lm = [0.5, 0.4, 0.2];
    assert_eq!(scene_light::night_lift(lit, surface, lm, 0.0, [1.0; 3]), lit);
    assert_eq!(
        scene_light::night_lift(lit, surface, [0.01; 3], 1.0, [1.0; 3]),
        lit,
        "no baked light, nothing to lift"
    );
}

#[test]
fn night_lift_restores_in_pool() {
    let lit = [0.1, 0.1, 0.15];
    let surface = [0.5, 0.4, 0.3];
    let lm = [0.35, 0.3, 0.1];
    let day = [0.9, 0.9, 0.8];
    let out = scene_light::night_lift(lit, surface, lm, 1.0, day);
    let want = [0.5 * 0.9 + 0.35, 0.4 * 0.9 + 0.3, 0.3 * 0.8 + 0.1];
    assert!(close(out, want), "{out:?} vs {want:?}");
}

/// Only strong baked light (a lamp's pool) is lifted at night: a town's dim overall lightmap colour is not, or
/// the whole town reads as day.
#[test]
fn night_lift_skips_dim_baked_light() {
    let lit = [0.1, 0.1, 0.15];
    let out = scene_light::night_lift(lit, [0.5, 0.4, 0.3], [0.07, 0.06, 0.05], 1.0, [0.9; 3]);
    assert_eq!(out, lit);
}

#[test]
fn cloud_offset_wraps() {
    assert_eq!(scene_light::cloud_offset(0.0), [0.0, 0.0]);
    let a = scene_light::cloud_offset(10.0);
    assert!(a[0] > 0.0 && a[1] > 0.0 && a[0] > a[1], "drifts: {a:?}");
    let far = scene_light::cloud_offset(1.0e7);
    assert!(far[0] < 256.0 && far[1] < 256.0, "wrapped: {far:?}");
}

/// The Night darkness row: its default is the designed moonlight, 0 the S5 one, 1 near black, darker all the way.
#[test]
fn night_darkness_scales_the_moonlight() {
    let (d, a) = scene_light::night_factors(scene_light::NIGHT_DARKNESS);
    assert!(close(d, scene_light::NIGHT_DIFFUSE) && close(a, scene_light::NIGHT_AMBIENT), "{d:?} {a:?}");
    let (d0, a0) = scene_light::night_factors(0.0);
    assert!(close(d0, [0.30, 0.38, 0.50]) && close(a0, [0.50, 0.62, 0.80]));
    let (d1, a1) = scene_light::night_factors(1.0);
    assert!(d1.iter().chain(a1.iter()).all(|c| *c > 0.0 && *c < 0.25), "near black: {d1:?} {a1:?}");
    for i in 0..3 {
        assert!(d1[i] < d[i] && d[i] < d0[i] && a1[i] < a[i] && a[i] < a0[i]);
    }
}

#[test]
fn night_light_identity_by_day() {
    let base = day();
    let out = scene_light::night_light(&base, 0.0, scene_light::NIGHT_DARKNESS);
    assert_eq!(out.diffuse_color, base.diffuse_color);
    assert_eq!(out.ambient_color, base.ambient_color);
}

#[test]
fn night_light_blue_moonlight() {
    let base = day();
    let out = scene_light::night_light(&base, 1.0, scene_light::NIGHT_DARKNESS);
    let (d, a) = (scene_light::NIGHT_DIFFUSE, scene_light::NIGHT_AMBIENT);
    let want_d = [1.0 * d[0], 0.9 * d[1], 0.8 * d[2]];
    let want_a = [0.4 * a[0], 0.4 * a[1], 0.5 * a[2]];
    assert!(d[0] < d[1] && d[1] < d[2] && a[0] < a[2], "blue moonlight");
    assert!(a.iter().all(|c| *c < 0.5), "dark: under half the day's sky");
    assert!(close([out.diffuse_color[0], out.diffuse_color[1], out.diffuse_color[2]], want_d));
    assert!(close([out.ambient_color[0], out.ambient_color[1], out.ambient_color[2]], want_a));
    assert_eq!(out.light_dir, base.light_dir);
    assert_eq!(out.glow, base.glow);
    let half = scene_light::night_light(&base, 0.5, scene_light::NIGHT_DARKNESS);
    assert!(half.diffuse_color[0] < base.diffuse_color[0] && half.diffuse_color[0] > want_d[0]);
}

#[test]
fn sprite_light_darkens_with_ambient() {
    let base = day();
    assert!(close(scene_light::sprite_light(&base, &base), [1.0; 3]), "day: no change");
    let night = scene_light::night_light(&base, 1.0, scene_light::NIGHT_DARKNESS);
    let f = scene_light::sprite_light(&base, &night);
    assert!(f.iter().all(|c| *c < 0.75), "sprites darken at night: {f:?}");
    assert!(f[2] > f[0], "bluer at night: {f:?}");
}

/// HD off, indoors or Night look off: today's light (the legacy R/G diffuse), no scene light.
#[test]
fn renderer_light_hd_off_is_today() {
    let base = day();
    let legacy = [0.5, 0.5, 0.8];
    let opts = hd_options();
    for (hd, indoor) in [(false, false), (false, true)] {
        let out = scene_light::frame_light(&base, legacy, 1.0, hd, indoor, &opts);
        assert_eq!(&out.diffuse_color[..3], &legacy, "hd {hd} indoor {indoor}");
        assert_eq!(out.ambient_color, base.ambient_color);
        assert_eq!(out.scene, [0.0; 4], "nothing new with HD off");
    }
    let indoor = scene_light::frame_light(&base, legacy, 1.0, true, true, &opts);
    assert_eq!(&indoor.diffuse_color[..3], &legacy, "HD night is outdoors only");
    assert_eq!(indoor.scene[1], 0.0, "no night lift indoors");
    assert_eq!(indoor.scene[2], 0.0, "no clouds indoors");
    assert_eq!(indoor.scene[0], 1.0, "lightmap treat + lamps still on indoors");

    let no_night = SceneOptions {
        night_look: false,
        ..opts
    };
    let out = scene_light::frame_light(&base, legacy, 1.0, true, false, &no_night);
    assert_eq!(&out.diffuse_color[..3], &legacy, "Night look off: today's night");
    assert_eq!(out.scene[1], 0.0);
}

#[test]
fn renderer_light_hd_night() {
    let base = day();
    let opts = hd_options();
    let out = scene_light::frame_light(&base, [0.5, 0.5, 0.8], 1.0, true, false, &opts);
    let moon = scene_light::night_light(&base, 1.0, scene_light::NIGHT_DARKNESS);
    assert_eq!(out.diffuse_color, moon.diffuse_color);
    assert_eq!(out.ambient_color, moon.ambient_color);
    assert_eq!(out.scene, [1.0, 1.0, 0.6, 16.0]);
    let day_light = [1.0, 1.0, 1.0];
    assert!(close([out.day_light[0], out.day_light[1], out.day_light[2]], day_light), "sun + sky, max 1");

    let off = SceneOptions {
        on: false,
        clouds: 0.0,
        ..opts
    };
    let out = scene_light::frame_light(&base, [1.0; 3], 0.0, true, false, &off);
    assert_eq!(out.scene, [0.0; 4], "all rows off");
}

#[test]
fn is_placed_skips_editor_origin() {
    assert!(!scene_light::is_placed([0.0, 0.0, 0.0]));
    assert!(scene_light::is_placed([0.0, -12.0, 0.0]));
    assert!(scene_light::is_placed([-310.5, -20.0, 44.0]));
}

#[test]
fn torch_by_night_outdoors() {
    let target = glam::Vec3::new(500.0, -20.0, 300.0);
    assert!(scene_light::torch(target, 0.0, true).is_none(), "no torch by day");
    assert!(scene_light::torch(target, 1.0, false).is_none(), "no torch indoors");
    let full = scene_light::torch(target, 1.0, true).unwrap();
    assert_eq!(full.position[..3], [500.0, -27.5, 300.0], "above the player (up is -y)");
    assert_eq!(full.color_range[3], scene_light::TORCH_RANGE);
    assert_eq!(scene_light::TORCH_RANGE, 70.0);
    let half = scene_light::torch(target, 0.5, true).unwrap();
    assert!((half.color_range[0] - full.color_range[0] * 0.5).abs() < 1e-6, "scales with night");
}

#[test]
fn torch_first_in_nearest() {
    use nostalre_renderer::PointLightGpu;
    use nostalre_renderer::global_uniforms::nearest_point_lights;
    let lamp = PointLightGpu {
        position: [10.0, 0.0, 10.0, 0.0],
        color_range: [1.0, 0.8, 0.5, 40.0],
    };
    let center = glam::Vec3::new(10.0, 0.0, 10.0);
    let torch = scene_light::torch(center, 1.0, true);
    let out = nearest_point_lights(&[lamp], center, torch);
    assert_eq!(out.count, 2);
    assert_eq!(out.lights[0].color_range[3], scene_light::TORCH_RANGE, "torch in slot 0");
    let day = nearest_point_lights(&[lamp], center, None);
    assert_eq!(day.count, 1);
    assert_eq!(day.lights[0].color_range, lamp.color_range);
}

#[test]
fn light_uniform_size_128() {
    assert_eq!(std::mem::size_of::<LightUniform>(), 128);
    assert_eq!(std::mem::offset_of!(LightUniform, glow), 52);
    assert_eq!(std::mem::offset_of!(LightUniform, scene), 64);
    assert_eq!(std::mem::offset_of!(LightUniform, cloud), 80);
    assert_eq!(std::mem::offset_of!(LightUniform, day_light), 96);
    let d = LightUniform::default();
    assert_eq!(d.scene, [0.0; 4], "scene light off unless HD asks");
}

const LIGHT_SHADERS: [(&str, &str); 7] = [
    ("terrain", nostalre_renderer::TERRAIN_WGSL),
    ("model", include_str!("../src/shaders/model.wgsl")),
    ("model_animated", include_str!("../src/shaders/model_animated.wgsl")),
    ("grass", include_str!("../src/shaders/grass.wgsl")),
    ("gr2_model", include_str!("../src/shaders/gr2_model.wgsl")),
    ("water", include_str!("../src/shaders/water.wgsl")),
    ("water_hd", nostalre_renderer::WATER_HD_WGSL),
];

fn parse(name: &str, src: &str) -> naga::Module {
    naga::front::wgsl::parse_str(src)
        .unwrap_or_else(|e| panic!("{name}.wgsl failed to parse: {}", e.emit_to_string(src)))
}

#[test]
fn light_structs_match_rust() {
    for (name, src) in LIGHT_SHADERS {
        let module = parse(name, src);
        let span = module
            .types
            .iter()
            .find(|(_, t)| t.name.as_deref() == Some("LightUniforms"))
            .map(|(_, t)| match &t.inner {
                naga::TypeInner::Struct { span, .. } => *span,
                other => panic!("{name}: LightUniforms is {other:?}"),
            })
            .unwrap_or_else(|| panic!("{name}.wgsl has no LightUniforms"));
        assert_eq!(span as usize, std::mem::size_of::<LightUniform>(), "{name}.wgsl");
    }
}

#[test]
fn light_shaders_validate() {
    for (name, src) in LIGHT_SHADERS {
        let module = parse(name, src);
        naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::all())
            .validate(&module)
            .unwrap_or_else(|e| panic!("{name}.wgsl failed validation: {e:?}"));
    }
}

/// The terrain draws the lamps (S5): it reads the shared point-light uniform, laid out as the Rust one.
#[test]
fn terrain_reads_point_lights() {
    let src = nostalre_renderer::TERRAIN_WGSL;
    let module = parse("terrain", src);
    let span = module
        .types
        .iter()
        .find(|(_, t)| t.name.as_deref() == Some("PointLights"))
        .map(|(_, t)| match &t.inner {
            naga::TypeInner::Struct { span, .. } => *span,
            other => panic!("PointLights is {other:?}"),
        })
        .expect("terrain.wgsl has PointLights");
    assert_eq!(
        span as usize,
        std::mem::size_of::<nostalre_renderer::global_uniforms::PointLightsUniform>()
    );
    assert!(src.contains("@group(0) @binding(2) var<uniform> point_lights"));
    assert!(src.contains("fn lightmap_treat("));
    assert!(src.contains("fn cloud_shadow("));
    assert!(src.contains("fn night_lift("));
}

fn lamp_at(x: f32, color: [f32; 3], range: f32) -> nostalre_renderer::PointLightGpu {
    nostalre_renderer::PointLightGpu {
        position: [x, 0.0, 0.0, 0.0],
        color_range: [color[0], color[1], color[2], range],
    }
}

#[test]
fn lamps_at_matches_cap_and_torch() {
    use nostalre_renderer::global_uniforms::nearest_point_lights;
    let lights = [lamp_at(0.0, [0.5, 0.4, 0.2], 50.0), lamp_at(1.0, [0.2, 0.2, 0.2], 50.0)];
    let near = nearest_point_lights(&lights, glam::Vec3::ZERO, None);
    let at = glam::Vec3::ZERO;
    let one = scene_light::lamps_at(&near, 1, at);
    assert!(close(one, [0.4, 0.32, 0.16]), "a lamp at the actor: its colour × 0.8: {one:?}");
    let both = scene_light::lamps_at(&near, 2, at);
    assert!(both[0] > one[0], "the second lamp counts with bound 2");
    assert_eq!(scene_light::lamps_at(&near, 0, at), [0.0; 3], "bound 0: none");
}

#[test]
fn lamps_at_zero_out_of_range() {
    use nostalre_renderer::global_uniforms::nearest_point_lights;
    let near = nearest_point_lights(&[lamp_at(0.0, [1.0; 3], 30.0)], glam::Vec3::ZERO, None);
    assert_eq!(scene_light::lamps_at(&near, 32, glam::Vec3::new(30.0, 0.0, 0.0)), [0.0; 3]);
    assert_eq!(scene_light::lamps_at(&near, 32, glam::Vec3::new(0.0, 0.0, 80.0)), [0.0; 3]);
}

#[test]
fn actor_factor_identity_by_day_no_lamps() {
    let s = [0.8, 0.9, 1.0];
    assert_eq!(scene_light::actor_factor(s, [0.3; 3], 0.0, [0.0; 3]), [1.0; 3]);
}

#[test]
fn actor_factor_day_look_in_pool_at_night() {
    let s = [0.45, 0.55, 0.75];
    // a strong bake: still past the lift's top (0.18) after the treatment rolls it off
    let f = scene_light::actor_factor(s, [1.0, 0.95, 0.9], 1.0, [0.0; 3]);
    for i in 0..3 {
        assert!((f[i] * s[i] - 1.0).abs() < 1e-4, "channel {i}: {f:?}");
    }
}

#[test]
fn actor_factor_stays_moonlit_outside_pools() {
    let s = [0.45, 0.55, 0.75];
    assert_eq!(scene_light::actor_factor(s, [0.01; 3], 1.0, [0.0; 3]), [1.0; 3]);
}

#[test]
fn actor_factor_adds_lamps() {
    let s = [0.5, 0.5, 0.5];
    let f = scene_light::actor_factor(s, [0.0; 3], 0.0, [0.2, 0.1, 0.0]);
    assert!(close(f, [1.4, 1.2, 1.0]), "{f:?}");
    let dark = scene_light::actor_factor([0.0; 3], [0.0; 3], 1.0, [0.2; 3]);
    assert!(dark.iter().all(|c| c.is_finite()), "no division by a black sky: {dark:?}");
}

/// A lamp model's light: warm, about 6 cells, at its head.
#[test]
fn lamp_lights_from_models() {
    let top = glam::Vec3::new(300.0, -45.0, 420.0);
    let l = scene_light::lamp_light(top);
    assert_eq!(l.position[..3], [300.0, -45.0, 420.0]);
    assert_eq!(l.color_range[3], scene_light::LAMP_RANGE);
    assert_eq!(scene_light::LAMP_RANGE, 60.0);
    let c = scene_light::LAMP_COLOR;
    assert!(close([l.color_range[0], l.color_range[1], l.color_range[2]], c), "{l:?}");
    assert!(c[0] >= c[1] && c[1] >= c[2], "warm: {c:?}");
}

/// A tall lamp's light hangs low enough to light the ground under it (Y grows down); a short one stays at its
/// head.
#[test]
fn tall_lamp_light_hangs_low() {
    // a 40-unit street lamp standing on y = 10: light 15 above its base, not at its top
    assert_eq!(scene_light::lamp_light_y(-30.0, 10.0), 10.0 - scene_light::LAMP_HANG);
    // a 6-unit lantern: at its head
    assert_eq!(scene_light::lamp_light_y(4.0, 10.0), 4.0);
    // the ground under a tall lamp gets a visible share of its light
    let under = scene_light::point_atten(scene_light::LAMP_HANG, scene_light::LAMP_RANGE);
    assert!(under > 0.2, "{under}");
}

/// The Lamp light row scales the lamps: 50 % as designed, 100 % twice that, 0 % none.
#[test]
fn lamp_strength_follows_the_row() {
    assert_eq!(scene_light::lamp_strength(1.0, 0.5), 1.0);
    assert_eq!(scene_light::lamp_strength(1.0, 1.0), 2.0);
    assert_eq!(scene_light::lamp_strength(1.0, 0.0), 0.0);
    assert_eq!(scene_light::lamp_strength(0.0, 1.0), 0.0, "never by day");
}

/// The lamps join the map's lights only at night, fading with it; by day the map's own lights alone.
#[test]
fn lamp_lights_night_only() {
    let rsw = [nostalre_renderer::PointLightGpu {
        position: [0.0; 4],
        color_range: [0.5, 0.5, 0.5, 40.0],
    }];
    let lamps = [scene_light::lamp_light(glam::Vec3::new(10.0, -40.0, 10.0))];
    let day = scene_light::night_lights(&rsw, &lamps, 0.0);
    assert_eq!(day.len(), 1, "by day: the map's lights only");
    let night = scene_light::night_lights(&rsw, &lamps, 1.0);
    assert_eq!(night.len(), 2);
    assert_eq!(night[1].color_range, lamps[0].color_range);
    let dusk = scene_light::night_lights(&rsw, &lamps, 0.5);
    assert!((dusk[1].color_range[0] - lamps[0].color_range[0] * 0.5).abs() < 1e-6, "fades with the night");
    assert_eq!(dusk[1].color_range[3], scene_light::LAMP_RANGE, "range kept");
}
