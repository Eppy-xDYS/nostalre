//! The flora art, drawn at load from a fixed seed: fern, brown mushrooms, red mushrooms, flowers and a leafy plant
//! side by side (`classify::Flora` order), one cell each, in a picture as big as the grass art's row so the two
//! stack into one texture. Fern and leafy plant are grey (the tuft tint colours them, like the grass); mushrooms
//! and flowers keep their own colours.

use super::classify::{FLORA_COLUMNS, FLORA_KINDS};

/// How each flora stands (`classify::Flora` order), sent to the shader as is: height (share of a tuft's),
/// card width / height, sway at the tip, and 1 when it keeps its own colours (else the ground's tint colours it).
pub const LOOKS: [[f32; 4]; FLORA_KINDS as usize] = [
    // fern: a wide low fan
    [0.6, 1.5, 0.5, 0.0],
    // mushrooms: small and stiff
    [0.4, 1.0, 0.0, 1.0],
    [0.42, 1.0, 0.0, 1.0],
    // flower: a slim stem, swaying
    [0.72, 0.8, 1.0, 1.0],
    // leafy plant: a low rosette
    [0.5, 1.3, 0.35, 0.0],
    // nest fern: long fronds arching out wide
    [0.75, 2.0, 0.3, 0.0],
];

/// Card width / height of flora `k`
const fn card_aspect(k: usize) -> f32 {
    LOOKS[k][1]
}

/// RGBA in 0 to 1, row-major
type Canvas = Vec<[f32; 4]>;

struct Painter {
    px: Canvas,
    w: usize,
    h: usize,
}

impl Painter {
    /// Paints `rgb` at `alpha` over what is there: later strokes cover earlier ones.
    fn put(&mut self, x: usize, y: usize, rgb: [f32; 3], alpha: f32) {
        let p = &mut self.px[y * self.w + x];
        if alpha <= 0.0 {
            return;
        }
        let under = p[3] * (1.0 - alpha);
        let total = alpha + under;
        for i in 0..3 {
            p[i] = (rgb[i] * alpha + p[i] * under) / total;
        }
        p[3] = total;
    }

    /// A domed ellipse lit from the upper left: darker toward its lower right rim.
    fn oval_shaded(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, rgb: [f32; 3]) {
        let (x0, x1) = (((cx - rx - 1.0).floor().max(0.0)) as usize, ((cx + rx + 1.0).ceil() as usize).min(self.w));
        let (y0, y1) = (((cy - ry - 1.0).floor().max(0.0)) as usize, ((cy + ry + 1.0).ceil() as usize).min(self.h));
        let light = [-0.45f32, -0.6, 0.66];
        for y in y0..y1 {
            for x in x0..x1 {
                let (dx, dy) = ((x as f32 + 0.5 - cx) / rx, (y as f32 + 0.5 - cy) / ry);
                let r2 = dx * dx + dy * dy;
                let a = ((1.0 - r2.sqrt()) * rx.min(ry)).clamp(0.0, 1.0);
                let nz = (1.0 - r2.min(1.0)).sqrt();
                let lit = (dx * light[0] + dy * light[1] + nz * light[2]).max(0.0);
                let k = 0.5 + 0.6 * lit;
                self.put(x, y, rgb.map(|c| (c * k).min(1.0)), a);
            }
        }
    }

    /// A filled ellipse, soft by one pixel at its edge.
    fn oval(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, rgb: [f32; 3]) {
        let (x0, x1) = (((cx - rx - 1.0).floor().max(0.0)) as usize, ((cx + rx + 1.0).ceil() as usize).min(self.w));
        let (y0, y1) = (((cy - ry - 1.0).floor().max(0.0)) as usize, ((cy + ry + 1.0).ceil() as usize).min(self.h));
        for y in y0..y1 {
            for x in x0..x1 {
                let (dx, dy) = ((x as f32 + 0.5 - cx) / rx, (y as f32 + 0.5 - cy) / ry);
                let a = ((1.0 - (dx * dx + dy * dy).sqrt()) * rx.min(ry)).clamp(0.0, 1.0);
                self.put(x, y, rgb, a);
            }
        }
    }

