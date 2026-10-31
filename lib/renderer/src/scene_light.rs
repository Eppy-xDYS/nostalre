//! HD scene light: softer baked lightmaps, floor-only baked shadows, map lamps on the ground, drifting cloud
//! shadows and a moonlit night where lamp pools stay lit. The terrain, model and grass shaders do the per-pixel
//! part; these are their CPU twins plus the light each frame writes. Off (HD off, rows off) the uniform's
//! `scene` is all 0 and the shaders take today's path.

use crate::global_uniforms::{LightUniform, PointLightGpu, PointLightsUniform};

const LUMA: [f32; 3] = [0.2126, 0.7152, 0.0722];

/// Baked colours pulled this far toward their brightest channel: yellow and blue lamps both lean white.
pub const LM_DESATURATE: f32 = 0.11;
/// Bright bakes roll off by `1 / (1 + this × luma)`, so lamps read as light, not coloured stains.
pub const LM_ROLLOFF: f32 = 3.5;
/// How much a floor's baked shadow still darkens it (1: all of it, as today).
pub const BAKED_SHADOW: f32 = 0.6;
/// Lamp falloff past the windowed inverse square.
pub const PL_FALLOFF: f32 = 40.0;
/// Summed lamp light above this (brightest channel) rolls off toward `KNEE_LIMIT` instead of clipping.
pub const KNEE: f32 = 0.70;
pub const KNEE_LIMIT: f32 = 1.15;
/// The summed lamps' share of the surface colour.
pub const LAMP_SCALE: f32 = 0.8;
/// Cloud drift, noise units per second (x, z).
pub const CLOUD_WIND: [f32; 2] = [0.018, 0.011];
/// Night as a factor on the map's day sun and sky: a dim blue moonlight where the scene stays readable.
pub const NIGHT_DIFFUSE: [f32; 3] = [0.30, 0.38, 0.50];
pub const NIGHT_AMBIENT: [f32; 3] = [0.50, 0.62, 0.80];
/// The player's torch at night: warm, about 7 cells across, held 7.5 units up.
pub const TORCH_COLOR: [f32; 3] = [0.7, 0.595, 0.385];
pub const TORCH_RANGE: f32 = 70.0;
pub const TORCH_HEIGHT: f32 = 7.5;

/// What the HD rows ask of the scene light.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneOptions {
    /// Lightmap treatment, baked shadows and lamps on the ground
    pub on: bool,
    pub night_look: bool,
    /// 0 to 1, 0 = no clouds
    pub clouds: f32,
    /// Lamps the terrain shades with (by quality preset)
    pub light_cap: u32,
    /// The sun crosses the sky once an hour (`day_cycle`)
    pub day_cycle: bool,
}

impl Default for SceneOptions {
    fn default() -> Self {
        Self {
            on: true,
            night_look: true,
            clouds: 0.6,
            light_cap: 16,
            day_cycle: true,
        }
    }
}

fn luma(c: [f32; 3]) -> f32 {
    c[0] * LUMA[0] + c[1] * LUMA[1] + c[2] * LUMA[2]
}

