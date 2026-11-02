// HD sun shadows (shadow.rs): the casters' depth seen from the sun. Depth only; leaves and fences cut out by their
// texture's alpha.

struct PassUniforms {
    light_vp: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> pass_u: PassUniforms;
@group(1) @binding(0) var caster_texture: texture_2d<f32>;
@group(1) @binding(1) var caster_sampler: sampler;
// Animated models' node matrices, 4 texels a matrix, 2048 matrices a row (matrix_texture.rs)
@group(2) @binding(0) var node_texture: texture_2d<f32>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
};

fn node_matrix(slot: u32) -> mat4x4<f32> {
    let x = i32((slot % 2048u) * 4u);
    let y = i32(slot / 2048u);
    return mat4x4<f32>(
        textureLoad(node_texture, vec2<i32>(x, y), 0),
        textureLoad(node_texture, vec2<i32>(x + 1, y), 0),
        textureLoad(node_texture, vec2<i32>(x + 2, y), 0),
        textureLoad(node_texture, vec2<i32>(x + 3, y), 0),
    );
}

@vertex
fn vs_ground(@location(0) position: vec3<f32>, @location(2) tex_coord: vec2<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = pass_u.light_vp * vec4<f32>(position, 1.0);
    out.tex_coord = tex_coord;
    return out;
}

@vertex
fn vs_model(@location(0) position: vec3<f32>, @location(2) tex_coord: vec2<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = pass_u.light_vp * vec4<f32>(position, 1.0);
    out.tex_coord = tex_coord;
    return out;
}

@vertex
fn vs_animated(
    @location(0) position: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
    @location(4) node_slot: u32,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = pass_u.light_vp * (node_matrix(node_slot) * vec4<f32>(position, 1.0));
    out.tex_coord = tex_coord;
    return out;
}

@fragment
fn fs_alpha(in: VertexOutput) {
    if (textureSample(caster_texture, caster_sampler, in.tex_coord).a < 0.5) {
        discard;
    }
}
