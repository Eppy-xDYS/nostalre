//! Grass is made and kept per 16 × 16-cell chunk near the camera target: which chunks to keep, and which of them to
//! draw this frame, nearest first, until the quality's tuft budget is spent.

use glam::{Mat4, Vec2, Vec3, Vec4};

use super::scatter::{GrassField, TUFT_MAX_HEIGHT};

pub const CHUNK_CELLS: i32 = 16;

/// A made chunk: its grid key, box (tuft tops included) and tuft count.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChunkBox {
    pub key: (i32, i32),
    pub min: Vec3,
    pub max: Vec3,
    pub tufts: u32,
}

impl ChunkBox {
    /// Heights grow downward: a tuft reaches `TUFT_MAX_HEIGHT` above (below in y) the highest ground.
    pub fn new(field: &GrassField, key: (i32, i32), tufts: u32) -> Self {
        let side = CHUNK_CELLS as f32 * field.cell_size();
        let (low, high) = field.chunk_height_range(key.0, key.1);
        Self {
            key,
            min: Vec3::new(key.0 as f32 * side, low - TUFT_MAX_HEIGHT, key.1 as f32 * side),
            max: Vec3::new((key.0 + 1) as f32 * side, high, (key.1 + 1) as f32 * side),
            tufts,
        }
    }
}

/// The growing chunks within `far` of `focus` on the ground plane, nearest first.
pub fn wanted_chunks(field: &GrassField, focus: Vec3, far: f32) -> Vec<(i32, i32)> {
    let side = CHUNK_CELLS as f32 * field.cell_size();
    let at = Vec2::new(focus.x, focus.z);
    let (cw, ch) = field.chunk_grid();
    let lo = ((at - far) / side).floor().max(Vec2::ZERO);
    let hi = ((at + far) / side).floor();
    let mut wanted: Vec<((i32, i32), f32)> = Vec::new();
    for cz in lo.y as i32..=(hi.y as i32).min(ch - 1) {
        for cx in lo.x as i32..=(hi.x as i32).min(cw - 1) {
            if !field.chunk_grows(cx, cz) {
                continue;
            }
            let min = Vec2::new(cx as f32, cz as f32) * side;
            let gap = at.clamp(min, min + side).distance(at);
            if gap <= far {
                let centre = (Vec2::new(cx as f32, cz as f32) + 0.5) * side;
                wanted.push(((cx, cz), centre.distance(at)));
            }
        }
    }
    wanted.sort_by(|a, b| a.1.total_cmp(&b.1));
    wanted.into_iter().map(|(key, _)| key).collect()
}

/// Indices into `chunks` to draw: inside the frustum of `view_proj` (wgpu depth 0 to 1) and within `far` of `focus`,
/// nearest first, as many as fit in `budget` tufts.
pub fn draw_list(chunks: &[ChunkBox], view_proj: Mat4, focus: Vec3, far: f32, budget: u32) -> Vec<usize> {
    let planes = frustum_planes(view_proj);
    let mut visible: Vec<(usize, f32)> = chunks
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            let gap = ground_gap(c, focus);
            (gap <= far && planes.iter().all(|p| box_in_front(*p, c.min, c.max))).then_some((i, gap))
        })
        .collect();
    visible.sort_by(|a, b| a.1.total_cmp(&b.1));
    let mut spent = 0u32;
    let mut drawn = Vec::new();
    for (i, _) in visible {
        let tufts = chunks[i].tufts;
        if spent.saturating_add(tufts) > budget {
            continue;
        }
        spent += tufts;
        drawn.push(i);
    }
    drawn
}

/// How far the chunk's box is from `focus` on the ground plane, 0 inside.
fn ground_gap(chunk: &ChunkBox, focus: Vec3) -> f32 {
    let dx = (chunk.min.x - focus.x).max(focus.x - chunk.max.x).max(0.0);
    let dz = (chunk.min.z - focus.z).max(focus.z - chunk.max.z).max(0.0);
    (dx * dx + dz * dz).sqrt()
}

/// Left, right, bottom, top, near, far, pointing inward.
fn frustum_planes(m: Mat4) -> [Vec4; 6] {
    let (r0, r1, r2, r3) = (m.row(0), m.row(1), m.row(2), m.row(3));
    [r3 + r0, r3 - r0, r3 + r1, r3 - r1, r2, r3 - r2]
}

/// Some part of the box is on the inner side of `plane`.
fn box_in_front(plane: Vec4, min: Vec3, max: Vec3) -> bool {
    let farthest = Vec3::new(
        if plane.x >= 0.0 { max.x } else { min.x },
        if plane.y >= 0.0 { max.y } else { min.y },
        if plane.z >= 0.0 { max.z } else { min.z },
    );
    plane.truncate().dot(farthest) + plane.w >= 0.0
}
