// The map lamps (and the player's torch) on a surface, as the terrain and the grass sum them, so grass under
// a lamp is lit like the ground it grows on. Prepended to terrain.wgsl and grass.wgsl (`TERRAIN_WGSL`,
// `GRASS_WGSL`); each declares its own `light` (LightUniforms, binding 1). Values as scene_light.rs.

struct PointLight {
    position: vec4<f32>,
    color_range: vec4<f32>,
};

struct PointLights {
    count: u32,
    lights: array<PointLight, 32>, // MAX_POINT_LIGHTS in global_uniforms.rs
};

@group(0) @binding(2) var<uniform> point_lights: PointLights;

const PL_FALLOFF: f32 = 40.0;
const KNEE: f32 = 0.70;
const KNEE_LIMIT: f32 = 1.15;
const LAMP_SCALE: f32 = 0.8;

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
