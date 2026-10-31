//! Settings → Graphics → HD (`settings.rs` includes this as a child module).

use super::*;
use nostalre_game::hd::{HdConfig, HdPreset};

fn values() -> SettingsValues {
    SettingsValues {
        chat_lines: vec![String::new(); SLOT_COUNT],
        ..SettingsValues::default()
    }
}

#[test]
fn hd_rows_on_graphics_page() {
    let entries = Page::Graphics.entries();
    let hd = entries
        .iter()
        .position(|e| matches!(e, Entry::Section("HD")))
        .expect("an HD section");
    let rows: Vec<Opt> = entries[hd + 1..]
        .iter()
        .map_while(|e| match e {
            Entry::Opt(o) => Some(*o),
            Entry::Section(_) => None,
        })
        .collect();
    assert_eq!(
        rows,
        vec![
            Opt::Hd,
            Opt::HdQuality,
            Opt::Bloom,
            Opt::LampGlow,
            Opt::LightPools,
            Opt::SunGlow,
            Opt::Grass,
            Opt::SceneLight,
            Opt::CloudShadows,
            Opt::NightLook,
            Opt::DayCycle,
            Opt::Grade,
            Opt::Vignette
        ]
    );
    assert!(rows.iter().all(|o| o.group() == Group::Graphics));
}

#[test]
fn hd_toggle_and_reset() {
    let mut v = values();
    let mut defaults = values();
    assert!(matches!(Opt::Hd.shown_value(&mut v), Shown::Toggle(false)));
    assert!(Opt::Hd.change(&mut v, Change::Flip));
    assert!(v.graphics.hd.enabled);
    assert!(Opt::Hd.reset(&mut v, &mut defaults));
    assert!(!v.graphics.hd.enabled, "HD is off by default");
}

#[test]
fn hd_quality_steps() {
    let mut v = values();
    let shown = |v: &mut SettingsValues| match Opt::HdQuality.shown_value(v) {
        Shown::Choice(label) => label,
        _ => panic!("a choice"),
    };
    assert_eq!(shown(&mut v), "High");
    assert!(Opt::HdQuality.change(&mut v, Change::Next));
    assert_eq!(v.graphics.hd.preset, HdPreset::Ultra);
    assert!(!Opt::HdQuality.change(&mut v, Change::Next), "Ultra is the last");
    assert!(Opt::HdQuality.change(&mut v, Change::Prev));
    assert!(Opt::HdQuality.change(&mut v, Change::Prev));
    assert!(Opt::HdQuality.change(&mut v, Change::Prev));
    assert_eq!(v.graphics.hd.preset, HdPreset::Low);
    assert!(!Opt::HdQuality.change(&mut v, Change::Prev), "Low is the first");
    assert!(Opt::HdQuality.change(&mut v, Change::Flip));
    assert_eq!(v.graphics.hd.preset, HdPreset::Medium, "Enter goes round");

    v.graphics.hd.preset = HdPreset::Custom;
    assert_eq!(shown(&mut v), "Custom");
    assert!(Opt::HdQuality.change(&mut v, Change::Flip));
    assert_eq!(v.graphics.hd.preset, HdPreset::Low, "Custom goes round to Low");
}

#[test]
fn bloom_slider() {
    let mut v = values();
    assert_eq!(v.graphics.hd, HdConfig::default());
    assert!(matches!(Opt::Bloom.shown_value(&mut v), Shown::Slider(60)));
    assert!(Opt::Bloom.change(&mut v, Change::Prev));
    assert!((v.graphics.hd.bloom - 0.55).abs() < 1e-6);
    for _ in 0..20 {
        Opt::Bloom.change(&mut v, Change::Prev);
    }
    assert_eq!(v.graphics.hd.bloom, 0.0, "0 %: off");
    assert!(matches!(Opt::Bloom.shown_value(&mut v), Shown::Slider(0)));
    for _ in 0..30 {
        Opt::Bloom.change(&mut v, Change::Next);
    }
    assert!((v.graphics.hd.bloom - 1.0).abs() < 1e-6, "100 % is the top");
}

