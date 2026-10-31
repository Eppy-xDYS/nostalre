//! HD colour: a filmic shoulder that rolls light above the knee off toward white, a colour grade per map family that
//! fades between maps, and a vignette. The composite shader does the same per pixel; these are its CPU twins.

/// Below this the curve leaves light as it is: high, so daylight white keeps its punch.
pub const KNEE: f32 = 0.9;
/// Light is scaled by this before the curve on a half-float target, so white lands just under 1.
pub const EXPOSURE: f32 = 1.05;
/// Seconds a map change fades the grade over.
pub const FADE_SECONDS: f32 = 1.0;
/// How dark the vignette makes the corners.
pub const VIGNETTE: f32 = 0.35;
/// The light the curve maps to white on a half-float target; an 8-bit one is never above 1, so gets no curve.
pub const FLOAT_WHITE: f32 = 2.5;

/// The shoulder: identity to the knee, then an exponential roll-off that reaches 1 at `white`. `white` at or
/// below 1 (an 8-bit target) only clamps.
pub fn shoulder(x: f32, white: f32) -> f32 {
    if white <= 1.0 || x <= KNEE {
        return x.min(1.0);
    }
    let span = 1.0 - KNEE;
    let reach = |v: f32| 1.0 - (-(v - KNEE) / span).exp();
    (KNEE + span * reach(x) / reach(white)).min(1.0)
}

/// The curve on a colour by its brightest channel, so stacked bright effects keep their hue instead of washing out
/// to white.
pub fn tone_map(c: [f32; 3], white: f32) -> [f32; 3] {
    let peak = c[0].max(c[1]).max(c[2]);
    if peak <= 0.0 {
        return [0.0; 3];
    }
    let scale = shoulder(peak, white) / peak;
    c.map(|v| v * scale)
}

/// `1` in the middle, down to `1 - strength` toward the corners.
pub fn vignette(uv: [f32; 2], strength: f32) -> f32 {
    let (x, y) = (uv[0] * 2.0 - 1.0, uv[1] * 2.0 - 1.0);
    let r = (x * x + y * y).sqrt();
    let t = ((r - 0.5) / (1.5 - 0.5)).clamp(0.0, 1.0);
    1.0 - strength * t * t * (3.0 - 2.0 * t)
}

/// A map's mood.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Neutral,
    /// Towns: sunlit stone and wood
    Warm,
    /// Fields and forests
    Verdant,
    Desert,
    /// Glast Heim, Niflheim: cold and drained
    Cold,
    /// Swamps, Geffen's dungeon
    Sickly,
    /// Volcanoes, Thor
    Ember,
    /// Lutie, ice caves
    Arctic,
}

/// Map name prefixes, first match wins: specific places before the towns their names start with.
const RULES: &[(&str, Family)] = &[
    ("gl_", Family::Cold),
    ("nif", Family::Cold),
    ("gef_dun", Family::Sickly),
    ("mag_dun", Family::Ember),
    ("thor_", Family::Ember),
    ("xmas", Family::Arctic),
    ("ice_", Family::Arctic),
    ("ra_icecave", Family::Arctic),
    ("moc_", Family::Desert),
    ("morocc", Family::Desert),
    ("in_sphinx", Family::Desert),
    ("veins", Family::Desert),
    ("prt_fild", Family::Verdant),
    ("pay_", Family::Verdant),
    ("payon", Family::Verdant),
    ("gef_fild", Family::Verdant),
    ("um_", Family::Verdant),
    ("umbala", Family::Verdant),
    ("mjolnir", Family::Verdant),
    ("yuno_fild", Family::Verdant),
    ("prontera", Family::Warm),
    ("izlude", Family::Warm),
    ("alberta", Family::Warm),
    ("geffen", Family::Warm),
    ("aldebaran", Family::Warm),
    ("comodo", Family::Warm),
    ("yuno", Family::Warm),
    ("hugel", Family::Warm),
    ("rachel", Family::Warm),
    ("royal", Family::Warm),
];

pub fn family_for_map(name: &str) -> Family {
    let name = name.to_ascii_lowercase();
    RULES
        .iter()
        .find(|(prefix, _)| name.starts_with(prefix))
        .map_or(Family::Neutral, |(_, family)| *family)
}

/// A grade, applied in this order: white balance, lift, gamma, gain, contrast about 0.5, saturation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GradeParams {
    /// Warmer above 0 (red up, blue down), cooler below
    pub temperature: f32,
    /// Green above 0, magenta below
    pub tint: f32,
    pub saturation: f32,
    pub contrast: f32,
    pub gamma: f32,
    pub lift: [f32; 3],
    pub gain: [f32; 3],
}

impl GradeParams {
    pub const IDENTITY: Self = Self {
        temperature: 0.0,
        tint: 0.0,
        saturation: 1.0,
        contrast: 1.0,
        gamma: 1.0,
        lift: [0.0; 3],
        gain: [1.0; 3],
    };

