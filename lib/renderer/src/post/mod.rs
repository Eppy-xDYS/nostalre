//! HD frame: the scene draws into its own target, and one composite pass writes it to the output before the UI.

pub mod bloom;
pub mod glow;
pub mod grade;

use bloom::{Bloom, BloomPipelines};
use grade::{GradeFade, GradeParams};

/// The composite pass, the shared full-screen vertex stage in front.
pub const COMPOSITE_WGSL: &str = concat!(
    include_str!("../shaders/fullscreen.wgsl"),
    include_str!("../shaders/post_composite.wgsl")
);

/// HD options fixed for the session: the scene target's format is baked into every scene pipeline.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HdOptions {
    pub enabled: bool,
    /// A half-float target when the adapter can render, filter and blend one; 8-bit otherwise
    pub float_target: bool,
    /// Bloom's mip count, 0 for none
    pub bloom_levels: u32,
    pub bloom_intensity: f32,
    /// Colour grade strength, 0 to 1
    pub grade: f32,
    pub vignette: bool,
    /// World glow levels (lamp, pools, sun), 0 to 1 each
    pub glow: [f32; 3],
    /// Grass density, 0 to 1
    pub grass: f32,
    /// Grass quality, 0 Low to 3 Ultra
    pub grass_tier: u8,
}

/// The adapter can draw into, filter and blend a format with these features.
pub fn float_target_supported(features: &wgpu::TextureFormatFeatures) -> bool {
    features.allowed_usages.contains(
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
    ) && features.flags.contains(
        wgpu::TextureFormatFeatureFlags::FILTERABLE | wgpu::TextureFormatFeatureFlags::BLENDABLE,
    )
}

/// The HD scene target's format: half float when wanted and supported, else the surface's own (non-sRGB).
pub fn pick_scene_format(
    surface_format: wgpu::TextureFormat,
    float_supported: bool,
    want_float: bool,
) -> wgpu::TextureFormat {
    if float_supported && want_float {
        wgpu::TextureFormat::Rgba16Float
    } else {
        surface_format.remove_srgb_suffix()
    }
}

struct Target {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl Target {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat, width: u32, height: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("hd_scene"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            // COPY_SRC: HD water copies the finished opaque scene to refract it
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        Self { texture, view }
    }
}

fn composite_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let texture = |binding, filterable| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    };
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("post_composite"),
        entries: &[
            texture(0, false),
            texture(1, true),
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    })
}

pub struct PostChain {
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
    target: Target,
    composite_layout: wgpu::BindGroupLayout,
    composite: wgpu::RenderPipeline,
    /// Remade with the target or the bloom mips
    bind_group: Option<wgpu::BindGroup>,
    effects: bool,
    bloom_levels: u32,
    bloom_intensity: f32,
    bloom_pipelines: Option<BloomPipelines>,
    bloom: Option<Bloom>,
    /// Bound for the composite when there is no bloom
    no_bloom: wgpu::TextureView,
    sampler: wgpu::Sampler,
    params: wgpu::Buffer,
    /// What was last written to `params`
    written: Option<grade::CompositeUniform>,
    grade_fade: GradeFade,
    grade_strength: f32,
    vignette: bool,
}

