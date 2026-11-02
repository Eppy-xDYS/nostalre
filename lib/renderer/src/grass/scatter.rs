//! Where tufts go: what each cell grows is worked out once per map (`GrassField`), and a chunk's tufts are made
//! whenever the camera comes near it, from a hash of the map name and the cell, so they are the same every time.

use nostalre_formats::gnd::GndFile;
use nostalre_formats::map_water::MapWater;

use super::Tuft;
use super::chunks::CHUNK_CELLS;
use super::classify::{self, Flora, Ground, KINDS, Kind, TextureInfo};

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
/// This share of a cell's tufts take another kind: a nearby ground's (so borders blend), else any other.
const MIX_SHARE: f32 = 0.2;
/// How far, in cells, a border's other kind reaches into a cell's tufts.
const BLEND_CELLS: i32 = 2;
/// Flora per grass tuft at the default Flora setting, by ground kind (dry, tall, bush): none on hills and dirt.
const FLORA_RATE: [f32; KINDS as usize] = [0.0, 1.0 / 120.0, 1.0 / 40.0];
/// The Flora setting the rates above are for
pub const FLORA_DEFAULT: f32 = 0.5;
/// Flora patches: a noise lattice this many cells apart gates where flora grows
const PATCH_CELLS: f32 = 9.0;
/// The noise from which flora starts, and where it is at full strength
const PATCH_EDGE: (f32, f32) = (0.6, 0.85);
/// Which flora, by its draw (0 to 1) against these cumulative shares: fern, nest fern, brown mushroom, red
/// mushroom, then flower
const FLORA_MIX_FOREST: [f32; 4] = [0.36, 0.52, 0.72, 0.85];
const FLORA_MIX_MEADOW: [f32; 4] = [0.2, 0.25, 0.32, 0.4];
/// Leafy plants per grass tuft at the default setting, by ground kind: commoner than the rest of the flora
const LEAFY_RATE: [f32; KINDS as usize] = [0.0, 1.0 / 40.0, 1.0 / 15.0];
/// Leafy plants follow the patches only this much: the rest grow anywhere
const LEAFY_PATCH: f32 = 0.5;
/// The tint is the ground's colour pulled 30 % toward its own grey, then green raised, red and blue lowered.
const GREY_PULL: f32 = 0.3;
const GREEN_SHIFT: [f32; 3] = [0.94, 1.1, 0.88];
/// The dry kind's tint is pulled this far toward straw (of the ground's brightness).
const STRAW_PULL: f32 = 0.4;
const STRAW: [f32; 3] = [1.16, 1.03, 0.65];

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
    /// `classify::Kind` of its ground
    kind: u8,
    /// The commonest other kind growing within `BLEND_CELLS`, `NO_KIND` when none
    near_kind: u8,
    /// Flora per grass tuft at the default setting: its kind's rate in a patch, 0 outside one
    flora: f32,
    /// Leafy plants per grass tuft at the default setting
    leafy: f32,
}

