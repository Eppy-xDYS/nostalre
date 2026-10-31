struct CameraUniforms {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    eye_pos: vec4<f32>,
};

struct LightUniforms {
    light_dir: vec4<f32>,
    diffuse_color: vec4<f32>,
    ambient_color: vec4<f32>,
    shadow_strength: f32,
    glow_lamp: f32,
    glow_pools: f32,
    glow_sun: f32,
    // HD scene light (scene_light.rs): on, night 0..1, cloud strength, terrain lamp cap; all 0: today's look
    scene: vec4<f32>,
    // cloud drift offset (xy)
    cloud: vec4<f32>,
    // the map's day sun + sky, for the night lift
    day_light: vec4<f32>,
    _reserved: vec4<f32>,
};

struct PointLight {
    position: vec4<f32>,
    color_range: vec4<f32>,
};

struct FogUniforms {
    color: vec4<f32>,
    near: f32,
    far: f32,
    enabled: f32,
};

struct CellLightUniforms {
    uv_scale: vec2<f32>,
    uv_bias: vec2<f32>,
    enabled: f32,
};

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> light: LightUniforms;
struct PointLights {
    count: u32,
    lights: array<PointLight, 32>, // MAX_POINT_LIGHTS in global_uniforms.rs
};

@group(0) @binding(2) var<uniform> point_lights: PointLights;
@group(0) @binding(3) var<uniform> fog: FogUniforms;
@group(0) @binding(4) var cell_light_texture: texture_2d<f32>;
@group(0) @binding(5) var cell_light_sampler: sampler;
@group(0) @binding(6) var<uniform> cell_light: CellLightUniforms;
@group(1) @binding(0) var model_texture: texture_2d<f32>;
@group(1) @binding(1) var model_sampler: sampler;

fn cell_light_at(world_pos: vec3<f32>) -> vec3<f32> {
    if (cell_light.enabled <= 0.0) {
        return vec3<f32>(0.0);
    }
    let uv = vec2<f32>(world_pos.x, world_pos.z) * cell_light.uv_scale + cell_light.uv_bias;
    if (any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0))) {
        return vec3<f32>(0.0);
    }
    return textureSample(cell_light_texture, cell_light_sampler, uv).rgb;
}

// HD world glow, twins of post/glow.rs: a bounded extra above white for the bloom; the levels are 0 without bloom.
const LAMP_GAIN: f32 = 1.5;
const SUN_GAIN: f32 = 0.5;
const SUN_KNEE: f32 = 0.8;

// emissive 1: the texture is named for a lamp; 0.5: only its model is, so only near-white texels glow
fn lamp_mask(c: vec3<f32>, emissive: f32) -> f32 {
    let peak = max(c.r, max(c.g, c.b));
    if (emissive > 0.75) {
        return smoothstep(0.55, 0.85, peak);
    }
    if (emissive > 0.25) {
        return smoothstep(0.8, 0.95, peak);
    }
    return 0.0;
}

