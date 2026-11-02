use glam::{Mat4, Vec3};
use nostalre_formats::gnd::{GndCell, GndFile, GndSurface};
use nostalre_renderer::grass::atlas::{self, ATLAS_HEIGHT, ATLAS_WIDTH, VARIANTS};
use nostalre_renderer::grass::chunks::{self, CHUNK_CELLS, ChunkBox};
use nostalre_renderer::grass::classify::{self, Ground, TextureInfo};
use nostalre_renderer::grass::scatter::{FieldInput, GrassField, TUFT_MAX_HEIGHT, Tier};
use nostalre_renderer::grass::{self, Tuft};

const GREEN: [f32; 3] = [0.35, 0.55, 0.25];

fn surface(texture_id: i16) -> GndSurface {
    GndSurface {
        tex_u: [0.0; 4],
        tex_v: [0.0; 4],
        texture_id,
        lightmap_id: -1,
        color_bgra: [255; 4],
    }
}

fn flat_cell(height: f32, surface_up: i32) -> GndCell {
    GndCell {
        height_sw: height,
        height_se: height,
        height_nw: height,
        height_ne: height,
        surface_up,
        surface_south: -1,
        surface_east: -1,
    }
}

/// `size` × `size` flat cells, all on texture 0.
fn field(size: i32) -> GndFile {
    GndFile {
        water: None,
        version: (1, 7),
        width: size,
        height: size,
        zoom: 10.0,
        textures: vec!["필드바닥\\prt_초원01.bmp".to_string()],
        lightmaps: Vec::new(),
        surfaces: vec![surface(0)],
        cells: (0..size * size).map(|_| flat_cell(0.0, 0)).collect(),
    }
}

fn grass_texture() -> Vec<TextureInfo> {
    vec![TextureInfo {
        class: Ground::Grass,
        rgb: GREEN,
    }]
}

fn grass_field(gnd: &GndFile, textures: &[TextureInfo], map: &str, floors: &[bool]) -> GrassField {
    GrassField::new(&FieldInput {
        gnd,
        map,
        textures,
        water: None,
        floors,
    })
}

/// Every tuft of the field at full density.
fn all_tufts(field: &GrassField, tier: Tier, density: f32) -> Vec<Tuft> {
    let (cw, ch) = field.chunk_grid();
    let mut tufts = Vec::new();
    for cz in 0..ch {
        for cx in 0..cw {
            tufts.extend(field.chunk_tufts(cx, cz, tier, density));
        }
    }
    tufts
}

// K1 classify

#[test]
fn classify_keyword_grass() {
    for name in [
        "필드바닥\\prt_초원01.bmp",
        "필드바닥\\숲속바닥-01.bmp",
        "필드바닥\\해변풀바닥04.bmp",
        "필드바닥\\묘르닐풀바닥-00.bmp",
        "field\\grass_01.bmp",
    ] {
        assert_eq!(classify::classify_texture(name, [0.5; 3]), Ground::Grass, "{name}");
    }
}

#[test]
fn classify_keyword_sparse() {
    for name in ["필드바닥\\prt_언덕04.bmp", "필드바닥\\prt_흙02.bmp"] {
        assert_eq!(classify::classify_texture(name, GREEN), Ground::Sparse, "{name}");
    }
}

#[test]
fn classify_keyword_never() {
    for name in [
        "필드바닥\\해변절벽01.bmp",
        "필드바닥\\해변물바닥01.bmp",
        "필드바닥\\prt_바위01.bmp",
        "필드바닥\\prt_도시01.bmp",
        "필드바닥\\moc_맨땅08.bmp",
        "필드바닥\\pe-wall-001.bmp",
        "BACKSIDE.bmp",
        "BLACK.BMP",
    ] {
        assert_eq!(classify::classify_texture(name, GREEN), Ground::Never, "{name}: name wins over colour");
    }
}

