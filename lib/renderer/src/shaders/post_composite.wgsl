// HD composite: the scene target plus bloom, through the shoulder, the colour grade and the vignette, to the
// output. The scene is the same size, so its texels are read directly; bloom is half size or smaller and filtered
// up. `post/grade.rs` has the same steps on the CPU. Joined after fullscreen.wgsl.

struct Params {
    // bloom intensity, curve white (0: no curve), vignette, exposure
    values: vec4<f32>,
    // temperature, tint, saturation, contrast
    balance: vec4<f32>,
    // lift rgb, gamma
    lift_gamma: vec4<f32>,
    // gain rgb, -
    gain: vec4<f32>,
};

@group(0) @binding(0) var scene: texture_2d<f32>;
@group(0) @binding(1) var bloom: texture_2d<f32>;
@group(0) @binding(2) var bloom_sampler: sampler;
@group(0) @binding(3) var<uniform> params: Params;

const KNEE: f32 = 0.9;

fn shoulder(x: f32, white: f32) -> f32 {
    if (white <= 1.0 || x <= KNEE) {
        return min(x, 1.0);
    }
    let span = 1.0 - KNEE;
    let reach = 1.0 - exp(-(x - KNEE) / span);
    let reach_white = 1.0 - exp(-(white - KNEE) / span);
    return min(KNEE + span * reach / reach_white, 1.0);
}

fn grade(c: vec3<f32>) -> vec3<f32> {
    let temperature = params.balance.x;
    let balance = vec3<f32>(1.0 + 0.08 * temperature, 1.0 + 0.06 * params.balance.y, 1.0 - 0.08 * temperature);
    var v = c * balance;
    v = v + params.lift_gamma.rgb * (1.0 - v);
    v = pow(max(v, vec3<f32>(0.0)), vec3<f32>(1.0 / params.lift_gamma.a)) * params.gain.rgb;
    v = (v - 0.5) * params.balance.w + 0.5;
    let l = dot(v, vec3<f32>(0.2126, 0.7152, 0.0722));
    return max(vec3<f32>(l) + (v - vec3<f32>(l)) * params.balance.z, vec3<f32>(0.0));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureLoad(scene, vec2<i32>(in.clip_position.xy), 0).rgb;
    let glow = textureSampleLevel(bloom, bloom_sampler, in.uv, 0.0).rgb;
    // never below 0, as `tone_map` on the CPU: a negative peak would scale to white
    let lit = max((color + glow * params.values.x) * params.values.w, vec3<f32>(0.0));
    // the curve on the brightest channel: bright effects keep their hue instead of washing out to white
    let peak = max(lit.r, max(lit.g, lit.b));
    let curved = lit * (shoulder(peak, params.values.y) / max(peak, 1e-4));
    let r = length(in.uv * 2.0 - 1.0);
    let edge = 1.0 - params.values.z * smoothstep(0.5, 1.5, r);
    return vec4<f32>(clamp(grade(curved) * edge, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
