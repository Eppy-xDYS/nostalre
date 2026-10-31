// The waves HD water draws, shared with the terrain so the caustics on the bed come from the very surface
// above it. Prepended to water_hd.wgsl and terrain.wgsl (`WATER_HD_WGSL`, `TERRAIN_WGSL`). Lengths in cells.

// A unit-ish gradient per lattice point, sin-free.
fn hash_grad(p: vec2<f32>) -> vec2<f32> {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * vec3<f32>(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.xx + p3.yz) * p3.zy) * 2.0 - 1.0;
}

// Gradient noise with a quintic fade, remapped to 0..1: (value, d/dx, d/dy). Unlike value noise its
// features have no square cells, which RO's grid-aligned camera would show as blocks.
fn gnoised(p: vec2<f32>) -> vec3<f32> {
    let i = floor(p);
    let f = p - i;
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let du = 30.0 * f * f * (f * (f - 2.0) + 1.0);
    let ga = hash_grad(i);
    let gb = hash_grad(i + vec2<f32>(1.0, 0.0));
    let gc = hash_grad(i + vec2<f32>(0.0, 1.0));
    let gd = hash_grad(i + vec2<f32>(1.0, 1.0));
    let va = dot(ga, f);
    let vb = dot(gb, f - vec2<f32>(1.0, 0.0));
    let vc = dot(gc, f - vec2<f32>(0.0, 1.0));
    let vd = dot(gd, f - vec2<f32>(1.0, 1.0));
    let k = va - vb - vc + vd;
    let value = va + u.x * (vb - va) + u.y * (vc - va) + u.x * u.y * k;
    let grad = ga + u.x * (gb - ga) + u.y * (gc - ga) + u.x * u.y * (ga - gb - gc + gd)
        + du * (u.yx * k + vec2<f32>(vb, vc) - va);
    // the value stays within about ±0.7
    return vec3<f32>(value / 1.4 + 0.5, grad / 1.4);
}

// Gradient noise value only, 0..1: for layers whose slope nothing reads.
fn gnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = p - i;
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let va = dot(hash_grad(i), f);
    let vb = dot(hash_grad(i + vec2<f32>(1.0, 0.0)), f - vec2<f32>(1.0, 0.0));
    let vc = dot(hash_grad(i + vec2<f32>(0.0, 1.0)), f - vec2<f32>(0.0, 1.0));
    let vd = dot(hash_grad(i + vec2<f32>(1.0, 1.0)), f - vec2<f32>(1.0, 1.0));
    return mix(mix(va, vb, u.x), mix(vc, vd, u.x), u.y) / 1.4 + 0.5;
}

// Each layer is turned its own way so no layer lines up with the grid or another: (cos, sin) of
// 0.43, 1.71, 2.62, 4.05, 0.9 and 2.3 radians, written out so no GPU works them out per pixel.
const TURN1: vec2<f32> = vec2<f32>(0.90897, 0.41687);
const TURN2: vec2<f32> = vec2<f32>(-0.13875, 0.99033);
const TURN3: vec2<f32> = vec2<f32>(-0.86703, 0.49826);
const TURN4: vec2<f32> = vec2<f32>(-0.615, -0.78853);
const TURN_FOAM1: vec2<f32> = vec2<f32>(0.62161, 0.78333);
const TURN_FOAM2: vec2<f32> = vec2<f32>(-0.66628, 0.74571);

// `p` turned by `turn` (cos, sin), scaled by `freq`, drifted.
fn layer_pos(p: vec2<f32>, freq: f32, turn: vec2<f32>, drift: vec2<f32>, seed: f32) -> vec2<f32> {
    return vec2<f32>(turn.x * p.x - turn.y * p.y, turn.y * p.x + turn.x * p.y) * freq + drift + seed;
}

// One wave layer: gradient noise at `freq` per cell, turned by `turn`; the gradient comes back in cell space.
fn wave_layer(p: vec2<f32>, freq: f32, turn: vec2<f32>, drift: vec2<f32>, seed: f32) -> vec3<f32> {
    let n = gnoised(layer_pos(p, freq, turn, drift, seed));
    // d/dp = freq * R^T * d/dq
    let g = vec2<f32>(turn.x * n.y + turn.y * n.z, -turn.y * n.y + turn.x * n.z) * freq;
    return vec3<f32>(n.x, g);
}

