//! HD day cycle: one game day per real hour on the UTC wall clock, so every player (and the server, which runs night
//! from minute 45 to the hour) sees the same time without a packet for it. The clock gives the sun's arc and the
//! dawn and dusk colours; the server's night (status 160) always wins over the clock.

use crate::global_uniforms::LightUniform;
use crate::scene_light::SceneOptions;

/// One cycle: the server's `day_duration` + `night_duration` (45 + 15 min).
pub const CYCLE_MS: u64 = 3_600_000;
/// Where the server's night starts (`day_duration` over the cycle).
pub const NIGHT_FROM: f32 = 0.75;
/// Dawn: the first 5 minutes; dusk: the 5 before night.
pub const DAWN_END: f32 = 5.0 / 60.0;
pub const DUSK_FROM: f32 = 40.0 / 60.0;
/// Night starts gathering this far into dusk, half way by the server's night.
const DUSK_DIM_FROM: f32 = 42.5 / 60.0;
const DUSK_DIM: f32 = 0.5;
/// The sun swings this far either side of the map's own direction, and sets this far from the zenith.
const SWING_DEG: f32 = 60.0;
const HORIZON_DEG: f32 = 75.0;
/// At the horizon the sun is this colour at this strength; the sky leans this way.
const GOLDEN_SUN: [f32; 3] = [1.0, 0.72, 0.50];
const GOLDEN_STRENGTH: f32 = 0.75;
const GOLDEN_SKY: [f32; 3] = [0.88, 0.86, 1.0];
/// The server checks the clock once a second and its night flag then crosses the network: for this long after
/// each boundary the clock leads even where the flag has not changed yet.
pub const GRACE: f32 = 15.0 / 3600.0;
/// A change the sky makes at once (server night, `/sky`) settles over about this long.
const EASE_SECONDS: f32 = 3.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Segment {
    Dawn,
    Day,
    Dusk,
    Night,
}

/// A light direction as the RSW gives it: degrees round the vertical and from the zenith.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SunAngles {
    pub longitude: f32,
    pub latitude: f32,
}

impl SunAngles {
    /// The world direction `load_map` makes of the RSW angles (up is -y).
    pub fn direction(&self) -> [f32; 3] {
        let (lon, lat) = (self.longitude.to_radians(), self.latitude.to_radians());
        glam::Vec3::new(-lon.cos() * lat.sin(), -lat.cos(), -lon.sin() * lat.sin())
            .normalize()
            .to_array()
    }
}

/// The sky at a moment: where the sun (or moon) is, its colour, the sky's colour, and how far into night.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sky {
    pub sun: SunAngles,
    pub sun_tint: [f32; 3],
    pub sky_tint: [f32; 3],
    pub night: f32,
}

impl Sky {
    fn plain(sun: SunAngles, night: f32) -> Self {
        Self {
            sun,
            sun_tint: [1.0; 3],
            sky_tint: [1.0; 3],
            night,
        }
    }

    /// One frame (`dt` seconds) of the way toward `target`: a sudden change settles in about 3 s, the clock's own
    /// slow changes follow at once.
    pub fn ease(&self, target: &Sky, dt: f32) -> Sky {
        let k = 1.0 - (-dt * 5.0 / EASE_SECONDS).exp();
        let mix = |a: f32, b: f32| a + (b - a) * k;
        let mix3 = |a: [f32; 3], b: [f32; 3]| std::array::from_fn(|i| mix(a[i], b[i]));
        Sky {
            sun: SunAngles {
                longitude: mix(self.sun.longitude, target.sun.longitude),
                latitude: mix(self.sun.latitude, target.sun.latitude),
            },
            sun_tint: mix3(self.sun_tint, target.sun_tint),
            sky_tint: mix3(self.sky_tint, target.sky_tint),
            night: mix(self.night, target.night),
        }
    }
}

/// Where in the hour `unix_ms` falls, 0 to 1.
pub fn phase(unix_ms: u64) -> f32 {
    (unix_ms % CYCLE_MS) as f32 / CYCLE_MS as f32
}

