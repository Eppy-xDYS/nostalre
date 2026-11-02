use nostalre_renderer::post::grade::{self, Family, GradeFade, GradeParams};

const FLOAT_WHITE: f32 = grade::FLOAT_WHITE;

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
}

#[test]
fn shoulder_identity_below_knee() {
    for x in [0.0, 0.1, 0.5, 0.77, grade::KNEE] {
        assert!(close(grade::shoulder(x, FLOAT_WHITE), x), "{x}");
    }
}

#[test]
fn shoulder_rolls_off_to_white() {
    assert!(close(grade::shoulder(FLOAT_WHITE, FLOAT_WHITE), 1.0));
    assert!(close(grade::shoulder(1.0, 1.0), 1.0), "8-bit: white stays white");
    let mut last = 0.0;
    for i in 0..=400 {
        let x = i as f32 * 0.025;
        let y = grade::shoulder(x, FLOAT_WHITE);
        assert!(y >= last - 1e-6, "monotonic at {x}");
        assert!(y <= 1.0, "never above 1 at {x}: {y}");
        last = y;
    }
}

#[test]
fn white_keeps_its_punch() {
    let white = grade::shoulder(grade::EXPOSURE, FLOAT_WHITE);
    assert!(white > 0.97, "daylight white after exposure and curve: {white}");
}

#[test]
fn tone_map_keeps_hue() {
    let c = [3.0, 1.5, 0.6];
    let out = grade::tone_map(c, FLOAT_WHITE);
    assert!(out[0] <= 1.0);
    assert!((out[0] / out[1] - 2.0).abs() < 1e-5, "ratios kept: {out:?}");
    assert!((out[1] / out[2] - 2.5).abs() < 1e-5);
    let mid = [0.4, 0.2, 0.1];
    assert_eq!(grade::tone_map(mid, FLOAT_WHITE), mid, "below the knee untouched");
}

#[test]
fn family_rules() {
    let cases = [
        ("prontera", Family::Warm),
        ("PRONTERA", Family::Warm),
        ("prt_fild08", Family::Verdant),
        ("moc_fild12", Family::Desert),
        ("gl_church", Family::Cold),
        ("gef_dun00", Family::Sickly),
        ("mag_dun01", Family::Ember),
        ("xmas", Family::Arctic),
        ("some_new_map", Family::Neutral),
    ];
    for (map, family) in cases {
        assert_eq!(grade::family_for_map(map), family, "{map}");
    }
}

#[test]
fn strength_zero_is_identity() {
    for family in [Family::Warm, Family::Cold, Family::Ember] {
        assert_eq!(GradeParams::for_family(family).scaled(0.0), GradeParams::IDENTITY);
    }
    for c in [[0.0, 0.0, 0.0], [0.2, 0.5, 0.9], [1.0, 1.0, 1.0]] {
        let out = GradeParams::IDENTITY.apply(c);
        for i in 0..3 {
            assert!(close(out[i], c[i]), "{c:?} -> {out:?}");
        }
    }
    assert_ne!(GradeParams::for_family(Family::Warm), GradeParams::IDENTITY);
}

#[test]
fn fade_endpoints_and_middle() {
    let warm = GradeParams::for_family(Family::Warm);
    let cold = GradeParams::for_family(Family::Cold);
    let mut fade = GradeFade::new(warm);
    fade.start(cold);
    assert_eq!(fade.current(), warm, "at 0 s the old grade");
    fade.advance(0.5);
    assert_eq!(fade.current(), GradeParams::lerp(&warm, &cold, 0.5));
    fade.advance(0.5);
    assert_eq!(fade.current(), cold, "at 1 s the new grade");
    fade.advance(3.0);
    assert_eq!(fade.current(), cold);
    assert!(!fade.fading());
}

#[test]
fn fade_restarts_from_what_is_on_screen() {
    let warm = GradeParams::for_family(Family::Warm);
    let cold = GradeParams::for_family(Family::Cold);
    let ember = GradeParams::for_family(Family::Ember);
    let mut fade = GradeFade::new(warm);
    fade.start(cold);
    fade.advance(0.25);
    let shown = fade.current();
    fade.start(ember);
    assert_eq!(fade.current(), shown, "no jump on the second map change");
    fade.advance(1.0);
    assert_eq!(fade.current(), ember);
}

#[test]
fn vignette_centre_and_corners() {
    assert!(close(grade::vignette([0.5, 0.5], 0.35), 1.0));
    for corner in [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]] {
        let v = grade::vignette(corner, 0.35);
        assert!((0.65..1.0).contains(&v), "{corner:?}: {v}");
    }
    assert!(close(grade::vignette([0.0, 0.0], 0.0), 1.0), "off");
}

#[test]
fn effects_off_is_identity() {
    let warm = GradeParams::for_family(Family::Warm);
    let on = grade::composite_uniform(true, 0.6, &warm, 0.35, FLOAT_WHITE);
    assert_eq!(on.bloom_intensity, 0.6);
    assert_eq!(on.curve_white, FLOAT_WHITE);
    assert_eq!(on.exposure, grade::EXPOSURE);
    let eight_bit = grade::composite_uniform(true, 0.0, &warm, 0.35, 1.0);
    assert_eq!(eight_bit.exposure, 1.0, "no room above white on 8-bit");
    let off = grade::composite_uniform(false, 0.6, &warm, 0.35, FLOAT_WHITE);
    assert_eq!(off.bloom_intensity, 0.0);
    assert_eq!(off.curve_white, 0.0, "0: no curve");
    assert_eq!(off.vignette, 0.0);
    assert_eq!(off.grade, GradeParams::IDENTITY);
}
