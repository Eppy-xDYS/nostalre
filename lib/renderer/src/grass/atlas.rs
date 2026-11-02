//! The blade art, drawn at load from a fixed seed: one tuft per grass kind side by side (`classify::Kind`: dry,
//! tall, bush), grey (the tuft tint colours them) and darker at the root. Row 0 is the tips. The mips keep the full size's coverage, so far grass does not
//! thin out as its alpha averages down.

/// Three kinds, each 240 × 256: a tuft card 0.94 as wide as tall
pub const ATLAS_WIDTH: u32 = 720;
pub const ATLAS_HEIGHT: u32 = 256;
/// The art is drawn this many times bigger, then averaged down: smooth edges
const SUPERSAMPLE: usize = 2;
/// Kinds in the procedural atlas
pub const KINDS: usize = super::classify::KINDS as usize;
/// The most kinds a blade picture may hold side by side
pub const VARIANTS: usize = 4;
/// Mips stop here: grass is gone before it is this small on screen
const MIN_MIP_WIDTH: u32 = 8;

/// The picture tufts are drawn with: the procedural atlas, or a blade picture from local game data (its own
/// colours; one clump, or up to 4 kinds side by side). Under it, the flora row (`flora::flora_art`) at the same
/// size: the texture is `width` × 3 `height` (the flora takes two rows).
pub struct BladeArt {
    /// RGBA8, full size first; each level the grass row then the two flora rows
    pub levels: Vec<Vec<u8>>,
    pub width: u32,
    /// Of one row
    pub height: u32,
    /// Side by side across the width
    pub variants: u32,
    /// Its colours count (a picture); else grey for the tint to colour
    pub colored: bool,
    /// Mean luminance of the opaque texels: the tint divides it out, so dark and bright art come out alike
    pub mean_luma: f32,
}

impl BladeArt {
    pub fn procedural() -> Self {
        let levels = blade_atlas();
        let mean_luma = opaque_mean_luma(&levels[0]);
        Self {
            levels: with_flora(levels, ATLAS_WIDTH as usize, ATLAS_HEIGHT as usize),
            width: ATLAS_WIDTH,
            height: ATLAS_HEIGHT,
            variants: KINDS as u32,
            colored: false,
            mean_luma,
        }
    }

    /// Mips halve while both sides stay even, down to `MIN_MIP_WIDTH` / 2 wide (at least one), keeping the full
    /// size's coverage.
    pub fn from_image(img: &image::RgbaImage) -> Self {
        let (w, h) = (img.width() as usize, img.height() as usize);
        let px: Vec<[f32; 4]> = img.pixels().map(|p| p.0.map(|c| c as f32 / 255.0)).collect();
        let base = px.iter().filter(|p| p[3] >= 0.5).count() as f32 / px.len().max(1) as f32;
        let mut levels = vec![rgba_bytes(&px)];
        let (mut cur, mut lw, mut lh) = (px, w, h);
        while lw % 2 == 0
            && lh % 2 == 0
            && lw >= 4
            && lh >= 4
            && (levels.len() == 1 || lw > MIN_MIP_WIDTH as usize / 2)
        {
            cur = halve_rgba(&cur, lw, lh);
            lw /= 2;
            lh /= 2;
            let mut alpha: Vec<f32> = cur.iter().map(|p| p[3]).collect();
            keep_coverage(&mut alpha, base);
            for (p, a) in cur.iter_mut().zip(alpha) {
                p[3] = a;
            }
            levels.push(rgba_bytes(&cur));
        }
        let mean_luma = opaque_mean_luma(&levels[0]);
        Self {
            levels: with_flora(levels, w, h),
            width: w as u32,
            height: h as u32,
            // N times as wide as tall: N kinds side by side (`classify::Kind`)
            variants: (w as f32 / h.max(1) as f32).round().clamp(1.0, VARIANTS as f32) as u32,
            colored: true,
            mean_luma,
        }
    }
}

/// Each grass level with the flora row of the same size under it.
fn with_flora(grass: Vec<Vec<u8>>, w: usize, h: usize) -> Vec<Vec<u8>> {
    let mut flora = super::flora::flora_art(w, h);
    let (mut fw, mut fh) = (w, h * 2);
    let mut out = Vec::with_capacity(grass.len());
    for (level, mut data) in grass.into_iter().enumerate() {
        if level > 0 {
            flora = super::flora::halve(&flora, fw, fh);
            fw = (fw / 2).max(1);
            fh = (fh / 2).max(1);
        }
        debug_assert_eq!(data.len() * 2, fw * fh * 4, "grass and flora rows match at level {level}");
        data.extend(rgba_bytes(&flora));
        out.push(data);
    }
    out
}

fn opaque_mean_luma(rgba: &[u8]) -> f32 {
    let (mut sum, mut n) = (0.0f32, 0u32);
    for p in rgba.chunks_exact(4).filter(|p| p[3] > 71) {
        sum += (0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32) / 255.0;
        n += 1;
    }
    if n == 0 { 0.5 } else { sum / n as f32 }
}