    pub fn for_family(family: Family) -> Self {
        let grade = |temperature, tint, saturation, contrast, gamma, lift, gain| Self {
            temperature,
            tint,
            saturation,
            contrast,
            gamma,
            lift,
            gain,
        };
        match family {
            Family::Neutral => grade(0.0, 0.0, 1.08, 1.08, 1.0, [0.0; 3], [1.0; 3]),
            Family::Warm => grade(0.6, 0.05, 1.12, 1.1, 0.98, [0.012, 0.006, 0.0], [1.05, 1.0, 0.92]),
            Family::Verdant => grade(0.2, 0.35, 1.18, 1.08, 0.98, [0.0, 0.008, 0.0], [0.98, 1.03, 0.95]),
            Family::Desert => grade(0.85, 0.0, 1.0, 1.1, 0.96, [0.02, 0.01, 0.0], [1.06, 1.0, 0.87]),
            Family::Cold => grade(-0.6, 0.0, 0.75, 1.14, 1.05, [0.0, 0.006, 0.02], [0.92, 0.98, 1.06]),
            Family::Sickly => grade(0.1, 0.55, 0.85, 1.1, 1.04, [0.0, 0.015, 0.0], [0.95, 1.03, 0.88]),
            Family::Ember => grade(0.9, -0.1, 1.08, 1.12, 0.95, [0.025, 0.0, 0.0], [1.08, 0.96, 0.85]),
            Family::Arctic => grade(-0.5, -0.05, 0.9, 1.06, 0.98, [0.012, 0.018, 0.03], [0.96, 1.0, 1.08]),
        }
    }

    pub fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        let mix = |x: f32, y: f32| x + (y - x) * t;
        let mix3 = |x: [f32; 3], y: [f32; 3]| [mix(x[0], y[0]), mix(x[1], y[1]), mix(x[2], y[2])];
        Self {
            temperature: mix(a.temperature, b.temperature),
            tint: mix(a.tint, b.tint),
            saturation: mix(a.saturation, b.saturation),
            contrast: mix(a.contrast, b.contrast),
            gamma: mix(a.gamma, b.gamma),
            lift: mix3(a.lift, b.lift),
            gain: mix3(a.gain, b.gain),
        }
    }

    /// The grade at `strength` (0: none, 1: full).
    pub fn scaled(&self, strength: f32) -> Self {
        if strength <= 0.0 {
            return Self::IDENTITY;
        }
        Self::lerp(&Self::IDENTITY, self, strength.min(1.0))
    }

    /// What the composite shader does with this grade.
    pub fn apply(&self, c: [f32; 3]) -> [f32; 3] {
        let balance = [
            1.0 + 0.08 * self.temperature,
            1.0 + 0.06 * self.tint,
            1.0 - 0.08 * self.temperature,
        ];
        let mut out = [0.0; 3];
        for i in 0..3 {
            let v = c[i] * balance[i];
            let v = v + self.lift[i] * (1.0 - v);
            let v = v.max(0.0).powf(1.0 / self.gamma) * self.gain[i];
            out[i] = (v - 0.5) * self.contrast + 0.5;
        }
        let l = 0.2126 * out[0] + 0.7152 * out[1] + 0.0722 * out[2];
        out.map(|v| (l + (v - l) * self.saturation).max(0.0))
    }
}

/// The grade on screen, fading toward the one a map change asked for.
#[derive(Debug, Clone, Copy)]
pub struct GradeFade {
    from: GradeParams,
    to: GradeParams,
    elapsed: f32,
}

impl GradeFade {
    pub fn new(params: GradeParams) -> Self {
        Self {
            from: params,
            to: params,
            elapsed: FADE_SECONDS,
        }
    }

    /// Fades to `to`, from what is on screen now (a change mid-fade does not jump).
    pub fn start(&mut self, to: GradeParams) {
        self.from = self.current();
        self.to = to;
        self.elapsed = 0.0;
    }

    pub fn advance(&mut self, seconds: f32) {
        self.elapsed = (self.elapsed + seconds.max(0.0)).min(FADE_SECONDS);
    }

    pub fn fading(&self) -> bool {
        self.elapsed < FADE_SECONDS && self.from != self.to
    }

    pub fn current(&self) -> GradeParams {
        if self.elapsed >= FADE_SECONDS {
            return self.to;
        }
        GradeParams::lerp(&self.from, &self.to, self.elapsed / FADE_SECONDS)
    }
}

/// What the composite pass is told each frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompositeUniform {
    pub bloom_intensity: f32,
    /// The curve's white point; 0: no curve
    pub curve_white: f32,
    /// Scale before the curve
    pub exposure: f32,
    pub vignette: f32,
    pub grade: GradeParams,
}

/// The composite's settings this frame; with the effects off (`/hd`), the scene as drawn.
pub fn composite_uniform(
    effects: bool,
    bloom_intensity: f32,
    grade: &GradeParams,
    vignette: f32,
    white: f32,
) -> CompositeUniform {
    if !effects {
        return CompositeUniform {
            bloom_intensity: 0.0,
            curve_white: 0.0,
            exposure: 1.0,
            vignette: 0.0,
            grade: GradeParams::IDENTITY,
        };
    }
    CompositeUniform {
        bloom_intensity,
        curve_white: white,
        // an 8-bit target has no room above white to expose into
        exposure: if white > 1.0 { EXPOSURE } else { 1.0 },
        vignette,
        grade: *grade,
    }
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct CompositeGpu {
    /// bloom intensity, curve white, vignette, exposure
    values: [f32; 4],
    /// temperature, tint, saturation, contrast
    balance: [f32; 4],
    /// lift rgb, gamma
    lift_gamma: [f32; 4],
    /// gain rgb, -
    gain: [f32; 4],
}

impl CompositeUniform {
    pub(super) fn to_gpu(self) -> CompositeGpu {
        let g = self.grade;
        CompositeGpu {
            values: [
                self.bloom_intensity,
                self.curve_white,
                self.vignette,
                self.exposure,
            ],
            balance: [g.temperature, g.tint, g.saturation, g.contrast],
            lift_gamma: [g.lift[0], g.lift[1], g.lift[2], g.gamma],
            gain: [g.gain[0], g.gain[1], g.gain[2], 0.0],
        }
    }
}
