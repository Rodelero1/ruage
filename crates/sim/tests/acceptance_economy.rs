//! **The acceptance match has to feed itself.** The economy end to end, from `config/acceptance.toml`, in the test suite.
//!
//! Written after the D200 merge (2026-11-13) passed every other test and left the acceptance bank holding **0 food at every
//! report** from 400 s to 2,400 s: four loaded villagers froze 1.7-2.4 tiles from their town centre with a one-point route
//! aimed at the building's own blocked anchor, and no single-unit fixture saw it - every one stages a drop site whose anchor
//! a man can reach.
//!
//! The numbers are the match's own: `main` banks 56 food at 400 s and has 22 units; the merged branch banked 0 and had 14.
//! This test asks for the weaker, structural thing - **some** food reaches the bank, and the villager count grows - so it
//! does not pin a trajectory that a later rule is allowed to change.

use std::path::PathBuf;

use ruage_script::{Runner, load_scenario_file};
use ruage_sim::{EntityKind, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// The acceptance match's world, set up exactly as `ruage-headless` sets it up.
fn acceptance() -> Option<(World, Runner, u32)> {
    let units_path = repo("config/gamedata/units.toml");
    if !units_path.exists() {
        eprintln!("skipping: no units.toml (generated, D8)");
        return None;
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
    let runner = Runner::new(&scenario, rate).expect("runner");
    Some((world, runner, rate))
}

fn play(world: &mut World, runner: &mut Runner, ticks: u64) {
    for _ in 0..ticks {
        runner.due(world);
        world.step().expect("step");
    }
}

fn villagers(world: &World, player: u8) -> usize {
    let role = world.role_unit("villager").expect("a villager role");
    world
        .entities()
        .iter()
        .filter(|entity| {
            entity.kind == EntityKind::Unit
                && entity.player == player
                && entity.unit.is_some_and(|unit| world.is_family(unit, role))
        })
        .count()
}

/// **Four hundred seconds in, the bank has food in it.** Thirty seconds of game time is enough for four villagers to fill
/// and hand in ten each; zero means the loads are not getting home.
#[test]
fn the_acceptance_match_banks_food() {
    let Some((mut world, mut runner, rate)) = acceptance() else {
        return;
    };
    let food = world.slot_of("food").expect("a food column");
    play(&mut world, &mut runner, 400 * u64::from(rate));

    let banked = world.stock(1, food);
    let crew = villagers(&world, 1);
    assert!(
        banked > 0,
        "the acceptance match banked no food by 400 s: {banked} in the bank, {crew} villagers",
    );
    assert!(crew >= 4, "{crew} villagers at 400 s");
}

/// **And the crew grows.** A bank that fills but never trains is the same stall with better bookkeeping.
#[test]
fn the_acceptance_match_trains_more_villagers() {
    let Some((mut world, mut runner, rate)) = acceptance() else {
        return;
    };
    play(&mut world, &mut runner, 400 * u64::from(rate));
    let early = villagers(&world, 1);
    play(&mut world, &mut runner, 400 * u64::from(rate));
    let late = villagers(&world, 1);

    assert!(
        late > early,
        "the crew did not grow between 400 s and 800 s: {early} then {late}",
    );
}

/// The instrument, not a rule: what the movement pass decides for the loaded carriers. `--ignored --nocapture`.
#[test]
#[ignore = "measurement, not a rule"]
fn what_the_carriers_are_told() {
    let Some((mut world, mut runner, rate)) = acceptance() else {
        return;
    };
    world.trace_steps();
    let food = world.slot_of("food").expect("a food column");
    play(&mut world, &mut runner, 400 * u64::from(rate));
    for entity in world.entities() {
        if entity.kind == ruage_sim::EntityKind::Unit
            && entity.player == 1
            
        {
            println!(
                "carry id={:?} at=({:.2},{:.2}) carry={} task={:?} last={:?}",
                entity.id,
                entity.position.0.raw() as f64 / world.scale() as f64,
                entity.position.1.raw() as f64 / world.scale() as f64,
                entity.amount.raw(),
                entity.task,
                world.last_step(entity.id),
            );
        }
    }
    println!("food in the bank: {}", world.stock(1, food));
}
