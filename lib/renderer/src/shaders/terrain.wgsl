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

struct PointLights {
    count: u32,
    lights: array<PointLight, 32>, // MAX_POINT_LIGHTS in global_uniforms.rs
};

struct FogUniforms {
    color: vec4<f32>,
    near: f32,
    far: f32,
    enabled: f32,
};

// The map's water, for caustics on the bed under it; strength 0 = none.
struct WaterGlobals {
    level: f32,
    time: f32,
    strength: f32,
    cell_size: f32,
    // the water's wave speed: the bed's light moves with the waves above it
    wave_speed: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> light: LightUniforms;
@group(0) @binding(2) var<uniform> point_lights: PointLights;
@group(0) @binding(3) var<uniform> fog: FogUniforms;
@group(0) @binding(7) var<uniform> water_globals: WaterGlobals;
@group(1) @binding(0) var ground_texture: texture_2d<f32>;
@group(1) @binding(1) var ground_sampler: sampler;
@group(2) @binding(0) var lightmap_texture: texture_2d<f32>;
@group(2) @binding(1) var lightmap_sampler: sampler;

// HD world glow, twins of post/glow.rs: a bounded extra above white for the bloom; the levels are 0 without bloom.
const POOL_THRESHOLD: f32 = 0.16;
const POOL_GAIN: f32 = 0.6;
const SUN_GAIN: f32 = 0.5;
const SUN_KNEE: f32 = 0.8;

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
    let k = smoothstep(0.08, 0.18, max(lm.r, max(lm.g, lm.b))) * light.scene.y;
    return mix(lit, surface * light.day_light.rgb + lm, k);
}

const BAKED_SHADOW: f32 = 0.6;
const PL_FALLOFF: f32 = 40.0;
const KNEE: f32 = 0.70;
const KNEE_LIMIT: f32 = 1.15;
const LAMP_SCALE: f32 = 0.8;

// the share of light a baked shadow leaves: floors only, the usual lit level (alpha 0.65 and up) fully lit
fn baked_shadow(alpha: f32, normal_up: f32) -> f32 {
    let floor_part = smoothstep(0.35, 0.70, normal_up);
    let shade = smoothstep(0.20, 0.65, clamp(alpha, 0.0, 1.0));
    return 1.0 + (shade - 1.0) * BAKED_SHADOW * floor_part;
}

// windowed inverse square: 1 at the lamp, 0 at its range
fn point_atten(d: f32, range: f32) -> f32 {
    if (range <= 0.0 || d >= range) {
        return 0.0;
    }
    let t2 = (d / range) * (d / range);
    let window = 1.0 - t2 * t2;
    return window * window / (1.0 + PL_FALLOFF * t2);
}

// summed lamps, overlapping hot ones rolled off instead of clipped
fn light_knee(sum: vec3<f32>) -> vec3<f32> {
    let e = max(sum.r, max(sum.g, sum.b));
    var scale = 1.0;
    if (e > KNEE) {
        let span = KNEE_LIMIT - KNEE;
        scale = (KNEE + span * (1.0 - exp(-(e - KNEE) / span))) / e;
    }
    return sum * scale * LAMP_SCALE;
}

// the map lamps nearest the camera (up to the preset's cap), plus the torch in slot 0 at night
fn lamps(world: vec3<f32>) -> vec3<f32> {
    let torch = select(0u, 1u, light.scene.y > 0.0);
    let count = min(point_lights.count, u32(light.scene.w) + torch);
    var sum = vec3<f32>(0.0);
    for (var i = 0u; i < count; i++) {
        let l = point_lights.lights[i];
        let d = length(world - l.position.xyz);
        if (d < l.color_range.a) {
            sum += l.color_range.rgb * point_atten(d, l.color_range.a);
        }
    }
    return light_knee(sum);
}

// HD sun shadows (shadow.rs): the map seen from the sun, its compare sampler, and strength (night applied), texel,
// lookups, on (0/1). Off: no lookups, today's look.
struct ShadowUniforms {
    light_vp: mat4x4<f32>,
    params: vec4<f32>,
};

@group(0) @binding(8) var shadow_map: texture_depth_2d;
@group(0) @binding(9) var shadow_sampler: sampler_comparison;
@group(0) @binding(10) var<uniform> shadow: ShadowUniforms;