/// Colour averaged by alpha, so clear texels do not darken the edges.
fn halve_rgba(px: &[[f32; 4]], w: usize, h: usize) -> Vec<[f32; 4]> {
    let (nw, nh) = (w / 2, h / 2);
    let mut out = vec![[0.0; 4]; nw * nh];
    for y in 0..nh {
        for x in 0..nw {
            let quad = [px[2 * y * w + 2 * x], px[2 * y * w + 2 * x + 1], px[(2 * y + 1) * w + 2 * x], px[(2 * y + 1) * w + 2 * x + 1]];
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

fn rgba_bytes(px: &[[f32; 4]]) -> Vec<u8> {
    px.iter()
        .flat_map(|p| p.map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8))
        .collect()
}

/// RGBA8 levels, full size first.
pub fn blade_atlas() -> Vec<Vec<u8>> {
    let (w, h) = (ATLAS_WIDTH as usize * SUPERSAMPLE, ATLAS_HEIGHT as usize * SUPERSAMPLE);
    // the styles' pixel sizes are for a 128 px tall cell
    let s = h as f32 / 128.0;
    let mut coverage = vec![0.0f32; w * h];
    let mut shade = vec![0.0f32; w * h];
    let mut rng = Rng(0x6772_6173_735f_3031);
    let vw = w / KINDS;
    for (k, style) in STYLES.iter().enumerate() {
        let blades = style.blades.0 + rng.below(style.blades.1 - style.blades.0 + 1);
        for _ in 0..blades {
            let root_x = (k * vw) as f32 + vw as f32 * (style.spread.0 + (style.spread.1 - style.spread.0) * rng.unit());
            let tall = h as f32 * (style.tall.0 + (style.tall.1 - style.tall.0) * rng.unit());
            let bend = (rng.unit() - 0.5) * vw as f32 * style.bend;
            let width = (style.width.0 + (style.width.1 - style.width.0) * rng.unit()) * s;
            let tone = style.tone.0 + (style.tone.1 - style.tone.0) * rng.unit();
            draw_blade(&mut coverage, &mut shade, w, h, root_x, tall, bend, width, tone);
            if style.seed_heads {
                draw_seed_head(&mut coverage, &mut shade, w, h, root_x + bend, tall, tone, s);
            }
        }
    }
    let (mut coverage, mut shade) = (coverage, shade);
    let (mut w, mut h) = (w, h);
    for _ in 1..SUPERSAMPLE {
        (coverage, shade) = halve(&coverage, &shade, w, h);
        w /= 2;
        h /= 2;
    }
    let mut levels = vec![to_rgba(&coverage, &shade)];
    // per kind: thin dry stalks and a dense bush would otherwise trade coverage as they shrink
    let bases: Vec<f32> = (0..KINDS).map(|k| opaque_share(&kind_column(&coverage, w, h, k))).collect();
    let (mut cov, mut sh, mut lw, mut lh) = (coverage, shade, w, h);
    while lw / 2 >= MIN_MIP_WIDTH as usize {
        (cov, sh) = halve(&cov, &sh, lw, lh);
        lw /= 2;
        lh /= 2;
        for (k, &base) in bases.iter().enumerate() {
            let mut column = kind_column(&cov, lw, lh, k);
            keep_coverage(&mut column, base);
            set_kind_column(&mut cov, lw, lh, k, &column);
        }
        levels.push(to_rgba(&cov, &sh));
    }
    levels
}

/// The texels of kind `k`'s share of a `w` wide level (a column belongs to the kind its left edge falls in).
fn kind_columns(w: usize, k: usize) -> std::ops::Range<usize> {
    (k * w).div_ceil(KINDS)..((k + 1) * w).div_ceil(KINDS)
}

fn kind_column(cov: &[f32], w: usize, h: usize, k: usize) -> Vec<f32> {
    (0..h).flat_map(|y| kind_columns(w, k).map(move |x| cov[y * w + x])).collect()
}

fn set_kind_column(cov: &mut [f32], w: usize, h: usize, k: usize, column: &[f32]) {
    let mut values = column.iter();
    for y in 0..h {
        for x in kind_columns(w, k) {
            cov[y * w + x] = *values.next().unwrap_or(&0.0);
        }
    }
}

/// How one kind's tuft is drawn: (low, high) ranges, heights as shares of the cell height
struct Style {
    blades: (usize, usize),
    /// Roots across the cell, shares of its width
    spread: (f32, f32),
    tall: (f32, f32),
    /// Tip drift, a share of the cell width
    bend: f32,
    /// Blade width in pixels at the root
    width: (f32, f32),
    tone: (f32, f32),
    seed_heads: bool,
}

/// In `classify::Kind` order
const STYLES: [Style; KINDS] = [
    // dry: fewer, thinner stalks of mixed height, each with a seed head, pale
    Style { blades: (26, 34), spread: (0.12, 0.88), tall: (0.45, 0.88), bend: 0.5, width: (1.6, 2.6), tone: (0.92, 1.0), seed_heads: true },
    // tall: a full meadow clump
    Style { blades: (38, 50), spread: (0.12, 0.88), tall: (0.55, 1.0), bend: 0.35, width: (2.0, 3.6), tone: (0.85, 1.0), seed_heads: false },
    // bush: short, dense, wide-spread, dark
    Style { blades: (64, 78), spread: (0.05, 0.95), tall: (0.25, 0.55), bend: 0.6, width: (2.8, 4.6), tone: (0.68, 0.84), seed_heads: false },
];

/// An oat-like head: a narrow oval at the stalk's tip.
#[allow(clippy::too_many_arguments)]
fn draw_seed_head(coverage: &mut [f32], shade: &mut [f32], w: usize, h: usize, tip_x: f32, tall: f32, tone: f32, s: f32) {
    let (cx, cy) = (tip_x, (h - 1) as f32 - tall + 4.0 * s);
    let (rx, ry) = (1.8 * s, 5.0 * s);
    let (x0, x1) = ((cx - rx - 1.0).floor().max(0.0) as usize, ((cx + rx + 1.0).ceil() as usize).min(w));
    let (y0, y1) = ((cy - ry - 1.0).floor().max(0.0) as usize, ((cy + ry + 1.0).ceil() as usize).min(h));
    for y in y0..y1 {
        for x in x0..x1 {
            let (dx, dy) = ((x as f32 + 0.5 - cx) / rx, (y as f32 + 0.5 - cy) / ry);
            let a = ((1.0 - (dx * dx + dy * dy).sqrt()) * rx).clamp(0.0, 1.0);
            let i = y * w + x;
            if a > coverage[i] {
                shade[i] = tone;
            }
            coverage[i] = coverage[i].max(a);
        }
    }
}

/// One tapered blade from the bottom row up to `tall`, its tip drifting `bend` sideways along a curve.
#[allow(clippy::too_many_arguments)]
fn draw_blade(
    coverage: &mut [f32],
    shade: &mut [f32],
    w: usize,
    h: usize,
    root_x: f32,
    tall: f32,
    bend: f32,
    width: f32,
    tone: f32,
) {
    for y in 0..h {
        let up = (h - 1 - y) as f32;
        if up > tall {
            continue;
        }
        let t = up / tall;
        let centre = root_x + bend * t * t;
        let half = width * 0.5 * (1.0 - t).powf(0.8);
        // darker toward the root
        let grey = tone * (0.7 + 0.3 * t);
        let (lo, hi) = ((centre - half - 1.0).floor().max(0.0) as usize, (centre + half + 1.0).ceil() as usize);
        for x in lo..hi.min(w) {
            let off = x as f32 + 0.5 - centre;
            let edge = half + 0.5 - off.abs();
            let a = edge.clamp(0.0, 1.0);
            let i = y * w + x;
            if a > coverage[i] {
                // rounded: lit along the middle, darker at the edges, one side a little lighter
                let across = (off / half.max(0.5)).clamp(-1.0, 1.0);
                shade[i] = grey * (0.74 + 0.26 * (1.0 - across * across) + 0.06 * -across);
            }
            coverage[i] = coverage[i].max(a);
        }
    }
}

fn halve(cov: &[f32], shade: &[f32], w: usize, h: usize) -> (Vec<f32>, Vec<f32>) {
    let (nw, nh) = (w / 2, h / 2);
    let mut c = vec![0.0; nw * nh];
    let mut s = vec![0.0; nw * nh];
    for y in 0..nh {
        for x in 0..nw {
            let idx = [(2 * y) * w + 2 * x, (2 * y) * w + 2 * x + 1, (2 * y + 1) * w + 2 * x, (2 * y + 1) * w + 2 * x + 1];
            let a: f32 = idx.iter().map(|&i| cov[i]).sum();
            c[y * nw + x] = a / 4.0;
            // shade weighted by coverage, so empty texels do not darken the blade edge
            s[y * nw + x] = if a > 0.0 { idx.iter().map(|&i| shade[i] * cov[i]).sum::<f32>() / a } else { 0.0 };
        }
    }
    (c, s)
}

/// Scales alpha so the share at or above one half matches `target`.
fn keep_coverage(cov: &mut [f32], target: f32) {
    let (mut lo, mut hi) = (1.0f32, 64.0f32);
    for _ in 0..20 {
        let mid = (lo + hi) / 2.0;
        let share = cov.iter().filter(|&&a| a * mid >= 0.5).count() as f32 / cov.len() as f32;
        if share < target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    for a in cov.iter_mut() {
        *a = (*a * hi).min(1.0);
    }
}

fn opaque_share(cov: &[f32]) -> f32 {
    cov.iter().filter(|&&a| a >= 0.5).count() as f32 / cov.len() as f32
}

fn to_rgba(cov: &[f32], shade: &[f32]) -> Vec<u8> {
    cov.iter()
        .zip(shade)
        .flat_map(|(&a, &s)| {
            let g = (s * 255.0).round() as u8;
            [g, g, g, (a * 255.0).round() as u8]
        })
        .collect()
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
