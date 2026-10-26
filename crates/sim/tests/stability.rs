//! Stability probes: long runs, repeated runs, and the invariants that must hold at the end.
//!
//! The long ones are `#[ignore]`d because they take seconds, not milliseconds. Run them with:
//!
//! ```text
//! cargo test --release -p ruage-sim --test stability -- --ignored --nocapture
//! ```

use std::path::PathBuf;

use ruage_data::{Economy, Map, Sim as Settings, Units};
use ruage_sim::World;

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn fixture() -> Option<(Settings, Map, Economy, Units, ruage_data::Resources)> {
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
        ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources.toml"),
    ))
}

fn world(fx: &(Settings, Map, Economy, Units, ruage_data::Resources), seed: u64) -> World {
    let (settings, map, economy, units, resources) = fx;
    let mut world = World::new(settings, seed).expect("world");
    world.seed_from(map, economy, units).expect("seeding");
    world
        .load_resources(resources)
        .expect("the pack fits the stockpile");
    world
}

#[test]
#[ignore = "runs four simulated hours; run explicitly with --ignored"]
fn four_hours_without_a_panic_or_a_leak() {
    let Some(fx) = fixture() else { return };
    let mut world = world(&fx, 3);
    let start_entities = world.entities().len();
    let mut samples = vec![start_entities];

    // 4 simulated hours at 20 Hz. Nothing is ordered, so this is the idle-world backbone: it should
    // neither grow nor shrink, and nothing in it should divide by zero, overflow or panic.
    for tick in 0..288_000u32 {
        world.step().expect("a long run must not fail");
        if tick % 96_000 == 0 {
            samples.push(world.entities().len());
        }
    }
    let end_entities = world.entities().len();
    println!("entities: {samples:?} -> {end_entities}");
    assert_eq!(
        samples,
        vec![start_entities; samples.len()],
        "an idle world must not accumulate or lose entities"
    );
    assert_eq!(world.tick(), 288_000, "no tick may be skipped");
}

#[test]
#[ignore = "runs twice over four simulated hours; run explicitly with --ignored"]
fn the_same_seed_gives_the_same_world_at_length() {
    // Determinism is checked in the scenario tests, but only over ten minutes. A divergence that
    // needs hours to appear - an accumulator, an unordered iteration - would slip past those.
    let Some(fx) = fixture() else { return };
    let mut first = world(&fx, 11);
    let mut second = world(&fx, 11);
    for _ in 0..288_000u32 {
        first.step().expect("first");
        second.step().expect("second");
    }
    assert_eq!(
        first.state_hash(),
        second.state_hash(),
        "two runs of the same seed must agree after four hours"
    );
    println!("hash after 288000 ticks: {:016x}", first.state_hash());
}

#[test]
fn a_world_reports_the_entities_it_actually_holds() {
    // Entities are the one collection that can be added to by the tick itself (production) and
    // removed from by it (death). If the count reported ever disagreed with the list, every
    // capacity bound downstream would be wrong.
    let Some(fx) = fixture() else { return };
    let world = world(&fx, 1);
    assert_eq!(world.entities().len(), 126, "the authored map seeds 126");
    assert!(
        world.entities().iter().all(|e| e.id.0 > 0),
        "ids start at 1"
    );
}
