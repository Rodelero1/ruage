//! **A scenario's length is a length, and the loader says so** (second review: [74] [173] [184]).
//!
//! `duration_seconds * tick_rate_hz` was cast with `as i64` in both runners, and `f64 as i64` **saturates**: a `NaN`
//! duration became **0 ticks** - a match that plays nothing, exits 0 and reports success - and `1e300` became
//! `i64::MAX` ticks, a match that never ends. Neither is a length. The cast was also the only thing standing between a
//! hand-written scenario file and a run that says nothing about why it did nothing.

use ruage_script::{Runner, load_scenario_str};

fn scenario(duration: &str) -> String {
    format!("schema = 2\nduration_seconds = {duration}\n")
}

#[test]
fn a_nan_duration_is_refused_by_the_loader() {
    let err = load_scenario_str(&scenario("nan"), "test.toml")
        .expect_err("a NaN duration is not a length and must be refused");
    assert!(
        err.contains("duration_seconds"),
        "the refusal must name the field, got: {err}"
    );
}

#[test]
fn a_negative_duration_is_refused_by_the_loader() {
    let err = load_scenario_str(&scenario("-1.0"), "test.toml")
        .expect_err("a negative duration must be refused");
    assert!(
        err.contains("duration_seconds"),
        "the refusal must name the field, got: {err}"
    );
}

#[test]
fn a_duration_too_long_for_a_tick_count_is_refused() {
    let scenario = load_scenario_str(&scenario("1e300"), "test.toml")
        .expect("the loader cannot know the tick rate, so it cannot bound the product");
    let err = Runner::new(&scenario, 20).err().expect(
        "1e300 seconds at 20 Hz is not a tick count, and `as i64` used to make it i64::MAX",
    );
    assert!(
        err.contains("duration_seconds"),
        "the refusal must name the field, got: {err}"
    );
}

#[test]
fn an_ordinary_duration_is_still_accepted() {
    let scenario = load_scenario_str(&scenario("60.0"), "test.toml").expect("60 s is a length");
    let runner = Runner::new(&scenario, 20).expect("a runner for an ordinary scenario");
    assert_eq!(
        runner.duration_ticks(),
        1_200,
        "the tick count a scripted match runs for is its duration times the rate"
    );
    assert_eq!(
        scenario.ticks(20).expect("60 s at 20 Hz"),
        1_200,
        "the runner and the binaries must share one definition of that arithmetic"
    );
}
