//! Which ground grows grass: a GND texture by its file name (Korean map makers named them by what they show), the
//! texture's colour when the name says nothing, then per cell its slope, the water and how dark it is.

use nostalre_formats::gnd::GndFile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ground {
    Grass,
    /// A few tufts: hills, dirt
    Sparse,
    Never,
}

/// A GND texture's class and its average colour (0 to 1), the tufts' tint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextureInfo {
    pub class: Ground,
    pub rgb: [f32; 3],
}

/// meadow, grass floor, forest floor, lawn
const GRASS_WORDS: [&str; 5] = ["초원", "풀바닥", "숲속바닥", "잔디", "grass"];
/// hill, dirt
const SPARSE_WORDS: [&str; 2] = ["언덕", "흙"];
/// cliff, water bed, rock, town, bare earth, and the void fillers
const NEVER_WORDS: [&str; 8] = ["절벽", "물바닥", "바위", "도시", "맨땅", "wall", "backside", "black"];

/// Corners further apart than this (one cell is 10 wide) are a slope too steep for grass.
pub const MAX_HEIGHT_SPREAD: f32 = 10.0;
/// Below this spread a cell grows at full density; above it thinner, to `STEEP_SHARE` at `MAX_HEIGHT_SPREAD`.
/// (2.5 was the limit once: a third of grassy cells on hilly fields grew nothing.)
pub const FULL_DENSITY_SPREAD: f32 = 2.5;
const STEEP_SHARE: f32 = 0.4;

/// The share of tufts a cell whose corner heights spread this much grows.
pub fn slope_share(spread: f32) -> f32 {
    let t = ((spread - FULL_DENSITY_SPREAD) / (MAX_HEIGHT_SPREAD - FULL_DENSITY_SPREAD)).clamp(0.0, 1.0);
    1.0 + (STEEP_SHARE - 1.0) * t
}
/// Ground darker than this is a filler or a shadowed pit.
const MIN_LUMA: f32 = 0.1;

pub fn classify_texture(path: &str, avg_rgb: [f32; 3]) -> Ground {
    let file = path.rsplit(['\\', '/']).next().unwrap_or(path).to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| file.contains(w));
    if has(&NEVER_WORDS) {
        Ground::Never
    } else if has(&GRASS_WORDS) {
        Ground::Grass
    } else if has(&SPARSE_WORDS) {
        Ground::Sparse
    } else if looks_green(avg_rgb) {
        Ground::Grass
    } else {
        Ground::Never
    }
}

fn looks_green([r, g, b]: [f32; 3]) -> bool {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let saturation = if max > 0.0 { (max - min) / max } else { 0.0 };
    g >= 1.1 * r.max(b) && saturation >= 0.2
}

pub fn luma([r, g, b]: [f32; 3]) -> f32 {
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// A model face this close above the ground (or just under it) is a floor grass must not poke through: a tuft's
/// height plus a little.
pub const FLOOR_REACH: f32 = 6.5;

/// The cells under a model floor: near-horizontal faces of `triangles` (world space) at most `FLOOR_REACH` above
/// the ground. Indexed like `gnd.cells`.
pub fn floor_cells(gnd: &GndFile, triangles: impl IntoIterator<Item = [[f32; 3]; 3]>) -> Vec<bool> {
    use glam::{Vec2, Vec3};
    let mut floors = vec![false; gnd.cells.len()];
    let zoom = gnd.zoom;
    for tri in triangles {
        let [a, b, c] = tri.map(Vec3::from_array);
        let n = (b - a).cross(c - a).normalize_or_zero();
        if n.y.abs() < 0.85 {
            continue;
        }
        let face_y = (a.y + b.y + c.y) / 3.0;
        let [pa, pb, pc] = [a, b, c].map(|v| Vec2::new(v.x, v.z));
        let lo = pa.min(pb).min(pc);
        let hi = pa.max(pb).max(pc);
        let centroid = (pa + pb + pc) / 3.0;
        let cell_of = |v: f32, limit: i32| ((v / zoom).floor() as i32).clamp(0, limit - 1);
        for y in cell_of(lo.y, gnd.height)..=cell_of(hi.y, gnd.height) {
            for x in cell_of(lo.x, gnd.width)..=cell_of(hi.x, gnd.width) {
                // a floor over any part of the cell: its centre or a corner (pulled in a little) under the face
                let holds_centroid = (centroid / zoom).floor() == Vec2::new(x as f32, y as f32);
                let covers = [(0.5, 0.5), (0.1, 0.1), (0.9, 0.1), (0.1, 0.9), (0.9, 0.9)]
                    .iter()
                    .any(|&(u, v)| inside(Vec2::new((x as f32 + u) * zoom, (y as f32 + v) * zoom), pa, pb, pc));
                if !holds_centroid && !covers {
                    continue;
                }
                let i = (y * gnd.width + x) as usize;
                let cell = &gnd.cells[i];
                let ground = (cell.height_sw + cell.height_se + cell.height_nw + cell.height_ne) / 4.0;
                // heights grow downward: a floor above the ground has a smaller y
                let above = ground - face_y;
                if (-0.5..=FLOOR_REACH).contains(&above) {
                    floors[i] = true;
                }
            }
        }
    }
    floors
}

fn inside(p: glam::Vec2, a: glam::Vec2, b: glam::Vec2, c: glam::Vec2) -> bool {
    let side = |u: glam::Vec2, v: glam::Vec2| (v - u).perp_dot(p - u);
    let (d1, d2, d3) = (side(a, b), side(b, c), side(c, a));
    !((d1 < 0.0 || d2 < 0.0 || d3 < 0.0) && (d1 > 0.0 || d2 > 0.0 || d3 > 0.0))
}

/// What cell (`x`, `y`) grows, none when it has no top, is steep, under `water` (heights grow downward) or dark.
pub fn cell_grows(
    gnd: &GndFile,
    x: i32,
    y: i32,
    textures: &[TextureInfo],
    water: Option<f32>,
) -> Option<Ground> {
    let cell = gnd.cells.get((y * gnd.width + x) as usize)?;
    let surface = gnd.surfaces.get(usize::try_from(cell.surface_up).ok()?)?;
    let info = textures.get(usize::try_from(surface.texture_id).ok()?)?;
    if info.class == Ground::Never || luma(info.rgb) < MIN_LUMA {
        return None;
    }
    let heights = [cell.height_sw, cell.height_se, cell.height_nw, cell.height_ne];
    let (low, high) = heights
        .iter()
        .fold((f32::MAX, f32::MIN), |(lo, hi), &h| (lo.min(h), hi.max(h)));
    if high - low > MAX_HEIGHT_SPREAD {
        return None;
    }
    if water.is_some_and(|level| high > level) {
        return None;
    }
    Some(info.class)
}