    /// A leaf in card space (shares of the card's height, `aspect` wide) of the cell from `x0`, `cw` × `h` px: from
    /// `root` along `angle` (0 up, positive right), `length` long, `width` at its widest, a darker midrib.
    #[allow(clippy::too_many_arguments)]
    fn leaf(&mut self, x0: f32, cw: f32, h: f32, aspect: f32, root: [f32; 2], angle: f32, length: f32, width: f32, tone: f32) {
        let dir = [angle.sin(), angle.cos()];
        let ground = h - 1.0;
        // only the pixels around the leaf: root to tip, half its width either side
        let tip = [root[0] + dir[0] * length, root[1] + dir[1] * length];
        let pad = width * 0.5 + 2.0 / h;
        let px = |u: f32| x0 + cw * (0.5 + u / aspect);
        let xs = ((px(root[0].min(tip[0]) - pad) - 1.0).max(x0).max(0.0) as usize)
            ..((px(root[0].max(tip[0]) + pad) + 1.0).max(0.0) as usize).min(((x0 + cw) as usize).min(self.w));
        let ys = ((ground - h * (root[1].max(tip[1]) + pad)) - 1.0).max(0.0) as usize
            ..((ground - h * (root[1].min(tip[1]) - pad) + 1.0).max(0.0) as usize).min(self.h);
        for y in ys {
            for x in xs.clone() {
                // this pixel in card space
                let u = ((x as f32 + 0.5 - x0) / cw - 0.5) * aspect;
                let v = (ground - (y as f32 + 0.5)) / h;
                let (du, dv) = (u - root[0], v - root[1]);
                let along = (du * dir[0] + dv * dir[1]) / length;
                let across = du * dir[1] - dv * dir[0];
                // a short bare stalk, then the blade: widest past the middle, pointed tip
                if !(0.12..=1.0).contains(&along) {
                    continue;
                }
                let t = (along - 0.12) / 0.88;
                let half = width * 0.5 * (std::f32::consts::PI * t).sin().powf(0.7) * (1.0 - 0.3 * t);
                let edge = (half - across.abs()) * h;
                let alpha = (edge + 0.5).clamp(0.0, 1.0);
                if alpha <= 0.0 {
                    continue;
                }
                let rib = (across.abs() * h < 1.0) as u8 as f32;
                // side veins slanting toward the tip, and edges a little darker than the middle
                let side = (across.abs() / half.max(1e-3)).min(1.0);
                let vein = ((along * 9.0 - side * 1.2).fract() < 0.08 && side > 0.15) as u8 as f32;
                let g = tone * (0.8 + 0.2 * t) * (1.0 - 0.25 * rib) * (1.0 - 0.12 * vein) * (1.0 - 0.18 * side * side);
                self.put(x, y, [g, g, g], alpha);
            }
        }
        // the stalk
        let to_px = |u: f32, v: f32| [x0 + cw * (0.5 + u / aspect), ground - h * v];
        let tip = to_px(root[0] + dir[0] * length * 0.15, root[1] + dir[1] * length * 0.15);
        self.stroke(to_px(root[0], root[1]), tip, 2.0 * h / 128.0, 0.8, [tone * 0.7; 3]);
    }

