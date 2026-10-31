use nostalre_game::day_night::DayNightState;

const FRAME: f32 = 1.0 / 60.0;

fn run(s: &mut DayNightState, seconds: f32) {
    for _ in 0..(seconds / FRAME).round() as usize {
        s.tick(FRAME);
    }
}

#[test]
fn night_amount_fades_in_three_seconds() {
    let mut s = DayNightState::default();
    s.on_map_loaded([1.0, 1.0, 0.9], [0.3, 0.3, 0.3]);
    // past the load's snap window: a night change in place fades
    run(&mut s, 2.0);
    assert_eq!(s.night_amount(), 0.0);
    s.set_night(true);
    run(&mut s, 1.5);
    assert!((s.night_amount() - 0.5).abs() < 0.02, "{}", s.night_amount());
    run(&mut s, 1.5 - FRAME);
    assert!(s.night_amount() < 1.0, "not before 3 s");
    run(&mut s, 2.0 * FRAME);
    assert_eq!(s.night_amount(), 1.0);
    s.set_night(false);
    run(&mut s, 3.0 + FRAME);
    assert_eq!(s.night_amount(), 0.0, "back in 3 s");
}

#[test]
fn night_amount_marks_dirty_while_fading() {
    let mut s = DayNightState::default();
    s.on_map_loaded([0.4, 0.4, 0.4], [0.2, 0.2, 0.2]);
    run(&mut s, 2.0);
    s.take_dirty();
    s.set_night(true);
    s.tick(FRAME);
    assert!(s.take_dirty(), "the HD night fades even on a map today's fade leaves alone");
    run(&mut s, 4.0);
    s.take_dirty();
    s.tick(FRAME);
    assert!(!s.take_dirty(), "settled");
}

/// Warping at night: the new map is already at its night light, not faded down from daylight in front of you.
#[test]
fn map_change_at_night_starts_at_night_light() {
    let mut s = DayNightState::default();
    s.on_map_loaded([1.0, 1.0, 0.9], [0.3, 0.3, 0.3]);
    s.set_night(true);
    run(&mut s, 4.0);
    let settled = s.world_diffuse();
    s.on_map_loaded([1.0, 1.0, 0.9], [0.3, 0.3, 0.3]);
    assert_eq!(s.world_diffuse(), settled, "no fade from daylight after the warp");
    s.take_dirty();
    s.tick(FRAME);
    assert!(!s.take_dirty(), "nothing left to fade");
    // by day the map loads at its day light, as before
    let mut d = DayNightState::default();
    d.on_map_loaded([0.8, 0.7, 0.6], [0.3, 0.3, 0.3]);
    assert_eq!(d.world_diffuse(), [0.8, 0.7, 0.6]);
}

/// Out of a building at night: the server says night again a moment after the outdoor map loads; that lands at
/// once, not as a 3 s fade in front of the player. A change long after the load (dusk, `@night`) still fades.
#[test]
fn night_news_right_after_a_map_load_lands_at_once() {
    let mut s = DayNightState::default();
    s.on_map_loaded([1.0, 1.0, 0.9], [0.3, 0.3, 0.3]);
    s.set_night(false); // indoors: no night there
    s.on_map_loaded([1.0, 1.0, 0.9], [0.3, 0.3, 0.3]);
    run(&mut s, 0.5);
    s.set_night(true);
    s.tick(FRAME);
    assert_eq!(s.night_amount(), 1.0, "at once");
    assert_eq!(s.world_diffuse()[0], 0.5, "the original dimming too");
    run(&mut s, 5.0);
    s.set_night(false);
    s.tick(FRAME);
    assert!(s.night_amount() > 0.9, "later changes fade: {}", s.night_amount());
}

#[test]
fn night_amount_survives_map_change() {
    let mut s = DayNightState::default();
    s.on_map_loaded([1.0, 1.0, 0.9], [0.3, 0.3, 0.3]);
    s.set_night(true);
    run(&mut s, 4.0);
    s.on_map_loaded([0.8, 0.8, 0.8], [0.4, 0.4, 0.4]);
    assert_eq!(s.night_amount(), 1.0, "a new map at night is night at once");
    s.reset();
    assert_eq!(s.night_amount(), 0.0, "logout clears it");
}
