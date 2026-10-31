use nostalre_game::chat_command::{ChatCommand, parse_chat_command};
use nostalre_game::hd::{self, HdConfig, HdPreset};

#[test]
fn hd_defaults_off() {
    assert!(!HdConfig::default().enabled);
    let parsed: HdConfig = serde_json::from_str("{}").unwrap();
    assert!(!parsed.enabled, "a config without `hd` keys is HD off");
}

#[test]
fn hd_roundtrip() {
    let config = HdConfig {
        enabled: true,
        preset: HdPreset::Ultra,
        ..HdConfig::default()
    };
    let json = serde_json::to_string(&config).unwrap();
    assert_eq!(serde_json::from_str::<HdConfig>(&json).unwrap(), config);
    let written: HdConfig =
        serde_json::from_str(r#"{"enabled": true, "preset": "low"}"#).unwrap();
    assert_eq!(written.preset, HdPreset::Low);
}

#[test]
fn only_low_keeps_an_8bit_target() {
    let with = |preset| HdConfig {
        enabled: true,
        preset,
        ..HdConfig::default()
    };
    assert!(!with(HdPreset::Low).wants_float_target());
    for preset in [
        HdPreset::Medium,
        HdPreset::High,
        HdPreset::Ultra,
        HdPreset::Custom,
    ] {
        assert!(with(preset).wants_float_target(), "{preset:?}");
    }
}

#[test]
fn bloom_levels_by_preset() {
    let levels = |preset| {
        HdConfig {
            preset,
            ..HdConfig::default()
        }
        .bloom_levels()
    };
    assert_eq!(levels(HdPreset::Low), 0);
    assert_eq!(levels(HdPreset::Medium), 3);
    assert_eq!(levels(HdPreset::High), 4);
    assert_eq!(levels(HdPreset::Ultra), 4);
    assert_eq!(levels(HdPreset::Custom), 4);
}

#[test]
fn bloom_default_and_clamp() {
    assert_eq!(HdConfig::default().bloom, 0.6);
    let parsed: HdConfig = serde_json::from_str(r#"{"bloom": 1.25}"#).unwrap();
    assert_eq!(parsed.bloom, 1.25);
    assert_eq!(
        serde_json::from_str::<HdConfig>(&serde_json::to_string(&parsed).unwrap()).unwrap(),
        parsed
    );
    let with = |bloom| HdConfig {
        bloom,
        ..HdConfig::default()
    };
    assert_eq!(with(5.0).bloom_intensity(), 2.0);
    assert_eq!(with(-1.0).bloom_intensity(), 0.0);
    assert_eq!(with(f32::NAN).bloom_intensity(), 0.0);
}

#[test]
fn hd_command_parses() {
    assert_eq!(parse_chat_command("/hd"), ChatCommand::ToggleHd);
}

#[test]
fn hd_toggle_session_only() {
    assert_eq!(hd::session_toggle(true, true), Some(false));
    assert_eq!(hd::session_toggle(true, false), Some(true));
    assert_eq!(
        hd::session_toggle(false, false),
        None,
        "HD not running: nothing to compare, Settings turns it on"
    );
}

#[test]
fn needs_restart_cases() {
    let with = |enabled, preset, bloom| HdConfig {
        enabled,
        preset,
        bloom,
        ..HdConfig::default()
    };
    let high = with(true, HdPreset::High, 0.6);
    assert!(
        hd::needs_restart(&with(false, HdPreset::High, 0.6), &high, false),
        "turned on while not running"
    );
    assert!(!hd::needs_restart(&high, &with(false, HdPreset::High, 0.6), true), "off stops it now");
    assert!(!hd::needs_restart(&high, &with(true, HdPreset::High, 0.2), true), "bloom is live");
    assert!(!hd::needs_restart(&high, &with(true, HdPreset::Ultra, 0.6), true), "both half float");
    assert!(hd::needs_restart(&high, &with(true, HdPreset::Low, 0.6), true), "half float to 8-bit");
    assert!(hd::needs_restart(&with(true, HdPreset::Low, 0.6), &high, true), "8-bit to half float");
    assert!(
        !hd::needs_restart(&with(false, HdPreset::High, 0.6), &with(false, HdPreset::Low, 0.6), false),
        "a quality change while HD is off and stays off"
    );
}

#[test]
fn preset_any_case_unknown_is_high() {
    let preset = |json: &str| serde_json::from_str::<HdConfig>(json).unwrap().preset;
    assert_eq!(preset(r#"{"preset": "Ultra"}"#), HdPreset::Ultra);
    assert_eq!(preset(r#"{"preset": "LOW"}"#), HdPreset::Low);
    assert_eq!(preset(r#"{"preset": "ultra-high"}"#), HdPreset::High);
}

#[test]
fn unparsable_hd_is_the_defaults() {
    #[derive(serde::Deserialize)]
    struct Outer {
        #[serde(deserialize_with = "hd::deserialize_lenient")]
        hd: HdConfig,
        other: u32,
    }
    let outer: Outer = serde_json::from_str(r#"{"hd": {"bloom": "bright"}, "other": 7}"#).unwrap();
    assert_eq!(outer.hd, HdConfig::default());
    assert_eq!(outer.other, 7, "the rest still loads");
    let outer: Outer =
        serde_json::from_str(r#"{"hd": {"enabled": true, "preset": "medium"}, "other": 1}"#).unwrap();
    assert!(outer.hd.enabled);
    assert_eq!(outer.hd.preset, HdPreset::Medium);
}

#[test]
fn grade_and_vignette_defaults_and_clamp() {
    let config = HdConfig::default();
    assert_eq!(config.grade, 0.8);
    assert!(config.vignette);
    let parsed: HdConfig = serde_json::from_str(r#"{"grade": 0.3, "vignette": false}"#).unwrap();
    assert_eq!(parsed.grade, 0.3);
    assert!(!parsed.vignette);
    let with = |grade| HdConfig {
        grade,
        ..HdConfig::default()
    };
    assert_eq!(with(3.0).grade_strength(), 1.0);
    assert_eq!(with(-1.0).grade_strength(), 0.0);
    assert_eq!(with(f32::NAN).grade_strength(), 0.0);
}

#[test]
fn glow_defaults_and_clamp() {
    let config = HdConfig::default();
    assert_eq!(config.lamp_glow, 0.6);
    assert_eq!(config.light_pools, 0.5);
    assert_eq!(config.sun_glow, 0.3);
    assert_eq!(config.glow_levels(), [0.6, 0.5, 0.3]);
    let with = |value| HdConfig {
        lamp_glow: value,
        light_pools: value,
        sun_glow: value,
        ..HdConfig::default()
    };
    assert_eq!(with(3.0).glow_levels(), [1.0; 3]);
    assert_eq!(with(-1.0).glow_levels(), [0.0; 3]);
    assert_eq!(with(f32::NAN).glow_levels(), [0.0; 3]);
}

#[test]
fn old_config_gets_glow_defaults() {
    let parsed: HdConfig =
        serde_json::from_str(r#"{"enabled": true, "bloom": 1.25, "vignette": false}"#).unwrap();
    assert!(parsed.enabled);
    assert_eq!(parsed.bloom, 1.25);
    assert!(!parsed.vignette);
    assert_eq!(parsed.glow_levels(), HdConfig::default().glow_levels());
    let set: HdConfig = serde_json::from_str(r#"{"lamp_glow": 0.2, "sun_glow": 0.0}"#).unwrap();
    assert_eq!(set.glow_levels(), [0.2, 0.5, 0.0]);
}

#[test]
fn grass_default_and_clamp() {
    assert_eq!(HdConfig::default().grass, 1.0);
    assert_eq!(HdConfig::default().grass_density(), 1.0);
    let old: HdConfig = serde_json::from_str(r#"{"enabled": true, "bloom": 0.5}"#).unwrap();
    assert_eq!(old.grass, 1.0, "a config from before grass gets it");
    let with = |grass| HdConfig {
        grass,
        ..HdConfig::default()
    };
    assert_eq!(with(0.4).grass_density(), 0.4);
    assert_eq!(with(3.0).grass_density(), 1.0);
    assert_eq!(with(-1.0).grass_density(), 0.0);
    assert_eq!(with(f32::NAN).grass_density(), 0.0);
}

#[test]
fn grass_tier_by_preset() {
    let tier = |preset| {
        HdConfig {
            preset,
            ..HdConfig::default()
        }
        .grass_tier()
    };
    assert_eq!(tier(HdPreset::Low), 0);
    assert_eq!(tier(HdPreset::Medium), 1);
    assert_eq!(tier(HdPreset::High), 2);
    assert_eq!(tier(HdPreset::Ultra), 3);
    assert_eq!(tier(HdPreset::Custom), 2);
}

#[test]
fn scene_defaults_and_clamp() {
    let config = HdConfig::default();
    assert!(config.scene_light);
    assert!(config.night_look);
    assert_eq!(config.cloud_shadows, 0.6);
    assert_eq!(config.cloud_strength(), 0.6);
    let old: HdConfig = serde_json::from_str(r#"{"enabled": true, "grass": 0.5}"#).unwrap();
    assert!(old.scene_light && old.night_look, "a config from before S5 gets the defaults");
    assert_eq!(old.cloud_shadows, 0.6);
    let with = |cloud_shadows| HdConfig {
        cloud_shadows,
        ..HdConfig::default()
    };
    assert_eq!(with(3.0).cloud_strength(), 1.0);
    assert_eq!(with(-1.0).cloud_strength(), 0.0);
    assert_eq!(with(f32::NAN).cloud_strength(), 0.0);
}

#[test]
fn terrain_light_cap_by_preset() {
    let cap = |preset| {
        HdConfig {
            preset,
            ..HdConfig::default()
        }
        .terrain_light_cap()
    };
    assert_eq!(cap(HdPreset::Low), 0);
    assert_eq!(cap(HdPreset::Medium), 8);
    assert_eq!(cap(HdPreset::High), 16);
    assert_eq!(cap(HdPreset::Ultra), 32);
    assert_eq!(cap(HdPreset::Custom), 16);
}

#[test]
fn scene_rows_are_live() {
    let on = HdConfig {
        enabled: true,
        ..HdConfig::default()
    };
    let off = HdConfig {
        scene_light: false,
        night_look: false,
        cloud_shadows: 0.0,
        ..on.clone()
    };
    assert!(!hd::needs_restart(&on, &off, true));
    assert!(!hd::needs_restart(&off, &on, true));
}

#[test]
fn day_cycle_default_on() {
    assert!(HdConfig::default().day_cycle);
    let old: HdConfig = serde_json::from_str(r#"{"enabled": true, "night_look": false}"#).unwrap();
    assert!(old.day_cycle, "a config from before S6 gets the cycle");
    let on = HdConfig {
        enabled: true,
        ..HdConfig::default()
    };
    let off = HdConfig {
        day_cycle: false,
        ..on.clone()
    };
    assert!(!hd::needs_restart(&on, &off, true), "live");
}

#[test]
fn sky_parses() {
    assert_eq!(parse_chat_command("/sky 42:00"), ChatCommand::Sky(Some(42 * 60)));
    assert_eq!(parse_chat_command("/sky 2:30"), ChatCommand::Sky(Some(150)));
    assert_eq!(parse_chat_command("/sky 0:00"), ChatCommand::Sky(Some(0)));
    assert_eq!(parse_chat_command("/sky off"), ChatCommand::Sky(None));
    for bad in ["/sky", "/sky 60:00", "/sky 12:60", "/sky noon", "/sky -1:00", "/sky 12"] {
        assert_eq!(parse_chat_command(bad), ChatCommand::SkyUsage, "{bad}");
    }
}
