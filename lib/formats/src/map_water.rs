//! A map's water: from the RSW when it has a level (RSW < 2.6), else from the GND (1.8+), which may split it
//! into zones with their own level and type.

use crate::gnd::{GndFile, GndWaterZone};
use crate::rsw::WaterSettings;

#[derive(Clone, Debug, PartialEq)]
pub struct MapWater {
    /// One zone covers the whole map; more form a `split` grid, row by row, x fastest.
    pub zones: Vec<GndWaterZone>,
    pub split: (i32, i32),
}

pub fn map_water(rsw: &WaterSettings, gnd: &GndFile) -> Option<MapWater> {
    if let Some(level) = rsw.level {
        // RSW defaults for fields older versions leave out
        let zone = GndWaterZone {
            level,
            water_type: rsw.water_type.unwrap_or(0),
            wave_height: rsw.wave_height.unwrap_or(1.0),
            wave_speed: rsw.wave_speed.unwrap_or(2.0),
            wave_pitch: rsw.wave_pitch.unwrap_or(50.0),
            anim_speed: rsw.anim_speed.unwrap_or(3) as i32,
        };
        return Some(MapWater {
            zones: vec![zone],
            split: (1, 1),
        });
    }
    let water = gnd.water.as_ref()?;
    let split = (water.split_width, water.split_height);
    let zoned = !water.zones.is_empty() && split.0 > 0 && split.1 > 0;
    Some(if zoned {
        MapWater {
            zones: water.zones.clone(),
            split,
        }
    } else {
        MapWater {
            zones: vec![water.main],
            split: (1, 1),
        }
    })
}

impl MapWater {
    /// The zone over GND cell (x, y).
    pub fn zone_at(&self, gnd: &GndFile, x: i32, y: i32) -> &GndWaterZone {
        let (sw, sh) = self.split;
        let zx = (x * sw / gnd.width.max(1)).clamp(0, sw - 1);
        let zy = (y * sh / gnd.height.max(1)).clamp(0, sh - 1);
        &self.zones[(zy * sw + zx) as usize]
    }

    /// The water types present, ascending, each once.
    pub fn types(&self) -> Vec<i32> {
        let mut types: Vec<i32> = self.zones.iter().map(|z| z.water_type).collect();
        types.sort_unstable();
        types.dedup();
        types
    }
}
