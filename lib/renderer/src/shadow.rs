//! HD sun shadows: one shadow map seen from the sun (or the moon, faintly), centred on the camera's focus, drawn
//! only when something changed. The ground and the map's models cast; the terrain, models and grass look it up.
//! While it runs the map's baked shadows (one fixed sun, in the lightmap alpha) step aside.

use glam::{Mat4, Vec3};

/// Half the box's width (world units, 40 cells), how far back up the sun the eye sits, and its depth range.
pub const HALF_EXTENT: f32 = 400.0;
const EYE_DISTANCE: f32 = 750.0;
const NEAR: f32 = 5.0;
const FAR: f32 = 2000.0;
/// Redraw when the focus moved this far (2 cells), the sun turned this much, or this many frames passed with
/// animated models about.
const REDRAW_DISTANCE: f32 = 20.0;
const REDRAW_DEGREES: f32 = 0.5;
const REDRAW_FRAMES: u32 = 30;
/// At night the moon's shadows are this much weaker.
const NIGHT_WEAKEN: f32 = 0.6;
/// The caster pass's depth bias (old port's polygon offset 2, 4).
pub const DEPTH_BIAS: wgpu::DepthBiasState = wgpu::DepthBiasState {
    constant: 4,
    slope_scale: 2.0,
    clamp: 0.0,
};
pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// The casters, depth only.
pub const SHADOW_DEPTH_WGSL: &str = include_str!("shaders/shadow_depth.wgsl");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShadowTier {
    pub size: u32,
    /// Lookups per pixel: 1 (the sampler's own 2×2 filter), 9 or 25
    pub taps: u32,
}

/// By HD preset (0 Low to 3 Ultra): none on Low.
pub fn tier(index: u8) -> Option<ShadowTier> {
    match index {
        0 => None,
        1 => Some(ShadowTier { size: 1024, taps: 1 }),
        2 => Some(ShadowTier { size: 2048, taps: 9 }),
        _ => Some(ShadowTier { size: 2048, taps: 25 }),
    }
}

/// Shadows run: HD effects on, outdoors, the row above 0 %, a preset that has them.
pub fn active(hd: bool, indoor: bool, strength: f32, tier: Option<ShadowTier>) -> bool {
    hd && !indoor && strength > 0.0 && tier.is_some()
}

pub struct ShadowBox {
    pub light_vp: Mat4,
    /// A shadow texel, world units
    pub texel: f32,
}

/// The box from the sun (`sun_dir` points toward it) round `focus`, its x/y snapped to whole texels so shadow
/// edges hold still as the focus walks.
pub fn light_box(sun_dir: Vec3, focus: Vec3, size_px: u32) -> ShadowBox {
    let dir = sun_dir.normalize();
    let up = if dir.z.abs() < 0.9 { Vec3::Z } else { Vec3::X };
    let mut view = Mat4::look_at_rh(focus + dir * EYE_DISTANCE, focus, up);
    let texel = 2.0 * HALF_EXTENT / size_px as f32;
    view.w_axis.x = (view.w_axis.x / texel).round() * texel;
    view.w_axis.y = (view.w_axis.y / texel).round() * texel;
    let proj = Mat4::orthographic_rh(-HALF_EXTENT, HALF_EXTENT, -HALF_EXTENT, HALF_EXTENT, NEAR, FAR);
    ShadowBox {
        light_vp: proj * view,
        texel,
    }
}

/// What a shadow map was drawn for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RedrawState {
    pub center: Vec3,
    pub dir: Vec3,
    pub models_ready: bool,
    /// Bumped on a map load, a tier change or shadows turning on
    pub generation: u64,
}

/// The map needs drawing again: never drawn, a new map or tier, models just finished loading, the focus moved 2
/// cells, the sun turned 0.5°, or 30 frames with animated models about.
pub fn needs_redraw(last: Option<&RedrawState>, now: &RedrawState, frames_since: u32, animated: bool) -> bool {
    let Some(last) = last else {
        return true;
    };
    last.generation != now.generation
        || (!last.models_ready && now.models_ready)
        || last.center.distance(now.center) > REDRAW_DISTANCE
        || last.dir.angle_between(now.dir).to_degrees() > REDRAW_DEGREES
        || (animated && frames_since >= REDRAW_FRAMES)
}

