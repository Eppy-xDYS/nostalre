use nostalre_formats::gat::{GatCell, GatFile};
use nostalre_formats::gnd::{GndCell, GndFile, GndSurface, Lightmap};
use nostalre_game::lightmap::ActorLightmap;

/// A 4x4 ground: the west half unlit (no baked colour), the east half with a red bake of 64. GAT at 2x, all flat.
fn split_map() -> (GndFile, GatFile) {
    let lightmaps = vec![
        Lightmap {
            shadow: [255; 64],
            color: [0; 192],
        },
        Lightmap {
            shadow: [255; 64],
            color: {
                let mut c = [0u8; 192];
                for texel in c.chunks_mut(3) {
                    texel[0] = 64;
                }
                c
            },
        },
    ];
    let gnd = GndFile {
        version: (1, 7),
        width: 4,
        height: 4,
        zoom: 10.0,
        textures: vec!["t.bmp".into()],
        lightmaps,
        surfaces: (0..16)
            .map(|i| GndSurface {
                tex_u: [0.0; 4],
                tex_v: [0.0; 4],
                texture_id: 0,
                lightmap_id: if i % 4 < 2 { 0 } else { 1 },
                color_bgra: [255; 4],
            })
            .collect(),
        cells: (0..16)
            .map(|i| GndCell {
                height_sw: 0.0,
                height_se: 0.0,
                height_nw: 0.0,
                height_ne: 0.0,
                surface_up: i,
                surface_south: -1,
                surface_east: -1,
            })
            .collect(),
    };
    let gat = GatFile {
        version: (1, 2),
        width: 8,
        height: 8,
        cells: (0..64)
            .map(|_| GatCell {
                height_sw: 0.0,
                height_se: 0.0,
                height_nw: 0.0,
                height_ne: 0.0,
                cell_flags: 0,
            })
            .collect(),
    };
    (gnd, gat)
}

#[test]
fn baked_at_pos_raw_colour() {
    let (gnd, gat) = split_map();
    let lm = ActorLightmap::build(&gnd, &gat).unwrap();
    let lit = lm.baked_at_pos(5.5, 5.5);
    assert!((lit[0] - 64.0 / 255.0).abs() < 1e-4, "{lit:?}");
    assert_eq!(lit[1], 0.0);
    assert_eq!(lit[2], 0.0);
    assert_eq!(lm.baked_at_pos(1.5, 1.5), [0.0; 3], "no bake in the west half");
}

#[test]
fn baked_at_pos_zero_off_map() {
    let (gnd, gat) = split_map();
    let lm = ActorLightmap::build(&gnd, &gat).unwrap();
    assert_eq!(lm.baked_at_pos(-1.0, 2.0), [0.0; 3]);
    assert_eq!(lm.baked_at_pos(8.5, 2.0), [0.0; 3]);
    assert_eq!(lm.baked_at_pos(2.0, 99.0), [0.0; 3]);
}