// How fast the waves drift for a map's RSW/GND wave speed (noise units a second).
fn wave_drift_speed(wave_speed: f32) -> f32 {
    return 0.08 + 0.12 * log(1.0 + clamp(wave_speed, 0.0, 8.0)) / log(9.0);
}

// The three layers the surface's shape is built from: (value 0..1, slope in cells), largest first.
fn wave1(p: vec2<f32>, t: f32, speed: f32) -> vec3<f32> {
    return wave_layer(p, WAVE1_FREQ, TURN1, wave1_drift(t, speed), 0.0);
}

fn wave2(p: vec2<f32>, t: f32, speed: f32) -> vec3<f32> {
    return wave_layer(p, WAVE2_FREQ, TURN2, wave2_drift(t, speed), 37.0);
}

fn wave3(p: vec2<f32>, t: f32, speed: f32) -> vec3<f32> {
    return wave_layer(p, WAVE3_FREQ, TURN3, wave3_drift(t, speed), 91.0);
}

const WAVE1_FREQ: f32 = 0.55;
const WAVE2_FREQ: f32 = 1.35;
const WAVE3_FREQ: f32 = 3.1;
// how much each layer's slope adds to the surface's (layer 1 counts 1)
const WAVE2_WEIGHT: f32 = 0.35;
const WAVE3_WEIGHT: f32 = 0.12;

fn wave1_drift(t: f32, speed: f32) -> vec2<f32> {
    return vec2<f32>(t * speed * 0.9, t * speed * 0.6);
}

fn wave2_drift(t: f32, speed: f32) -> vec2<f32> {
    return vec2<f32>(-t * speed * 0.7, t * speed * 0.8);
}

fn wave3_drift(t: f32, speed: f32) -> vec2<f32> {
    return vec2<f32>(t * speed * 1.4, -t * speed * 1.1);
}

// The Laplacian (d2/dx2 + d2/dy2) of `gnoised`'s value, from the same lattice: how sharply the noise bends.
fn gnoise_laplacian(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = p - i;
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let du = 30.0 * f * f * (f * (f - 2.0) + 1.0);
    let ddu = 60.0 * f * (f * (2.0 * f - 3.0) + 1.0);
    let ga = hash_grad(i);
    let gb = hash_grad(i + vec2<f32>(1.0, 0.0));
    let gc = hash_grad(i + vec2<f32>(0.0, 1.0));
    let gd = hash_grad(i + vec2<f32>(1.0, 1.0));
    let va = dot(ga, f);
    let vb = dot(gb, f - vec2<f32>(1.0, 0.0));
    let vc = dot(gc, f - vec2<f32>(0.0, 1.0));
    let vd = dot(gd, f - vec2<f32>(1.0, 1.0));
    let k = va - vb - vc + vd;
    let kg = ga - gb - gc + gd;
    // value = va + u.x (vb - va) + u.y (vc - va) + u.x u.y k; va..vd are linear in f
    let dxx = 2.0 * du.x * (gb.x - ga.x) + ddu.x * (vb - va) + 2.0 * du.x * u.y * kg.x + ddu.x * u.y * k;
    let dyy = 2.0 * du.y * (gc.y - ga.y) + ddu.y * (vc - va) + 2.0 * du.y * u.x * kg.y + ddu.y * u.x * k;
    return (dxx + dyy) / 1.4;
}

// The Laplacian of the surface's shape (before its amplitude) from the two large, slow layers. The fine
// layer bends the most (a bend scales with the frequency squared) but drifts fast: its focused spots popped
// in and out on the bed like dancing lights. Turning a layer does not change its Laplacian.
fn wave_laplacian(p: vec2<f32>, t: f32, speed: f32) -> f32 {
    let l1 = gnoise_laplacian(layer_pos(p, WAVE1_FREQ, TURN1, wave1_drift(t, speed), 0.0));
    let l2 = gnoise_laplacian(layer_pos(p, WAVE2_FREQ, TURN2, wave2_drift(t, speed), 37.0));
    return l1 * WAVE1_FREQ * WAVE1_FREQ + l2 * WAVE2_FREQ * WAVE2_FREQ * WAVE2_WEIGHT;
}
