use nostalre_renderer::LightUniform;
use nostalre_renderer::day_cycle::{self, Segment, Sky, SunAngles};

const MAP_SUN: SunAngles = SunAngles {
    longitude: 45.0,
    latitude: 40.0,
};

fn at_minute(m: f32) -> f32 {
    m / 60.0
}

fn close(a: f32, b: f32, tol: f32) -> bool {
    (a - b).abs() <= tol
}

fn close3(a: [f32; 3], b: [f32; 3], tol: f32) -> bool {
    a.iter().zip(&b).all(|(x, y)| (x - y).abs() <= tol)
}

/// The sky the clock gives with the server agreeing (night only in the night segment).
fn clock_sky(p: f32) -> Sky {
    day_cycle::sky(p, day_cycle::segment(p) == Segment::Night, true, MAP_SUN)
}

#[test]
fn phase_wraps_each_hour() {
    let hour = 3_600_000u64;
    assert_eq!(day_cycle::phase(0), 0.0);
    assert_eq!(day_cycle::phase(hour * 487_000), 0.0, "each UTC hour starts at 0");
    assert!(close(day_cycle::phase(hour * 5 + hour / 2), 0.5, 1e-6));
    assert!(close(day_cycle::phase(hour * 9 + 45 * 60_000), 0.75, 1e-6));
    assert!(day_cycle::phase(hour - 1) < 1.0);
}

#[test]
fn segments_at_boundaries() {
    assert_eq!(day_cycle::segment(at_minute(0.0)), Segment::Dawn);
    assert_eq!(day_cycle::segment(at_minute(4.99)), Segment::Dawn);
    assert_eq!(day_cycle::segment(at_minute(5.0)), Segment::Day);
    assert_eq!(day_cycle::segment(at_minute(39.99)), Segment::Day);
    assert_eq!(day_cycle::segment(at_minute(40.0)), Segment::Dusk);
    assert_eq!(day_cycle::segment(at_minute(44.99)), Segment::Dusk);
    assert_eq!(day_cycle::segment(at_minute(45.0)), Segment::Night);
    assert_eq!(day_cycle::segment(at_minute(59.99)), Segment::Night);
}

#[test]
fn noon_is_map_light() {
    let sky = clock_sky(at_minute(22.5));
    assert!(close(sky.sun.longitude, MAP_SUN.longitude, 1e-3), "{:?}", sky.sun);
    assert!(close(sky.sun.latitude, MAP_SUN.latitude, 1e-3), "{:?}", sky.sun);
    assert!(close3(sky.sun_tint, [1.0; 3], 1e-4));
    assert!(close3(sky.sky_tint, [1.0; 3], 1e-4));
    assert_eq!(sky.night, 0.0);
}

#[test]
fn sunrise_east_sunset_west() {
    let rise = clock_sky(at_minute(0.0));
    let set = clock_sky(at_minute(44.999));
    assert!(close(rise.sun.longitude, MAP_SUN.longitude - 60.0, 0.05), "{:?}", rise.sun);
    assert!(close(set.sun.longitude, MAP_SUN.longitude + 60.0, 0.05), "{:?}", set.sun);
}

#[test]
fn low_sun_near_horizon() {
    let rise = clock_sky(at_minute(0.0));
    let set = clock_sky(at_minute(44.999));
    assert!(close(rise.sun.latitude, 75.0, 0.05), "{:?}", rise.sun);
    assert!(close(set.sun.latitude, 75.0, 0.05), "{:?}", set.sun);
    assert!(clock_sky(at_minute(10.0)).sun.latitude < 75.0);
}

#[test]
fn golden_only_near_ends() {
    let rise = clock_sky(at_minute(0.0));
    assert!(close3(rise.sun_tint, [0.75, 0.72 * 0.75, 0.50 * 0.75], 1e-3), "{:?}", rise.sun_tint);
    for m in [12.0, 16.0, 22.5, 28.0, 33.0] {
        let sky = clock_sky(at_minute(m));
        assert!(close3(sky.sun_tint, [1.0; 3], 0.02), "minute {m}: {:?}", sky.sun_tint);
        assert!(close3(sky.sky_tint, [1.0; 3], 0.02), "minute {m}: {:?}", sky.sky_tint);
    }
    let dusk = clock_sky(at_minute(43.0));
    assert!(dusk.sun_tint[2] < dusk.sun_tint[0] * 0.8, "warm at dusk: {:?}", dusk.sun_tint);
}

