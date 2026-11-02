use nostalre_renderer::LightUniform;
use nostalre_renderer::model::{AnimatedModelVertex, ModelVertex};
use nostalre_renderer::post::HdOptions;
use nostalre_renderer::post::glow::{self, POOL_GAIN, POOL_THRESHOLD, SUN_GAIN};
use nostalre_renderer::post::lamp_glass;

const LEVELS: [f32; 3] = [0.6, 0.5, 0.3];

fn bloom_on() -> HdOptions {
    HdOptions {
        enabled: true,
        float_target: true,
        bloom_levels: 4,
        bloom_intensity: 0.6,
        ..HdOptions::default()
    }
}

#[test]
fn lamp_names_match() {
    for name in [
        "data\\texture\\외부소품\\lamp.bmp",
        "data\\texture\\프론테라\\pron-lamp1.bmp",
        "data\\texture\\내부소품\\dg1_torch.bmp",
        "data\\texture\\내부소품\\candlelight3 copy.bmp",
        "data\\texture\\eclage\\ecl_in_light.tga",
        "data\\texture\\기타마을내부\\cave_inlight01.bmp",
        "data\\texture\\bifrost\\bif_light01.bmp",
        "DATA\\TEXTURE\\외부소품\\LAMP.BMP",
        "data\\model\\프론테라\\휘장가로등.rsm",
        "data\\model\\prontera_re\\streetlamp_01.rsm",
        "data\\model\\페이욘\\화로.rsm",
        "data\\model\\인던01\\인던01횃불대01.rsm",
    ] {
        assert!(glow::is_lamp_name(name), "{name}");
    }
}

#[test]
fn lamp_names_reject() {
    for name in [
        "data\\texture\\기타마을\\lighthouseisland_01.bmp",
        "data\\texture\\내부소품\\delight1.bmp",
        "data\\texture\\lighthalzen\\lhz_dun_b02.bmp",
        "data\\texture\\effect\\lightning.bmp",
        "data\\texture\\프론테라\\stone.bmp",
        "data\\model\\페이욘\\주점.rsm",
        "data\\texture\\프론테라\\UP.BMP",
        "",
        // look-alikes of the S2d words
        "data\\model\\등대섬\\그물.rsm",
        "data\\model\\기타\\등대01.rsm",
        "data\\model\\나무잡초꽃\\덤불01.rsm",
        "data\\model\\나무잡초꽃\\모로코 나뭇등걸.rsm",
        "data\\model\\해변마을\\해변가_불가사리1.rsm",
        "data\\model\\아요타야\\와불.rsm",
        "data\\model\\페이욘\\이불채.rsm",
        "data\\texture\\effect\\highlight01.bmp",
        "data\\model\\기타\\등나무.rsm",
    ] {
        assert!(!glow::is_lamp_name(name), "{name}");
    }
}

/// Every lamp model the S2d survey of all maps found (chandeliers, wall and ceiling lamps, lanterns, fireplaces,
/// braziers...) counts as a lamp.
const SURVEY_LAMPS: &str = include_str!("data/lamp_models.txt");

fn survey_lamps() -> impl Iterator<Item = &'static str> {
    SURVEY_LAMPS.lines().map(str::trim).filter(|l| !l.is_empty())
}

#[test]
fn survey_lamp_models_are_lamps() {
    let missed: Vec<&str> = survey_lamps().filter(|m| !glow::is_lamp_name(m)).collect();
    assert!(missed.is_empty(), "not lamps by name: {missed:?}");
}

#[test]
fn glass_table_covers_survey() {
    let missing: Vec<&str> = survey_lamps()
        .filter(|m| !lamp_glass::LAMP_GLASS.iter().any(|(model, _)| model == m))
        .collect();
    assert!(missing.is_empty(), "{} lamps without glass: {missing:?}", missing.len());
}

#[test]
fn glass_table_entries_valid() {
    let mut seen = std::collections::HashSet::new();
    for (model, glass) in lamp_glass::LAMP_GLASS {
        assert!(seen.insert(*model), "{model} twice");
        assert_eq!(*model, model.to_lowercase(), "{model}: keys lowercase");
        assert!(glow::is_lamp_name(model), "{model} is not a lamp name");
        // an empty list is a lamp with nothing to light (an empty iron lantern, a brazier with no flame drawn)
        for g in *glass {
            let [u0, v0, u1, v1] = g.uv;
            assert!(0.0 <= u0 && u0 < u1 && u1 <= 1.0, "{model} {}: u {u0}..{u1}", g.texture);
            assert!(0.0 <= v0 && v0 < v1 && v1 <= 1.0, "{model} {}: v {v0}..{v1}", g.texture);
            assert_eq!(g.texture, g.texture.to_lowercase(), "{model}: texture lowercase");
        }
    }
}