#[test]
fn classify_colour_fallback() {
    assert_eq!(classify::classify_texture("필드바닥\\게펜필드-04.bmp", GREEN), Ground::Grass);
    assert_eq!(
        classify::classify_texture("필드바닥\\게펜필드-04.bmp", [0.5, 0.45, 0.4]),
        Ground::Never
    );
    assert_eq!(classify::classify_texture("x.bmp", [0.3, 0.3, 0.3]), Ground::Never, "grey");
}

#[test]
fn cell_needs_top_surface() {
    let mut gnd = field(2);
    gnd.cells[0].surface_up = -1;
    let tex = grass_texture();
    assert_eq!(classify::cell_grows(&gnd, 0, 0, &tex, None), None);
    assert_eq!(classify::cell_grows(&gnd, 1, 0, &tex, None), Some(Ground::Grass));
}

#[test]
fn cell_rejects_steep() {
    // grass grows up grassy slopes (to about 45 degrees: 10 units over a 10-unit cell), not up cliffs
    let mut gnd = field(2);
    gnd.cells[0].height_ne = -10.1;
    gnd.cells[1].height_ne = -9.9;
    let tex = grass_texture();
    assert_eq!(classify::cell_grows(&gnd, 0, 0, &tex, None), None);
    assert_eq!(classify::cell_grows(&gnd, 1, 0, &tex, None), Some(Ground::Grass));
}

#[test]
fn slopes_grow_thinner() {
    assert_eq!(classify::slope_share(0.0), 1.0);
    assert_eq!(classify::slope_share(classify::FULL_DENSITY_SPREAD), 1.0, "gentle ground at full density");
    let steep = classify::slope_share(classify::MAX_HEIGHT_SPREAD);
    assert!((steep - 0.4).abs() < 1e-6, "{steep}");
    assert!(classify::slope_share(6.0) < 1.0 && classify::slope_share(6.0) > steep, "thinning on the way");
}

#[test]
fn cell_rejects_underwater() {
    // heights grow downward: a cell lower than the water (larger y) is under it
    let mut gnd = field(2);
    gnd.cells[0] = flat_cell(5.0, 0);
    let tex = grass_texture();
    assert_eq!(classify::cell_grows(&gnd, 0, 0, &tex, Some(1.0)), None);
    assert_eq!(classify::cell_grows(&gnd, 1, 0, &tex, Some(1.0)), Some(Ground::Grass));
}

#[test]
fn cell_rejects_dark_ground() {
    let gnd = field(1);
    let tex = vec![TextureInfo {
        class: Ground::Grass,
        rgb: [0.05, 0.08, 0.04],
    }];
    assert_eq!(classify::cell_grows(&gnd, 0, 0, &tex, None), None);
}

#[test]
fn floor_cells_under_low_flat_faces() {
    let gnd = field(4);
    // a floor 1 unit above the ground (heights grow downward) over cells (1..=2, 1)
    let floor = [[10.0, -1.0, 10.0], [30.0, -1.0, 10.0], [30.0, -1.0, 20.0]];
    let floor2 = [[10.0, -1.0, 10.0], [30.0, -1.0, 20.0], [10.0, -1.0, 20.0]];
    // a roof high up and a wall: neither blocks grass
    let roof = [[0.0, -40.0, 30.0], [40.0, -40.0, 30.0], [40.0, -40.0, 40.0]];
    let wall = [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [10.0, -20.0, 0.0]];
    let mask = classify::floor_cells(&gnd, [floor, floor2, roof, wall]);
    let blocked: Vec<(i32, i32)> = (0..16)
        .filter(|&i| mask[i as usize])
        .map(|i| (i % 4, i / 4))
        .collect();
    assert_eq!(blocked, vec![(1, 1), (2, 1)]);
}

#[test]
fn scatter_skips_floor_cells() {
    let gnd = field(2);
    let tex = grass_texture();
    let floors = [true, false, false, false];
    let tufts = all_tufts(&grass_field(&gnd, &tex, "m", &floors), Tier::Ultra, 1.0);
    assert!(!tufts.is_empty());
    assert!(tufts.iter().all(|t| !(t.root[0] < 10.0 && t.root[2] < 10.0)), "none in cell 0");
}