#[test]
fn grade_slider_and_vignette_toggle() {
    let mut v = values();
    assert!(matches!(Opt::Grade.shown_value(&mut v), Shown::Slider(80)));
    assert!(Opt::Grade.change(&mut v, Change::Prev));
    assert!((v.graphics.hd.grade - 0.75).abs() < 1e-6);
    for _ in 0..20 {
        Opt::Grade.change(&mut v, Change::Prev);
    }
    assert_eq!(v.graphics.hd.grade, 0.0, "0 %: off");
    assert!(matches!(Opt::Vignette.shown_value(&mut v), Shown::Toggle(true)));
    assert!(Opt::Vignette.change(&mut v, Change::Flip));
    assert!(!v.graphics.hd.vignette);
    let mut defaults = values();
    assert!(Opt::Vignette.reset(&mut v, &mut defaults));
    assert!(Opt::Grade.reset(&mut v, &mut defaults));
    assert!(v.graphics.hd.vignette);
    assert!((v.graphics.hd.grade - 0.8).abs() < 1e-6);
}

const GLOW_ROWS: [Opt; 3] = [Opt::LampGlow, Opt::LightPools, Opt::SunGlow];

#[test]
fn glow_rows_in_hd_section() {
    let entries = Page::Graphics.entries();
    let hd = entries
        .iter()
        .position(|e| matches!(e, Entry::Section("HD")))
        .expect("an HD section");
    let rows: Vec<Opt> = entries[hd + 1..]
        .iter()
        .map_while(|e| match e {
            Entry::Opt(o) => Some(*o),
            Entry::Section(_) => None,
        })
        .collect();
    let bloom = rows.iter().position(|o| *o == Opt::Bloom).unwrap();
    assert_eq!(rows[bloom + 1..bloom + 4], GLOW_ROWS, "right after Bloom");
    let labels: Vec<&str> = GLOW_ROWS.iter().map(|o| o.label()).collect();
    assert_eq!(labels, ["Lamp glow", "Light pools", "Sun glow"]);
    assert!(GLOW_ROWS.iter().all(|o| o.group() == Group::Graphics));
}

#[test]
fn glow_slider_maps_0_to_1() {
    let field = |v: &SettingsValues, o: Opt| match o {
        Opt::LampGlow => v.graphics.hd.lamp_glow,
        Opt::LightPools => v.graphics.hd.light_pools,
        _ => v.graphics.hd.sun_glow,
    };
    for (opt, default) in GLOW_ROWS.into_iter().zip([60, 50, 30]) {
        let mut v = values();
        assert!(
            matches!(opt.shown_value(&mut v), Shown::Slider(s) if s == default),
            "{opt:?} default"
        );
        assert!(opt.change(&mut v, Change::Prev));
        assert!((field(&v, opt) - (default as f32 / 100.0 - 0.05)).abs() < 1e-6);
        for _ in 0..30 {
            opt.change(&mut v, Change::Prev);
        }
        assert_eq!(field(&v, opt), 0.0, "{opt:?} 0 %: off");
        for _ in 0..30 {
            opt.change(&mut v, Change::Next);
        }
        assert!((field(&v, opt) - 1.0).abs() < 1e-6, "{opt:?} 100 % is the top");
        let mut defaults = values();
        assert!(opt.reset(&mut v, &mut defaults));
        assert!(matches!(opt.shown_value(&mut v), Shown::Slider(s) if s == default));
    }
}

#[test]
fn glow_rows_live_no_restart() {
    for opt in GLOW_ROWS {
        assert!(!opt.help().contains("restart"), "{opt:?} applies live");
        assert!(opt.help().contains("0%: off"), "{opt:?} says 0 is off");
        assert_eq!(opt.verb(), None, "{opt:?} moves with left and right");
    }
}

#[test]
fn grass_row_in_hd_section() {
    let rows: Vec<Opt> = Page::Graphics
        .entries()
        .iter()
        .filter_map(|e| match e {
            Entry::Opt(o) => Some(*o),
            Entry::Section(_) => None,
        })
        .collect();
    let sun = rows.iter().position(|o| *o == Opt::SunGlow).unwrap();
    assert_eq!(rows[sun + 1], Opt::Grass, "right after Sun glow");
    assert_eq!(Opt::Grass.label(), "Grass");
    assert_eq!(Opt::Grass.group(), Group::Graphics);
}

