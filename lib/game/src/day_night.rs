pub use nostalre_effects::EFST_SKE;

const FADE_RATE_PER_SEC: f32 = 0.3;
const NIGHT_FLOOR: f32 = 0.5;
/// The HD night (moonlight) fades in and out over this long.
const NIGHT_FADE_SECONDS: f32 = 3.0;

pub struct DayNightState {
    night: bool,
    diffuse_rg: [f32; 2],
    base_diffuse: [f32; 3],
    ambient: [f32; 3],
    /// 0 day to 1 night, for the HD night; kept across maps
    amount: f32,
    dirty: bool,
}

impl Default for DayNightState {
    fn default() -> Self {
        Self {
            night: false,
            diffuse_rg: [1.0, 1.0],
            base_diffuse: [1.0, 1.0, 1.0],
            ambient: [0.3, 0.3, 0.3],
            amount: 0.0,
            dirty: false,
        }
    }
}

impl DayNightState {
    pub fn set_night(&mut self, night: bool) {
        self.night = night;
    }

    /// A new map's day light; the night (if any) carries over at once.
    pub fn on_map_loaded(&mut self, diffuse: [f32; 3], ambient: [f32; 3]) {
        self.base_diffuse = diffuse;
        self.diffuse_rg = [diffuse[0], diffuse[1]];
        self.ambient = ambient;
        self.dirty = true;
    }

    pub fn reset(&mut self) {
        *self = Self {
            dirty: true,
            ..Self::default()
        };
    }

    pub fn tick(&mut self, delta: f32) {
        let target = if self.night { 1.0 } else { 0.0 };
        if self.amount != target {
            let step = delta / NIGHT_FADE_SECONDS;
            self.amount = if self.amount < target {
                (self.amount + step).min(target)
            } else {
                (self.amount - step).max(target)
            };
            self.dirty = true;
        }
        let rate = FADE_RATE_PER_SEC * delta;
        for c in 0..2 {
            let before = self.diffuse_rg[c];
            if self.night {
                if self.base_diffuse[c] > NIGHT_FLOOR {
                    self.diffuse_rg[c] = (self.diffuse_rg[c] - rate).max(NIGHT_FLOOR);
                }
            } else {
                self.diffuse_rg[c] = (self.diffuse_rg[c] + rate).min(self.base_diffuse[c]);
            }
            if self.diffuse_rg[c] != before {
                self.dirty = true;
            }
        }
    }

    pub fn take_dirty(&mut self) -> bool {
        std::mem::take(&mut self.dirty)
    }

    /// How far into the night (0 day, 1 night), fading over 3 s.
    pub fn night_amount(&self) -> f32 {
        self.amount
    }

    pub fn world_diffuse(&self) -> [f32; 3] {
        [self.diffuse_rg[0], self.diffuse_rg[1], self.base_diffuse[2]]
    }

    pub fn sprite_light(&self) -> [f32; 3] {
        let now = self.world_diffuse();
        let mut factor = [1.0; 3];
        for c in 0..3 {
            let env_now = 1.0 - (1.0 - now[c]) * (1.0 - self.ambient[c]);
            let env_day = 1.0 - (1.0 - self.base_diffuse[c]) * (1.0 - self.ambient[c]);
            let denom = env_day + self.ambient[c];
            if denom > 1e-4 {
                factor[c] = (env_now + self.ambient[c]) / denom;
            }
        }
        factor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn night_decays_rg_recovers_and_survives_map_change() {
        let mut s = DayNightState::default();
        s.on_map_loaded([1.0, 1.0, 0.9], [0.3, 0.3, 0.3]);
        assert!(s.take_dirty());

        s.set_night(true);
        for _ in 0..200 {
            s.tick(0.1);
        }
        let d = s.world_diffuse();
        assert_eq!(d[0], NIGHT_FLOOR);
        assert_eq!(d[1], NIGHT_FLOOR);
        assert_eq!(d[2], 0.9);
        assert!(s.sprite_light()[0] < 1.0);

        s.on_map_loaded([1.0, 1.0, 0.9], [0.3, 0.3, 0.3]);
        assert_eq!(s.world_diffuse()[0], 1.0);
        for _ in 0..200 {
            s.tick(0.1);
        }
        assert_eq!(s.world_diffuse()[0], NIGHT_FLOOR);

        s.set_night(false);
        for _ in 0..200 {
            s.tick(0.1);
        }
        assert_eq!(s.world_diffuse(), [1.0, 1.0, 0.9]);
        assert_eq!(s.sprite_light(), [1.0, 1.0, 1.0]);
    }

    #[test]
    fn dark_map_never_brightens_or_oscillates() {
        let mut s = DayNightState::default();
        s.on_map_loaded([0.4, 0.5, 0.4], [0.2, 0.2, 0.2]);
        let day = s.world_diffuse();

        s.set_night(true);
        for _ in 0..50 {
            s.tick(0.1);
        }
        assert_eq!(s.world_diffuse(), day);

        s.set_night(false);
        for _ in 0..50 {
            s.tick(0.1);
        }
        assert_eq!(s.world_diffuse(), day);
    }
}