// How much of the sun `world` loses, 0 to 1 (0 while shadows are off); `taps`: 1 (the sampler's own 2x2 filter),
// 9 or 25. Fades out over the box's last tenth so its edge never shows.
fn sun_shadow_taps(world: vec3<f32>, n_dot_l: f32, taps: f32) -> f32 {
    if (shadow.params.w <= 0.0) {
        return 0.0;
    }
    let c = shadow.light_vp * vec4<f32>(world, 1.0);
    let p = c.xyz / c.w;
    let edge = max(abs(p.x), abs(p.y));
    if (edge >= 1.0 || p.z <= 0.0 || p.z >= 1.0) {
        return 0.0;
    }
    let uv = vec2<f32>(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
    let depth = p.z - max(0.004 * (1.0 - clamp(n_dot_l, 0.0, 1.0)), 0.001);
    var lit = 0.0;
    if (taps < 2.0) {
        lit = textureSampleCompareLevel(shadow_map, shadow_sampler, uv, depth);
    } else {
        let r = select(1, 2, taps > 10.0);
        let step = 1.0 / vec2<f32>(textureDimensions(shadow_map));
        var n = 0.0;
        for (var y = -r; y <= r; y++) {
            for (var x = -r; x <= r; x++) {
                let at = uv + vec2<f32>(f32(x), f32(y)) * step;
                lit += textureSampleCompareLevel(shadow_map, shadow_sampler, at, depth);
                n += 1.0;
            }
        }
        lit /= n;
    }
    return (1.0 - lit) * (1.0 - smoothstep(0.9, 1.0, edge));
}

fn sun_shadow(world: vec3<f32>, n_dot_l: f32) -> f32 {
    return sun_shadow_taps(world, n_dot_l, shadow.params.z);
}

// the sun's part of `total` taken away by shadow `s` at the row's strength; the sky's part stays
fn shadowed(total: vec3<f32>, ambient: vec3<f32>, s: f32) -> vec3<f32> {
    let amb = min(total, ambient);
    return amb + (total - amb) * (1.0 - s * shadow.params.x);
}

// the map's baked shadow (lightmap alpha, one fixed sun) steps aside while sun shadows run
fn baked_alpha(a: f32) -> f32 {
    return select(a, 1.0, shadow.params.w > 0.0);
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

// Water bends sunlight toward the normal as it enters (Snell's law, water 1.333). Under a surface that
// bulges up the rays meet and the bed is brighter; under a dip they part and it is darker. For gentle waves
// the light reaching the bed is about 1 - (1 - 1/1.333) * depth * Laplacian(height): no rays traced, and
// the same waves as the water (water_waves.wgsl), so the light moves with the surface you see.
const WATER_FOCUS: f32 = 0.25;
// gentle RO waves bend light too little to see; this scales the focusing, not where it falls
const CAUSTIC_GAIN: f32 = 6.0;

// Light on ground under the water, relative to the sun reaching it: positive where the waves focus it,
// negative where they spread it; fading in from the waterline and out with depth.
fn caustics(world: vec3<f32>) -> f32 {
    let depth = (world.y - water_globals.level) / water_globals.cell_size;
    if (water_globals.strength <= 0.0 || depth <= 0.0) {
        return 0.0;
    }
    let p = world.xz / water_globals.cell_size;
    let speed = wave_drift_speed(water_globals.wave_speed);
    // the water's own wave size for this depth (water_hd.wgsl): shallow water carries small waves
    let amp = mix(0.08, 0.30, smoothstep(0.0, 1.2, depth));
    let bend = wave_laplacian(p, water_globals.time, speed) * amp;
    let focus = 1.0 - WATER_FOCUS * CAUSTIC_GAIN * min(depth, 2.5) * bend;
    // focused light gathers into thin bright lines, spread light only dims a little
    let light = clamp(focus, 0.0, 3.0);
    let c = select(light - 1.0, (light * light - 1.0) * 0.5, light > 1.0);
    let fade = smoothstep(0.0, 0.3, depth) * exp(-depth * 0.45);
    return c * fade * water_globals.strength;
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
    @location(3) lightmap_coord: vec2<f32>,
    @location(4) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) lightmap_coord: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) color: vec4<f32>,
    @location(4) world_position: vec3<f32>,
    @location(5) view_pos: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(in.position, 1.0);
    out.tex_coord = in.tex_coord;
    out.lightmap_coord = in.lightmap_coord;
    out.normal = in.normal;
    out.color = in.color;
    out.world_position = in.position;
    let view_pos = camera.view * vec4<f32>(in.position, 1.0);
    out.view_pos = view_pos.xyz;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(ground_texture, ground_sampler, in.tex_coord);

    // It seems map makers fill the void around a map with cells whose UVs point at a
    // colour-keyed patch, so a cell that keys out must leave the background.
    if tex_color.a < 0.81 {
        discard;
    }

    let lightmap = textureSample(lightmap_texture, lightmap_sampler, in.lightmap_coord);

    let sunlight = light.diffuse_color.rgb;
    let ambient = light.ambient_color.rgb;

    let normal = normalize(in.normal);
    let n_dot_l = max(dot(normal, normalize(light.light_dir.xyz)), 0.0);
    let shade = baked_alpha(lightmap.a);
    let tex_gamma = linear_to_srgb(tex_color.rgb);
    let sun_lost = sun_shadow(in.world_position, n_dot_l);
    var combined_light = shadowed(
        clamp(sunlight * n_dot_l + ambient, vec3<f32>(0.0), vec3<f32>(1.0)),
        ambient,
        sun_lost,
    );
    var baked = lightmap.rgb;
    var lamp = vec3<f32>(0.0);
    if (light.scene.x > 0.0) {
        // softer bakes, shadows on floors only (up is -y), the map's lamps on the ground
        combined_light *= baked_shadow(shade, -normal.y);
        baked = lightmap_treat(lightmap.rgb);
    } else {
        combined_light *= shade;
    }
    if (light.scene.x > 0.0 || light.scene.y > 0.0) {
        lamp = in.color.rgb * tex_gamma * lamps(in.world_position);
    }

    var color = clamp(
        in.color.rgb * combined_light * tex_gamma + baked + lamp,
        vec3<f32>(0.0),
        vec3<f32>(1.0),
    );
    color *= cloud_shadow(in.world_position);
    if (light.scene.y > 0.0) {
        color = night_lift(color, in.color.rgb * tex_gamma, baked);
    }

    // sunlight focused by the waves, where the sun reaches the bed
    color += sunlight * n_dot_l * shade * (1.0 - sun_lost) * caustics(in.world_position);
    color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));

    let pool = max(lightmap.rgb - vec3<f32>(POOL_THRESHOLD), vec3<f32>(0.0)) / (1.0 - POOL_THRESHOLD);
    color += pool * light.glow_pools * POOL_GAIN;
    color += sun_extra(color, n_dot_l * shade * (1.0 - sun_lost));

    color = apply_fog(color, in.view_pos);

    let alpha = tex_color.a * in.color.a;

    return vec4<f32>(color, alpha);
}
