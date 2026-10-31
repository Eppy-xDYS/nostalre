//! HD bloom: light above white (stacked additive effects) spread into a soft halo. A bright-pass at
//! half resolution, filtered down a chain of mips and back up, added to the scene before the composite clamps.

/// Luma a pixel must pass to bloom: only light brighter than white, so daylit walls and sky stay as they are.
pub const KNEE: f32 = 1.0;
/// A bright-pass channel's ceiling: stacked additive effects run far above white and would flood the screen.
pub const BRIGHT_CAP: f32 = 2.0;

/// The bloom passes, the shared full-screen vertex stage in front.
pub const BLOOM_WGSL: &str = concat!(
    include_str!("../shaders/fullscreen.wgsl"),
    include_str!("../shaders/bloom.wgsl")
);

fn luma(c: [f32; 3]) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

/// The bright-pass of `bloom.wgsl`: the part of `c` above `knee`, in the same hue, capped.
pub fn prefilter(c: [f32; 3], knee: f32) -> [f32; 3] {
    let l = luma(c);
    let scale = (l - knee).max(0.0) / l.max(1e-4);
    c.map(|v| (v * scale).min(BRIGHT_CAP))
}

/// The sizes of the chain's mips: half the surface, then halved again `levels - 1` times, never below 1.
pub fn mip_sizes(width: u32, height: u32, levels: u32) -> Vec<(u32, u32)> {
    let mut size = (width, height);
    (0..levels)
        .map(|_| {
            size = ((size.0 / 2).max(1), (size.1 / 2).max(1));
            size
        })
        .collect()
}

/// The mips worth running on a scene target: an 8-bit one never goes above the knee, so it gets none.
pub fn levels_for_target(format: wgpu::TextureFormat, levels: u32) -> u32 {
    if format == wgpu::TextureFormat::Rgba16Float {
        levels
    } else {
        0
    }
}

/// Whether the bloom passes run this frame.
pub fn should_run(effects: bool, intensity: f32, levels: u32) -> bool {
    effects && intensity > 0.0 && levels > 0
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct PassParams {
    texel: [f32; 2],
    knee: f32,
    cap: f32,
}

pub(super) const ADDITIVE: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent::REPLACE,
};

pub(super) fn linear_sampler(device: &wgpu::Device) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("bloom"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    })
}

/// A full-screen pipeline over one bind group layout.
pub(super) fn fullscreen_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    shader: &wgpu::ShaderModule,
    entry_point: &str,
    format: wgpu::TextureFormat,
    blend: Option<wgpu::BlendState>,
) -> wgpu::RenderPipeline {
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(entry_point),
        bind_group_layouts: &[layout],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(entry_point),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(entry_point),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    })
}

/// Bloom's pipelines: fixed for the session's scene format, kept across resizes.
pub(super) struct BloomPipelines {
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    prefilter: wgpu::RenderPipeline,
    down: wgpu::RenderPipeline,
    up: wgpu::RenderPipeline,
}

impl BloomPipelines {
    pub(super) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let entry = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty,
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bloom_pass"),
            entries: &[
                entry(
                    0,
                    wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                entry(1, wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)),
                entry(
                    2,
                    wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bloom"),
            source: wgpu::ShaderSource::Wgsl(BLOOM_WGSL.into()),
        });
        Self {
            prefilter: fullscreen_pipeline(device, &layout, &shader, "fs_prefilter", format, None),
            down: fullscreen_pipeline(device, &layout, &shader, "fs_down", format, None),
            up: fullscreen_pipeline(device, &layout, &shader, "fs_up", format, Some(ADDITIVE)),
            sampler: linear_sampler(device),
            layout,
        }
    }
}

struct Mip {
    view: wgpu::TextureView,
    size: (u32, u32),
}

/// One full-screen pass of the chain: what it reads and which mip it writes.
struct Pass {
    bind_group: wgpu::BindGroup,
    target: usize,
}

/// Bloom's mips and the passes between them, for one scene size.
pub(super) struct Bloom {
    mips: Vec<Mip>,
    prefilter: Pass,
    down: Vec<Pass>,
    up: Vec<Pass>,
}

impl Bloom {
    pub(super) fn new(
        device: &wgpu::Device,
        pipelines: &BloomPipelines,
        format: wgpu::TextureFormat,
        scene: &wgpu::TextureView,
        scene_size: (u32, u32),
        levels: u32,
    ) -> Self {
        use wgpu::util::DeviceExt;
        let mips: Vec<Mip> = mip_sizes(scene_size.0, scene_size.1, levels)
            .into_iter()
            .map(|size| {
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("bloom_mip"),
                    size: wgpu::Extent3d {
                        width: size.0,
                        height: size.1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                });
                Mip {
                    view: texture.create_view(&Default::default()),
                    size,
                }
            })
            .collect();
        let pass = |source: &wgpu::TextureView, source_size: (u32, u32), target: usize| {
            let params = PassParams {
                texel: [1.0 / source_size.0 as f32, 1.0 / source_size.1 as f32],
                knee: KNEE,
                cap: BRIGHT_CAP,
            };
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("bloom_pass"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("bloom_pass"),
                layout: &pipelines.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(source),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&pipelines.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: buffer.as_entire_binding(),
                    },
                ],
            });
            Pass { bind_group, target }
        };
        let prefilter = pass(scene, scene_size, 0);
        let down = (1..mips.len())
            .map(|i| pass(&mips[i - 1].view, mips[i - 1].size, i))
            .collect();
        let up = (0..mips.len().saturating_sub(1))
            .rev()
            .map(|i| pass(&mips[i + 1].view, mips[i + 1].size, i))
            .collect();
        Self {
            mips,
            prefilter,
            down,
            up,
        }
    }

    pub(super) fn mip_sizes(&self) -> Vec<(u32, u32)> {
        self.mips.iter().map(|m| m.size).collect()
    }

    /// The finished bloom, the size of the first mip.
    pub(super) fn output(&self) -> &wgpu::TextureView {
        &self.mips[0].view
    }

    fn run_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        pipeline: &wgpu::RenderPipeline,
        pass: &Pass,
        clear: bool,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("bloom"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.mips[pass.target].view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: if clear {
                        wgpu::LoadOp::Clear(wgpu::Color::BLACK)
                    } else {
                        wgpu::LoadOp::Load
                    },
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, &pass.bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }

    /// The bright-pass into the first mip.
    pub(super) fn prefilter(&self, encoder: &mut wgpu::CommandEncoder, pipelines: &BloomPipelines) {
        self.run_pass(encoder, &pipelines.prefilter, &self.prefilter, true);
    }

    pub(super) fn filter(&self, encoder: &mut wgpu::CommandEncoder, pipelines: &BloomPipelines) {
        for pass in &self.down {
            self.run_pass(encoder, &pipelines.down, pass, true);
        }
        for pass in &self.up {
            self.run_pass(encoder, &pipelines.up, pass, false);
        }
    }
}