fn max3(c: [f32; 3]) -> f32 {
    c[0].max(c[1]).max(c[2])
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A baked light colour pulled toward its brightest channel at the same luma, then rolled off when bright.
pub fn lightmap_treat(lm: [f32; 3]) -> [f32; 3] {
    let top = max3(lm);
    let neutral = lm.map(|c| c + (top - c) * LM_DESATURATE);
    let (y0, y1) = (luma(lm), luma(neutral));
    let keep = if y1 > 1e-6 { y0 / y1 } else { 1.0 };
    let lm = neutral.map(|c| c * keep);
    let roll = 1.0 + LM_ROLLOFF * luma(lm);
    lm.map(|c| c / roll)
}

/// The share of light a baked shadow leaves: floors only (`normal_up` is the normal's upward part), and the
/// usual lit level (alpha 0.65 and up) stays fully lit.
pub fn baked_shadow(alpha: f32, normal_up: f32) -> f32 {
    let floor = smoothstep(0.35, 0.70, normal_up);
    let shade = smoothstep(0.20, 0.65, alpha.clamp(0.0, 1.0));
    1.0 + (shade - 1.0) * BAKED_SHADOW * floor
}

/// A lamp's share at distance `d`: windowed inverse square, 1 at the lamp, 0 at its range.
pub fn point_atten(d: f32, range: f32) -> f32 {
    if range <= 0.0 || d >= range {
        return 0.0;
    }
    let t2 = (d / range).powi(2);
    let window = (1.0 - t2 * t2).powi(2);
    window / (1.0 + PL_FALLOFF * t2)
}

/// Summed lamp light, overlapping hot lamps rolled off instead of clipped, at the lamps' share.
pub fn light_knee(sum: [f32; 3]) -> [f32; 3] {
    let e = max3(sum);
    let scale = if e > KNEE {
        let span = KNEE_LIMIT - KNEE;
        (KNEE + span * (1.0 - (-(e - KNEE) / span).exp())) / e
    } else {
        1.0
    };
    sum.map(|c| c * scale * LAMP_SCALE)
}

/// At night, a pixel inside a baked light pool goes back toward its daylit look, so lamps and the walls round
/// them stay lit.
pub fn night_lift(lit: [f32; 3], surface: [f32; 3], lm: [f32; 3], night: f32, day: [f32; 3]) -> [f32; 3] {
    let k = smoothstep(0.02, 0.2, max3(lm)) * night;
    if k <= 0.0 {
        return lit;
    }
    std::array::from_fn(|i| lit[i] + (surface[i] * day[i] + lm[i] - lit[i]) * k)
}

/// The clouds' drift after `seconds`, wrapped so the shader's noise keeps its precision.
pub fn cloud_offset(seconds: f32) -> [f32; 2] {
    CLOUD_WIND.map(|w| (w * seconds) % 256.0)
}

/// The map's light at `night` (0 day, 1 full night).
pub fn night_light(base: &LightUniform, night: f32) -> LightUniform {
    let mut out = *base;
    for i in 0..3 {
        out.diffuse_color[i] = base.diffuse_color[i] * (1.0 + (NIGHT_DIFFUSE[i] - 1.0) * night);
        out.ambient_color[i] = base.ambient_color[i] * (1.0 + (NIGHT_AMBIENT[i] - 1.0) * night);
    }
    out
}

/// The sprites' light factor for `now` against the map's `day` light (1 by day), as today's day/night fade
/// works it out, with the sky dimmed too.
pub fn sprite_light(day: &LightUniform, now: &LightUniform) -> [f32; 3] {
    std::array::from_fn(|i| {
        let env = |d: f32, a: f32| 1.0 - (1.0 - d) * (1.0 - a);
        let (d0, a0) = (day.diffuse_color[i], day.ambient_color[i]);
        let (d1, a1) = (now.diffuse_color[i], now.ambient_color[i]);
        let denom = env(d0, a0) + a0;
        if denom > 1e-4 { (env(d1, a1) + a1) / denom } else { 1.0 }
    })
}

/// The light to write this frame. `legacy_diffuse` is today's day/night fade; it stays unless the HD night
/// runs (HD effects on, outdoors, Night look on). `scene` stays 0 with HD effects off.
pub fn frame_light(
    base: &LightUniform,
    legacy_diffuse: [f32; 3],
    night: f32,
    hd: bool,
    indoor: bool,
    opts: &SceneOptions,
) -> LightUniform {
    let hd_night = hd && !indoor && opts.night_look;
    let mut out = if hd_night {
        night_light(base, night)
    } else {
        let mut today = *base;
        today.diffuse_color[..3].copy_from_slice(&legacy_diffuse);
        today
    };
    if hd {
        let clouds = if indoor { 0.0 } else { opts.clouds.clamp(0.0, 1.0) };
        out.scene = [
            if opts.on { 1.0 } else { 0.0 },
            if hd_night { night } else { 0.0 },
            clouds,
            if opts.on { opts.light_cap as f32 } else { 0.0 },
        ];
        for i in 0..3 {
            out.day_light[i] = (base.diffuse_color[i] + base.ambient_color[i]).min(1.0);
        }
    }
    out
}

/// The lamps' light at `world`, as the terrain sums it: the first `bound` slots (the preset's cap, plus the torch
/// at night), windowed falloff, the knee. Sprites face the camera, so no Lambert term.
pub fn lamps_at(lights: &PointLightsUniform, bound: u32, world: glam::Vec3) -> [f32; 3] {
    let mut sum = [0.0; 3];
    for light in lights.lights.iter().take(lights.count.min(bound) as usize) {
        let range = light.color_range[3];
        let d = glam::Vec3::from_slice(&light.position[..3]).distance(world);
        let a = point_atten(d, range);
        for (s, c) in sum.iter_mut().zip(&light.color_range[..3]) {
            *s += c * a;
        }
    }
    if sum == [0.0; 3] {
        return sum;
    }
    light_knee(sum)
}

/// A sprite's factor over the sprites' world light `sprite_light`: at night, inside a baked light pool (`baked`,
/// the raw lightmap colour under it) it goes back toward its daylit look, as the ground does; lamps add on top.
pub fn actor_factor(sprite_light: [f32; 3], baked: [f32; 3], night: f32, lamps: [f32; 3]) -> [f32; 3] {
    let k = smoothstep(0.02, 0.2, max3(lightmap_treat(baked))) * night;
    std::array::from_fn(|i| {
        let s = sprite_light[i];
        let lit = s + (1.0 - s) * k + lamps[i];
        if lit == s { 1.0 } else { lit / s.max(1e-3) }
    })
}

/// A light the map editor never placed sits at the origin (royal ships 112 of them).
pub fn is_placed(position: [f32; 3]) -> bool {
    position != [0.0; 3]
}

/// The torch over the player (`target`) at `night`; none by day or indoors. Up is -y.
pub fn torch(target: glam::Vec3, night: f32, outdoors: bool) -> Option<PointLightGpu> {
    if night <= 0.0 || !outdoors {
        return None;
    }
    Some(PointLightGpu {
        position: [target.x, target.y - TORCH_HEIGHT, target.z, 0.0],
        color_range: [
            TORCH_COLOR[0] * night,
            TORCH_COLOR[1] * night,
            TORCH_COLOR[2] * night,
            TORCH_RANGE,
        ],
    })
}
