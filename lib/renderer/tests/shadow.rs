use glam::{Vec3, Vec4};
use nostalre_renderer::shadow::{self, RedrawState, ShadowTier, ShadowUniform};

fn sun(lon_deg: f32, lat_deg: f32) -> Vec3 {
    let (lon, lat) = (lon_deg.to_radians(), lat_deg.to_radians());
    Vec3::new(-lon.cos() * lat.sin(), -lat.cos(), -lon.sin() * lat.sin()).normalize()
}

fn clip(b: &shadow::ShadowBox, p: Vec3) -> Vec3 {
    let c = b.light_vp * Vec4::new(p.x, p.y, p.z, 1.0);
    c.truncate() / c.w
}

fn inside(c: Vec3) -> bool {
    c.x.abs() <= 1.0 && c.y.abs() <= 1.0 && (0.0..=1.0).contains(&c.z)
}

const FOCUS: Vec3 = Vec3::new(1500.0, -20.0, 1800.0);

/// A 40-cell square round the focus (±200 units), at the focus height ± 60, lands in the box for any sun from
/// straight down to 75° off the zenith.
#[test]
fn box_contains_focus_area() {
    for (lon, lat) in [(0.0, 0.0), (45.0, 40.0), (200.0, 60.0), (300.0, 75.0), (90.0, 75.0)] {
        let b = shadow::light_box(sun(lon, lat), FOCUS, 2048);
        for dx in [-200.0, 0.0, 200.0] {
            for dz in [-200.0, 0.0, 200.0] {
                for dy in [-60.0, 0.0, 60.0] {
                    let p = FOCUS + Vec3::new(dx, dy, dz);
                    assert!(inside(clip(&b, p)), "sun ({lon}, {lat}) point {p}: {}", clip(&b, p));
                }
            }
        }
    }
}

/// The box looks down the sun: a point and the same point moved toward the sun land on the same texel.
#[test]
fn box_follows_sun() {
    for (lon, lat) in [(45.0, 40.0), (300.0, 75.0)] {
        let d = sun(lon, lat);
        let b = shadow::light_box(d, FOCUS, 2048);
        let a = clip(&b, FOCUS);
        // the light direction points toward the sun (as the shaders' n·l reads it)
        let up_sun = clip(&b, FOCUS + d * 100.0);
        assert!((a.x - up_sun.x).abs() < 1e-4 && (a.y - up_sun.y).abs() < 1e-4, "{a} vs {up_sun}");
        assert!(up_sun.z < a.z, "nearer the sun = nearer the light: {} vs {}", up_sun.z, a.z);
    }
}

#[test]
fn snap_stable_under_subtexel_move() {
    let d = sun(45.0, 40.0);
    let size = 2048;
    let texel_world = 2.0 * shadow::HALF_EXTENT / size as f32;
    let p = Vec3::new(1600.0, -10.0, 1750.0);
    let a = shadow::light_box(d, FOCUS, size);
    let b = shadow::light_box(d, FOCUS + Vec3::new(0.2 * texel_world, 0.0, 0.1 * texel_world), size);
    // depth may shift (the eye moves with the focus); the texel grid must not
    assert_eq!(clip(&a, p).truncate(), clip(&b, p).truncate(), "a fifth of a texel: no change");
    let far = shadow::light_box(d, FOCUS + Vec3::new(10.3 * texel_world, 0.0, 0.0), size);
    let texel_clip = 2.0 / size as f32;
    for shift in [clip(&far, p).x - clip(&a, p).x, clip(&far, p).y - clip(&a, p).y] {
        let n = shift / texel_clip;
        assert!((n - n.round()).abs() < 1e-2, "moves by whole texels: {n}");
    }
}

fn state(center: Vec3, dir: Vec3) -> RedrawState {
    RedrawState {
        center,
        dir,
        models_ready: true,
        generation: 1,
    }
}

#[test]
fn redraw_on_move_turn_ready_only() {
    let d = sun(45.0, 40.0);
    let last = state(FOCUS, d);
    assert!(shadow::needs_redraw(None, &last, 0, false), "first frame");
    assert!(!shadow::needs_redraw(Some(&last), &last, 100, false), "still: never");
    let near = state(FOCUS + Vec3::new(19.0, 0.0, 0.0), d);
    assert!(!shadow::needs_redraw(Some(&last), &near, 5, false), "under 2 cells");
    let moved = state(FOCUS + Vec3::new(21.0, 0.0, 0.0), d);
    assert!(shadow::needs_redraw(Some(&last), &moved, 0, false), "2 cells");
    assert!(!shadow::needs_redraw(Some(&last), &state(FOCUS, sun(45.4, 40.0)), 0, false), "< 0.5°");
    assert!(shadow::needs_redraw(Some(&last), &state(FOCUS, sun(46.0, 40.0)), 0, false), "> 0.5°");
    let loading = RedrawState {
        models_ready: false,
        ..last
    };
    assert!(shadow::needs_redraw(Some(&loading), &last, 0, false), "models finished loading");
    let new_map = RedrawState {
        generation: 2,
        ..last
    };
    assert!(shadow::needs_redraw(Some(&last), &new_map, 0, false), "map or tier changed");
    assert!(!shadow::needs_redraw(Some(&last), &last, 29, true), "animated: not yet");
    assert!(shadow::needs_redraw(Some(&last), &last, 30, true), "animated: every 30 frames");
}