#[test]
fn night_ramps_at_dawn_and_dusk() {
    assert!(close(clock_sky(at_minute(0.0)).night, 1.0, 1e-4));
    assert!(close(clock_sky(at_minute(5.0)).night, 0.0, 1e-4));
    let mid_dawn = clock_sky(at_minute(2.5)).night;
    assert!(mid_dawn > 0.2 && mid_dawn < 0.8, "{mid_dawn}");
    assert_eq!(clock_sky(at_minute(30.0)).night, 0.0);
    assert_eq!(clock_sky(at_minute(42.5)).night, 0.0);
    assert!(close(clock_sky(at_minute(44.999)).night, 0.5, 0.01));
    assert_eq!(clock_sky(at_minute(50.0)).night, 1.0);
}

#[test]
fn moon_crosses_back_at_night() {
    let start = clock_sky(at_minute(45.0));
    let end = clock_sky(at_minute(59.999));
    assert!(close(start.sun.longitude, MAP_SUN.longitude + 60.0, 0.05), "{:?}", start.sun);
    assert!(close(end.sun.longitude, MAP_SUN.longitude - 60.0, 0.05), "{:?}", end.sun);
    assert!(close3(clock_sky(at_minute(52.5)).sun_tint, [1.0; 3], 1e-4), "moonlight colour is S5's");
}

#[test]
fn server_night_wins_by_day() {
    let sky = day_cycle::sky(at_minute(10.0), true, true, MAP_SUN);
    assert_eq!(sky.night, 1.0);
    assert_eq!(sky.sun, MAP_SUN);
    assert_eq!(sky.sun_tint, [1.0; 3]);
}

#[test]
fn server_day_wins_by_night() {
    let sky = day_cycle::sky(at_minute(50.0), false, true, MAP_SUN);
    assert_eq!(sky.night, 0.0);
    assert_eq!(sky.sun, MAP_SUN);
    assert_eq!(sky.sun_tint, [1.0; 3]);
    assert_eq!(sky.sky_tint, [1.0; 3]);
}

/// One second of clock at a time, the server agreeing: the sun never jumps; colours and night only where the
/// server's night begins and ends (minute 45 and the hour), which `ease` blends.
#[test]
fn sky_is_continuous() {
    let mut last = clock_sky(0.0);
    for s in 1..=3600 {
        let p = (s % 3600) as f32 / 3600.0;
        let now = clock_sky(p);
        let dir = |sky: &Sky| sky.sun.direction();
        assert!(close3(dir(&now), dir(&last), 0.02), "sun jumps at second {s}");
        if s != 45 * 60 && s != 3600 {
            assert!(close3(now.sun_tint, last.sun_tint, 0.02), "sun tint jumps at second {s}");
            assert!(close3(now.sky_tint, last.sky_tint, 0.02), "sky tint jumps at second {s}");
            assert!(close(now.night, last.night, 0.02), "night jumps at second {s}");
        }
        last = now;
    }
}

#[test]
fn ease_settles_in_three_seconds() {
    let from = day_cycle::sky(at_minute(10.0), false, true, MAP_SUN);
    let to = day_cycle::sky(at_minute(10.0), true, true, MAP_SUN);
    let mut shown = from;
    let frame = 1.0 / 60.0;
    for _ in 0..30 {
        shown = shown.ease(&to, frame);
    }
    assert!(shown.night < 0.6, "not at once: {}", shown.night);
    for _ in 0..150 {
        shown = shown.ease(&to, frame);
    }
    assert!(shown.night > 0.99, "settled after 3 s: {}", shown.night);
    assert_eq!(from.ease(&from, frame), from, "nothing to ease");
}

#[test]
fn sun_angles_match_load_map() {
    // the same formula load_map uses for the RSW light
    let d = MAP_SUN.direction();
    let (lon, lat) = (45f32.to_radians(), 40f32.to_radians());
    let want = glam::Vec3::new(-lon.cos() * lat.sin(), -lat.cos(), -lon.sin() * lat.sin()).normalize();
    assert!(close3(d, want.to_array(), 1e-5));
}