impl PostChain {
    /// HD's frame for these options, none when HD is off: HD off keeps today's direct-to-swapchain frame.
    pub fn for_options(
        device: &wgpu::Device,
        hd: &HdOptions,
        scene_format: wgpu::TextureFormat,
        output_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Option<Self> {
        hd.enabled.then(|| {
            let mut chain = Self::new(device, scene_format, output_format, width, height);
            chain.set_bloom(device, hd.bloom_levels, hd.bloom_intensity);
            chain.set_grade(hd.grade, hd.vignette);
            chain
        })
    }

    /// `output_format`: what the composite writes, the swapchain's (or the screen distortion's) view.
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        output_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        let (width, height) = (width.max(1), height.max(1));
        let composite_layout = composite_layout(device);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("post_composite"),
            source: wgpu::ShaderSource::Wgsl(COMPOSITE_WGSL.into()),
        });
        let composite = bloom::fullscreen_pipeline(
            device,
            &composite_layout,
            &shader,
            "fs_main",
            output_format,
            None,
        );
        Self {
            format,
            width,
            height,
            target: Target::new(device, format, width, height),
            composite_layout,
            composite,
            bind_group: None,
            effects: true,
            bloom_levels: 0,
            bloom_intensity: 0.0,
            bloom_pipelines: None,
            bloom: None,
            no_bloom: device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("no_bloom"),
                    size: wgpu::Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&Default::default()),
            sampler: bloom::linear_sampler(device),
            params: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("post_composite"),
                size: std::mem::size_of::<grade::CompositeGpu>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            written: None,
            grade_fade: GradeFade::new(GradeParams::IDENTITY),
            grade_strength: 0.0,
            vignette: false,
        }
    }

    /// Colour grade strength (0 to 1) and whether the corners darken.
    pub fn set_grade(&mut self, strength: f32, vignette: bool) {
        self.grade_strength = strength;
        self.vignette = vignette;
    }

    /// The new map's mood, faded to over a second.
    pub fn set_map(&mut self, map_name: &str) {
        self.grade_fade
            .start(GradeParams::for_family(grade::family_for_map(map_name)));
    }

    /// The curve's white point: only a half-float target holds light above white.
    fn curve_white(&self) -> f32 {
        if self.format == wgpu::TextureFormat::Rgba16Float {
            grade::FLOAT_WHITE
        } else {
            1.0
        }
    }

    /// Bloom's mip count (0: none) and strength. An 8-bit target never blooms, so it gets no mips.
    pub fn set_bloom(&mut self, device: &wgpu::Device, levels: u32, intensity: f32) {
        self.bloom_intensity = intensity;
        let levels = bloom::levels_for_target(self.format, levels);
        // a strength change alone (the Settings slider) keeps the mips
        if levels == self.bloom_levels && (levels == 0 || self.bloom.is_some()) {
            return;
        }
        self.bloom_levels = levels;
        if levels > 0 && self.bloom_pipelines.is_none() {
            self.bloom_pipelines = Some(BloomPipelines::new(device, self.format));
        }
        self.rebuild_bloom(device);
    }

    /// The mips for the current size; the pipelines stay.
    fn rebuild_bloom(&mut self, device: &wgpu::Device) {
        self.bloom = match &self.bloom_pipelines {
            Some(pipelines) if self.bloom_levels > 0 => Some(Bloom::new(
                device,
                pipelines,
                self.format,
                &self.target.view,
                (self.width, self.height),
                self.bloom_levels,
            )),
            _ => None,
        };
        self.bind_group = None;
    }

    /// The options as they stand now (bloom and grade change live; the target's format does not).
    pub fn live_options(&self) -> HdOptions {
        HdOptions {
            enabled: true,
            float_target: self.format == wgpu::TextureFormat::Rgba16Float,
            bloom_levels: self.bloom_levels,
            bloom_intensity: self.bloom_intensity,
            grade: self.grade_strength,
            vignette: self.vignette,
            ..HdOptions::default()
        }
    }

    pub fn has_bloom(&self) -> bool {
        self.bloom.is_some()
    }

    pub fn bloom_mip_sizes(&self) -> Vec<(u32, u32)> {
        self.bloom.as_ref().map(Bloom::mip_sizes).unwrap_or_default()
    }


    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Remakes the target when the size changed; a zero side is kept at 1.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let (width, height) = (width.max(1), height.max(1));
        if (width, height) == (self.width, self.height) {
            return;
        }
        self.width = width;
        self.height = height;
        self.target = Target::new(device, self.format, width, height);
        self.rebuild_bloom(device);
    }

    pub fn scene_texture(&self) -> &wgpu::Texture {
        &self.target.texture
    }

    pub fn scene_view(&self) -> &wgpu::TextureView {
        &self.target.view
    }

    /// `/hd` while HD runs: the post effects on or off; off shows the scene as drawn.
    pub fn set_effects(&mut self, on: bool) {
        self.effects = on;
    }

    pub fn effects(&self) -> bool {
        self.effects
    }

    /// Bloom, then the finished scene to `output`. `delta`: seconds since the last frame, for the grade's fade.
    pub fn run(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
        delta: f32,
    ) {
        self.grade_fade.advance(delta);
        let bloom_on = bloom::should_run(self.effects, self.bloom_intensity, self.bloom_levels);
        if bloom_on
            && let (Some(bloom), Some(pipelines)) = (&self.bloom, &self.bloom_pipelines)
        {
            nostalre_profiling::profile_scope!("post-bloom");
            bloom.prefilter(encoder, pipelines);
            bloom.filter(encoder, pipelines);
        }
        let uniform = grade::composite_uniform(
            self.effects,
            if bloom_on { self.bloom_intensity } else { 0.0 },
            &self.grade_fade.current().scaled(self.grade_strength),
            if self.vignette { grade::VIGNETTE } else { 0.0 },
            self.curve_white(),
        );
        if self.written != Some(uniform) {
            queue.write_buffer(&self.params, 0, bytemuck::bytes_of(&uniform.to_gpu()));
            self.written = Some(uniform);
        }

        nostalre_profiling::profile_scope!("post-composite");
        let bloom_view = self.bloom.as_ref().map_or(&self.no_bloom, Bloom::output);
        let bind_group = self.bind_group.get_or_insert_with(|| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("post_composite"),
                layout: &self.composite_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&self.target.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(bloom_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: self.params.as_entire_binding(),
                    },
                ],
            })
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("post_composite"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    // every pixel is written
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&self.composite);
        pass.set_bind_group(0, &*bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}