// sunlit highlights, near-white colours only: `sun_lit` is how directly the sun falls (0 to 1), scaled by the
// sun's brightness
fn sun_extra(color: vec3<f32>, sun_lit: f32) -> vec3<f32> {
    let sun = dot(light.diffuse_color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let near_white = smoothstep(SUN_KNEE, 1.0, max(color.r, max(color.g, color.b)));
    return color * near_white * sun_lit * sun * light.glow_sun * SUN_GAIN;
}

// HD scene light, twins of scene_light.rs. `light.scene` all 0 leaves today's look.
const LUMA: vec3<f32> = vec3<f32>(0.2126, 0.7152, 0.0722);
const LM_DESATURATE: f32 = 0.11;
const LM_ROLLOFF: f32 = 3.5;
// noise units per world unit: about 14 cells a cloud
const CLOUD_SCALE: f32 = 0.014;
const CLOUD_DARK: f32 = 0.55;

// a baked light pulled toward its brightest channel at the same luma, then rolled off when bright
fn lightmap_treat(lm: vec3<f32>) -> vec3<f32> {
    let top = max(lm.r, max(lm.g, lm.b));
    let neutral = mix(lm, vec3<f32>(top), LM_DESATURATE);
    let y1 = dot(neutral, LUMA);
    let kept = neutral * select(1.0, dot(lm, LUMA) / y1, y1 > 1e-6);
    return kept / (1.0 + LM_ROLLOFF * dot(kept, LUMA));
}

fn cloud_hash(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = cloud_hash(i);
    let b = cloud_hash(i + vec2<f32>(1.0, 0.0));
    let c = cloud_hash(i + vec2<f32>(0.0, 1.0));
    let d = cloud_hash(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// drifting cloud shadows over the world XZ: a factor on the lit colour; 2 octaves, 3 on Ultra (lamp cap 32)
fn cloud_shadow(world: vec3<f32>) -> f32 {
    if (light.scene.z <= 0.0) {
        return 1.0;
    }
    var p = world.xz * CLOUD_SCALE + light.cloud.xy;
    let octaves = select(2, 3, light.scene.w >= 32.0);
    var v = 0.0;
    var a = 0.5;
    var total = 0.0;
    for (var i = 0; i < octaves; i++) {
        v += a * value_noise(p);
        total += a;
        p *= 2.0;
        a *= 0.5;
    }
    let cloud = smoothstep(0.38, 0.68, v / total * 0.9375);
    return mix(1.0, CLOUD_DARK, cloud * light.scene.z);
}

// at night, a pixel in a baked light pool goes back toward its daylit look: lamps and the walls round them stay lit
fn night_lift(lit: vec3<f32>, surface: vec3<f32>, lm: vec3<f32>) -> vec3<f32> {
    let k = smoothstep(0.02, 0.2, max(lm.r, max(lm.g, lm.b))) * light.scene.y;
    return mix(lit, surface * light.day_light.rgb + lm, k);
}

fn linear_to_srgb(c: vec3<f32>) -> vec3<f32> {
    let lo = c * 12.92;
    let hi = 1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(hi, lo, c <= vec3<f32>(0.0031308));
}

fn apply_fog(color: vec3<f32>, view_pos: vec3<f32>) -> vec3<f32> {
    if (fog.enabled <= 0.0) {
        return color;
    }
    let fog_amount = clamp(
        (length(view_pos) - fog.near) / (fog.far - fog.near),
        0.0,
        1.0,
    );
    return mix(color, fog.color.rgb, fog_amount);
}

fn point_light_attenuation(d: f32, r: f32) -> f32 {
    let n = min(d, r) / (r + 1e-4);
    let a = saturate(1.0 - n * n);
    return a * a;
}

fn point_light_contribution(world_pos: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    var acc = vec3<f32>(0.0);
    let count = point_lights.count;
    for (var i: u32 = 0u; i < count; i = i + 1u) {
        let lp = point_lights.lights[i].position.xyz;
        let lc = point_lights.lights[i].color_range.rgb;
        let lr = point_lights.lights[i].color_range.a;
        if (lr <= 0.0) { continue; }
        let to_frag = world_pos - lp;
        let d = length(to_frag);
        if (d >= lr) { continue; }
        let dir = to_frag / max(d, 1e-4);
        let lambert = max(-dot(dir, normal), 0.0);
        acc += lc * lambert * point_light_attenuation(d, lr);
    }
    return acc;
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
    @location(3) alpha: f32,
    @location(4) lit_scale: f32,
    @location(5) unlit: f32,
    @location(6) emissive: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) alpha: f32,
    @location(3) world_position: vec3<f32>,
    @location(4) view_pos: vec3<f32>,
    @location(5) lit_scale: f32,
    @location(6) unlit: f32,
    @location(7) emissive: f32,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(in.position, 1.0);
    out.tex_coord = in.tex_coord;
    out.normal = in.normal;
    out.alpha = in.alpha;
    out.world_position = in.position;
    out.lit_scale = in.lit_scale;
    out.unlit = in.unlit;
    out.emissive = in.emissive;
    let view_pos = camera.view * vec4<f32>(in.position, 1.0);
    out.view_pos = view_pos.xyz;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(model_texture, model_sampler, in.tex_coord);

    if tex_color.a < 0.81 {
        discard;
    }

    let n_dot_l = max(dot(normalize(in.normal), normalize(light.light_dir.xyz)), 0.0);
    let diffuse = light.diffuse_color.rgb * n_dot_l;
    let shaded = clamp(diffuse + light.ambient_color.rgb, vec3<f32>(0.0), vec3<f32>(1.0))
        * in.lit_scale;
    let lighting = mix(shaded, vec3<f32>(1.0), in.unlit);

    let tex_gamma = linear_to_srgb(tex_color.rgb);
    var cell = cell_light_at(in.world_position);
    if (light.scene.x > 0.0) {
        cell = lightmap_treat(cell);
    }
    var color = tex_gamma * (lighting + cell);
    let pl = point_light_contribution(in.world_position, normalize(in.normal));
    color += tex_gamma * pl;
    color *= mix(cloud_shadow(in.world_position), 1.0, in.unlit);
    if (light.scene.y > 0.0) {
        // lamps and walls standing in a baked light pool keep its light at night
        color = night_lift(color + cell * light.scene.y, tex_gamma, cell);
    }

    color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));
    color += tex_gamma * lamp_mask(tex_gamma, in.emissive) * light.glow_lamp * LAMP_GAIN;
    color += sun_extra(color, n_dot_l * in.lit_scale * (1.0 - in.unlit));
    color = apply_fog(color, in.view_pos);

    return vec4<f32>(color, tex_color.a * in.alpha);
}
