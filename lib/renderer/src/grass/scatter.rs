//! Where tufts go: what each cell grows is worked out once per map (`GrassField`), and a chunk's tufts are made
//! whenever the camera comes near it, from a hash of the map name and the cell, so they are the same every time.

use nostalre_formats::gnd::GndFile;
use nostalre_formats::map_water::MapWater;

use super::Tuft;
use super::chunks::CHUNK_CELLS;
use super::classify::{self, Ground, TextureInfo};

/// Grass quality from the HD preset: tufts per cell, the visible budget, blades per tuft.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Low,
    Medium,
    High,
    Ultra,
}

impl Tier {
    /// 0 Low … 3 Ultra; above is Ultra.
    pub fn from_index(index: u8) -> Self {
        match index {
            0 => Self::Low,
            1 => Self::Medium,
            2 => Self::High,
            _ => Self::Ultra,
        }
    }

    pub fn per_cell(self) -> f32 {
        match self {
            Self::Low => 4.0,
            Self::Medium => 6.0,
            Self::High => 8.0,
            Self::Ultra => 10.0,
        }
    }

    /// Tufts drawn per frame at most: the nearest chunks fill it first.
    pub fn budget(self) -> u32 {
        match self {
            Self::Low => 20_000,
            Self::Medium => 40_000,
            Self::High => 80_000,
            Self::Ultra => 140_000,
        }
    }
}

/// The tallest tuft stands this far above its root: about a character's hip. Each tuft is one
/// whole clump of blades, so a few per cell cover the ground.
pub const TUFT_MAX_HEIGHT: f32 = 9.5;
const SIZE_MIN: f32 = 0.85;
const SIZE_MAX: f32 = 1.6;
/// Sparse ground (hills, dirt) gets this share of a grass cell's tufts.
const SPARSE_SHARE: f32 = 0.15;
/// The tint is the ground's colour pulled 30 % toward its own grey, then green raised, red and blue lowered.
const GREY_PULL: f32 = 0.3;
const GREEN_SHIFT: [f32; 3] = [0.94, 1.1, 0.88];

pub struct FieldInput<'a> {
    pub gnd: &'a GndFile,
    /// Seeds the layout
    pub map: &'a str,
    pub textures: &'a [TextureInfo],
    /// The map's water (each cell's zone level), none without water
    pub water: Option<&'a MapWater>,
    /// Cells under a model floor (`classify::floor_cells`), none when empty
    pub floors: &'a [bool],
}

#[derive(Clone, Copy)]
struct FieldCell {
    /// sw, se, nw, ne
    heights: [f32; 4],
    /// Share of a full cell's tufts: 0 grows none
    share: f32,
    rgb: [f32; 3],
}

/// What grows where on a map.
pub struct GrassField {
    width: i32,
    height: i32,
    zoom: f32,
    seed: u64,
    cells: Vec<FieldCell>,
    /// Per chunk, whether any of its cells grows grass
    chunk_grows: Vec<bool>,
}

impl GrassField {
    pub fn new(input: &FieldInput) -> Self {
        let gnd = input.gnd;
        let (cw, ch) = chunk_grid_of(gnd.width, gnd.height);
        let mut chunk_grows = vec![false; (cw * ch) as usize];
        let mut cells = Vec::with_capacity(gnd.cells.len());
        for y in 0..gnd.height {
            for x in 0..gnd.width {
                let i = (y * gnd.width + x) as usize;
                let cell = &gnd.cells[i];
                let heights = [cell.height_sw, cell.height_se, cell.height_nw, cell.height_ne];
                let grows = if input.floors.get(i) == Some(&true) {
                    None
                } else {
                    classify::cell_grows(
                        gnd,
                        x,
                        y,
                        input.textures,
                        input.water.map(|w| w.zone_at(gnd, x, y).level),
                    )
                };
                let (share, rgb) = match grows {
                    Some(ground) => {
                        let surface = &gnd.surfaces[cell.surface_up as usize];
                        let rgb = input.textures[surface.texture_id as usize].rgb;
                        let base = if ground == Ground::Sparse { SPARSE_SHARE } else { 1.0 };
                        let (low, high) = heights
                            .iter()
                            .fold((f32::MAX, f32::MIN), |(lo, hi), &h| (lo.min(h), hi.max(h)));
                        (base * classify::slope_share(high - low), rgb)
                    }
                    None => (0.0, [0.0; 3]),
                };
                if share > 0.0 {
                    chunk_grows[((y / CHUNK_CELLS) * cw + x / CHUNK_CELLS) as usize] = true;
                }
                cells.push(FieldCell { heights, share, rgb });
            }
        }
        Self {
            width: gnd.width,
            height: gnd.height,
            zoom: gnd.zoom,
            seed: map_seed(input.map),
            cells,
            chunk_grows,
        }
    }

    /// Chunks across and down.
    pub fn chunk_grid(&self) -> (i32, i32) {
        chunk_grid_of(self.width, self.height)
    }

    pub fn cell_size(&self) -> f32 {
        self.zoom
    }

    pub fn is_empty(&self) -> bool {
        !self.chunk_grows.contains(&true)
    }