#[test]
fn sun_light_keeps_ambient() {
    let total = [1.2, 1.1, 1.0];
    let ambient = [0.4, 0.4, 0.5];
    let shaded = shadow::sun_light(total, ambient, 1.0, 0.65, 0.0);
    for i in 0..3 {
        let want = ambient[i] + (total[i] - ambient[i]) * 0.35;
        assert!((shaded[i] - want).abs() < 1e-5, "{shaded:?}");
    }
    assert_eq!(shadow::sun_light(total, ambient, 0.0, 0.65, 0.0), total, "no shadow");
    assert_eq!(shadow::sun_light(total, ambient, 1.0, 0.0, 0.0), total, "0 %: off");
    let dim = shadow::sun_light([0.3; 3], [0.5; 3], 1.0, 0.65, 0.0);
    assert_eq!(dim, [0.3; 3], "sky brighter than the total: nothing to take");
}

#[test]
fn night_weakens_shadow() {
    let total = [1.0; 3];
    let ambient = [0.2; 3];
    let day = shadow::sun_light(total, ambient, 1.0, 1.0, 0.0);
    let night = shadow::sun_light(total, ambient, 1.0, 1.0, 1.0);
    let lost = |v: [f32; 3]| total[0] - v[0];
    assert!((lost(night) - lost(day) * 0.4).abs() < 1e-5, "{day:?} {night:?}");
}

#[test]
fn tiers_by_preset() {
    assert_eq!(shadow::tier(0), None, "Low: none");
    assert_eq!(shadow::tier(1), Some(ShadowTier { size: 1024, taps: 1 }));
    assert_eq!(shadow::tier(2), Some(ShadowTier { size: 2048, taps: 9 }));
    assert_eq!(shadow::tier(3), Some(ShadowTier { size: 2048, taps: 25 }));
}

#[test]
fn shadow_off_cases() {
    let high = shadow::tier(2);
    assert!(shadow::active(true, false, 0.65, high));
    assert!(!shadow::active(false, false, 0.65, high), "HD off");
    assert!(!shadow::active(true, true, 0.65, high), "indoors");
    assert!(!shadow::active(true, false, 0.0, high), "0 %");
    assert!(!shadow::active(true, false, 0.65, shadow::tier(0)), "Low");
}

#[test]
fn shadow_uniform_size() {
    assert_eq!(std::mem::size_of::<ShadowUniform>(), 80);
    assert_eq!(ShadowUniform::OFF.params[3], 0.0, "off: no lookups");
}

const WORLD_SHADERS: [(&str, &str); 7] = [
    ("terrain", include_str!("../src/shaders/terrain.wgsl")),
    ("model", include_str!("../src/shaders/model.wgsl")),
    ("model_animated", include_str!("../src/shaders/model_animated.wgsl")),
    ("grass", include_str!("../src/shaders/grass.wgsl")),
    ("gr2_model", include_str!("../src/shaders/gr2_model.wgsl")),
    ("water", include_str!("../src/shaders/water.wgsl")),
    ("water_hd", include_str!("../src/shaders/water_hd.wgsl")),
];

fn parse(name: &str, src: &str) -> naga::Module {
    naga::front::wgsl::parse_str(src)
        .unwrap_or_else(|e| panic!("{name}.wgsl failed to parse: {}", e.emit_to_string(src)))
}

fn validate(name: &str, module: &naga::Module) {
    naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::all())
        .validate(module)
        .unwrap_or_else(|e| panic!("{name}.wgsl failed validation: {e:?}"));
}

#[test]
fn all_world_shaders_validate() {
    for (name, src) in WORLD_SHADERS {
        validate(name, &parse(name, src));
    }
    let depth = include_str!("../src/shaders/shadow_depth.wgsl");
    let module = parse("shadow_depth", depth);
    validate("shadow_depth", &module);
    for entry in ["vs_ground", "vs_model", "vs_animated", "fs_alpha"] {
        assert!(module.entry_points.iter().any(|e| e.name == entry), "shadow_depth lacks {entry}");
    }
}

/// The receivers look the shadow up only while it runs, and then drop the map's baked shadow.
#[test]
fn receivers_sample_and_skip_when_off() {
    for (name, src) in &WORLD_SHADERS[..4] {
        let span = parse(name, src)
            .types
            .iter()
            .find(|(_, t)| t.name.as_deref() == Some("ShadowUniforms"))
            .map(|(_, t)| match &t.inner {
                naga::TypeInner::Struct { span, .. } => *span,
                other => panic!("{name}: ShadowUniforms is {other:?}"),
            })
            .unwrap_or_else(|| panic!("{name}.wgsl has no ShadowUniforms"));
        assert_eq!(span as usize, std::mem::size_of::<ShadowUniform>(), "{name}");
        assert!(src.contains("fn sun_shadow("), "{name} lacks sun_shadow");
        assert!(src.contains("shadow.params.w"), "{name} does not gate on shadow.params.w");
    }
    for (name, src) in [WORLD_SHADERS[0], WORLD_SHADERS[3]] {
        assert!(src.contains("fn baked_alpha("), "{name} lacks baked_alpha (H6)");
    }
}