    /// A strap leaf along `points` (card space, as `leaf`) `halves[i]` wide each side, shaded per pixel: the side
    /// facing the light (`lit_left`) brighter than the other like a folded leaf, darker wavy edges, fine slanted
    /// veins, a dark midrib with a light line beside it, lighter toward the tip.
    #[allow(clippy::too_many_arguments)]
    fn strap(&mut self, x0: f32, cw: f32, h: f32, aspect: f32, points: &[[f32; 2]], halves: &[f32], tone: f32, lit_left: bool) {
        let ground = h - 1.0;
        let to_card = |x: usize, y: usize| [((x as f32 + 0.5 - x0) / cw - 0.5) * aspect, (ground - (y as f32 + 0.5)) / h];
        let px = |u: f32| x0 + cw * (0.5 + u / aspect);
        let py = |v: f32| ground - h * v;
        let segments = points.len() - 1;
        // each segment visits only the pixels within its widest half of it, keeping per pixel the nearest point
        // of the whole path: (distance², along, signed across)
        let (cell_lo, cell_hi) = (x0.max(0.0) as usize, ((x0 + cw) as usize).min(self.w));
        let width = cell_hi.saturating_sub(cell_lo);
        let mut nearest = vec![(f32::MAX, 0.0f32, 0.0f32); width * self.h];
        for i in 0..segments {
            let (a, b) = (points[i], points[i + 1]);
            let reach = halves[i].max(halves[i + 1]) + 1.5 / h;
            let xs = (px(a[0].min(b[0]) - reach) - 1.0).max(cell_lo as f32) as usize
                ..((px(a[0].max(b[0]) + reach) + 1.0).max(0.0) as usize).min(cell_hi);
            let ys = (py(a[1].max(b[1]) + reach) - 1.0).max(0.0) as usize
                ..((py(a[1].min(b[1]) - reach) + 1.0).max(0.0) as usize).min(self.h);
            let d = [b[0] - a[0], b[1] - a[1]];
            let len2 = (d[0] * d[0] + d[1] * d[1]).max(1e-9);
            for y in ys.clone() {
                for x in xs.clone() {
                    let c = to_card(x, y);
                    let f = (((c[0] - a[0]) * d[0] + (c[1] - a[1]) * d[1]) / len2).clamp(0.0, 1.0);
                    let e = [c[0] - a[0] - d[0] * f, c[1] - a[1] - d[1] * f];
                    let dist2 = e[0] * e[0] + e[1] * e[1];
                    let slot = &mut nearest[y * width + x - cell_lo];
                    if dist2 < slot.0 {
                        let side = (d[0] * e[1] - d[1] * e[0]).signum();
                        *slot = (dist2, (i as f32 + f) / segments as f32, side * dist2.sqrt());
                    }
                }
            }
        }
        for y in 0..self.h {
            for x in cell_lo..cell_hi {
                let best = nearest[y * width + x - cell_lo];
                if best.0 == f32::MAX {
                    continue;
                }
                let (t, across) = (best.1, best.2);
                let i = ((t * segments as f32) as usize).min(segments - 1);
                let f = t * segments as f32 - i as f32;
                let half = halves[i] + (halves[i + 1] - halves[i]) * f;
                let alpha = ((half - across.abs()) * h + 0.5).clamp(0.0, 1.0);
                if alpha <= 0.0 || half <= 0.0 {
                    continue;
                }
                let s = (across / half).clamp(-1.0, 1.0);
                let lit = if lit_left { -s } else { s };
                let mut g = tone * (0.72 + 0.28 * t);
                // folded: one half catches the light
                g *= 1.0 + 0.16 * lit.clamp(-1.0, 1.0).signum() * lit.abs().powf(0.35);
                // darker toward the wavy edge
                g *= 1.0 - 0.32 * s.abs().powi(4);
                // fine veins slanting out toward the tip
                let vein = (t * 30.0 - s.abs() * 3.0).fract();
                // (none at the very tip, where the nearest point is the end and the pattern would ring it)
                g *= 1.0 - 0.17 * (vein < 0.2 && s.abs() > 0.1 && t < 0.93) as u8 as f32;
                // the midrib, and a light line beside it on the lit side
                let rib = s.abs() * half * h;
                if rib < 1.1 {
                    g *= 0.5;
                } else if rib < 2.6 && lit > 0.0 {
                    g *= 1.18;
                }
                self.put(x, y, [g.min(1.0); 3], alpha);
            }
        }
    }

    /// A stroke from `a` to `b`, `width` thick at `a` tapering to `taper` × at `b`.
    fn stroke(&mut self, a: [f32; 2], b: [f32; 2], width: f32, taper: f32, rgb: [f32; 3]) {
        let pad = width + 1.0;
        let (x0, x1) = ((a[0].min(b[0]) - pad).floor().max(0.0) as usize, ((a[0].max(b[0]) + pad).ceil() as usize).min(self.w));
        let (y0, y1) = ((a[1].min(b[1]) - pad).floor().max(0.0) as usize, ((a[1].max(b[1]) + pad).ceil() as usize).min(self.h));
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let len2 = (dx * dx + dy * dy).max(1e-6);
        for y in y0..y1 {
            for x in x0..x1 {
                let (px, py) = (x as f32 + 0.5 - a[0], y as f32 + 0.5 - a[1]);
                let t = ((px * dx + py * dy) / len2).clamp(0.0, 1.0);
                let (ex, ey) = (px - dx * t, py - dy * t);
                let half = width * 0.5 * (1.0 + (taper - 1.0) * t);
                let alpha = (half + 0.5 - (ex * ex + ey * ey).sqrt()).clamp(0.0, 1.0);
                self.put(x, y, rgb, alpha);
            }
        }
    }
}