    pub fn chunk_grows(&self, cx: i32, cz: i32) -> bool {
        let (cw, ch) = self.chunk_grid();
        (0..cw).contains(&cx) && (0..ch).contains(&cz) && self.chunk_grows[(cz * cw + cx) as usize]
    }

    /// The lowest and highest ground height in chunk (`cx`, `cz`).
    pub fn chunk_height_range(&self, cx: i32, cz: i32) -> (f32, f32) {
        let mut range = (f32::MAX, f32::MIN);
        for y in cz * CHUNK_CELLS..((cz + 1) * CHUNK_CELLS).min(self.height) {
            for x in cx * CHUNK_CELLS..((cx + 1) * CHUNK_CELLS).min(self.width) {
                for h in self.cells[(y * self.width + x) as usize].heights {
                    range = (range.0.min(h), range.1.max(h));
                }
            }
        }
        range
    }

    /// The tufts of chunk (`cx`, `cz`) at `density` (0 to 1): a lower density keeps a subset of the same tufts.
    pub fn chunk_tufts(&self, cx: i32, cz: i32, tier: Tier, density: f32) -> Vec<Tuft> {
        let density = density.clamp(0.0, 1.0);
        let mut tufts = Vec::new();
        if density <= 0.0 || !self.chunk_grows(cx, cz) {
            return tufts;
        }
        for y in cz * CHUNK_CELLS..((cz + 1) * CHUNK_CELLS).min(self.height) {
            for x in cx * CHUNK_CELLS..((cx + 1) * CHUNK_CELLS).min(self.width) {
                let cell = self.cells[(y * self.width + x) as usize];
                if cell.share > 0.0 {
                    self.cell_tufts(x, y, cell, tier.per_cell() * cell.share, density, &mut tufts);
                }
            }
        }
        tufts
    }

    fn cell_tufts(&self, x: i32, y: i32, cell: FieldCell, count: f32, density: f32, out: &mut Vec<Tuft>) {
        let whole = count.floor() as u32;
        let extra = (unit(hash(self.seed, x, y, u32::MAX)) < count.fract()) as u32;
        for i in 0..whole + extra {
            let h = hash(self.seed, x, y, i);
            // the slider keeps the tufts ranked under it: the same ones at every density
            if unit(mix(h, 5)) >= density {
                continue;
            }
            let (u, v) = (unit(mix(h, 1)), unit(mix(h, 2)));
            let [sw, se, nw, ne] = cell.heights;
            let south = sw + (se - sw) * u;
            let north = nw + (ne - nw) * u;
            out.push(Tuft {
                root: [
                    (x as f32 + u) * self.zoom,
                    south + (north - south) * v,
                    (y as f32 + v) * self.zoom,
                ],
                packed: pack(mix(h, 3), cell.rgb),
            });
        }
    }
}

fn chunk_grid_of(width: i32, height: i32) -> (i32, i32) {
    (
        (width + CHUNK_CELLS - 1) / CHUNK_CELLS,
        (height + CHUNK_CELLS - 1) / CHUNK_CELLS,
    )
}

/// Bits 0-15 tint (RGB 565), 16-21 yaw, 22-26 size, 27-28 blade variant, 29-31 lean.
fn pack(h: u64, ground: [f32; 3]) -> u32 {
    let grey = ground[0] * 0.299 + ground[1] * 0.587 + ground[2] * 0.114;
    let shade = 0.82 + unit(mix(h, 4)) * 0.36;
    let tint: [f32; 3] = std::array::from_fn(|i| {
        ((ground[i] + (grey - ground[i]) * GREY_PULL) * GREEN_SHIFT[i] * shade).clamp(0.0, 1.0)
    });
    let r = (tint[0] * 31.0).round() as u32;
    let g = (tint[1] * 63.0).round() as u32;
    let b = (tint[2] * 31.0).round() as u32;
    let bits = (h >> 32) as u32;
    (r << 11 | g << 5 | b) | (bits & 0x3f) << 16 | (bits >> 6 & 0x1f) << 22 | (bits >> 11 & 0x3) << 27 | (bits >> 13 & 0x7) << 29
}

impl Tuft {
    /// 0.85 to 1.6
    pub fn size(&self) -> f32 {
        SIZE_MIN + (self.packed >> 22 & 0x1f) as f32 / 31.0 * (SIZE_MAX - SIZE_MIN)
    }

    /// In world units
    pub fn height(&self) -> f32 {
        self.size() / SIZE_MAX * TUFT_MAX_HEIGHT
    }
}

fn map_seed(map: &str) -> u64 {
    // FNV-1a
    map.bytes()
        .fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3))
}

fn hash(seed: u64, x: i32, y: i32, i: u32) -> u64 {
    mix(seed ^ (x as u32 as u64) << 40 ^ (y as u32 as u64) << 20, i as u64 + 1)
}

/// splitmix64 of `h` and a salt
fn mix(h: u64, salt: u64) -> u64 {
    let mut z = h.wrapping_add(salt.wrapping_mul(0x9e37_79b9_7f4a_7c15));
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// 0 to 1 from a hash
fn unit(h: u64) -> f32 {
    (h >> 40) as f32 / (1u64 << 24) as f32
}
