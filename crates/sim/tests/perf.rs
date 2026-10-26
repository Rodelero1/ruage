//! Performance probes. Ignored by default: they print timings rather than assert behaviour, and a
//! test that only prints is not a test. Run them deliberately:
//!
//! ```text
//! cargo test --release -p ruage-sim --test perf -- --ignored --nocapture
//! ```

use std::path::PathBuf;
use std::time::Instant;

use ruage_data::{Economy, Map, Sim as Settings, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn fixture() -> Option<(Settings, Map, Economy, Units)> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        eprintln!("skipping: no units.toml (generated, D8)");
        return None;
    }
    Some((
        ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim.toml"),
        ruage_data::load_map_file(&repo("config/map.toml")).expect("map.toml"),
        ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy.toml"),
        ruage_data::load_units_file(&units).expect("units.toml"),
    ))
}

/// Idle units at rest: the cheapest possible world, so whatever cost shows up is per-entity
/// bookkeeping rather than pathfinding.
fn world_with(fx: &(Settings, Map, Economy, Units), extra_units: usize) -> World {
    let (settings, map, economy, units) = fx;
    let mut world = World::new(settings, 11).expect("world");
    world.seed_from(map, economy, units).expect("seeding");
    let scale = world.scale();
    for (next, i) in (10_000u32..).zip(0..extra_units) {
        let id = EntityId(next);
        let x = 2 + (i as i64 % 60);
        let y = 2 + (i as i64 / 60) % 60;
        world
            .submit_checked(Command::Spawn {
                id,
                player: u8::try_from(1 + (i % 8)).expect("1..=8"),
                kind: EntityKind::Unit,
                unit: Some(83),
                position: (Fixed::from_raw(x * scale), Fixed::from_raw(y * scale)),
                speed: Fixed::ZERO,
                hp: 25,
                amount: 0,
            })
            .expect("spawn");
    }
    world.step().expect("one step to apply the queued spawns");
    world
}

#[test]
#[ignore = "prints timings; run explicitly with --ignored --nocapture"]
fn idle_world_scales_with_entities() {
    let Some(fx) = fixture() else { return };
    let ticks = 2_000u32;
    for extra in [0usize, 250, 500, 1000, 2000] {
        let mut world = world_with(&fx, extra);
        let entities = world.entities().len();
        let start = Instant::now();
        for _ in 0..ticks {
            world.step().expect("step");
        }
        let elapsed = start.elapsed();
        let per_tick = elapsed.as_secs_f64() / f64::from(ticks);
        let per_tick_us = per_tick * 1e6;
        let per_entity_ns = per_tick_us * 1e3 / entities as f64;
        println!(
            "{entities:>5} entities  {per_tick_us:>9.2} us/tick  {per_entity_ns:>7.1} ns/entity/tick"
        );
    }
}

#[test]
#[ignore = "prints timings; run explicitly with --ignored --nocapture"]
fn a_ten_minute_match_in_release() {
    let Some(fx) = fixture() else { return };
    let mut world = world_with(&fx, 0);
    let entities = world.entities().len();
    let ticks = 12_000u32;
    let start = Instant::now();
    for _ in 0..ticks {
        world.step().expect("step");
    }
    let elapsed = start.elapsed();
    println!(
        "10 min at 20 Hz ({ticks} ticks, {entities} entities): {:.1} ms, {:.2} us/tick",
        elapsed.as_secs_f64() * 1e3,
        elapsed.as_secs_f64() / f64::from(ticks) * 1e6
    );
}