#[test]
fn grass_slider_live() {
    let mut v = values();
    assert!(matches!(Opt::Grass.shown_value(&mut v), Shown::Slider(100)));
    assert!(!Opt::Grass.change(&mut v, Change::Next), "100 % is the top");
    assert!(Opt::Grass.change(&mut v, Change::Prev));
    assert!((v.graphics.hd.grass - 0.95).abs() < 1e-6);
    for _ in 0..30 {
        Opt::Grass.change(&mut v, Change::Prev);
    }
    assert_eq!(v.graphics.hd.grass, 0.0, "0 %: off");
    let mut defaults = values();
    assert!(Opt::Grass.reset(&mut v, &mut defaults));
    assert_eq!(v.graphics.hd.grass, 1.0);
    assert!(!Opt::Grass.help().contains("restart"), "applies live");
    assert!(Opt::Grass.help().contains("0%: off"));
    assert_eq!(Opt::Grass.verb(), None);
}

const SCENE_ROWS: [Opt; 3] = [Opt::SceneLight, Opt::CloudShadows, Opt::NightLook];

#[test]
fn scene_rows_after_grass() {
    let rows: Vec<Opt> = Page::Graphics
        .entries()
        .iter()
        .filter_map(|e| match e {
            Entry::Opt(o) => Some(*o),
            Entry::Section(_) => None,
        })
        .collect();
    let grass = rows.iter().position(|o| *o == Opt::Grass).unwrap();
    assert_eq!(rows[grass + 1..grass + 4], SCENE_ROWS, "right after Grass");
    let labels: Vec<&str> = SCENE_ROWS.iter().map(|o| o.label()).collect();
    assert_eq!(labels, ["Scene light", "Cloud shadows", "Night look"]);
    assert!(SCENE_ROWS.iter().all(|o| o.group() == Group::Graphics));
    assert!(SCENE_ROWS.iter().all(|o| !o.help().contains("restart")), "all live");
}

#[test]
fn scene_toggles_and_reset() {
    let scene: fn(&SettingsValues) -> bool = |v| v.graphics.hd.scene_light;
    let night: fn(&SettingsValues) -> bool = |v| v.graphics.hd.night_look;
    for (opt, field) in [(Opt::SceneLight, scene), (Opt::NightLook, night)] {
        let mut v = values();
        assert!(matches!(opt.shown_value(&mut v), Shown::Toggle(true)), "{opt:?} on by default");
        assert!(opt.change(&mut v, Change::Flip));
        assert!(!field(&v), "{opt:?} flipped off");
        let mut defaults = values();
        assert!(opt.reset(&mut v, &mut defaults));
        assert!(field(&v), "{opt:?} reset on");
    }
}

#[test]
fn cloud_slider() {
    let mut v = values();
    assert!(matches!(Opt::CloudShadows.shown_value(&mut v), Shown::Slider(60)));
    assert!(Opt::CloudShadows.change(&mut v, Change::Prev));
    assert!((v.graphics.hd.cloud_shadows - 0.55).abs() < 1e-6);
    for _ in 0..30 {
        Opt::CloudShadows.change(&mut v, Change::Prev);
    }
    assert_eq!(v.graphics.hd.cloud_shadows, 0.0, "0 %: off");
    for _ in 0..30 {
        Opt::CloudShadows.change(&mut v, Change::Next);
    }
    assert!((v.graphics.hd.cloud_shadows - 1.0).abs() < 1e-6, "100 % is the top");
    let mut defaults = values();
    assert!(Opt::CloudShadows.reset(&mut v, &mut defaults));
    assert!((v.graphics.hd.cloud_shadows - 0.6).abs() < 1e-6);
    assert!(Opt::CloudShadows.help().contains("0%: off"));
    assert_eq!(Opt::CloudShadows.verb(), None);
}

#[test]
fn day_cycle_row_after_night_look() {
    let rows: Vec<Opt> = Page::Graphics
        .entries()
        .iter()
        .filter_map(|e| match e {
            Entry::Opt(o) => Some(*o),
            Entry::Section(_) => None,
        })
        .collect();
    let night = rows.iter().position(|o| *o == Opt::NightLook).unwrap();
    assert_eq!(rows[night + 1], Opt::DayCycle, "right after Night look");
    assert_eq!(Opt::DayCycle.label(), "Day cycle");
    assert!(!Opt::DayCycle.help().contains("restart"), "live");
    let mut v = values();
    assert!(matches!(Opt::DayCycle.shown_value(&mut v), Shown::Toggle(true)));
    assert!(Opt::DayCycle.change(&mut v, Change::Flip));
    assert!(!v.graphics.hd.day_cycle);
    let mut defaults = values();
    assert!(Opt::DayCycle.reset(&mut v, &mut defaults));
    assert!(v.graphics.hd.day_cycle);
}
