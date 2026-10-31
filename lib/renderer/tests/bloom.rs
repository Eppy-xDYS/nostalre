use nostalre_renderer::post::{PostChain, bloom};
use wgpu::TextureFormat;

fn device() -> (wgpu::Device, wgpu::Queue) {
    wgpu::Device::noop(&wgpu::DeviceDescriptor::default())
}

fn luma(c: [f32; 3]) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

#[test]
fn knee_zero_below() {
    assert_eq!(bloom::prefilter([0.9, 0.9, 0.9], bloom::KNEE), [0.0; 3]);
    assert_eq!(bloom::prefilter([1.0, 1.0, 1.0], bloom::KNEE), [0.0; 3], "white itself does not bloom");
    assert_eq!(bloom::prefilter([0.0, 0.0, 0.0], bloom::KNEE), [0.0; 3]);
}

#[test]
fn knee_keeps_hue() {
    let c = [3.0, 1.5, 0.6];
    let out = bloom::prefilter(c, bloom::KNEE);
    let l = luma(c);
    let scale = (l - bloom::KNEE) / l;
    for i in 0..3 {
        assert!((out[i] - c[i] * scale).abs() < 1e-5, "{out:?}");
    }
    assert!((out[0] / out[1] - c[0] / c[1]).abs() < 1e-5, "hue ratio kept");
}

#[test]
fn stacked_light_is_capped() {
    let out = bloom::prefilter([40.0, 40.0, 40.0], bloom::KNEE);
    assert_eq!(out, [bloom::BRIGHT_CAP; 3]);
}

#[test]
fn mip_sizes_for_odd_surfaces() {
    assert_eq!(
        bloom::mip_sizes(1081, 601, 4),
        vec![(540, 300), (270, 150), (135, 75), (67, 37)]
    );
    assert_eq!(bloom::mip_sizes(1920, 1080, 3), vec![(960, 540), (480, 270), (240, 135)]);
}

#[test]
fn mip_sizes_tiny_surface() {
    assert_eq!(bloom::mip_sizes(3, 1, 4), vec![(1, 1), (1, 1), (1, 1), (1, 1)]);
    assert!(bloom::mip_sizes(640, 360, 0).is_empty());
}

#[test]
fn low_has_no_chain() {
    let (device, _) = device();
    let mut chain = PostChain::new(&device, TextureFormat::Rgba16Float, TextureFormat::Bgra8Unorm, 640, 360);
    chain.set_bloom(&device, 0, 0.6);
    assert!(!chain.has_bloom());
    chain.set_bloom(&device, 4, 0.6);
    assert!(chain.has_bloom());
    assert_eq!(chain.bloom_mip_sizes().len(), 4);
}

#[test]
fn strength_change_keeps_the_chain() {
    let (device, _) = device();
    let mut chain = PostChain::new(&device, TextureFormat::Rgba16Float, TextureFormat::Bgra8Unorm, 640, 360);
    chain.set_bloom(&device, 4, 0.6);
    chain.set_bloom(&device, 4, 0.2);
    assert_eq!(chain.bloom_mip_sizes().len(), 4);
    chain.set_bloom(&device, 3, 0.2);
    assert_eq!(chain.bloom_mip_sizes().len(), 3, "a quality change remakes it");
    chain.set_bloom(&device, 0, 0.2);
    assert!(!chain.has_bloom());
}

#[test]
fn no_bloom_on_8bit_target() {
    let (device, _) = device();
    let mut chain = PostChain::new(&device, TextureFormat::Bgra8Unorm, TextureFormat::Bgra8Unorm, 640, 360);
    chain.set_bloom(&device, 4, 0.6);
    assert!(!chain.has_bloom(), "nothing on an 8-bit target is above the knee");
    assert_eq!(bloom::levels_for_target(TextureFormat::Rgba16Float, 4), 4);
    assert_eq!(bloom::levels_for_target(TextureFormat::Rgba8Unorm, 4), 0);
}

#[test]
fn skipped_when_off() {
    assert!(bloom::should_run(true, 0.6, 4));
    assert!(!bloom::should_run(false, 0.6, 4), "/hd turned the effects off");
    assert!(!bloom::should_run(true, 0.0, 4), "intensity 0");
    assert!(!bloom::should_run(true, 0.6, 0), "no levels (Low)");
}

#[test]
fn chain_follows_resize() {
    let (device, _) = device();
    let mut chain = PostChain::new(&device, TextureFormat::Rgba16Float, TextureFormat::Bgra8Unorm, 1920, 1080);
    chain.set_bloom(&device, 3, 0.6);
    assert_eq!(chain.bloom_mip_sizes()[0], (960, 540));
    chain.resize(&device, 1280, 720);
    assert_eq!(chain.bloom_mip_sizes(), vec![(640, 360), (320, 180), (160, 90)]);
}

#[test]
fn bloom_shaders_validate() {
    let shaders = [
        ("bloom", bloom::BLOOM_WGSL),
        ("post_composite", nostalre_renderer::post::COMPOSITE_WGSL),
    ];
    for (name, src) in shaders {
        let module = naga::front::wgsl::parse_str(src)
            .unwrap_or_else(|e| panic!("{name}.wgsl failed to parse: {}", e.emit_to_string(src)));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap_or_else(|e| panic!("{name}.wgsl failed validation: {e:?}"));
    }
}
