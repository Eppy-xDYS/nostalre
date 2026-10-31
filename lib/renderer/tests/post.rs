use nostalre_renderer::post::{self, PostChain};
use wgpu::{TextureFormat, TextureFormatFeatureFlags, TextureFormatFeatures, TextureUsages};

fn device() -> (wgpu::Device, wgpu::Queue) {
    wgpu::Device::noop(&wgpu::DeviceDescriptor::default())
}

fn float_features(flags: TextureFormatFeatureFlags) -> TextureFormatFeatures {
    TextureFormatFeatures {
        allowed_usages: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        flags,
    }
}

#[test]
fn float_target_needs_filtering_and_blending() {
    let full = TextureFormatFeatureFlags::FILTERABLE | TextureFormatFeatureFlags::BLENDABLE;
    assert!(post::float_target_supported(&float_features(full)));
    assert!(!post::float_target_supported(&float_features(
        TextureFormatFeatureFlags::FILTERABLE
    )));
    assert!(!post::float_target_supported(&float_features(
        TextureFormatFeatureFlags::BLENDABLE
    )));
    let not_renderable = TextureFormatFeatures {
        allowed_usages: TextureUsages::TEXTURE_BINDING,
        flags: full,
    };
    assert!(!post::float_target_supported(&not_renderable));
}

#[test]
fn picks_float_when_supported() {
    assert_eq!(
        post::pick_scene_format(TextureFormat::Bgra8UnormSrgb, true, true),
        TextureFormat::Rgba16Float
    );
}

#[test]
fn falls_back_to_surface_format() {
    assert_eq!(
        post::pick_scene_format(TextureFormat::Bgra8UnormSrgb, false, true),
        TextureFormat::Bgra8Unorm,
        "no float support: the surface format, without sRGB"
    );
}

#[test]
fn low_preset_uses_8bit() {
    assert_eq!(
        post::pick_scene_format(TextureFormat::Rgba8UnormSrgb, true, false),
        TextureFormat::Rgba8Unorm
    );
}

#[test]
fn disabled_has_no_chain() {
    let (device, _) = device();
    let chain = |enabled| {
        let hd = post::HdOptions {
            enabled,
            ..Default::default()
        };
        PostChain::for_options(
            &device,
            &hd,
            TextureFormat::Rgba16Float,
            TextureFormat::Bgra8Unorm,
            640,
            360,
        )
    };
    assert!(chain(false).is_none());
    assert!(chain(true).is_some());
}

#[test]
fn target_follows_resize() {
    let (device, _) = device();
    let mut chain = PostChain::new(&device, TextureFormat::Rgba16Float, TextureFormat::Bgra8Unorm, 640, 360);
    assert_eq!(chain.size(), (640, 360));
    assert_eq!(chain.scene_texture().width(), 640);
    chain.resize(&device, 1920, 1080);
    assert_eq!(chain.size(), (1920, 1080));
    assert_eq!(chain.scene_texture().width(), 1920);
    assert_eq!(chain.scene_texture().height(), 1080);
    assert_eq!(chain.scene_texture().format(), TextureFormat::Rgba16Float);
}

#[test]
fn zero_size_clamped() {
    let (device, _) = device();
    let mut chain = PostChain::new(&device, TextureFormat::Rgba8Unorm, TextureFormat::Bgra8Unorm, 0, 0);
    assert_eq!(chain.size(), (1, 1));
    chain.resize(&device, 0, 720);
    assert_eq!(chain.size(), (1, 720));
}

#[test]
fn scene_copy_from_target() {
    let (device, _) = device();
    let chain = PostChain::new(&device, TextureFormat::Rgba16Float, TextureFormat::Bgra8Unorm, 320, 240);
    let usage = chain.scene_texture().usage();
    assert!(
        usage.contains(
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC
        ),
        "HD water copies the opaque scene out of the target: {usage:?}"
    );
}

#[test]
fn shaders_validate() {
    let src = post::COMPOSITE_WGSL;
    let module = naga::front::wgsl::parse_str(src)
        .unwrap_or_else(|e| panic!("post_composite.wgsl failed to parse: {}", e.emit_to_string(src)));
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap_or_else(|e| panic!("post_composite.wgsl failed validation: {e:?}"));
}
