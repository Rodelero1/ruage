//! **How often the levelled tier actually answers** (D338): a measurement, not a rule.
//!
//! Run it in release, ignored, and read the four numbers:
//! `cargo test --release -p ruage-sim --test tier_probe -- --ignored --nocapture`
//!
//! It exists because D334 found the tier answering **0 of 121** long walks - the gate compared `avoid.is_empty()` and every long
//! walk in a busy match carries a stander - and because a green suite plus an unchanged hash is not evidence that a branch ran.
//! The count comes from the engine's own instrumentation (`Ground::tier_answers`), so this file cannot disagree with the sim.

use std::path::PathBuf;

use ruage_script::{Runner, load_scenario_file};
use ruage_sim::World;

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// The acceptance match's world, set up exactly as `ruage-headless` sets it up.
#[test]
#[ignore = "a measurement over a full match: run it in release with --ignored"]
fn how_many_routes_the_tier_answers() {
    let units_path = repo("config/gamedata/units.toml");
    if !units_path.exists() {
        eprintln!("skipping: no units.toml (generated, D8)");
        return;
    }
    let resources = ruage_data::load_resources_file(&repo("config/resources.toml")).expect("pack");
    let settings = ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim");
    let map = ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map");
    let economy = ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy");
    let units = ruage_data::load_units_file(&units_path).expect("units");
    let combat = ruage_data::load_combat_file(&repo("config/combat.toml")).expect("combat");
    let techs = ruage_data::load_techs_file(&repo("config/gamedata/techs.toml")).expect("techs");
    let effects =
        ruage_data::load_effects_file(&repo("config/gamedata/effects.toml")).expect("effects");
    let civs = ruage_data::load_civs_file(&repo("config/gamedata/civs.toml")).expect("civs");
    let scenario = load_scenario_file(&repo("config/acceptance.toml")).expect("scenario");

    let mut world = World::new(&settings, 7).expect("world");
    world.seed_from(&map, &economy, &units).expect("seed");
    world.load_resources(&resources).expect("pack");
    world.apply_policy(&scenario.policy);
    world.load_combat(&combat);
    world.load_research(&techs, &effects).expect("research");
    world.load_civs(&civs).expect("civs");
    let rate = world.tick_rate_hz();
    let mut runner = Runner::new(&scenario, rate).expect("runner");

    for _ in 0..48_000 {
        runner.due(&mut world);
        world.step().expect("step");
    }
    println!(
        "tier answers {} of {} routes; {} re-paths; hash {:016x}",
        world.tier_answers(),
        world.routes_planned(),
        world.repaths(),
        world.state_hash()
    );
}
