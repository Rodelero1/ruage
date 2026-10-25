//! Changing the tick rate is supposed to be a config edit, not a code change (D10). These tests
//! are the proof, and they are written before `minimum_scale` exists.

use ruage_data::{MaybeFixed, load_sim_str, minimum_scale};

fn sim_config(tick_rate_hz: u32, scale: i64) -> String {
    format!(
        r#"
schema = 1
[sim]
tick_rate_hz = {tick_rate_hz}
fixed_point_scale = {scale}
dat_value_decimals = 4
per_tick_decimals = 3
max_entities = 4096
state_hash_interval_ticks = 1
command_apply = "next_tick"
[determinism]
floats_in_sim_state = false
iteration_order = "entity_id"
rng_streams = ["sim", "map"]
[rng]
algorithm = "pcg32"
seed_source = "explicit"
[paths]
gamedata = "gamedata"
overrides = "overrides.toml"
"#
    )
}

#[test]
fn the_minimum_scale_matches_the_hand_computed_table() {
    // lcm(10^dat_value_decimals, tick_rate_hz * 10^per_tick_decimals) = lcm(10000, 1000 * hz).
    // Rates divisible by 10 need nothing extra; 25 needs double; 144 needs five times.
    for (hz, expected) in [
        (10, 10_000),
        (20, 20_000),
        (25, 50_000),
        (30, 30_000),
        (60, 60_000),
        (120, 120_000),
        (144, 720_000),
        (240, 240_000),
    ] {
        assert_eq!(minimum_scale(hz, 4, 3).unwrap(), expected, "{hz} Hz");
    }
}

#[test]
fn a_rate_of_zero_has_no_scale() {
    assert!(minimum_scale(0, 4, 3).is_err());
}

#[test]
fn every_plausible_rate_loads_at_its_derived_scale() {
    for hz in [10, 15, 20, 25, 30, 40, 50, 60, 100, 120, 144, 240] {
        let scale = minimum_scale(hz, 4, 3).unwrap();
        let sim = load_sim_str(&sim_config(hz, scale), "sim.toml")
            .unwrap_or_else(|err| panic!("{hz} Hz at scale {scale} must load: {err:?}"));
        assert_eq!(sim.tick_rate_hz, hz);
        assert_eq!(sim.fixed_point_scale, scale);
    }
}

#[test]
fn a_rate_change_without_a_scale_change_is_refused() {
    // The exact mistake experimenting invites: change 20 to 60, leave the scale at 20000. Then
    // 34 of 57 dat speeds stop landing on a whole number of units per tick and motion drifts.
    let err = load_sim_str(&sim_config(60, 20_000), "sim.toml")
        .expect_err("60 Hz at scale 20000 must be refused");
    let text = format!("{err:?}");
    assert!(
        text.contains("60000"),
        "the error should name the requirement, got: {text}"
    );
}

#[test]
fn one_second_of_travel_is_exact_at_every_rate() {
    // A speed is tiles per second, so its displacement in one tick is speed/T tiles, and D10
    // guarantees that lands on a whole number of scale units. Five speeds including the two
    // grains that set the decimals: 0.737 (3 places) and 8.8 (fastest on the map).
    for hz in [10, 20, 25, 30, 60, 144, 240] {
        let scale = minimum_scale(hz, 4, 3).unwrap();
        for speed in ["0.8", "1.35", "0.737", "1.54", "8.8"] {
            let raw = MaybeFixed::from_f64(speed.parse::<f64>().unwrap())
                .resolve(scale, "speed")
                .unwrap()
                .raw();
            assert_eq!(
                raw % i64::from(hz),
                0,
                "speed {speed} does not land on a whole unit at {hz} Hz (scale {scale})"
            );
            // And back: per-tick displacement times the rate is the per-second displacement.
            assert_eq!((raw / i64::from(hz)) * i64::from(hz), raw);
        }
    }
}