const NO_KIND: u8 = u8::MAX;

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
                let kind = grows.and_then(Ground::kind).map_or(NO_KIND, |k| k as u8);
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
                cells.push(FieldCell { heights, share, rgb, kind, near_kind: NO_KIND, flora: 0.0, leafy: 0.0 });
            }
        }
        set_near_kinds(&mut cells, gnd.width, gnd.height);
        let seed = map_seed(input.map);
        set_flora(&mut cells, gnd.width, seed);
        Self {
            width: gnd.width,
            height: gnd.height,
            zoom: gnd.zoom,
            seed,
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
    /// `flora` (0 to 1, `FLORA_DEFAULT` the usual) adds ferns, mushrooms and flowers; the grass stays the same.
    pub fn chunk_tufts(&self, cx: i32, cz: i32, tier: Tier, density: f32, flora: f32) -> Vec<Tuft> {
        let flora = flora.clamp(0.0, 1.0) / FLORA_DEFAULT;
        let density = density.clamp(0.0, 1.0);
        let mut tufts = Vec::new();
        if density <= 0.0 || !self.chunk_grows(cx, cz) {
            return tufts;
        }
        for y in cz * CHUNK_CELLS..((cz + 1) * CHUNK_CELLS).min(self.height) {
            for x in cx * CHUNK_CELLS..((cx + 1) * CHUNK_CELLS).min(self.width) {
                let cell = self.cells[(y * self.width + x) as usize];
                if cell.share > 0.0 {
                    let count = tier.per_cell() * cell.share;
                    self.cell_tufts(x, y, cell, count, density, &mut tufts);
                    self.cell_flora(x, y, cell, count * cell.flora * flora, density, &mut tufts);
                    self.cell_leafy(x, y, cell, count * cell.leafy * flora, density, &mut tufts);
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
            out.push(Tuft {
                root: self.root(x, y, cell, h),
                packed: pack(mix(h, 3), cell.rgb, tuft_kind(mix(h, 6), cell)),
            });
        }
    }

    /// `count` flora tufts (a fraction: that chance of one more), on hashes apart from the grass's.
    fn cell_flora(&self, x: i32, y: i32, cell: FieldCell, count: f32, density: f32, out: &mut Vec<Tuft>) {
        if count <= 0.0 {
            return;
        }
        let whole = count.floor() as u32;
        let extra = (unit(hash(self.seed ^ FLORA_SALT, x, y, u32::MAX)) < count.fract()) as u32;
        for i in 0..whole + extra {
            let h = hash(self.seed ^ FLORA_SALT, x, y, i);
            if unit(mix(h, 5)) >= density {
                continue;
            }
            let mix_by = if cell.kind == Kind::Bush as u8 { FLORA_MIX_FOREST } else { FLORA_MIX_MEADOW };
            let draw = unit(mix(h, 8));
            let flora = if draw < mix_by[0] {
                Flora::Fern
            } else if draw < mix_by[1] {
                Flora::NestFern
            } else if draw < mix_by[2] {
                Flora::BrownMushroom
            } else if draw < mix_by[3] {
                Flora::RedMushroom
            } else {
                Flora::Flower
            };
            out.push(Tuft {
                root: self.root(x, y, cell, h),
                packed: pack(mix(h, 3), cell.rgb, flora.kind()),
            });
        }
    }

    /// `count` leafy plants, on hashes apart from the grass's and the other flora's.
    fn cell_leafy(&self, x: i32, y: i32, cell: FieldCell, count: f32, density: f32, out: &mut Vec<Tuft>) {
        if count <= 0.0 {
            return;
        }
        let whole = count.floor() as u32;
        let extra = (unit(hash(self.seed ^ LEAFY_SALT, x, y, u32::MAX)) < count.fract()) as u32;
        for i in 0..whole + extra {
            let h = hash(self.seed ^ LEAFY_SALT, x, y, i);
            if unit(mix(h, 5)) >= density {
                continue;
            }
            out.push(Tuft {
                root: self.root(x, y, cell, h),
                packed: pack(mix(h, 3), cell.rgb, Flora::Leafy.kind()),
            });
        }
    }

    /// A spot in cell (`x`, `y`) on its ground, from `h`.
    fn root(&self, x: i32, y: i32, cell: FieldCell, h: u64) -> [f32; 3] {
        let (u, v) = (unit(mix(h, 1)), unit(mix(h, 2)));
        let [sw, se, nw, ne] = cell.heights;
        let south = sw + (se - sw) * u;
        let north = nw + (ne - nw) * u;
        [(x as f32 + u) * self.zoom, south + (north - south) * v, (y as f32 + v) * self.zoom]
    }
}

const FLORA_SALT: u64 = 0x666c_6f72_6121;
const LEAFY_SALT: u64 = 0x6c65_6166_7921;

/// Each growing cell's flora rate: its kind's, scaled by a patch noise whose mean over the field's flora ground
/// is 1, so the patches move flora around without changing how much there is.
fn set_flora(cells: &mut [FieldCell], width: i32, seed: u64) {
    let mut gates = Vec::with_capacity(cells.len());
    let (mut sum, mut n) = (0.0f32, 0u32);
    for (i, cell) in cells.iter().enumerate() {
        let rate = if cell.share > 0.0 { FLORA_RATE.get(cell.kind as usize).copied().unwrap_or(0.0) } else { 0.0 };
        let (x, y) = (i as i32 % width, i as i32 / width);
        let gate = if rate > 0.0 { patch(seed, x as f32 + 0.5, y as f32 + 0.5) } else { 0.0 };
        if rate > 0.0 {
            sum += gate;
            n += 1;
        }
        gates.push((rate, gate));
    }
    let mean = if n > 0 { sum / n as f32 } else { 0.0 };
    for (cell, (rate, gate)) in cells.iter_mut().zip(gates) {
        let patch = if mean > 0.0 { gate / mean } else { 0.0 };
        cell.flora = rate * patch;
        let leafy = if cell.share > 0.0 { LEAFY_RATE.get(cell.kind as usize).copied().unwrap_or(0.0) } else { 0.0 };
        cell.leafy = leafy * (1.0 - LEAFY_PATCH + LEAFY_PATCH * patch);
    }
}

/// 0 outside a flora patch, up to 1 inside: smoothed value noise on a lattice `PATCH_CELLS` apart.
fn patch(seed: u64, x: f32, y: f32) -> f32 {
    let (gx, gy) = (x / PATCH_CELLS, y / PATCH_CELLS);
    let (ix, iy) = (gx.floor() as i32, gy.floor() as i32);
    let s = |t: f32| t * t * (3.0 - 2.0 * t);
    let (fx, fy) = (s(gx - ix as f32), s(gy - iy as f32));
    let at = |dx: i32, dy: i32| unit(hash(seed ^ FLORA_SALT, ix + dx, iy + dy, 0x7a7c));
    let top = at(0, 0) + (at(1, 0) - at(0, 0)) * fx;
    let bottom = at(0, 1) + (at(1, 1) - at(0, 1)) * fx;
    let n = top + (bottom - top) * fy;
    let t = ((n - PATCH_EDGE.0) / (PATCH_EDGE.1 - PATCH_EDGE.0)).clamp(0.0, 1.0);
    s(t)
}

fn set_near_kinds(cells: &mut [FieldCell], width: i32, height: i32) {
    for y in 0..height {
        for x in 0..width {
            let own = cells[(y * width + x) as usize];
            if own.share <= 0.0 {
                continue;
            }
            let mut counts = [0u32; KINDS as usize];
            for ny in (y - BLEND_CELLS).max(0)..=(y + BLEND_CELLS).min(height - 1) {
                for nx in (x - BLEND_CELLS).max(0)..=(x + BLEND_CELLS).min(width - 1) {
                    let n = cells[(ny * width + nx) as usize];
                    if n.share > 0.0 && n.kind != own.kind {
                        counts[n.kind as usize] += 1;
                    }
                }
            }
            let (best, &count) = counts.iter().enumerate().max_by_key(|&(_, c)| *c).unwrap_or((0, &0));
            if count > 0 {
                cells[(y * width + x) as usize].near_kind = best as u8;
            }
        }
    }
}

/// Its ground's kind, or for `MIX_SHARE` of the tufts a nearby ground's kind, else another one.
fn tuft_kind(h: u64, cell: FieldCell) -> u32 {
    let own = cell.kind as u32 % KINDS;
    if unit(h) >= MIX_SHARE {
        own
    } else if cell.near_kind != NO_KIND {
        cell.near_kind as u32
    } else {
        (own + 1 + (h & 1) as u32) % KINDS
    }
}

fn chunk_grid_of(width: i32, height: i32) -> (i32, i32) {
    (
        (width + CHUNK_CELLS - 1) / CHUNK_CELLS,
        (height + CHUNK_CELLS - 1) / CHUNK_CELLS,
    )
}

/// Bits 0-15 tint (RGB 565), 16-20 yaw, 21-25 size, 26-29 kind (`classify::Kind`, then `classify::Flora` from
/// `KINDS` on), 30-31 lean.
fn pack(h: u64, ground: [f32; 3], kind: u32) -> u32 {
    let grey = ground[0] * 0.299 + ground[1] * 0.587 + ground[2] * 0.114;
    let shade = 0.82 + unit(mix(h, 4)) * 0.36;
    let straw = if kind == classify::Kind::Dry as u32 { STRAW_PULL } else { 0.0 };
    let tint: [f32; 3] = std::array::from_fn(|i| {
        let green = (ground[i] + (grey - ground[i]) * GREY_PULL) * GREEN_SHIFT[i];
        ((green + (STRAW[i] * grey - green) * straw) * shade).clamp(0.0, 1.0)
    });
    let r = (tint[0] * 31.0).round() as u32;
    let g = (tint[1] * 63.0).round() as u32;
    let b = (tint[2] * 31.0).round() as u32;
    let bits = (h >> 32) as u32;
    (r << 11 | g << 5 | b) | (bits & 0x1f) << 16 | (bits >> 6 & 0x1f) << 21 | (kind & 0xf) << 26 | (bits >> 13 & 0x3) << 30
}

impl Tuft {
    /// 0.85 to 1.6
    pub fn size(&self) -> f32 {
        SIZE_MIN + (self.packed >> 21 & 0x1f) as f32 / 31.0 * (SIZE_MAX - SIZE_MIN)
    }

    /// `classify::Kind` as its number, or `classify::Flora::kind`
    pub fn kind(&self) -> u32 {
        self.packed >> 26 & 0xf
    }

    pub fn is_flora(&self) -> bool {
        self.kind() >= KINDS
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