const FERN_GREY: [f32; 3] = [0.82, 0.82, 0.82];
const STEM: [f32; 3] = [0.88, 0.84, 0.72];
const CAP_RED: [f32; 3] = [0.72, 0.16, 0.1];
const CAP_BROWN: [f32; 3] = [0.55, 0.38, 0.24];
const CAP_TAN: [f32; 3] = [0.74, 0.6, 0.42];
const CAP_DARK: [f32; 3] = [0.36, 0.24, 0.15];
const SPOT: [f32; 3] = [0.95, 0.93, 0.88];
const FLOWER_STEM: [f32; 3] = [0.3, 0.5, 0.2];
const PETALS: [[f32; 3]; 4] = [[0.96, 0.95, 0.9], [0.98, 0.84, 0.25], [0.66, 0.46, 0.86], [0.95, 0.55, 0.7]];
const HEART: [f32; 3] = [0.95, 0.75, 0.2];

/// The flora picture, `w` × 2 `h`: two rows of `FLORA_COLUMNS` cells (each `w` / 3 × `h`), full size.
pub fn flora_art(w: usize, h: usize) -> Canvas {
    let cells = (w / FLORA_COLUMNS as usize).max(1);
    // drawn twice as big, then averaged down: smooth edges
    let big = draw_strip(cells * 2, h * 2);
    let strip = Painter { px: halve(&big.px, big.w, big.h), w: big.w / 2, h: (big.h / 2).max(1) };
    fold(&strip, cells, w, h)
}

/// The rows of `FLORA_COLUMNS` cells from a strip of all of them, side by side in a `w` wide picture.
fn fold(strip: &Painter, cell_w: usize, w: usize, h: usize) -> Canvas {
    let columns = FLORA_COLUMNS as usize;
    let rows = (FLORA_KINDS as usize).div_ceil(columns);
    let mut out = vec![[0.0; 4]; w * h * rows];
    for k in 0..FLORA_KINDS as usize {
        let (col, row) = (k % columns, k / columns);
        for y in 0..h {
            for x in 0..cell_w.min(w - col * cell_w) {
                out[(row * h + y) * w + col * cell_w + x] = strip.px[y * strip.w + k * cell_w + x];
            }
        }
    }
    out
}

