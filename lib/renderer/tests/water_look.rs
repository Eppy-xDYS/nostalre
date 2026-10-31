//! How much of each map's own RO water texture shows through HD water as detail.

use nostalre_renderer::water::water_material;

#[test]
fn water_types_show_their_texture_as_light_detail() {
    for water_type in [0, 1, 2, 3, 5, 7] {
        let detail = water_material(water_type).detail;
        // enough to keep the map's look, never enough to bring back the texture's blocky tiles
        assert!(detail > 0.0 && detail <= 0.3, "type {water_type}: {detail}");
    }
}

#[test]
fn lava_keeps_its_own_texture_path() {
    for water_type in [4, 6] {
        assert_eq!(water_material(water_type).detail, 0.0, "type {water_type}");
    }
}
