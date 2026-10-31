//! The blade art, drawn at load from a fixed seed: four tuft variants side by side, grey (the tuft tint colours
//! them) and darker at the root. Row 0 is the tips. The mips keep the full size's coverage, so far grass does not
//! thin out as its alpha averages down.

pub const ATLAS_WIDTH: u32 = 256;
pub const ATLAS_HEIGHT: u32 = 128;
pub const VARIANTS: usize = 4;
/// Mips stop here: grass is gone before it is this small on screen
const MIN_MIP_WIDTH: u32 = 8;

/// The picture tufts are drawn with: the procedural atlas, or a blade picture from local game data (one clump,
/// its own colours).
pub struct BladeArt {
    /// RGBA8, full size first
    pub levels: Vec<Vec<u8>>,
    pub width: u32,
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
            levels,
            width: ATLAS_WIDTH,
            height: ATLAS_HEIGHT,
            variants: VARIANTS as u32,
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
            levels,
            width: w as u32,
            height: h as u32,
            variants: 1,
            colored: true,
            mean_luma,
        }
    }
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
    let (w, h) = (ATLAS_WIDTH as usize, ATLAS_HEIGHT as usize);
    let mut coverage = vec![0.0f32; w * h];
    let mut shade = vec![0.0f32; w * h];
    let mut rng = Rng(0x6772_6173_735f_3031);
    let vw = w / VARIANTS;
    for v in 0..VARIANTS {
        // many fine blades: the tufts read as a soft carpet, not as separate spikes
        let blades = 22 + rng.below(8);
        for _ in 0..blades {
            let root_x = (v * vw) as f32 + vw as f32 * (0.12 + 0.76 * rng.unit());
            let tall = h as f32 * (0.4 + 0.58 * rng.unit());
            let bend = (rng.unit() - 0.5) * vw as f32 * 0.4;
            let width = 1.6 + 1.6 * rng.unit();
            let tone = 0.85 + 0.15 * rng.unit();
            draw_blade(&mut coverage, &mut shade, w, h, root_x, tall, bend, width, tone);
        }
    }
    let mut levels = vec![to_rgba(&coverage, &shade)];
    let base = opaque_share(&coverage);
    let (mut cov, mut sh, mut lw, mut lh) = (coverage, shade, w, h);
    while lw / 2 >= MIN_MIP_WIDTH as usize {
        (cov, sh) = halve(&cov, &sh, lw, lh);
        lw /= 2;
        lh /= 2;
        keep_coverage(&mut cov, base);
        levels.push(to_rgba(&cov, &sh));
    }
    levels
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
            let edge = half + 0.5 - (x as f32 + 0.5 - centre).abs();
            let a = edge.clamp(0.0, 1.0);
            let i = y * w + x;
            if a > coverage[i] {
                shade[i] = grey;
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
