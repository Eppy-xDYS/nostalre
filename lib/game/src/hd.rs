//! HD graphics settings (`custom.hd` in the config) and what `/hd` does with them.

use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HdPreset {
    Low,
    Medium,
    #[default]
    High,
    Ultra,
    Custom,
}

/// Any spelling's case; a name it does not know is High with a warning, not a config that fails to load (whose
/// next save would write the defaults over the user's file).
impl<'de> Deserialize<'de> for HdPreset {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let name = String::deserialize(deserializer)?;
        Ok(match name.to_ascii_lowercase().as_str() {
            "low" => Self::Low,
            "medium" => Self::Medium,
            "high" => Self::High,
            "ultra" => Self::Ultra,
            "custom" => Self::Custom,
            _ => {
                tracing::warn!("custom.hd.preset {name:?} unknown, using high");
                Self::High
            }
        })
    }
}

/// Off unless opted into.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HdConfig {
    /// Read at startup: the scene pipelines are built for the HD target's format.
    pub enabled: bool,
    pub preset: HdPreset,
    /// Bloom strength, 0 to 2
    pub bloom: f32,
    /// Colour grade strength, 0 to 1
    pub grade: f32,
    pub vignette: bool,
    /// World glow strengths, 0 to 1: lamp glass and flames, baked light pools on the ground, sunlit highlights
    pub lamp_glow: f32,
    pub light_pools: f32,
    pub sun_glow: f32,
    /// Grass density, 0 to 1 (0: none)
    pub grass: f32,
    /// Softer lightmaps, floor-only baked shadows and map lamps lighting the ground
    pub scene_light: bool,
    /// Drifting cloud shadows outdoors, 0 to 1 (0: none)
    pub cloud_shadows: f32,
    /// Moonlit night outdoors (off: the original's mild dimming)
    pub night_look: bool,
    /// The sun crosses the sky once an hour, dawn and dusk between day and night
    pub day_cycle: bool,
}

impl Default for HdConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            preset: HdPreset::default(),
            bloom: 0.6,
            grade: 0.8,
            vignette: true,
            lamp_glow: 0.6,
            light_pools: 0.5,
            sun_glow: 0.3,
            grass: 1.0,
            scene_light: true,
            cloud_shadows: 0.6,
            night_look: true,
            day_cycle: true,
        }
    }
}

/// `custom.hd` that does not parse (a value of the wrong type) is the defaults with a warning, so the rest of
/// the config still loads. For `#[serde(deserialize_with)]`.
pub fn deserialize_lenient<'de, D: Deserializer<'de>>(deserializer: D) -> Result<HdConfig, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Lenient {
        Parsed(HdConfig),
        Unparsed(serde::de::IgnoredAny),
    }
    Ok(match Lenient::deserialize(deserializer)? {
        Lenient::Parsed(config) => config,
        Lenient::Unparsed(_) => {
            tracing::warn!("custom.hd does not parse, using the defaults");
            HdConfig::default()
        }
    })
}

impl HdConfig {
    /// Low keeps an 8-bit scene target: half the bandwidth of a half-float one.
    pub fn wants_float_target(&self) -> bool {
        self.preset != HdPreset::Low
    }

    /// Bloom's mip count: none on Low, a shorter (narrower) chain on Medium.
    pub fn bloom_levels(&self) -> u32 {
        match self.preset {
            HdPreset::Low => 0,
            HdPreset::Medium => 3,
            HdPreset::High | HdPreset::Ultra | HdPreset::Custom => 4,
        }
    }

    pub fn bloom_intensity(&self) -> f32 {
        clamp_or_zero(self.bloom, 2.0)
    }

    pub fn grade_strength(&self) -> f32 {
        clamp_or_zero(self.grade, 1.0)
    }

    pub fn grass_density(&self) -> f32 {
        clamp_or_zero(self.grass, 1.0)
    }

    /// Grass quality, 0 (Low) to 3 (Ultra): tufts per cell and the map's cap.
    pub fn grass_tier(&self) -> u8 {
        match self.preset {
            HdPreset::Low => 0,
            HdPreset::Medium => 1,
            HdPreset::High | HdPreset::Custom => 2,
            HdPreset::Ultra => 3,
        }
    }

    pub fn cloud_strength(&self) -> f32 {
        clamp_or_zero(self.cloud_shadows, 1.0)
    }

    /// Map lamps the terrain shades with: none on Low, up to all 32 on Ultra.
    pub fn terrain_light_cap(&self) -> u32 {
        match self.preset {
            HdPreset::Low => 0,
            HdPreset::Medium => 8,
            HdPreset::High | HdPreset::Custom => 16,
            HdPreset::Ultra => 32,
        }
    }

    /// Lamp, pools, sun, each 0 to 1.
    pub fn glow_levels(&self) -> [f32; 3] {
        [self.lamp_glow, self.light_pools, self.sun_glow].map(|v| clamp_or_zero(v, 1.0))
    }
}

/// `value` within 0..=`max`; a NaN is 0
fn clamp_or_zero(value: f32, max: f32) -> f32 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, max)
    }
}

/// `/hd`: the effects flipped for this session only, for comparing; none when HD is not running. `running`: HD
/// was on at startup; `post_on`: its effects now.
pub fn session_toggle(running: bool, post_on: bool) -> Option<bool> {
    running.then_some(!post_on)
}

/// Whether going from `old` to `new` in Settings takes a restart: HD turned on while not running, or a quality
/// that moves between the 8-bit and the half-float target. Turning HD off stops its effects at once.
pub fn needs_restart(old: &HdConfig, new: &HdConfig, running: bool) -> bool {
    if !new.enabled {
        return false;
    }
    !running || old.wants_float_target() != new.wants_float_target()
}