// K2 scatter (per chunk, D20)

fn bytes(tufts: &[Tuft]) -> &[u8] {
    bytemuck::cast_slice(tufts)
}

#[test]
fn scatter_deterministic_per_map() {
    let gnd = field(32);
    let tex = grass_texture();
    let a = grass_field(&gnd, &tex, "prt_fild08", &[]).chunk_tufts(1, 0, Tier::High, 1.0);
    let b = grass_field(&gnd, &tex, "prt_fild08", &[]).chunk_tufts(1, 0, Tier::High, 1.0);
    assert!(!a.is_empty());
    assert_eq!(bytes(&a), bytes(&b), "a chunk regenerates identically");
}

#[test]
fn scatter_differs_between_maps() {
    let gnd = field(32);
    let tex = grass_texture();
    let a = grass_field(&gnd, &tex, "prt_fild08", &[]).chunk_tufts(0, 0, Tier::High, 1.0);
    let b = grass_field(&gnd, &tex, "prt_fild07", &[]).chunk_tufts(0, 0, Tier::High, 1.0);
    assert_ne!(bytes(&a), bytes(&b));
}

#[test]
fn chunk_holds_its_cells_only() {
    let gnd = field(32);
    let tufts = grass_field(&gnd, &grass_texture(), "m", &[]).chunk_tufts(1, 0, Tier::High, 1.0);
    let side = CHUNK_CELLS as f32 * gnd.zoom;
    assert!(tufts.iter().all(|t| (side..2.0 * side).contains(&t.root[0]) && (0.0..side).contains(&t.root[2])));
}

#[test]
fn dense_per_cell() {
    let gnd = field(16);
    let field = grass_field(&gnd, &grass_texture(), "m", &[]);
    let cells = (CHUNK_CELLS * CHUNK_CELLS) as f32;
    for (tier, per_cell) in [(Tier::Low, 4.0), (Tier::Medium, 6.0), (Tier::High, 8.0), (Tier::Ultra, 10.0)] {
        let n = field.chunk_tufts(0, 0, tier, 1.0).len() as f32 / cells;
        assert!((n - per_cell).abs() < 0.5, "{tier:?}: {n} per cell");
        assert_eq!(tier.per_cell(), per_cell);
    }
}

#[test]
fn scatter_slider_zero_none() {
    let gnd = field(16);
    let field = grass_field(&gnd, &grass_texture(), "prt_fild08", &[]);
    assert!(field.chunk_tufts(0, 0, Tier::Ultra, 0.0).is_empty());
    let full = field.chunk_tufts(0, 0, Tier::Ultra, 1.0);
    let half = field.chunk_tufts(0, 0, Tier::Ultra, 0.5);
    assert!(half.len() * 3 < full.len() * 2 && half.len() * 3 > full.len(), "half {} of {}", half.len(), full.len());
    assert!(half.iter().all(|t| full.contains(t)), "the slider thins the same tufts, it does not move them");
}

#[test]
fn sparse_cells_get_fewer() {
    let gnd = field(16);
    let grass = grass_field(&gnd, &grass_texture(), "m", &[]).chunk_tufts(0, 0, Tier::High, 1.0).len();
    let sparse_tex = vec![TextureInfo {
        class: Ground::Sparse,
        rgb: GREEN,
    }];
    let sparse = grass_field(&gnd, &sparse_tex, "m", &[]).chunk_tufts(0, 0, Tier::High, 1.0).len();
    assert!(sparse > 0 && sparse * 4 < grass, "sparse {sparse}, grass {grass}");
}

#[test]
fn empty_field_knows_it() {
    let gnd = field(4);
    let never = vec![TextureInfo {
        class: Ground::Never,
        rgb: GREEN,
    }];
    assert!(grass_field(&gnd, &never, "m", &[]).is_empty());
    assert!(!grass_field(&gnd, &grass_texture(), "m", &[]).is_empty());
}

