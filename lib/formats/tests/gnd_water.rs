//! Water stored in the GND (version 1.8+), as maps with an RSW 2.6+ keep it, and the pick between RSW and GND water.

use nostalre_formats::gnd::{GndFile, GndWaterZone};
use nostalre_formats::grf::GrfArchive;
use nostalre_formats::map_water::{MapWater, map_water};
use nostalre_formats::rsw::{RswFile, WaterSettings};

/// A 1×1 GND of `version` up to the end of its cells.
fn gnd_bytes(version: (u8, u8), width: i32, height: i32) -> Vec<u8> {
    let mut d = Vec::new();
    d.extend_from_slice(b"GRGN");
    d.extend_from_slice(&[version.0, version.1]);
    d.extend_from_slice(&width.to_le_bytes());
    d.extend_from_slice(&height.to_le_bytes());
    d.extend_from_slice(&10.0f32.to_le_bytes());
    // no textures (count, name length), no lightmaps (count, w, h, cells), no surfaces
    for v in [0i32, 80, 0, 8, 8, 1, 0] {
        d.extend_from_slice(&v.to_le_bytes());
    }
    for _ in 0..width * height {
        for _ in 0..4 {
            d.extend_from_slice(&0.0f32.to_le_bytes());
        }
        for _ in 0..3 {
            d.extend_from_slice(&(-1i32).to_le_bytes());
        }
    }
    d
}

fn push_zone(d: &mut Vec<u8>, z: GndWaterZone) {
    d.extend_from_slice(&z.level.to_le_bytes());
    d.extend_from_slice(&z.water_type.to_le_bytes());
    d.extend_from_slice(&z.wave_height.to_le_bytes());
    d.extend_from_slice(&z.wave_speed.to_le_bytes());
    d.extend_from_slice(&z.wave_pitch.to_le_bytes());
    d.extend_from_slice(&z.anim_speed.to_le_bytes());
}

fn zone(level: f32, water_type: i32) -> GndWaterZone {
    GndWaterZone {
        level,
        water_type,
        wave_height: 1.5,
        wave_speed: 2.0,
        wave_pitch: 70.0,
        anim_speed: 3,
    }
}