/// The light at a point the shadow covers by `shadow` (0 to 1): it takes the sun's part away at `strength`, never
/// the sky's; at night (the moon) it is weaker.
pub fn sun_light(total: [f32; 3], ambient: [f32; 3], shadow: f32, strength: f32, night: f32) -> [f32; 3] {
    let k = shadow * strength * (1.0 - NIGHT_WEAKEN * night);
    if k <= 0.0 {
        return total;
    }
    std::array::from_fn(|i| {
        let amb = ambient[i].min(total[i]);
        amb + (total[i] - amb) * (1.0 - k)
    })
}

/// The receivers' view of the shadow map.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShadowUniform {
    pub light_vp: [[f32; 4]; 4],
    /// strength (night applied), texel (world units), lookups, on (0/1)
    pub params: [f32; 4],
}

impl ShadowUniform {
    pub const OFF: Self = Self {
        light_vp: [[0.0; 4]; 4],
        params: [0.0; 4],
    };
}

/// The shadow map and the caster pass's bindings.
pub struct ShadowMap {
    pub tier: ShadowTier,
    texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pass_buffer: wgpu::Buffer,
    pub pass_bind_group: wgpu::BindGroup,
}

impl ShadowMap {
    pub fn new(device: &wgpu::Device, pass_layout: &wgpu::BindGroupLayout, tier: ShadowTier) -> Self {
        let texture = depth_texture(device, tier.size);
        let view = texture.create_view(&Default::default());
        let pass_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shadow_pass"),
            size: std::mem::size_of::<[[f32; 4]; 4]>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let pass_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow_pass"),
            layout: pass_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: pass_buffer.as_entire_binding(),
            }],
        });
        Self {
            tier,
            texture,
            view,
            pass_buffer,
            pass_bind_group,
        }
    }

    pub fn write_pass(&self, queue: &wgpu::Queue, light_vp: Mat4) {
        queue.write_buffer(&self.pass_buffer, 0, bytemuck::cast_slice(&light_vp.to_cols_array()));
    }

    /// The caster pass: cleared to the far plane; the casters draw into it.
    pub fn begin<'e>(&self, encoder: &'e mut wgpu::CommandEncoder) -> wgpu::RenderPass<'e> {
        let _ = &self.texture;
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("shadow"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        })
    }
}

/// A depth texture the casters draw into and the receivers sample.
pub fn depth_texture(device: &wgpu::Device, size: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("shadow_map"),
        size: wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

/// The caster pass's own bind group layout (group 0): the light's view-projection. The shadow map can't be bound
/// to a pass that draws into it, so the casters don't use the world's group 0.
pub fn pass_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("shadow_pass"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

/// A depth-only caster pipeline for one vertex layout (`entry`: `vs_ground`, `vs_model`, `vs_animated`); `nodes`
/// is the animated models' node-matrix layout (group 2).
pub fn caster_pipeline(
    device: &wgpu::Device,
    pass_layout: &wgpu::BindGroupLayout,
    texture_layout: &wgpu::BindGroupLayout,
    nodes: Option<&wgpu::BindGroupLayout>,
    vertex: wgpu::VertexBufferLayout<'static>,
    entry: &str,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("shadow_depth"),
        source: wgpu::ShaderSource::Wgsl(SHADOW_DEPTH_WGSL.into()),
    });
    let mut layouts = vec![pass_layout, texture_layout];
    layouts.extend(nodes);
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("shadow_caster"),
        bind_group_layouts: &layouts,
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("shadow_caster"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some(entry),
            buffers: &[vertex],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_alpha"),
            targets: &[],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: Default::default(),
            bias: DEPTH_BIAS,
        }),
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    })
}
