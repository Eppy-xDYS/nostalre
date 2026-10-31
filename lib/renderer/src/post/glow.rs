//! HD world glow: lamp glass, baked light pools and sunlit highlights pushed a little above white so the bloom
//! picks them up. Each is a bounded extra on top of the clamped colour, 0 whenever bloom cannot show it. The
//! terrain and model shaders do the same per pixel; these are their CPU twins.

use super::HdOptions;

/// A lamp texel at full mask and 100 % reaches about 2.5 (bloom caps at 2).
pub const LAMP_GAIN: f32 = 1.5;
/// Baked light at or below this stays as it is: the lightmap's dim fill everywhere.
pub const POOL_THRESHOLD: f32 = 0.16;
/// Full baked light at 100 %.
pub const POOL_GAIN: f32 = 0.6;
/// Near-white sunlit stone at 30 % lands about 1.15.
pub const SUN_GAIN: f32 = 0.5;
/// Sun glow starts at this brightness (brightest channel) and is full at white: mid-tone sand stays as it is.
pub const SUN_KNEE: f32 = 0.8;

const LAMP_WORDS: [&str; 12] = [
    "lamp", "torch", "candle", "lantern", "brazier", "가로등", "램프", "촛불", "횃불", "등불", "화로", "양초",
];

/// A model or texture is a light source by its file name: lamp, torch, candle, lantern, brazier (also in Korean:
/// 가로등 street lamp, 화로 brazier...), or a `light` word (`bif_light01`, `cave_inlight01`), not `lighthouse` or
/// `lightning`. The folder does not count. A lamp model's textures mostly have plain names (`UP.BMP`), so a model
/// match flags all of its textures; the mask then picks the bright parts.
pub fn is_lamp_name(path: &str) -> bool {
    let file = path.rsplit(['\\', '/']).next().unwrap_or(path).to_lowercase();
    let stem = file.rsplit_once('.').map_or(file.as_str(), |(stem, _)| stem);
    LAMP_WORDS.iter().any(|w| stem.contains(w))
        || stem
            .split(['_', '-', ' '])
            .map(|word| word.trim_matches(|c: char| c.is_ascii_digit()))
            .any(|word| word == "light" || word == "inlight")
}

/// A vertex's `emissive`: its texture is named for a lamp
pub const EMISSIVE_NAMED: f32 = 1.0;
/// Only its model is: the texture may be the pole or a stone base as bright as the glass
pub const EMISSIVE_MODEL: f32 = 0.5;

/// The vertex `emissive` for a face: 0 when neither the model nor the texture is a lamp.
pub fn emissive(lamp_model: bool, texture_path: &str) -> f32 {
    if is_lamp_name(texture_path) {
        EMISSIVE_NAMED
    } else if lamp_model {
        EMISSIVE_MODEL
    } else {
        0.0
    }
}

/// How much of a lamp texel glows: its bright parts (the glass, the flame), not the dark frame. A texture flagged
/// only by its model needs to be nearly white, so light stone and poles stay as they are.
pub fn lamp_mask(rgb: [f32; 3], emissive: f32) -> f32 {
    let peak = rgb[0].max(rgb[1]).max(rgb[2]);
    if emissive > 0.75 {
        smoothstep(0.55, 0.85, peak)
    } else if emissive > 0.25 {
        smoothstep(0.8, 0.95, peak)
    } else {
        0.0
    }
}

/// The glow a baked light pool adds at 100 %, per channel.
pub fn pool_extra(lightmap: [f32; 3]) -> [f32; 3] {
    lightmap.map(|c| (c - POOL_THRESHOLD).max(0.0) / (1.0 - POOL_THRESHOLD) * POOL_GAIN)
}

/// The glow sunlight adds at 100 % to `color` (0 to 1): `sun_lit` is how directly the sun falls times its
/// brightness, 0 to 1. Only colours near white get it.
pub fn sun_extra(color: [f32; 3], sun_lit: f32) -> [f32; 3] {
    let near_white = smoothstep(SUN_KNEE, 1.0, color[0].max(color[1]).max(color[2]));
    color.map(|c| c * near_white * sun_lit * SUN_GAIN)
}

/// The glow levels (lamp, pools, sun) the shaders get: none unless bloom runs on a half-float target, since
/// light above white only clips otherwise.
pub fn effective(levels: [f32; 3], hd: &HdOptions, effects_on: bool) -> [f32; 3] {
    let blooms = hd.enabled
        && effects_on
        && hd.float_target
        && hd.bloom_levels > 0
        && hd.bloom_intensity > 0.0;
    if blooms { levels } else { [0.0; 3] }
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
