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
    // world glow (unused here)
    _glow_lamp: f32,
    _glow_pools: f32,
    _glow_sun: f32,
    // HD scene light (scene_light.rs): on, night 0..1, cloud strength, terrain lamp cap; all 0: today's look
    scene: vec4<f32>,
    // cloud drift offset (xy)
    cloud: vec4<f32>,
    // the map's day sun + sky, for the night lift
    day_light: vec4<f32>,
    _reserved: vec4<f32>,
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

struct GrassUniforms {
    // the camera target on the ground: tufts shrink away toward `far` from it
    focus: vec2<f32>,
    time: f32,
    far: f32,
    wind: vec2<f32>,
    // 2, or 3 when some tufts get a third quad
    max_quads: f32,
    // 1 / the art's mean brightness: dark and bright art come out alike
    gain: f32,
    variants: f32,
    // 1: the art's own colours count (a picture); 0: grey art the tint colours
    colored: f32,
    // width / height of one variant
    aspect: f32,
    _pad: f32,
};

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> light: LightUniforms;
@group(0) @binding(3) var<uniform> fog: FogUniforms;
@group(0) @binding(4) var cell_light_texture: texture_2d<f32>;
@group(0) @binding(5) var cell_light_sampler: sampler;
@group(0) @binding(6) var<uniform> cell_light: CellLightUniforms;
@group(1) @binding(0) var art: texture_2d<f32>;
@group(1) @binding(1) var art_sampler: sampler;
@group(2) @binding(0) var<uniform> grass: GrassUniforms;

// twins of grass/scatter.rs
const SIZE_MIN: f32 = 0.85;
const SIZE_MAX: f32 = 1.6;
const MAX_HEIGHT: f32 = 9.5;
// a quad stands this share of its height above the root; the rest sinks into the ground so no tuft floats on a slope
const ABOVE: f32 = 0.88;
const MAX_LEAN: f32 = 0.16;
// wind push at the tip, in world units
const WIND: f32 = 1.2;
// share of the tuft over which the base dissolves into the ground
const ROOT_FADE: f32 = 0.25;
const SATURATION: f32 = 0.6;
const CORE_CUT: f32 = 0.5;
const PI: f32 = 3.14159265;

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) root: vec3<f32>,
    @location(1) packed: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tint: vec3<f32>,
    @location(2) seed: f32,
    @location(3) world_position: vec3<f32>,
    @location(4) view_pos: vec3<f32>,
    // above the root, in world units
    @location(5) rise: f32,
};

fn hash12(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn hash11(x: f32) -> f32 {
    return hash12(vec2<f32>(x, x * 1.618));
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let p = in.packed;
    let seed = hash12(in.root.xz);
    let quads = select(2.0, 3.0, grass.max_quads > 2.5 && hash11(seed * 91.0) > 0.45);
    let quad = in.vertex_index / 6u;
    if (f32(quad) >= quads) {
        // an unused third quad: outside the clip volume
        out.clip_position = vec4<f32>(2.0, 2.0, 2.0, 1.0);
        return out;
    }

    let tint = vec3<f32>(f32((p >> 11u) & 31u) / 31.0, f32((p >> 5u) & 63u) / 63.0, f32(p & 31u) / 31.0);
    let yaw = f32((p >> 16u) & 63u) / 64.0 * 2.0 * PI;
    let size = SIZE_MIN + f32((p >> 22u) & 31u) / 31.0 * (SIZE_MAX - SIZE_MIN);
    let variant = f32((p >> 27u) & 3u) % grass.variants;
    let lean_amount = f32((p >> 29u) & 7u) / 7.0 * MAX_LEAN;

    let dist = distance(in.root.xz, grass.focus);
    let grow = 1.0 - smoothstep(grass.far * 0.72, grass.far, dist);
    let tall = size / SIZE_MAX * MAX_HEIGHT / ABOVE;

    // corners 0, 1: tip left, right; 2, 3: root left, right
    var corners = array<u32, 6>(0u, 1u, 3u, 3u, 2u, 0u);
    let corner = corners[in.vertex_index % 6u];
    let tip = corner < 2u;
    let side = select(1.0, -1.0, corner == 0u || corner == 2u);
    let theta = yaw + f32(quad) * PI / quads;
    let across = vec2<f32>(cos(theta), sin(theta)) * (0.5 * tall * grass.aspect) * side;
    // heights grow downward: up is -y
    var offset = vec3<f32>(across.x, select(tall * (1.0 - ABOVE), -tall * ABOVE, tip), across.y);
    if (tip) {
        let lean_dir = yaw * 1.7 + seed * 6.2831853;
        offset.x += cos(lean_dir) * lean_amount * tall;
        offset.z += sin(lean_dir) * lean_amount * tall;
    }
    let flip = hash11(seed * 13.0 + f32(quad)) > 0.5;
    let u = select(1.0, 0.0, (side < 0.0) != flip);
    let uv = vec2<f32>((variant + u) / grass.variants, select(1.0, 0.0, tip));

    // the tip whips, the base stays planted
    let tip_share = 1.0 - uv.y;
    let phase = seed * 6.2831853;
    let gust = sin(grass.time * 1.7 + phase + (in.root.x + in.root.z) * 0.018);
    let flick = sin(grass.time * 4.3 + phase * 2.3) * 0.35;
    var world = in.root + offset * grow;
    world.x += grass.wind.x * tip_share * tip_share * WIND * grow * (gust * 0.5 + 0.5 + flick);
    world.z += grass.wind.y * tip_share * tip_share * WIND * grow * (gust * 0.5 + 0.5 + flick);

    out.clip_position = camera.view_proj * vec4<f32>(world, 1.0);
    out.uv = uv;
    out.tint = tint;
    out.seed = seed;
    out.world_position = world;
    out.view_pos = (camera.view * vec4<f32>(world, 1.0)).xyz;
    out.rise = in.root.y - world.y;
    return out;
}

// the ground's baked light (rgb) and shadow (a) under the tuft; none without lightmaps
fn cell_light_at(world_pos: vec3<f32>) -> vec4<f32> {
    if (cell_light.enabled <= 0.0) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    let uv = vec2<f32>(world_pos.x, world_pos.z) * cell_light.uv_scale + cell_light.uv_bias;
    if (any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0))) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    return textureSample(cell_light_texture, cell_light_sampler, uv);
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