/// Every flora cell side by side, `cell_w` × `h` each.
fn draw_strip(cell_w: usize, h: usize) -> Painter {
    let w = cell_w * FLORA_KINDS as usize;
    let mut p = Painter { px: vec![[0.0; 4]; w * h], w, h };
    let cw = cell_w as f32;
    let hf = h as f32;
    // sizes from the cell, so a big picture's row gets the same drawing at its own scale
    let s = hf / 128.0;
    let mut rng = Rng(0x666c_6f72_615f_3031);
    let ground = hf - 1.0;

    // fern: fronds fanning out from the root, each a curved spine with leaflets shrinking to the tip
    let root = [cw * 0.5, ground];
    let fronds = 9;
    for i in 0..fronds {
        let spread = (i as f32 + 0.5) / fronds as f32 * 2.0 - 1.0;
        let reach = cw * (0.36 + 0.1 * rng.unit());
        let rise = hf * (0.55 + 0.25 * (1.0 - spread.abs())) * (0.85 + 0.15 * rng.unit());
        let steps = 14;
        let mut prev = root;
        for k in 1..=steps {
            let t = k as f32 / steps as f32;
            // up, then bending out and over
            let pt = [root[0] + spread * reach * t.powf(0.8), ground - rise * (2.0 * t - t * t)];
            p.stroke(prev, pt, 2.2 * s * (1.0 - 0.6 * t), 1.0, FERN_GREY);
            let leaf = 7.0 * s * (1.0 - t) + 1.5 * s;
            let dir = [pt[0] - prev[0], pt[1] - prev[1]];
            let len = (dir[0] * dir[0] + dir[1] * dir[1]).sqrt().max(1e-3);
            let normal = [-dir[1] / len, dir[0] / len];
            for side in [-1.0, 1.0] {
                let tip = [pt[0] + normal[0] * leaf * side + dir[0] * 0.4, pt[1] + normal[1] * leaf * side + dir[1] * 0.4];
                p.stroke(pt, tip, 2.4 * s, 0.3, FERN_GREY);
            }
            prev = pt;
        }
    }

    // card space: (across, up) in shares of the card's height; a card is `card_aspect` wide, its cell `cw` px
    let card = |k: usize, u: f32, v: f32| [k as f32 * cw + cw * (0.5 + u / card_aspect(k)), ground - hf * v];
    let px_x = |k: usize, u: f32| cw * u / card_aspect(k);

    // brown mushrooms: a cluster of tan and dark caps, big and small
    for (u, tall, width, cap) in [
        (0.0, 0.62, 0.5, CAP_BROWN),
        (-0.3, 0.38, 0.32, CAP_TAN),
        (0.32, 0.45, 0.36, CAP_DARK),
        (-0.12, 0.22, 0.2, CAP_TAN),
    ] {
        mushroom(&mut p, card(1, u, 0.0), card(1, u, tall), px_x(1, width / 2.0), hf * width * 0.28, cap, false);
    }
    // red mushrooms: spotted caps, one big and two smaller
    for (u, tall, width) in [(0.05, 0.66, 0.54), (-0.3, 0.36, 0.3), (0.33, 0.42, 0.34)] {
        mushroom(&mut p, card(2, u, 0.0), card(2, u, tall), px_x(2, width / 2.0), hf * width * 0.28, CAP_RED, true);
    }

    // flowers: thin stems, each with a five-petal head
    let squash = px_x(3, 1.0) / hf;
    let flowers = 9;
    for i in 0..flowers {
        let u = -0.4 + 0.8 * (i as f32 + rng.unit()) / flowers as f32;
        let tall = 0.45 + 0.45 * rng.unit();
        let foot = card(3, u, 0.0);
        let head = card(3, u + (rng.unit() - 0.5) * 0.15, tall);
        p.stroke(foot, head, 1.8 * s, 0.7, FLOWER_STEM);
        let colour = PETALS[rng.below(PETALS.len())];
        let r = 4.5 * s * (0.8 + 0.4 * rng.unit());
        for k in 0..5 {
            let a = k as f32 / 5.0 * std::f32::consts::TAU + rng.unit();
            p.oval_shaded(head[0] + a.cos() * r * 0.8 * squash, head[1] + a.sin() * r * 0.8, r * 0.6 * squash, r * 0.6, colour);
        }
        p.oval_shaded(head[0], head[1], r * 0.4 * squash, r * 0.4, HEART);
    }

    // leafy plant: a rosette of broad leaves on short stalks, grey for the tint, darker toward the middle
    let leaves = 8;
    for i in 0..leaves {
        let spread = (i as f32 + 0.5) / leaves as f32 * 2.0 - 1.0;
        let angle = spread * 1.25 + (rng.unit() - 0.5) * 0.2;
        let length = 0.55 + 0.35 * (1.0 - spread.abs()) * rng.unit() + 0.1;
        let tone = 0.62 + 0.3 * rng.unit();
        p.leaf(4.0 * cw, cw, hf, card_aspect(4), [0.0, 0.0], angle, length, length * 0.42, tone);
    }

    // bird's nest fern: long straps from a centre, arching out and down; the side straps first (behind, darker),
    // the upright ones last
    let straps = 14;
    let mut order: Vec<(f32, f32, f32, f32, f32, f32)> = (0..straps)
        .map(|i| {
            let spread = (i as f32 + 0.5) / straps as f32 * 2.0 - 1.0;
            let angle = spread * 1.05 + (rng.unit() - 0.5) * 0.2;
            let length = 1.0 + 0.12 * rng.unit() - 0.15 * spread.abs();
            // about five times as long as wide: a frond, not a paddle
            let width = 0.085 + 0.025 * rng.unit();
            let tone = 0.62 + 0.3 * rng.unit();
            (spread, angle, length, width, tone, rng.unit() * 6.0)
        })
        .collect();
    order.sort_by(|a, b| b.0.abs().total_cmp(&a.0.abs()));
    for (spread, angle, length, width, tone, wave) in order {
        let steps = 20;
        let mut points = Vec::with_capacity(steps + 1);
        let mut halves = Vec::with_capacity(steps + 1);
        for k in 0..=steps {
            let t = k as f32 / steps as f32;
            // out along the angle and arching over: the further it leans, the more its outer half bends down
            let droop = (0.15 + 0.5 * spread.abs()) * t * t * t;
            // (kept inside the card: its half width is 1)
            let out = angle.sin() * length * t * (1.0 + 0.15 * t) * 0.92;
            points.push([out, (0.03 + angle.cos() * length * t - droop * length).clamp(0.03, 0.97)]);
            let ripple = 1.0 + 0.04 * (t * 19.0 + wave).sin();
            // a narrow stalk, widest a little past the middle, then tapering to a point
            let shape = (std::f32::consts::PI * t.powf(1.25)).sin().max(0.0).powf(0.85);
            halves.push(width * (0.12 + 0.88 * shape) * (t < 1.0) as u8 as f32 * ripple);
        }
        let behind = 1.0 - 0.18 * spread.abs();
        p.strap(5.0 * cw, cw, hf, card_aspect(5), &points, &halves, tone * behind, spread < 0.0);
    }
    p
}

