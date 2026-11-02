use std::collections::HashSet;

use nostalre_renderer::texture::split_map_textures;

#[test]
fn map_textures_drop_all_but_pinned() {
    let world = ["data\\texture\\grid.tga", "data\\texture\\model\\tree.bmp", "data\\texture\\water\\water000.jpg"]
        .map(String::from)
        .to_vec();
    let pinned: HashSet<String> = ["data\\texture\\grid.tga".to_string()].into();
    let (keep, drop) = split_map_textures(world, &pinned);
    assert_eq!(keep, ["data\\texture\\grid.tga"]);
    assert_eq!(drop, ["data\\texture\\model\\tree.bmp", "data\\texture\\water\\water000.jpg"]);
}

#[test]
fn map_textures_nothing_pinned_drops_all() {
    let (keep, drop) = split_map_textures(vec!["a.bmp".into(), "b.bmp".into()], &HashSet::new());
    assert!(keep.is_empty());
    assert_eq!(drop.len(), 2);
}
