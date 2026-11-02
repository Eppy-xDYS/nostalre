//! Where to look for flora on real maps: per map, the flora count and the walk cells with the most flowers and
//! the most mushrooms nearby.
//! Run with `NOSTALRE_GAME_DATA=<dir with data.grf, official_data.grf> cargo test -p nostalre-renderer --test
//! flora_spots -- --ignored --nocapture` (`NOSTALRE_MAPS=prt_fild08,pay_fild01` to pick the maps).

use nostalre_formats::gnd::GndFile;
use nostalre_formats::grf::GrfArchive;
use nostalre_renderer::grass::classify::{self, Flora, TextureInfo};
use nostalre_renderer::grass::scatter::{FLORA_DEFAULT, FieldInput, GrassField, Tier};

const MAPS: &str = "prt_fild08,prt_fild01,pay_fild01,gef_fild04,gef_fild07,mjolnir_01";

#[test]
#[ignore = "needs the game data"]
fn flora_spots() {
    let Ok(dir) = std::env::var("NOSTALRE_GAME_DATA") else {
        eprintln!("Skipping: NOSTALRE_GAME_DATA not set");
        return;
    };
    let grf = GrfArchive::open_layered(&[format!("{dir}/data.grf"), format!("{dir}/official_data.grf")], None)
        .expect("grf");
    let maps = std::env::var("NOSTALRE_MAPS").unwrap_or_else(|_| MAPS.to_string());
    for map in maps.split(',') {
        let Ok(data) = grf.read_file(&format!("data\\{map}.gnd")) else {
            eprintln!("{map}: no gnd");
            continue;
        };
        let gnd = GndFile::parse(&data).expect("gnd");
        let textures: Vec<TextureInfo> = gnd
            .textures
            .iter()
            .map(|name| {
                let path = nostalre_resources::texture::named(name);
                let rgb = nostalre_renderer::texture::average_color(&path, &grf).unwrap_or([0.0; 3]);
                TextureInfo { class: classify::classify_texture(name, rgb), rgb }
            })
            .collect();
        let field = GrassField::new(&FieldInput { gnd: &gnd, map, textures: &textures, water: None, floors: &[] });
        let (cw, ch) = field.chunk_grid();
        let cells = (gnd.width * gnd.height) as usize;
        let (mut flowers, mut shrooms) = (vec![0u32; cells], vec![0u32; cells]);
        let (mut ferns, mut mushrooms, mut total) = (0, 0, 0);
        for cz in 0..ch {
            for cx in 0..cw {
                for t in field.chunk_tufts(cx, cz, Tier::High, 1.0, FLORA_DEFAULT) {
                    let k = t.kind();
                    total += t.is_flora() as u32;
                    ferns += (k == Flora::Fern.kind()) as u32;
                    let mushroom = k == Flora::BrownMushroom.kind() || k == Flora::RedMushroom.kind();
                    mushrooms += mushroom as u32;
                    let i = ((t.root[2] / gnd.zoom) as i32 * gnd.width + (t.root[0] / gnd.zoom) as i32) as usize;
                    if k == Flora::Flower.kind() {
                        flowers[i] += 1;
                    } else if mushroom {
                        shrooms[i] += 1;
                    }
                }
            }
        }
        let all: u32 = flowers.iter().sum();
        let (fb, fx, fy) = densest(&flowers, gnd.width, gnd.height);
        let (mb, mx, my) = densest(&shrooms, gnd.width, gnd.height);
        eprintln!(
            "{map}: flora {total} (ferns {ferns}, mushrooms {mushrooms}, flowers {all}); \
             flowers ({fb}) /warp {map} {fx} {fy}; mushrooms ({mb}) /warp {map} {mx} {my}"
        );
    }
}

/// The 6 x 6 GND cells (12 x 12 walk cells) with the most: their count and centre in walk cells (a GND cell is
/// 2 x 2 walk cells).
fn densest(per_cell: &[u32], width: i32, height: i32) -> (u32, i32, i32) {
    let (mut best, mut at) = (0, (0, 0));
    for y in 0..height - 6 {
        for x in 0..width - 6 {
            let n: u32 = (0..36).map(|i| per_cell[((y + i / 6) * width + x + i % 6) as usize]).sum();
            if n > best {
                (best, at) = (n, (x + 3, y + 3));
            }
        }
    }
    (best, at.0 * 2, at.1 * 2)
}