pub fn segment(p: f32) -> Segment {
    if p < DAWN_END {
        Segment::Dawn
    } else if p < DUSK_FROM {
        Segment::Day
    } else if p < NIGHT_FROM {
        Segment::Dusk
    } else {
        Segment::Night
    }
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A body crossing the sky over `t` (0 rising, 1 setting): swung either side of the map's direction, at the
/// map's height half way and down at the horizon at both ends.
fn arc(map: SunAngles, t: f32) -> SunAngles {
    let height = (std::f32::consts::PI * t).sin();
    SunAngles {
        longitude: map.longitude + (t - 0.5) * 2.0 * SWING_DEG,
        latitude: HORIZON_DEG + (map.latitude - HORIZON_DEG) * height,
    }
}

/// Just past a boundary, where the server's flag may still be on its way.
fn in_grace(p: f32) -> bool {
    p < GRACE || (NIGHT_FROM..NIGHT_FROM + GRACE).contains(&p)
}

/// The sky at phase `p` of the hour. `server_night`: the server says it is night (status 160); past the grace
/// after a boundary it wins over the clock both ways: night by day (a GM's `@night`), the map's own light by night
/// (`@day`, a map without night). `darkens`: this map has night (the server has made it night here); a map without
/// still gets the sun's arc and its colours, but no dimming at dusk or dawn.
pub fn sky(p: f32, server_night: bool, darkens: bool, map_sun: SunAngles) -> Sky {
    let seg = segment(p);
    let clock_night = seg == Segment::Night;
    if !darkens && clock_night && !server_night {
        return Sky::plain(map_sun, 0.0);
    }
    if server_night != clock_night && !(darkens && in_grace(p)) {
        return Sky::plain(map_sun, if server_night { 1.0 } else { 0.0 });
    }
    if seg == Segment::Night {
        // the moon crosses back the other way, so dawn starts where it sets
        let q = (p - NIGHT_FROM) / (1.0 - NIGHT_FROM);
        return Sky::plain(arc(map_sun, 1.0 - q), 1.0);
    }
    let t = p / NIGHT_FROM;
    let golden = (1.0 - (std::f32::consts::PI * t).sin()).powi(3);
    let strength = 1.0 + (GOLDEN_STRENGTH - 1.0) * golden;
    let night = match seg {
        Segment::Dawn if darkens => 1.0 - smoothstep(0.0, DAWN_END, p),
        Segment::Dusk if darkens => DUSK_DIM * smoothstep(DUSK_DIM_FROM, NIGHT_FROM, p),
        _ => 0.0,
    };
    Sky {
        sun: arc(map_sun, t),
        sun_tint: GOLDEN_SUN.map(|c| (1.0 + (c - 1.0) * golden) * strength),
        sky_tint: GOLDEN_SKY.map(|c| 1.0 + (c - 1.0) * golden),
        night,
    }
}

/// Whether this map has night, learnt from the real clock (`p`) and the server: night from the server → yes;
/// the clock's night past the grace with the server still at day → no (a map without `nightenabled`, or `@day`);
/// else as before. A new map starts at yes.
pub fn learn_darkens(darkens: bool, p: f32, server_night: bool) -> bool {
    if server_night {
        true
    } else if segment(p) == Segment::Night && !in_grace(p) {
        false
    } else {
        darkens
    }
}

/// The cycle drives the light: HD effects on, outdoors, the Day cycle row on.
pub fn active(hd: bool, indoor: bool, opts: &SceneOptions) -> bool {
    hd && !indoor && opts.day_cycle
}

/// The map's light under `sky`: the sun's direction, its colour on the sun, the sky's on the ambient. The night
/// itself is S5's (`scene_light::frame_light` with `sky.night`).
pub fn apply(base: &LightUniform, sky: &Sky) -> LightUniform {
    let mut out = *base;
    let d = sky.sun.direction();
    out.light_dir = [d[0], d[1], d[2], 0.0];
    for i in 0..3 {
        out.diffuse_color[i] *= sky.sun_tint[i];
        out.ambient_color[i] *= sky.sky_tint[i];
    }
    out
}