/// The sky drives the light only with HD effects on, outdoors and the Day cycle row on; otherwise S5's light.
#[test]
fn frame_light_off_indoors_or_hd_off() {
    use nostalre_renderer::scene_light::SceneOptions;
    let on = SceneOptions::default();
    assert!(on.day_cycle, "on by default");
    assert!(day_cycle::active(true, false, &on));
    assert!(!day_cycle::active(false, false, &on), "HD off");
    assert!(!day_cycle::active(true, true, &on), "indoors");
    let off = SceneOptions {
        day_cycle: false,
        ..on
    };
    assert!(!day_cycle::active(true, false, &off), "row off");
}

fn base() -> LightUniform {
    LightUniform {
        light_dir: {
            let d = MAP_SUN.direction();
            [d[0], d[1], d[2], 0.0]
        },
        diffuse_color: [1.0, 0.95, 0.9, 1.0],
        ambient_color: [0.4, 0.4, 0.45, 1.0],
        ..LightUniform::default()
    }
}

#[test]
fn apply_noon_is_base() {
    let b = base();
    let out = day_cycle::apply(&b, &clock_sky(at_minute(22.5)));
    for i in 0..3 {
        assert!(close(out.light_dir[i], b.light_dir[i], 1e-4));
        assert!(close(out.diffuse_color[i], b.diffuse_color[i], 1e-4));
        assert!(close(out.ambient_color[i], b.ambient_color[i], 1e-4));
    }
}

#[test]
fn light_dir_follows_sky() {
    let sky = clock_sky(at_minute(41.0));
    let out = day_cycle::apply(&base(), &sky);
    assert!(close3([out.light_dir[0], out.light_dir[1], out.light_dir[2]], sky.sun.direction(), 1e-5));
    assert!(out.diffuse_color[2] < base().diffuse_color[2] * 0.8, "golden sun: {:?}", out.diffuse_color);
}

/// The server checks the clock once a second and its flag crosses the network: just past a boundary the clock
/// leads, no flash of midday before the night (or of the map sun before dawn).
#[test]
fn grace_follows_the_clock_at_boundaries() {
    let early = day_cycle::sky(at_minute(45.0) + 2.0 / 3600.0, false, true, MAP_SUN);
    let dusk_end = clock_sky(at_minute(44.999));
    assert!(early.night >= 0.5, "no midday flash: {}", early.night);
    assert!(close(early.sun.latitude, dusk_end.sun.latitude, 0.5), "{:?}", early.sun);
    let dawn = day_cycle::sky(2.0 / 3600.0, true, true, MAP_SUN);
    assert!(close(dawn.sun.longitude, MAP_SUN.longitude - 60.0, 0.5), "the moon sets, no swing: {:?}", dawn.sun);
    let late = day_cycle::sky(at_minute(45.5), false, true, MAP_SUN);
    assert_eq!(late.night, 0.0, "past the grace the server wins");
    let gm_night = day_cycle::sky(at_minute(10.0), true, true, MAP_SUN);
    assert_eq!(gm_night.night, 1.0, "no grace by day");
}

/// A map the server never makes night on: the sun still moves and turns golden, but nothing dims.
#[test]
fn map_without_night_keeps_its_light() {
    let dusk = day_cycle::sky(at_minute(44.5), false, false, MAP_SUN);
    assert_eq!(dusk.night, 0.0);
    assert!(dusk.sun_tint[2] < dusk.sun_tint[0], "still golden");
    assert_eq!(day_cycle::sky(at_minute(1.0), false, false, MAP_SUN).night, 0.0, "no dark dawn");
    let night = day_cycle::sky(at_minute(45.0), false, false, MAP_SUN);
    assert_eq!(night, day_cycle::sky(at_minute(50.0), false, false, MAP_SUN), "map light at once, no grace");
    assert_eq!(night.night, 0.0);
}

#[test]
fn learn_darkens_from_the_server() {
    assert!(day_cycle::learn_darkens(false, at_minute(10.0), true), "server night: yes");
    assert!(day_cycle::learn_darkens(true, at_minute(45.0) + 2.0 / 3600.0, false), "in the grace: keep");
    assert!(!day_cycle::learn_darkens(true, at_minute(46.0), false), "night past the grace, server day: no");
    assert!(!day_cycle::learn_darkens(false, at_minute(10.0), false), "by day: keep");
    assert!(day_cycle::learn_darkens(true, at_minute(10.0), false), "by day: keep");
}
