//! Water meshes for maps whose GND splits the water into zones, each with its own level and type.

use nostalre_formats::gnd::{GndCell, GndFile, GndSurface, GndWater, GndWaterZone};
use nostalre_formats::map_water::map_water;
use nostalre_formats::rsw::WaterSettings;
use nostalre_renderer::water::{build_zone_mesh, caustics_level};

fn zone(level: f32, water_type: i32) -> GndWaterZone {
    GndWaterZone {
        level,
        water_type,
        wave_height: 1.0,
        wave_speed: 2.0,
        wave_pitch: 50.0,
        anim_speed: 3,
    }
}

/// A 4×4 map, ground at `ground` everywhere, water split 2×2 into `zones`.
fn zoned_gnd(ground: f32, zones: [GndWaterZone; 4]) -> GndFile {
    GndFile {
        version: (1, 9),
        width: 4,
        height: 4,
        zoom: 10.0,
        textures: vec![],
        lightmaps: vec![],
        surfaces: vec![GndSurface {
            tex_u: [0.0; 4],
            tex_v: [0.0; 4],
            texture_id: 0,
            lightmap_id: 0,
            color_bgra: [255; 4],
        }],
        cells: (0..16)
            .map(|_| GndCell {
                height_sw: ground,
                height_se: ground,
                height_nw: ground,
                height_ne: ground,
                surface_up: 0,
                surface_south: -1,
                surface_east: -1,
            })
            .collect(),
        water: Some(GndWater {
            main: zones[0],
            split_width: 2,
            split_height: 2,
            zones: zones.to_vec(),
        }),
    }
}

fn no_rsw_water() -> WaterSettings {
    WaterSettings {
        level: None,
        water_type: None,
        wave_height: None,
        wave_speed: None,
        wave_pitch: None,
        anim_speed: None,
    }
}

/// Cells (x, y) that got a quad, from the quads' south-west corners.
fn wet_cells(vertices: &[nostalre_renderer::water::WaterVertex]) -> Vec<(i32, i32)> {
    let mut cells: Vec<(i32, i32)> = vertices
        .chunks(4)
        .map(|q| {
            (
                (q[0].position[0] / 10.0) as i32,
                (q[0].position[2] / 10.0) as i32,
            )
        })
        .collect();
    cells.sort();
    cells
}

#[test]
fn water_mesh_uses_zone_level() {
    // ground at 0 (Y grows down): a zone at -20 is 20 above it (wet), a zone at 20 lies under it (dry)
    let gnd = zoned_gnd(
        0.0,
        [zone(-20.0, 0), zone(20.0, 0), zone(20.0, 0), zone(-5.0, 0)],
    );
    let water = map_water(&no_rsw_water(), &gnd).unwrap();
    let (vertices, indices) = build_zone_mesh(&gnd, &water, 0);
    assert_eq!(
        wet_cells(&vertices),
        vec![
            (0, 0),
            (0, 1),
            (1, 0),
            (1, 1),
            (2, 2),
            (2, 3),
            (3, 2),
            (3, 3)
        ]
    );
    assert_eq!(indices.len(), 8 * 6);
    for q in vertices.chunks(4) {
        let expected = if q[0].position[0] < 20.0 { -20.0 } else { -5.0 };
        assert!(
            q.iter().all(|v| v.position[1] == expected),
            "quad at {:?}",
            q[0].position
        );
    }
}

#[test]
fn each_water_type_gets_its_own_mesh() {
    let gnd = zoned_gnd(
        0.0,
        [
            zone(-20.0, 0),
            zone(-20.0, 5),
            zone(-20.0, 5),
            zone(-20.0, 0),
        ],
    );
    let water = map_water(&no_rsw_water(), &gnd).unwrap();
    assert_eq!(water.types(), vec![0, 5]);
    let (clear, _) = build_zone_mesh(&gnd, &water, 0);
    let (swamp, _) = build_zone_mesh(&gnd, &water, 5);
    assert_eq!(
        wet_cells(&clear),
        vec![
            (0, 0),
            (0, 1),
            (1, 0),
            (1, 1),
            (2, 2),
            (2, 3),
            (3, 2),
            (3, 3)
        ]
    );
    assert_eq!(
        wet_cells(&swamp),
        vec![
            (0, 2),
            (0, 3),
            (1, 2),
            (1, 3),
            (2, 0),
            (2, 1),
            (3, 0),
            (3, 1)
        ]
    );
}

#[test]
fn caustics_only_when_the_water_has_one_level() {
    let one = zoned_gnd(
        0.0,
        [
            zone(-20.0, 0),
            zone(-20.0, 5),
            zone(-20.0, 5),
            zone(-20.0, 0),
        ],
    );
    let one = map_water(&no_rsw_water(), &one).unwrap();
    assert_eq!(caustics_level(&one), Some(-20.0));
    let many = zoned_gnd(
        0.0,
        [
            zone(-20.0, 0),
            zone(-5.0, 0),
            zone(-20.0, 0),
            zone(-20.0, 0),
        ],
    );
    let many = map_water(&no_rsw_water(), &many).unwrap();
    assert_eq!(caustics_level(&many), None);
}
