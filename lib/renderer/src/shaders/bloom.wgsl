// Bloom: the bright-pass into the first mip, each mip filtered down into the next, then each filtered up and added
// onto the next larger one (that pipeline blends additively). Joined after fullscreen.wgsl.

struct Params {
    // one texel of the source
    texel: vec2<f32>,
    knee: f32,
    cap: f32,
};

@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var src_sampler: sampler;
@group(0) @binding(2) var<uniform> params: Params;

fn tap(uv: vec2<f32>, x: f32, y: f32) -> vec3<f32> {
    return textureSampleLevel(src, src_sampler, uv + params.texel * vec2<f32>(x, y), 0.0).rgb;
}

// Light above the knee only, scaled down by how far it is above, so the hue stays; capped, so stacked additive
// effects glow instead of flooding the screen
@fragment
fn fs_prefilter(in: VertexOutput) -> @location(0) vec4<f32> {
    // four bilinear taps: a 4x4 box of the full-res scene
    let c = 0.25 * (tap(in.uv, -1.0, -1.0) + tap(in.uv, 1.0, -1.0) + tap(in.uv, -1.0, 1.0) + tap(in.uv, 1.0, 1.0));
    let l = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
    let bright = c * max(l - params.knee, 0.0) / max(l, 1e-4);
    return vec4<f32>(min(bright, vec3<f32>(params.cap)), 1.0);
}

// 13 taps in overlapping boxes: wide and stable under motion
@fragment
fn fs_down(in: VertexOutput) -> @location(0) vec4<f32> {
    let a = tap(in.uv, -2.0, -2.0);
    let b = tap(in.uv, 0.0, -2.0);
    let c = tap(in.uv, 2.0, -2.0);
    let d = tap(in.uv, -1.0, -1.0);
    let e = tap(in.uv, 1.0, -1.0);
    let f = tap(in.uv, -2.0, 0.0);
    let g = tap(in.uv, 0.0, 0.0);
    let h = tap(in.uv, 2.0, 0.0);
    let i = tap(in.uv, -1.0, 1.0);
    let j = tap(in.uv, 1.0, 1.0);
    let k = tap(in.uv, -2.0, 2.0);
    let l = tap(in.uv, 0.0, 2.0);
    let m = tap(in.uv, 2.0, 2.0);
    var sum = (d + e + i + j) * 0.125;
    sum += (a + b + f + g) * 0.03125;
    sum += (b + c + g + h) * 0.03125;
    sum += (f + g + k + l) * 0.03125;
    sum += (g + h + l + m) * 0.03125;
    return vec4<f32>(sum, 1.0);
}

// 3x3 tent
@fragment
fn fs_up(in: VertexOutput) -> @location(0) vec4<f32> {
    var sum = tap(in.uv, 0.0, 0.0) * 4.0;
    sum += (tap(in.uv, -1.0, 0.0) + tap(in.uv, 1.0, 0.0) + tap(in.uv, 0.0, -1.0) + tap(in.uv, 0.0, 1.0)) * 2.0;
    sum += tap(in.uv, -1.0, -1.0) + tap(in.uv, 1.0, -1.0) + tap(in.uv, -1.0, 1.0) + tap(in.uv, 1.0, 1.0);
    return vec4<f32>(sum / 16.0, 1.0);
}