#[test]
fn root_on_slope_height() {
    let mut gnd = field(1);
    gnd.cells[0] = GndCell {
        height_sw: 0.0,
        height_se: -1.2,
        height_nw: 0.8,
        height_ne: -0.4,
        ..flat_cell(0.0, 0)
    };
    let tufts = all_tufts(&grass_field(&gnd, &grass_texture(), "slope", &[]), Tier::Ultra, 1.0);
    assert!(!tufts.is_empty());
    for t in &tufts {
        let [x, y, z] = t.root;
        let (u, v) = (x / gnd.zoom, z / gnd.zoom);
        assert!((0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v), "inside the cell: {u} {v}");
        let south = -1.2 * u;
        let north = 0.8 + (-0.4 - 0.8) * u;
        let want = south + (north - south) * v;
        assert!((y - want).abs() < 0.01, "root {y}, bilinear {want}");
    }
}

#[test]
fn tuft_height_bounded() {
    let gnd = field(16);
    let tufts = all_tufts(&grass_field(&gnd, &grass_texture(), "m", &[]), Tier::Ultra, 1.0);
    // under a character's waist
    assert!(tufts.iter().all(|t| t.height() > 0.0 && t.height() <= TUFT_MAX_HEIGHT.min(10.0)));
}

// K3 chunks: which to keep, which to draw

fn camera(eye: Vec3, target: Vec3) -> Mat4 {
    Mat4::perspective_rh(15_f32.to_radians(), 16.0 / 9.0, 1.0, 100_000.0) * Mat4::look_at_rh(eye, target, Vec3::NEG_Y)
}

#[test]
fn wanted_chunks_near_focus_nearest_first() {
    let gnd = field(160); // 10 × 10 chunks of 160 units
    let field = grass_field(&gnd, &grass_texture(), "m", &[]);
    let focus = Vec3::new(800.0, 0.0, 800.0);
    let wanted = chunks::wanted_chunks(&field, focus, 320.0);
    assert!(!wanted.is_empty());
    assert!(wanted.len() <= 36, "only chunks within reach: {}", wanted.len());
    assert!(wanted.contains(&(4, 4)) && wanted.contains(&(5, 5)));
    assert!(!wanted.contains(&(0, 0)) && !wanted.contains(&(9, 9)));
    let dist = |(cx, cz): (i32, i32)| {
        let c = (glam::Vec2::new(cx as f32, cz as f32) + 0.5) * 160.0;
        c.distance(glam::Vec2::new(focus.x, focus.z))
    };
    assert!(wanted.windows(2).all(|w| dist(w[0]) <= dist(w[1]) + 1e-3), "nearest first");
}

#[test]
fn wanted_chunks_skip_bare_ground() {
    let mut gnd = field(32);
    let never = vec![TextureInfo { class: Ground::Never, rgb: GREEN }, TextureInfo { class: Ground::Grass, rgb: GREEN }];
    gnd.surfaces.push(surface(1));
    // only chunk (1, 0) grows grass
    for y in 0..16 {
        for x in 16..32 {
            gnd.cells[(y * 32 + x) as usize].surface_up = 1;
        }
    }
    let field = grass_field(&gnd, &never, "m", &[]);
    assert_eq!(chunks::wanted_chunks(&field, Vec3::new(160.0, 0.0, 160.0), 10_000.0), vec![(1, 0)]);
}

fn boxes(n: i32, tufts_each: u32) -> Vec<ChunkBox> {
    let side = CHUNK_CELLS as f32 * 10.0;
    (0..n)
        .flat_map(|cz| (0..n).map(move |cx| (cx, cz)))
        .map(|(cx, cz)| ChunkBox {
            key: (cx, cz),
            min: Vec3::new(cx as f32 * side, -TUFT_MAX_HEIGHT, cz as f32 * side),
            max: Vec3::new((cx + 1) as f32 * side, 0.0, (cz + 1) as f32 * side),
            tufts: tufts_each,
        })
        .collect()
}