/// A mushroom: a stem from `foot` up to `top`, a domed cap `rx` × `ry` px over a pale underside, spots if `spots`.
fn mushroom(p: &mut Painter, foot: [f32; 2], top: [f32; 2], rx: f32, ry: f32, cap: [f32; 3], spots: bool) {
    // the stem: shaded, then lit down its left side
    let stem = rx * 0.55;
    p.stroke(foot, top, stem, 0.85, STEM.map(|c| c * 0.78));
    p.stroke([foot[0] - stem * 0.12, foot[1]], [top[0] - stem * 0.1, top[1]], stem * 0.62, 0.85, STEM);
    p.stroke([foot[0] - stem * 0.22, foot[1]], [top[0] - stem * 0.2, top[1]], stem * 0.2, 0.85, STEM.map(|c| (c * 1.08).min(1.0)));
    // the gills: a pale underside with dark lines fanning from the stem
    p.oval(top[0], top[1] + ry * 0.45, rx * 0.92, ry * 0.36, STEM.map(|c| c * 0.86));
    for k in 0..9 {
        let u = (k as f32 + 0.5) / 9.0 * 2.0 - 1.0;
        let edge = [top[0] + u * rx * 0.88, top[1] + ry * 0.55];
        p.stroke([top[0] + u * stem * 0.4, top[1] + ry * 0.62], edge, 1.0_f32.max(rx * 0.04), 1.0, STEM.map(|c| c * 0.62));
    }
    p.oval_shaded(top[0], top[1] - ry * 0.15, rx, ry * 0.85, cap);
    if spots {
        for (sx, sy, r) in [(-0.5, -0.05, 0.12), (0.1, -0.5, 0.1), (0.55, -0.1, 0.13), (-0.05, 0.05, 0.09), (0.32, -0.38, 0.08), (-0.3, -0.45, 0.07)] {
            p.oval_shaded(top[0] + rx * sx, top[1] - ry * 0.15 + ry * sy, rx * r, ry * r * 1.3, SPOT);
        }
    }
}

/// Colour averaged by alpha, so clear texels do not darken the edges; `w`, `h` may be odd (the last row or column
/// drops, as a GPU mip does).
pub fn halve(px: &[[f32; 4]], w: usize, h: usize) -> Canvas {
    let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
    let mut out = vec![[0.0; 4]; nw * nh];
    for y in 0..nh {
        for x in 0..nw {
            let at = |dx: usize, dy: usize| px[(2 * y + dy).min(h - 1) * w + (2 * x + dx).min(w - 1)];
            let quad = [at(0, 0), at(1, 0), at(0, 1), at(1, 1)];
            let a: f32 = quad.iter().map(|p| p[3]).sum();
            let mut c = [0.0, 0.0, 0.0, a / 4.0];
            if a > 0.0 {
                for i in 0..3 {
                    c[i] = quad.iter().map(|p| p[i] * p[3]).sum::<f32>() / a;
                }
            }
            out[y * nw + x] = c;
        }
    }
    out
}

/// xorshift64
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}