/// A listed lamp's faces glow where their UV centre falls in one of its rects (any case, either slash), and nowhere
/// else; a lamp not in the table keeps the mask rule (`None`).
#[test]
fn glass_face_by_uv_centre() {
    let (model, glass) = *lamp_glass::LAMP_GLASS
        .iter()
        .find(|(_, glass)| !glass.is_empty())
        .unwrap();
    let g = &glass[0];
    let inside = [(g.uv[0] + g.uv[2]) / 2.0, (g.uv[1] + g.uv[3]) / 2.0];
    let upper = format!("DATA\\MODEL\\{}", model.to_uppercase());
    assert_eq!(glow::glass_emissive(&upper, &g.texture.to_uppercase(), inside), Some(glow::EMISSIVE_NAMED));
    let outside = [if g.uv[0] > 0.01 { 0.0 } else { 1.0 }, if g.uv[1] > 0.01 { 0.0 } else { 1.0 }];
    let in_any = glass.iter().any(|o| {
        o.texture == g.texture
            && (o.uv[0]..=o.uv[2]).contains(&outside[0])
            && (o.uv[1]..=o.uv[3]).contains(&outside[1])
    });
    if !in_any {
        assert_eq!(glow::glass_emissive(model, g.texture, outside), Some(0.0), "outside the glass");
    }
    assert_eq!(glow::glass_emissive(model, "other\\texture.bmp", inside), Some(0.0), "another texture");
    assert_eq!(glow::glass_emissive("외부소품\\가로등01.rsm", "프론테라\\pron-lamp1.bmp", inside), None);
    assert_eq!(glow::glass_emissive("프론테라\\주점.rsm", "프론테라\\up.bmp", inside), None);
}

#[test]
fn mask_edges() {
    let named = glow::EMISSIVE_NAMED;
    assert_eq!(glow::lamp_mask([0.55, 0.2, 0.1], named), 0.0);
    assert_eq!(glow::lamp_mask([0.0, 0.0, 0.0], named), 0.0);
    assert_eq!(glow::lamp_mask([0.2, 0.85, 0.1], named), 1.0);
    assert_eq!(glow::lamp_mask([1.0, 1.0, 1.0], named), 1.0);
    let mut last = 0.0;
    for i in 0..=100 {
        let x = i as f32 / 100.0;
        let m = glow::lamp_mask([x, 0.0, 0.0], named);
        assert!(m >= last, "monotonic at {x}");
        last = m;
    }
    assert_eq!(glow::lamp_mask([1.0, 1.0, 1.0], 0.0), 0.0, "not a lamp");
}

#[test]
fn model_only_lamp_needs_near_white() {
    let model = glow::EMISSIVE_MODEL;
    assert_eq!(glow::lamp_mask([0.8, 0.75, 0.7], model), 0.0, "light stone stays");
    assert!(glow::lamp_mask([0.8, 0.75, 0.7], glow::EMISSIVE_NAMED) > 0.5);
    assert_eq!(glow::lamp_mask([0.95, 0.9, 0.7], model), 1.0, "lit glass glows");
}

#[test]
fn emissive_by_name() {
    assert_eq!(glow::emissive(false, "data\\texture\\프론테라\\pron-lamp1.bmp"), glow::EMISSIVE_NAMED);
    assert_eq!(glow::emissive(true, "data\\texture\\프론테라\\pron-lamp1.bmp"), glow::EMISSIVE_NAMED);
    assert_eq!(glow::emissive(true, "data\\texture\\프론테라\\UP.BMP"), glow::EMISSIVE_MODEL);
    assert_eq!(glow::emissive(false, "data\\texture\\프론테라\\UP.BMP"), 0.0);
}