#[test]
fn draw_list_within_budget_nearest_first() {
    let b = boxes(8, 4_000);
    let focus = Vec3::new(640.0, 0.0, 640.0);
    let view = camera(focus + Vec3::new(0.0, -20_000.0, -100.0), focus);
    let drawn = chunks::draw_list(&b, view, focus, 100_000.0, 30_000);
    let total: u32 = drawn.iter().map(|&i| b[i].tufts).sum();
    assert!(total <= 30_000, "{total}");
    assert_eq!(drawn.len(), 7, "as many as fit");
    let near = drawn.iter().map(|&i| b[i].key).collect::<Vec<_>>();
    assert!(near.contains(&(3, 3)) && near.contains(&(4, 4)), "the chunks at the focus come first: {near:?}");
}

#[test]
fn cull_behind_camera() {
    let b = boxes(4, 100);
    let target = Vec3::new(320.0, 0.0, 600.0);
    let eye = Vec3::new(320.0, -300.0, 450.0);
    let drawn = chunks::draw_list(&b, camera(eye, target), target, 10_000.0, u32::MAX);
    assert!(!drawn.is_empty() && drawn.len() < b.len(), "{} of {}", drawn.len(), b.len());
    assert!(drawn.iter().all(|&i| b[i].key.1 >= 2), "the near rows behind the eye drop");
}

#[test]
fn cull_beyond_far() {
    let b = boxes(8, 100);
    let focus = Vec3::new(80.0, 0.0, 80.0);
    let view = camera(focus + Vec3::new(0.0, -20_000.0, -100.0), focus);
    let near = chunks::draw_list(&b, view, focus, 100.0, u32::MAX).len();
    let wide = chunks::draw_list(&b, view, focus, 100_000.0, u32::MAX).len();
    assert!(near > 0 && near < wide, "near {near}, wide {wide}");
}

#[test]
fn budget_per_tier() {
    assert_eq!(Tier::Low.budget(), 20_000);
    assert_eq!(Tier::Medium.budget(), 40_000);
    assert_eq!(Tier::High.budget(), 80_000);
    assert_eq!(Tier::Ultra.budget(), 140_000);
}

// K4 atlas

fn coverage(level: &[u8], width: usize, variant: usize) -> f32 {
    let vw = width / VARIANTS;
    let height = level.len() / 4 / width;
    let mut opaque = 0;
    for y in 0..height {
        for x in variant * vw..(variant + 1) * vw {
            if level[(y * width + x) * 4 + 3] >= 128 {
                opaque += 1;
            }
        }
    }
    opaque as f32 / (vw * height) as f32
}

#[test]
fn atlas_deterministic() {
    let a = atlas::blade_atlas();
    assert_eq!(a, atlas::blade_atlas());
    assert_eq!(a[0].len(), (ATLAS_WIDTH * ATLAS_HEIGHT * 4) as usize);
}

#[test]
fn atlas_has_coverage() {
    let a = atlas::blade_atlas();
    for v in 0..VARIANTS {
        let c = coverage(&a[0], ATLAS_WIDTH as usize, v);
        assert!((0.15..=0.45).contains(&c), "variant {v}: {c}");
    }
}

#[test]
fn atlas_root_darker_than_tip() {
    let a = atlas::blade_atlas();
    let w = ATLAS_WIDTH as usize;
    let h = ATLAS_HEIGHT as usize;
    let mean = |rows: std::ops::Range<usize>| {
        let (mut sum, mut n) = (0.0, 0);
        for y in rows {
            for x in 0..w {
                let p = &a[0][(y * w + x) * 4..][..4];
                if p[3] >= 128 {
                    sum += p[0] as f32;
                    n += 1;
                }
            }
        }
        sum / n.max(1) as f32
    };
    // row 0 is the tip, the last row the root
    assert!(mean(h - h / 8..h) < mean(h / 4..h / 2));
}