fn with_water(
    version: (u8, u8),
    main: GndWaterZone,
    split: (i32, i32),
    zones: &[GndWaterZone],
) -> Vec<u8> {
    let mut d = gnd_bytes(version, 4, 4);
    push_zone(&mut d, main);
    d.extend_from_slice(&split.0.to_le_bytes());
    d.extend_from_slice(&split.1.to_le_bytes());
    for z in zones {
        push_zone(&mut d, *z);
    }
    d
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

#[test]
fn gnd_water_block_parsed() {
    let gnd = GndFile::parse(&with_water((1, 8), zone(-15.0, 2), (1, 1), &[])).unwrap();
    let water = gnd.water.expect("1.8 water block");
    assert_eq!(water.main, zone(-15.0, 2));
    assert_eq!((water.split_width, water.split_height), (1, 1));
    assert!(water.zones.is_empty());
}

#[test]
fn gnd_water_zones_parsed() {
    let zones = [zone(10.0, 0), zone(20.0, 5), zone(30.0, 0), zone(40.0, 3)];
    let gnd = GndFile::parse(&with_water((1, 9), zone(10.0, 0), (2, 2), &zones)).unwrap();
    let water = gnd.water.expect("1.9 water block");
    assert_eq!(water.zones, zones.to_vec());
}

#[test]
fn gnd_without_water_block_still_parses() {
    let old = GndFile::parse(&gnd_bytes((1, 7), 4, 4)).unwrap();
    assert!(old.water.is_none());
    let mut cut = with_water((1, 8), zone(-15.0, 0), (1, 1), &[]);
    cut.truncate(cut.len() - 10);
    let cut = GndFile::parse(&cut).expect("a short water block is no error");
    assert!(cut.water.is_none());
    assert_eq!(cut.cells.len(), 16);
}

#[test]
fn rsw_water_wins_over_gnd_water() {
    let gnd = GndFile::parse(&with_water((1, 8), zone(-15.0, 2), (1, 1), &[])).unwrap();
    let rsw = WaterSettings {
        level: Some(30.0),
        water_type: Some(1),
        wave_height: Some(1.0),
        wave_speed: Some(2.0),
        wave_pitch: Some(50.0),
        anim_speed: Some(3),
    };
    let water = map_water(&rsw, &gnd).unwrap();
    assert_eq!(water.zone_at(&gnd, 0, 0).level, 30.0);
    assert_eq!(water.zone_at(&gnd, 0, 0).water_type, 1);
}

#[test]
fn water_falls_back_to_gnd_when_rsw_has_none() {
    let gnd = GndFile::parse(&with_water((1, 8), zone(-15.0, 2), (1, 1), &[])).unwrap();
    let water = map_water(&no_rsw_water(), &gnd).unwrap();
    assert_eq!(*water.zone_at(&gnd, 3, 3), zone(-15.0, 2));
    let old = GndFile::parse(&gnd_bytes((1, 7), 4, 4)).unwrap();
    assert!(map_water(&no_rsw_water(), &old).is_none());
}

#[test]
fn zone_of_cell_maps_corners() {
    let zones = [zone(10.0, 0), zone(20.0, 5), zone(30.0, 0), zone(40.0, 3)];
    let gnd = GndFile::parse(&with_water((1, 9), zone(10.0, 0), (2, 2), &zones)).unwrap();
    let water: MapWater = map_water(&no_rsw_water(), &gnd).unwrap();
    assert_eq!(water.zone_at(&gnd, 0, 0).level, 10.0);
    assert_eq!(water.zone_at(&gnd, 3, 3).level, 40.0);
    // x fastest: cell (2,1) of a 4×4 map lies in zone (1,0)
    assert_eq!(water.zone_at(&gnd, 2, 1).level, 20.0);
    assert_eq!(water.zone_at(&gnd, 1, 2).level, 30.0);
    assert_eq!(water.types(), vec![0, 3, 5]);
}

#[test]
fn absurd_zone_split_is_no_water() {
    let mut d = gnd_bytes((1, 9), 4, 4);
    push_zone(&mut d, zone(10.0, 0));
    d.extend_from_slice(&i32::MAX.to_le_bytes());
    d.extend_from_slice(&i32::MAX.to_le_bytes());
    let gnd = GndFile::parse(&d).expect("a bad split is no error");
    assert!(gnd.water.is_none());
}

/// Every map in the local game data parses its GND and finds its water. Run with
/// `NOSTALRE_GAME_DATA=<dir with data.grf, official_data.grf> cargo test -- --ignored`.
#[test]
#[ignore = "needs the game data"]
fn all_maps_load_their_water() {
    let Ok(dir) = std::env::var("NOSTALRE_GAME_DATA") else {
        eprintln!("Skipping: NOSTALRE_GAME_DATA not set");
        return;
    };
    let grfs = [
        format!("{dir}/data.grf"),
        format!("{dir}/official_data.grf"),
    ];
    let grf = GrfArchive::open_layered(&grfs, None).expect("grf");
    let mut maps: Vec<String> = grf
        .layered_file_list()
        .into_iter()
        .map(|f| f.name.to_lowercase().replace('/', "\\"))
        .filter_map(|n| {
            n.strip_prefix("data\\")
                .and_then(|n| n.strip_suffix(".rsw"))
                .map(str::to_string)
        })
        .filter(|n| !n.contains('\\'))
        .collect();
    maps.sort();
    maps.dedup();
    assert!(maps.len() > 1000, "found {} maps", maps.len());
    let (mut wet, mut gnd_wet) = (0, 0);
    for map in &maps {
        let (Ok(rsw), Ok(gnd)) = (
            grf.read_file(&format!("data\\{map}.rsw")),
            grf.read_file(&format!("data\\{map}.gnd")),
        ) else {
            continue;
        };
        let rsw = RswFile::parse(&rsw).unwrap_or_else(|e| panic!("{map} rsw: {e:?}"));
        let gnd = GndFile::parse(&gnd).unwrap_or_else(|e| panic!("{map} gnd: {e:?}"));
        if map_water(&rsw.water, &gnd).is_some() {
            wet += 1;
            gnd_wet += usize::from(rsw.water.level.is_none());
        }
    }
    eprintln!(
        "{} maps, {wet} with water, {gnd_wet} from the GND",
        maps.len()
    );
    assert!(gnd_wet >= 190, "GND water maps: {gnd_wet}");
}
