use glam::{Mat4, Vec3};
use nostalre_formats::gnd::{GndCell, GndFile, GndSurface};
use nostalre_renderer::grass::atlas::{self, ATLAS_HEIGHT, ATLAS_WIDTH, KINDS};
use nostalre_renderer::grass::chunks::{self, CHUNK_CELLS, ChunkBox};
use nostalre_renderer::grass::classify::{self, Flora, Ground, Kind, TextureInfo};
use nostalre_renderer::grass::scatter::{FLORA_DEFAULT, FieldInput, GrassField, TUFT_MAX_HEIGHT, Tier};
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
            tufts.extend(field.chunk_tufts(cx, cz, tier, density, 0.0));
        }
    }
    tufts
}

// K1 classify

#[test]
fn classify_keyword_grass() {
    for name in [
        "필드바닥\\prt_초원01.bmp",
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
    let a = grass_field(&gnd, &tex, "prt_fild08", &[]).chunk_tufts(1, 0, Tier::High, 1.0, 0.0);
    let b = grass_field(&gnd, &tex, "prt_fild08", &[]).chunk_tufts(1, 0, Tier::High, 1.0, 0.0);
    assert!(!a.is_empty());
    assert_eq!(bytes(&a), bytes(&b), "a chunk regenerates identically");
}

#[test]
fn scatter_differs_between_maps() {
    let gnd = field(32);
    let tex = grass_texture();
    let a = grass_field(&gnd, &tex, "prt_fild08", &[]).chunk_tufts(0, 0, Tier::High, 1.0, 0.0);
    let b = grass_field(&gnd, &tex, "prt_fild07", &[]).chunk_tufts(0, 0, Tier::High, 1.0, 0.0);
    assert_ne!(bytes(&a), bytes(&b));
}

#[test]
fn chunk_holds_its_cells_only() {
    let gnd = field(32);
    let tufts = grass_field(&gnd, &grass_texture(), "m", &[]).chunk_tufts(1, 0, Tier::High, 1.0, 0.0);
    let side = CHUNK_CELLS as f32 * gnd.zoom;
    assert!(tufts.iter().all(|t| (side..2.0 * side).contains(&t.root[0]) && (0.0..side).contains(&t.root[2])));
}

#[test]
fn dense_per_cell() {
    let gnd = field(16);
    let field = grass_field(&gnd, &grass_texture(), "m", &[]);
    let cells = (CHUNK_CELLS * CHUNK_CELLS) as f32;
    for (tier, per_cell) in [(Tier::Low, 4.0), (Tier::Medium, 6.0), (Tier::High, 8.0), (Tier::Ultra, 10.0)] {
        let n = field.chunk_tufts(0, 0, tier, 1.0, 0.0).len() as f32 / cells;
        assert!((n - per_cell).abs() < 0.5, "{tier:?}: {n} per cell");
        assert_eq!(tier.per_cell(), per_cell);
    }
}

#[test]
fn scatter_slider_zero_none() {
    let gnd = field(16);
    let field = grass_field(&gnd, &grass_texture(), "prt_fild08", &[]);
    assert!(field.chunk_tufts(0, 0, Tier::Ultra, 0.0, 0.0).is_empty());
    let full = field.chunk_tufts(0, 0, Tier::Ultra, 1.0, 0.0);
    let half = field.chunk_tufts(0, 0, Tier::Ultra, 0.5, 0.0);
    assert!(half.len() * 3 < full.len() * 2 && half.len() * 3 > full.len(), "half {} of {}", half.len(), full.len());
    assert!(half.iter().all(|t| full.contains(t)), "the slider thins the same tufts, it does not move them");
}

#[test]
fn sparse_cells_get_fewer() {
    let gnd = field(16);
    let grass = grass_field(&gnd, &grass_texture(), "m", &[]).chunk_tufts(0, 0, Tier::High, 1.0, 0.0).len();
    let sparse_tex = vec![TextureInfo {
        class: Ground::Sparse,
        rgb: GREEN,
    }];
    let sparse = grass_field(&gnd, &sparse_tex, "m", &[]).chunk_tufts(0, 0, Tier::High, 1.0, 0.0).len();
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
    let vw = width / KINDS;
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
    for v in 0..KINDS {
        let c = coverage(&a[0], ATLAS_WIDTH as usize, v);
        assert!((0.12..=0.6).contains(&c), "kind {v}: {c}");
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
    assert_eq!(art.levels[0].len(), 4 * 4 * 4 * 3, "the grass row over two flora rows");
    assert!(art.levels.len() >= 2, "mips");
}

#[test]
fn procedural_art_is_grey() {
    let art = atlas::BladeArt::procedural();
    assert_eq!(art.variants, KINDS as u32);
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

// S4b F1 grass kinds by ground

const DARK_GREEN: [f32; 3] = [0.12, 0.25, 0.08];

#[test]
fn classify_forest_floor() {
    assert_eq!(classify::classify_texture("필드바닥\\숲속바닥-01.bmp", [0.5; 3]), Ground::Forest);
    assert_eq!(classify::classify_texture("필드바닥\\게펜필드-04.bmp", DARK_GREEN), Ground::Forest, "dark green");
    assert_eq!(classify::classify_texture("필드바닥\\게펜필드-04.bmp", GREEN), Ground::Grass, "bright green");
}

#[test]
fn ground_kind_map() {
    assert_eq!(Ground::Sparse.kind(), Some(Kind::Dry));
    assert_eq!(Ground::Grass.kind(), Some(Kind::Tall));
    assert_eq!(Ground::Forest.kind(), Some(Kind::Bush));
    assert_eq!(Ground::Never.kind(), None);
    // the kinds are the picture's columns, left to right
    assert_eq!([Kind::Dry as u32, Kind::Tall as u32, Kind::Bush as u32], [0, 1, 2]);
}

fn opaque_art(w: u32, h: u32) -> image::RgbaImage {
    image::RgbaImage::from_fn(w, h, |x, _| image::Rgba([90, 140, 60, if x % 3 == 0 { 255 } else { 0 }]))
}

#[test]
fn wide_picture_holds_kinds() {
    assert_eq!(atlas::BladeArt::from_image(&opaque_art(768, 256)).variants, 3);
    assert_eq!(atlas::BladeArt::from_image(&opaque_art(384, 128)).variants, 3);
    assert_eq!(atlas::BladeArt::from_image(&opaque_art(256, 128)).variants, 2);
    assert_eq!(atlas::BladeArt::from_image(&opaque_art(1024, 128)).variants, 4, "at most 4");
}

#[test]
fn single_picture_unchanged() {
    let art = atlas::BladeArt::from_image(&opaque_art(128, 128));
    assert_eq!(art.variants, 1);
    assert!(art.colored);
}

fn info(class: Ground, rgb: [f32; 3]) -> Vec<TextureInfo> {
    vec![TextureInfo { class, rgb }]
}

fn kind_share(tufts: &[Tuft], kind: Kind) -> f32 {
    tufts.iter().filter(|t| t.kind() == kind as u32).count() as f32 / tufts.len().max(1) as f32
}

#[test]
fn kind_follows_ground() {
    let gnd = field(32);
    for (class, rgb, kind) in [
        (Ground::Sparse, GREEN, Kind::Dry),
        (Ground::Grass, GREEN, Kind::Tall),
        (Ground::Forest, DARK_GREEN, Kind::Bush),
    ] {
        let tufts = all_tufts(&grass_field(&gnd, &info(class, rgb), "m", &[]), Tier::Ultra, 1.0);
        assert!(tufts.len() > 1000, "{class:?}: {} tufts", tufts.len());
        let own = kind_share(&tufts, kind);
        assert!((0.75..=0.85).contains(&own), "{class:?}: own kind {own}");
        assert!(tufts.iter().all(|t| t.kind() <= 2), "{class:?}: kind in range");
    }
}

#[test]
fn forest_grows_like_grass() {
    let gnd = field(16);
    let grass = all_tufts(&grass_field(&gnd, &info(Ground::Grass, GREEN), "m", &[]), Tier::Ultra, 1.0);
    let forest = all_tufts(&grass_field(&gnd, &info(Ground::Forest, DARK_GREEN), "m", &[]), Tier::Ultra, 1.0);
    assert_eq!(grass.len(), forest.len());
}

#[test]
fn kinds_blend_at_borders() {
    // west half hills (dry), east half meadow (tall)
    let mut gnd = field(32);
    gnd.textures.push("필드바닥\\prt_언덕04.bmp".to_string());
    gnd.surfaces.push(surface(1));
    for y in 0..32 {
        for x in 0..16 {
            gnd.cells[(y * 32 + x) as usize].surface_up = 1;
        }
    }
    let tex = vec![
        TextureInfo { class: Ground::Grass, rgb: GREEN },
        TextureInfo { class: Ground::Sparse, rgb: GREEN },
    ];
    let tufts = all_tufts(&grass_field(&gnd, &tex, "m", &[]), Tier::Ultra, 1.0);
    let column = |t: &Tuft| (t.root[0] / gnd.zoom).floor() as i32;
    let near: Vec<Tuft> = tufts.iter().copied().filter(|t| column(t) == 16).collect();
    let far: Vec<Tuft> = tufts.iter().copied().filter(|t| column(t) >= 24).collect();
    let (dry_near, dry_far) = (kind_share(&near, Kind::Dry), kind_share(&far, Kind::Dry));
    assert!(dry_near > dry_far + 0.03, "meadow next to the hill takes its dry kind: {dry_near} vs {dry_far}");
    assert!(dry_far < 0.15, "far from the hill few dry: {dry_far}");
}

#[test]
fn floor_reach_covers_tuft() {
    // a floor lower than the tallest tuft lets grass poke through it
    const { assert!(classify::FLOOR_REACH >= TUFT_MAX_HEIGHT + 1.0) };
}

// S4b F2 own art: three kinds

#[test]
fn procedural_has_three_kinds() {
    let art = atlas::BladeArt::procedural();
    assert_eq!(art.variants, 3);
    assert_eq!(KINDS, 3);
    let aspect = art.width as f32 / art.variants as f32 / art.height as f32;
    assert!((aspect - 0.94).abs() < 0.01, "card as wide as before: {aspect}");
}

/// Opaque share of kind `k`'s top quarter rows: how much of it reaches high.
fn top_coverage(level: &[u8], k: usize) -> f32 {
    let (w, h) = (ATLAS_WIDTH as usize, ATLAS_HEIGHT as usize);
    let vw = w / KINDS;
    let mut opaque = 0;
    for y in 0..h / 4 {
        for x in k * vw..(k + 1) * vw {
            opaque += (level[(y * w + x) * 4 + 3] >= 128) as usize;
        }
    }
    opaque as f32 / (vw * h / 4) as f32
}

#[test]
fn kinds_differ_in_height_and_coverage() {
    let a = atlas::blade_atlas();
    let (dry, tall, bush) = (Kind::Dry as usize, Kind::Tall as usize, Kind::Bush as usize);
    let w = ATLAS_WIDTH as usize;
    assert!(top_coverage(&a[0], tall) > top_coverage(&a[0], bush), "tall reaches higher than bush");
    assert_eq!(top_coverage(&a[0], bush), 0.0, "bush stays low");
    assert!(top_coverage(&a[0], dry) > 0.0, "dry stalks reach up");
    let low = |k: usize| {
        let (h, vw) = (ATLAS_HEIGHT as usize, w / KINDS);
        let mut opaque = 0;
        for y in h * 2 / 3..h {
            for x in k * vw..(k + 1) * vw {
                opaque += (a[0][(y * w + x) * 4 + 3] >= 128) as usize;
            }
        }
        opaque as f32 / (vw * (h - h * 2 / 3)) as f32
    };
    assert!(low(bush) > low(tall), "bush densest near the ground: {} vs {}", low(bush), low(tall));
    assert!(low(bush) > low(dry), "{} vs {}", low(bush), low(dry));
}

fn tint(t: &Tuft) -> [f32; 3] {
    let p = t.packed;
    [(p >> 11 & 31) as f32 / 31.0, (p >> 5 & 63) as f32 / 63.0, (p & 31) as f32 / 31.0]
}

#[test]
fn dry_kind_tinted_straw() {
    let gnd = field(16);
    let tufts = all_tufts(&grass_field(&gnd, &info(Ground::Grass, GREEN), "m", &[]), Tier::Ultra, 1.0);
    let red_share = |kind: Kind| {
        let of: Vec<[f32; 3]> = tufts.iter().filter(|t| t.kind() == kind as u32).map(tint).collect();
        of.iter().map(|c| c[0] / c[1].max(0.01)).sum::<f32>() / of.len().max(1) as f32
    };
    assert!(red_share(Kind::Dry) > red_share(Kind::Tall) + 0.15, "dry {} tall {}", red_share(Kind::Dry), red_share(Kind::Tall));
}

// S4b F3 flora

fn tufts_with_flora(field: &GrassField, flora: f32) -> Vec<Tuft> {
    let (cw, ch) = field.chunk_grid();
    let mut tufts = Vec::new();
    for cz in 0..ch {
        for cx in 0..cw {
            tufts.extend(field.chunk_tufts(cx, cz, Tier::Ultra, 1.0, flora));
        }
    }
    tufts
}

/// Flora but the leafy plants (a ground cover with its own rate)
fn rare_flora(t: &Tuft) -> bool {
    t.is_flora() && t.kind() != Flora::Leafy.kind()
}

/// (rare flora, grass) tufts on a 64 x 64 field of one ground
fn flora_share(class: Ground, rgb: [f32; 3]) -> (usize, usize) {
    let gnd = field(64);
    let tufts = tufts_with_flora(&grass_field(&gnd, &info(class, rgb), "gef_fild04", &[]), FLORA_DEFAULT);
    let flora = tufts.iter().filter(|t| rare_flora(t)).count();
    (flora, tufts.iter().filter(|t| !t.is_flora()).count())
}

#[test]
fn leafy_plants_common_on_green_ground() {
    let gnd = field(64);
    let rate = |class, rgb| {
        let tufts = tufts_with_flora(&grass_field(&gnd, &info(class, rgb), "gef_fild04", &[]), FLORA_DEFAULT);
        let leafy = tufts.iter().filter(|t| t.kind() == Flora::Leafy.kind()).count() as f32;
        leafy / tufts.iter().filter(|t| !t.is_flora()).count().max(1) as f32
    };
    let forest = rate(Ground::Forest, DARK_GREEN);
    assert!((1.0 / 15.0 * 0.7..=1.0 / 15.0 * 1.3).contains(&forest), "forest floor 1 in {}", 1.0 / forest);
    let meadow = rate(Ground::Grass, GREEN);
    assert!((1.0 / 40.0 * 0.7..=1.0 / 40.0 * 1.3).contains(&meadow), "meadow 1 in {}", 1.0 / meadow);
    assert_eq!(rate(Ground::Sparse, GREEN), 0.0, "none on hills and dirt");
}

#[test]
fn flora_is_rare() {
    let (forest, forest_grass) = flora_share(Ground::Forest, DARK_GREEN);
    let rate = forest as f32 / forest_grass as f32;
    assert!((1.0 / 40.0 * 0.7..=1.0 / 40.0 * 1.3).contains(&rate), "forest floor 1 in {}", 1.0 / rate);
    let (meadow, meadow_grass) = flora_share(Ground::Grass, GREEN);
    let rate = meadow as f32 / meadow_grass as f32;
    assert!((1.0 / 120.0 * 0.7..=1.0 / 120.0 * 1.3).contains(&rate), "meadow 1 in {}", 1.0 / rate);
}

#[test]
fn flora_is_clustered() {
    let gnd = field(64);
    let tufts = tufts_with_flora(&grass_field(&gnd, &info(Ground::Forest, DARK_GREEN), "gef_fild04", &[]), FLORA_DEFAULT);
    let mut per_cell = vec![0u32; 64 * 64];
    for t in tufts.iter().filter(|t| rare_flora(t)) {
        let (x, y) = ((t.root[0] / gnd.zoom) as usize, (t.root[2] / gnd.zoom) as usize);
        per_cell[y * 64 + x] += 1;
    }
    let mean = per_cell.iter().sum::<u32>() as f32 / per_cell.len() as f32;
    // per 8 x 8 block, the densest quarter of blocks against the field's mean
    let mut blocks: Vec<f32> = (0..64)
        .map(|b| {
            let (bx, by) = (b % 8 * 8, b / 8 * 8);
            (0..64).map(|i| per_cell[(by + i / 8) * 64 + bx + i % 8]).sum::<u32>() as f32 / 64.0
        })
        .collect();
    blocks.sort_by(|a, b| b.total_cmp(a));
    let top = blocks[..16].iter().sum::<f32>() / 16.0;
    assert!(top >= 3.0 * mean, "patches: densest quarter {top} vs mean {mean}");
    assert!(blocks[48..].iter().all(|&b| b < mean), "and bare stretches");
}

#[test]
fn no_flora_on_sparse_or_paths() {
    let (sparse, _) = flora_share(Ground::Sparse, GREEN);
    assert_eq!(sparse, 0, "hills and dirt");
    let (never, grass) = flora_share(Ground::Never, GREEN);
    assert_eq!((never, grass), (0, 0), "paths, towns");
}

#[test]
fn flora_kinds_by_ground() {
    let gnd = field(64);
    let count = |class, rgb, flora: Flora| {
        tufts_with_flora(&grass_field(&gnd, &info(class, rgb), "gef_fild04", &[]), 1.0)
            .iter()
            .filter(|t| t.kind() == flora.kind())
            .count()
    };
    assert!(count(Ground::Forest, DARK_GREEN, Flora::Fern) > count(Ground::Forest, DARK_GREEN, Flora::Flower), "ferns in forests");
    assert!(count(Ground::Forest, DARK_GREEN, Flora::NestFern) > count(Ground::Grass, GREEN, Flora::NestFern), "nest ferns in forests");
    assert!(count(Ground::Grass, GREEN, Flora::Flower) > count(Ground::Grass, GREEN, Flora::Fern), "flowers in meadows");
    assert!(count(Ground::Forest, DARK_GREEN, Flora::BrownMushroom) > count(Ground::Forest, DARK_GREEN, Flora::RedMushroom), "brown commoner");
    assert!(count(Ground::Forest, DARK_GREEN, Flora::RedMushroom) > 0);
}

#[test]
fn packed_kind_bits_round_trip() {
    let gnd = field(32);
    let tufts = tufts_with_flora(&grass_field(&gnd, &info(Ground::Forest, DARK_GREEN), "gef_fild04", &[]), 1.0);
    let kinds: std::collections::BTreeSet<u32> = tufts.iter().map(|t| t.kind()).collect();
    assert_eq!(kinds, (0..9).collect(), "every grass kind and flora present, nothing else");
    for flora in [Flora::Fern, Flora::BrownMushroom, Flora::RedMushroom, Flora::Flower, Flora::Leafy, Flora::NestFern] {
        assert!(flora.kind() >= 3 && flora.kind() < 16, "4 bits");
    }
    assert!(tufts.iter().all(|t| t.size() >= 0.85 && t.size() <= 1.6), "size bits untouched");
}

#[test]
fn flora_zero_is_grass_only() {
    let gnd = field(32);
    let field = grass_field(&gnd, &info(Ground::Forest, DARK_GREEN), "gef_fild04", &[]);
    let none = tufts_with_flora(&field, 0.0);
    assert!(none.iter().all(|t| !t.is_flora()));
    let grass_at_default: Vec<Tuft> = tufts_with_flora(&field, FLORA_DEFAULT).into_iter().filter(|t| !t.is_flora()).collect();
    assert_eq!(bytes(&none), bytes(&grass_at_default), "flora adds tufts, the grass stays the same");
    let more = tufts_with_flora(&field, 1.0).iter().filter(|t| t.is_flora()).count();
    let default = tufts_with_flora(&field, FLORA_DEFAULT).iter().filter(|t| t.is_flora()).count();
    assert!(more > default, "{more} vs {default}");
}

#[test]
fn art_has_flora_row() {
    for art in [atlas::BladeArt::procedural(), atlas::BladeArt::from_image(&opaque_art(768, 256))] {
        let (w, h) = (art.width as usize, art.height as usize);
        let level = &art.levels[0];
        assert_eq!(level.len(), w * h * 3 * 4);
        // each flora cell holds a drawing: two rows of three under the grass row
        let cw = w / 3;
        let cell = |f: usize| {
            let (x0, y0) = (f % 3 * cw, h * (1 + f / 3));
            (y0..y0 + h).flat_map(move |y| (x0..x0 + cw).map(move |x| (y, x)))
        };
        for f in 0..6 {
            let opaque = cell(f).filter(|&(y, x)| level[(y * w + x) * 4 + 3] >= 128).count();
            assert!(opaque > cw * h / 50, "flora {f}: {opaque}");
        }
        // mushrooms and flowers keep colours
        let colourful = (1..4)
            .flat_map(cell)
            .map(|(y, x)| &level[(y * w + x) * 4..][..4])
            .filter(|p| p[3] >= 128 && (p[0] as i32 - p[1] as i32).abs() > 60)
            .count();
        assert!(colourful > 50, "{colourful}");
    }
}
