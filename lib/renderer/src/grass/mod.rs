//! HD grass: tufts of crossed blade quads on grassy ground, swaying a little, shrinking into the ground with distance.
//! The tufts are instances; the vertex stage builds each quad from its vertex index, so there is no mesh buffer.
//! Only the chunks near the camera target are made (a few per frame as it moves) and kept; far ones are dropped.

pub mod atlas;
pub mod chunks;
pub mod classify;
pub mod scatter;

use nostalre_formats::gnd::GndFile;
use nostalre_formats::map_water::MapWater;

use crate::device::DEPTH_FORMAT;
use crate::global_uniforms::GlobalUniforms;
use chunks::{CHUNK_CELLS, ChunkBox};
use classify::TextureInfo;
use scatter::{FieldInput, GrassField, Tier};

/// The grass shader, after the lamps it shares with the terrain (scene_lamps.wgsl).
pub const GRASS_WGSL: &str = concat!(
    include_str!("../shaders/scene_lamps.wgsl"),
    include_str!("../shaders/grass.wgsl")
);

/// Tufts fade out toward this distance from the camera target (32 cells): past the screen's edge at the
/// default zoom.
pub const FAR: f32 = 320.0;
/// An optional blade picture in the local game data (a loose file or a GRF entry), drawn instead of the procedural
/// blades. Not shipped: whoever plays drops their own picture there.
pub const BLADE_ART_PATH: &str = "data\\texture\\nostalre\\grass.png";
/// Chunks made per frame at most, nearest first: walking never stalls a frame on a burst of new grass.
const NEW_CHUNKS_PER_FRAME: usize = 4;

/// One tuft: its root on the ground, and tint, yaw, size, variant and lean packed (see `scatter::pack`).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Tuft {
    pub root: [f32; 3],
    pub packed: u32,
}

impl Tuft {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Tuft>() as u64,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Uint32],
    };
}

/// Two crossed quads; from High up, some tufts (by their seed) get a third.
pub fn vertices_per_tuft(tier: Tier) -> u32 {
    if matches!(tier, Tier::High | Tier::Ultra) { 18 } else { 12 }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GrassUniform {
    focus: [f32; 2],
    time: f32,
    far: f32,
    wind: [f32; 2],
    max_quads: f32,
    gain: f32,
    variants: f32,
    colored: f32,
    aspect: f32,
    _pad: f32,
}

/// What the shader needs to know about the art in use.
#[derive(Clone, Copy)]
struct ArtInfo {
    gain: f32,
    variants: f32,
    colored: f32,
    aspect: f32,
}

impl ArtInfo {
    fn of(art: &atlas::BladeArt) -> Self {
        Self {
            gain: 1.0 / art.mean_luma.max(0.05),
            variants: art.variants as f32,
            colored: if art.colored { 1.0 } else { 0.0 },
            aspect: art.width as f32 / art.variants as f32 / art.height as f32,
        }
    }
}

/// What a map's grass is built from.
pub struct GrassMap<'a> {
    pub gnd: &'a GndFile,
    pub name: &'a str,
    /// Per GND texture
    pub textures: &'a [TextureInfo],
    pub water: Option<&'a MapWater>,
    /// Cells under a model floor
    pub floors: &'a [bool],
}

/// A made chunk; no buffer when none of its tufts survived the density.
struct Resident {
    bounds: ChunkBox,
    buffer: Option<wgpu::Buffer>,
}

pub struct GrassRenderer {
    core: wgpu::RenderPipeline,
    rim: wgpu::RenderPipeline,
    atlas: wgpu::BindGroup,
    art: ArtInfo,
    texture_layout: wgpu::BindGroupLayout,
    uniform: wgpu::Buffer,
    uniform_group: wgpu::BindGroup,
    field: Option<GrassField>,
    resident: Vec<Resident>,
    /// Indices into `resident` to draw this frame
    draw: Vec<usize>,
    tier: Tier,
    density: f32,
}