fn apply_fog(color: vec3<f32>, view_pos: vec3<f32>) -> vec3<f32> {
    if (fog.enabled <= 0.0) {
        return color;
    }
    let fog_amount = clamp((length(view_pos) - fog.near) / (fog.far - fog.near), 0.0, 1.0);
    return mix(color, fog.color.rgb, fog_amount);
}

fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.299, 0.587, 0.114));
}

// coverage after the base dissolves along a ragged line, so no tuft meets the ground on a straight cut
fn coverage(in: VertexOutput, texel: vec4<f32>) -> f32 {
    let h = 1.0 - in.uv.y;
    let col = in.uv.x * 14.0 * grass.variants + in.seed * 37.0;
    let n = mix(hash11(floor(col)), hash11(floor(col) + 1.0), smoothstep(0.0, 1.0, fract(col)));
    let fade = ROOT_FADE * (0.35 + 1.3 * n);
    return texel.a * smoothstep(0.0, fade, h);
}

fn shade(in: VertexOutput, texel: vec4<f32>) -> vec3<f32> {
    // on ground that is not green, the art's own colour gives way to the ground's
    let green = clamp((in.tint.g / max(max(in.tint.r, in.tint.b), 0.001) - 1.0) * 6.0, 0.0, 1.0);
    let art_colour = mix(vec3<f32>(luma(texel.rgb)), texel.rgb, green * grass.colored);
    var surface = art_colour * in.tint * grass.gain * mix(1.0, 0.66, in.uv.y * in.uv.y);
    surface = mix(vec3<f32>(luma(surface)), surface, SATURATION);

    let cell = cell_light_at(in.world_position);
    // lit like the flat ground under it
    let n_dot_l = max(dot(vec3<f32>(0.0, -1.0, 0.0), normalize(light.light_dir.xyz)), 0.0);
    // one lookup: tufts sit in the shade of what stands over them
    let sun = shadowed(
        clamp(light.diffuse_color.rgb * n_dot_l + light.ambient_color.rgb, vec3<f32>(0.0), vec3<f32>(1.0)),
        light.ambient_color.rgb,
        sun_shadow_taps(in.world_position, n_dot_l, 1.0),
    );
    // the baked light fades with height above the root, as on models
    let cell_rgb = select(cell.rgb, lightmap_treat(cell.rgb), light.scene.x > 0.0);
    let baked = cell_rgb * clamp(1.0 - max(in.rise, 0.0) * 0.02, 0.15, 1.0);
    // the lamps go in with the sun and baked light, before cloud shadow and night lift, as on the terrain
    var lamp = vec3<f32>(0.0);
    if (light.scene.x > 0.0 || light.scene.y > 0.0) {
        lamp = surface * lamps(in.world_position);
    }
    var colour = clamp(surface * sun * baked_alpha(cell.a) + baked + lamp, vec3<f32>(0.0), vec3<f32>(1.0));
    colour *= cloud_shadow(in.world_position);
    if (light.scene.y > 0.0) {
        colour = clamp(night_lift(colour, surface, baked), vec3<f32>(0.0), vec3<f32>(1.0));
    }
    return apply_fog(colour, in.view_pos);
}

// the core: alpha tested, writes depth
@fragment
fn fs_core(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(art, art_sampler, in.uv);
    if (coverage(in, texel) < CORE_CUT) {
        discard;
    }
    return vec4<f32>(shade(in, texel), 1.0);
}

// the soft rim: the blade edges and the dissolving base, blended over the core without writing depth
@fragment
fn fs_rim(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(art, art_sampler, in.uv);
    let a = coverage(in, texel);
    if (a >= CORE_CUT || a < 0.02) {
        discard;
    }
    return vec4<f32>(shade(in, texel), a);
}