#[test]
fn mips_keep_coverage() {
    let a = atlas::blade_atlas();
    assert!(a.len() >= 4, "a mip chain");
    let base = coverage(&a[0], ATLAS_WIDTH as usize, 0);
    for (level, data) in a.iter().enumerate().take(4) {
        let width = (ATLAS_WIDTH >> level) as usize;
        let c = coverage(data, width, 0);
        assert!((c - base).abs() <= base * 0.25, "mip {level}: {c} vs {base}");
    }
}

// D21: an optional blade picture from local game data

/// A 4 × 4 picture: opaque mid-green in the lower half, clear above.
fn tiny_art() -> image::RgbaImage {
    image::RgbaImage::from_fn(4, 4, |_, y| if y >= 2 { image::Rgba([60, 160, 40, 255]) } else { image::Rgba([0, 0, 0, 0]) })
}

#[test]
fn art_from_picture() {
    let art = atlas::BladeArt::from_image(&tiny_art());
    assert_eq!(art.variants, 1);
    assert!(art.colored, "a picture keeps its colours");
    assert_eq!((art.width, art.height), (4, 4));
    let luma = (0.299 * 60.0 + 0.587 * 160.0 + 0.114 * 40.0) / 255.0;
    assert!((art.mean_luma - luma).abs() < 1e-3, "mean of the opaque texels: {}", art.mean_luma);
    assert_eq!(art.levels[0].len(), 4 * 4 * 4);
    assert!(art.levels.len() >= 2, "mips");
}

#[test]
fn procedural_art_is_grey() {
    let art = atlas::BladeArt::procedural();
    assert_eq!(art.variants, VARIANTS as u32);
    assert!(!art.colored);
    assert_eq!((art.width, art.height), (ATLAS_WIDTH, ATLAS_HEIGHT));
    assert!(art.mean_luma > 0.0 && art.mean_luma <= 1.0);
}

#[test]
fn art_path_is_local_data() {
    assert_eq!(grass::BLADE_ART_PATH, "data\\texture\\nostalre\\grass.png");
}

// K5 pipeline

#[test]
fn instance_layout_matches() {
    assert_eq!(std::mem::size_of::<Tuft>(), 16);
    let layout = Tuft::LAYOUT;
    assert_eq!(layout.array_stride, 16);
    assert_eq!(layout.step_mode, wgpu::VertexStepMode::Instance);
    assert_eq!(layout.attributes[0].offset as usize, std::mem::offset_of!(Tuft, root));
    assert_eq!(layout.attributes[1].offset as usize, std::mem::offset_of!(Tuft, packed));
    assert_eq!(layout.attributes[1].format, wgpu::VertexFormat::Uint32);
}

#[test]
fn vertex_count_per_tier() {
    assert_eq!(grass::vertices_per_tuft(Tier::Low), 12);
    assert_eq!(grass::vertices_per_tuft(Tier::Medium), 12);
    assert_eq!(grass::vertices_per_tuft(Tier::High), 18, "up to 3 quads, the tuft's seed picks 2 or 3");
    assert_eq!(grass::vertices_per_tuft(Tier::Ultra), 18);
}

#[test]
fn grass_shader_validates() {
    let src = grass::GRASS_WGSL;
    let module = naga::front::wgsl::parse_str(src)
        .unwrap_or_else(|e| panic!("grass.wgsl failed to parse: {}", e.emit_to_string(src)));
    naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::all())
        .validate(&module)
        .unwrap_or_else(|e| panic!("grass.wgsl failed validation: {e:?}"));
}

#[test]
fn tier_from_index() {
    assert_eq!(Tier::from_index(0), Tier::Low);
    assert_eq!(Tier::from_index(3), Tier::Ultra);
    assert_eq!(Tier::from_index(9), Tier::Ultra);
}