impl GrassRenderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        global_uniforms: &GlobalUniforms,
        texture_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("grass"),
            size: std::mem::size_of::<GrassUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("grass"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let uniform_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("grass"),
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        let layouts = [&global_uniforms.bind_group_layout, texture_layout, &uniform_layout];
        let art = atlas::BladeArt::procedural();
        Self {
            core: pipeline(device, format, &layouts, false),
            rim: pipeline(device, format, &layouts, true),
            atlas: atlas_bind_group(device, queue, texture_layout, &art),
            art: ArtInfo::of(&art),
            texture_layout: texture_layout.clone(),
            uniform,
            uniform_group,
            field: None,
            resident: Vec::new(),
            draw: Vec::new(),
            tier: Tier::High,
            density: 1.0,
        }
    }

    /// Draws tufts with `art` from now on.
    pub fn set_art(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, art: &atlas::BladeArt) {
        self.atlas = atlas_bind_group(device, queue, &self.texture_layout, art);
        self.art = ArtInfo::of(art);
    }

    /// Works out what the map grows; its chunks are made as the camera comes near.
    pub fn set_map(&mut self, map: &GrassMap, tier: Tier, density: f32) {
        let field = GrassField::new(&FieldInput {
            gnd: map.gnd,
            map: map.name,
            textures: map.textures,
            water: map.water,
            floors: map.floors,
        });
        tracing::info!("grass: {} ({tier:?})", if field.is_empty() { "none" } else { "grows" });
        self.field = (!field.is_empty()).then_some(field);
        self.tier = tier;
        self.density = density;
        self.resident.clear();
        self.draw.clear();
    }

    /// The slider: made chunks are remade at the new density over the next frames.
    pub fn set_density(&mut self, density: f32) {
        self.density = density;
        self.resident.clear();
        self.draw.clear();
    }

    /// The quality: made chunks are remade with it over the next frames.
    pub fn set_tier(&mut self, tier: Tier) {
        if tier != self.tier {
            self.tier = tier;
            self.resident.clear();
            self.draw.clear();
        }
    }

    pub fn clear(&mut self) {
        self.field = None;
        self.resident.clear();
        self.draw.clear();
    }

    /// Makes the nearest missing chunks, drops far ones, picks this frame's chunks, and sets the sway time.
    pub fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view_proj: glam::Mat4,
        focus: glam::Vec3,
        time: f32,
    ) {
        let Some(field) = &self.field else {
            return;
        };
        let wanted = chunks::wanted_chunks(field, focus, FAR);
        // keep a chunk's width past FAR, so turning on the spot does not remake the edge
        let keep = chunks::wanted_chunks(field, focus, FAR + CHUNK_CELLS as f32 * field.cell_size());
        self.resident.retain(|r| keep.contains(&r.bounds.key));
        let missing = wanted
            .iter()
            .filter(|key| !self.resident.iter().any(|r| r.bounds.key == **key))
            .take(NEW_CHUNKS_PER_FRAME)
            .copied()
            .collect::<Vec<_>>();
        for key in missing {
            let tufts = field.chunk_tufts(key.0, key.1, self.tier, self.density);
            let buffer = (!tufts.is_empty()).then(|| {
                use wgpu::util::DeviceExt;
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("grass_tufts"),
                    contents: bytemuck::cast_slice(&tufts),
                    usage: wgpu::BufferUsages::VERTEX,
                })
            });
            self.resident.push(Resident {
                bounds: ChunkBox::new(field, key, tufts.len() as u32),
                buffer,
            });
        }
        let bounds: Vec<ChunkBox> = self.resident.iter().map(|r| r.bounds).collect();
        self.draw = chunks::draw_list(&bounds, view_proj, focus, FAR, self.tier.budget());
        let uniform = GrassUniform {
            focus: [focus.x, focus.z],
            time,
            far: FAR,
            wind: [0.8, 0.6],
            max_quads: (vertices_per_tuft(self.tier) / 6) as f32,
            gain: self.art.gain,
            variants: self.art.variants,
            colored: self.art.colored,
            aspect: self.art.aspect,
            _pad: 0.0,
        };
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&uniform));
    }

    pub fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, global_uniforms: &'a GlobalUniforms) {
        if self.draw.is_empty() {
            return;
        }
        pass.set_bind_group(0, &global_uniforms.bind_group, &[]);
        pass.set_bind_group(1, &self.atlas, &[]);
        pass.set_bind_group(2, &self.uniform_group, &[]);
        let verts = vertices_per_tuft(self.tier);
        // the opaque cores first (they write depth), then the soft rims over them
        for pipeline in [&self.core, &self.rim] {
            pass.set_pipeline(pipeline);
            for &i in &self.draw {
                let resident = &self.resident[i];
                if let Some(buffer) = &resident.buffer {
                    pass.set_vertex_buffer(0, buffer.slice(..));
                    pass.draw(0..verts, 0..resident.bounds.tufts);
                }
            }
        }
    }
}

fn atlas_bind_group(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    art: &atlas::BladeArt,
) -> wgpu::BindGroup {
    let levels = &art.levels;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("grass_atlas"),
        size: wgpu::Extent3d {
            width: art.width,
            height: art.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: levels.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (level, data) in levels.iter().enumerate() {
        let (w, h) = ((art.width >> level).max(1), (art.height >> level).max(1));
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: level as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * w),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
    }
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("grass_atlas"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
        ..Default::default()
    });
    let view = texture.create_view(&Default::default());
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("grass_atlas"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    })
}

/// The core (`rim` false: alpha tested, writes depth) or the soft rim (blended, depth tested only).
fn pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    layouts: &[&wgpu::BindGroupLayout; 3],
    rim: bool,
) -> wgpu::RenderPipeline {
    let label = if rim { "grass_rim" } else { "grass_core" };
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("grass"),
        source: wgpu::ShaderSource::Wgsl(GRASS_WGSL.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(label),
        bind_group_layouts: layouts,
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[Tuft::LAYOUT],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some(if rim { "fs_rim" } else { "fs_core" }),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: rim.then_some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: !rim,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    })
}