#[test]
fn pool_below_threshold_zero() {
    let t = POOL_THRESHOLD;
    assert_eq!(glow::pool_extra([t, t, t]), [0.0; 3]);
    assert_eq!(glow::pool_extra([0.1, 0.0, 0.05]), [0.0; 3]);
    let full = glow::pool_extra([1.0, 1.0, 1.0]);
    for c in full {
        assert!((c - POOL_GAIN).abs() < 1e-5, "{c}");
    }
    let warm = glow::pool_extra([0.6, 0.16, 0.0]);
    assert!(warm[0] > 0.0 && warm[1] == 0.0 && warm[2] == 0.0, "per channel: {warm:?}");
}

#[test]
fn sun_extra_only_near_white() {
    assert_eq!(glow::sun_extra([0.75, 0.6, 0.4], 1.0), [0.0; 3], "mid-tone sand stays");
    assert_eq!(glow::sun_extra([1.0, 1.0, 1.0], 0.0), [0.0; 3], "no sun, no glow");
    let white = glow::sun_extra([1.0, 1.0, 1.0], 1.0);
    for c in white {
        assert!((c - SUN_GAIN).abs() < 1e-6, "{c}");
    }
    let near = glow::sun_extra([0.9, 0.9, 0.9], 1.0)[0];
    assert!(near > 0.0 && near < white[0]);
}

#[test]
fn glow_passes_levels_with_bloom() {
    assert_eq!(glow::effective(LEVELS, &bloom_on(), true), LEVELS);
}

#[test]
fn glow_zero_on_8bit() {
    let opts = HdOptions {
        float_target: false,
        ..bloom_on()
    };
    assert_eq!(glow::effective(LEVELS, &opts, true), [0.0; 3]);
}

#[test]
fn glow_zero_without_bloom() {
    let no_levels = HdOptions {
        bloom_levels: 0,
        ..bloom_on()
    };
    assert_eq!(glow::effective(LEVELS, &no_levels, true), [0.0; 3]);
    let no_strength = HdOptions {
        bloom_intensity: 0.0,
        ..bloom_on()
    };
    assert_eq!(glow::effective(LEVELS, &no_strength, true), [0.0; 3]);
}

#[test]
fn glow_zero_effects_off() {
    assert_eq!(glow::effective(LEVELS, &bloom_on(), false), [0.0; 3]);
    let hd_off = HdOptions {
        enabled: false,
        ..bloom_on()
    };
    assert_eq!(glow::effective(LEVELS, &hd_off, true), [0.0; 3]);
}

#[test]
fn light_uniform_glow_offset() {
    assert_eq!(std::mem::size_of::<LightUniform>(), 128);
    assert_eq!(std::mem::offset_of!(LightUniform, glow), 52);
    assert_eq!(LightUniform::default().glow, [0.0; 3], "no glow unless HD asks");
}

/// The shaders reading the glow declare it at the uniform's offsets 52/56/60 (three f32 after
/// `shadow_strength`; a vec3 would align to 64).
#[test]
fn shaders_declare_glow() {
    for (name, src) in [
        ("terrain", nostalre_renderer::TERRAIN_WGSL),
        ("model", include_str!("../src/shaders/model.wgsl")),
        ("model_animated", include_str!("../src/shaders/model_animated.wgsl")),
    ] {
        assert!(
            src.contains("shadow_strength: f32,\n    glow_lamp: f32,\n    glow_pools: f32,\n    glow_sun: f32,"),
            "{name}.wgsl LightUniforms lacks the glow fields"
        );
    }
}

fn last_attribute(layout: &wgpu::VertexBufferLayout) -> wgpu::VertexAttribute {
    *layout.attributes.last().unwrap()
}

#[test]
fn model_vertex_layout_matches() {
    let attr = last_attribute(&ModelVertex::LAYOUT);
    assert_eq!(attr.shader_location, 6);
    assert_eq!(attr.offset as usize, std::mem::offset_of!(ModelVertex, emissive));
    assert_eq!(attr.format, wgpu::VertexFormat::Float32);
    assert_eq!(ModelVertex::LAYOUT.array_stride as usize, std::mem::size_of::<ModelVertex>());

    let attr = last_attribute(&AnimatedModelVertex::LAYOUT);
    assert_eq!(attr.shader_location, 7);
    assert_eq!(
        attr.offset as usize,
        std::mem::offset_of!(AnimatedModelVertex, emissive)
    );
    assert_eq!(attr.format, wgpu::VertexFormat::Float32);
}
