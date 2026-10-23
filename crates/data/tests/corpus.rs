//! Corpus test: the fixed-point invariants must hold for the *actual* generated data,
//! not for a handful of literals chosen by hand.
//!
//! Two rules, matching the loader in `ruage-data`:
//!
//! 1. every dat value is exactly representable at the configured scale;
//! 2. every quantity integrated once per tick (speed) lands on a whole number of scale
//!    units per tick, so no unit drifts no matter how long it walks.
//!
//! `config/gamedata/` is generated from the user's own install and is never committed (D8),
//! so this test skips when the file is absent — the same convention as the generator's
//! real-dat integration test.

use std::path::PathBuf;

use ruage_data::load_sim_file;

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn decimal_places(text: &str) -> u32 {
    match text.split_once('.') {
        Some((_, frac)) => frac.trim_end_matches('0').len() as u32,
        None => 0,
    }
}

#[test]
fn every_dat_value_in_the_corpus_is_exact_per_tick() {
    let units_path = repo_path("config/gamedata/units.toml");
    if !units_path.exists() {
        eprintln!(
            "skipping: {} is not present (generated data, D8)",
            units_path.display()
        );
        return;
    }

    let sim = load_sim_file(&repo_path("config/sim.toml")).expect("config/sim.toml must load");
    let text = std::fs::read_to_string(&units_path).expect("read units.toml");
    let doc: toml::Value = toml::from_str(&text).expect("units.toml must be valid TOML");
    let units = doc
        .get("unit")
        .and_then(|u| u.as_array())
        .expect("units.toml must have [[unit]] records");

    let tick = i64::from(sim.tick_rate_hz);
    let mut worst_value_decimals = 0;
    let mut worst_motion_decimals = 0;
    let mut checked = 0_usize;

    for unit in units {
        let id = unit.get("id").and_then(|v| v.as_integer()).unwrap_or(-1);
        for field in ["speed", "work_rate", "los"] {
            let Some(value) = unit.get(field).and_then(|v| v.as_float()) else {
                continue;
            };
            let rendered = format!("{value}");
            let places = decimal_places(&rendered);
            worst_value_decimals = worst_value_decimals.max(places);
            if field == "speed" {
                worst_motion_decimals = worst_motion_decimals.max(places);
            }

            // Rule 1 — representable exactly, never rounded.
            let fixed = ruage_fixed::Fixed::parse(&rendered, sim.fixed_point_scale)
                .unwrap_or_else(|err| panic!("unit {id} {field} = {rendered}: {err}"));

            // Rule 2 — only for quantities integrated per tick.
            if field == "speed" {
                assert_eq!(
                    fixed.raw() % tick,
                    0,
                    "unit {id} speed {rendered} is {} scale units per tick, not a whole number",
                    fixed.raw() as f64 / tick as f64
                );
            }
            checked += 1;
        }
    }

    assert!(
        checked > 3000,
        "expected to check thousands of values, saw {checked}"
    );
    assert_eq!(
        worst_value_decimals, sim.dat_value_decimals,
        "config declares dat_value_decimals = {} but the corpus needs {worst_value_decimals}",
        sim.dat_value_decimals
    );
    assert_eq!(
        worst_motion_decimals, sim.per_tick_decimals,
        "config declares per_tick_decimals = {} but speeds need {worst_motion_decimals}",
        sim.per_tick_decimals
    );
}
