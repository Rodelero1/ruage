//! The scripted skirmish: both sides play, and they meet.
//!
//! Written before the implementation (D7) as far as the assertions go; the content (`config/skirmish.toml`)
//! is hand-written and this is the test that keeps it honest.

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_sim::{EntityKind, Outcome, Runner, World, load_scenario_file};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Fixture {
    settings: ruage_data::Sim,
    map: Map,
    economy: Economy,
    resources: ruage_data::Resources,
    units: Units,
    combat: ruage_data::Combat,
    techs: ruage_data::Techs,
    effects: ruage_data::Effects,
    civs: ruage_data::Civs,
}

fn fixture() -> Option<Fixture> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        return None;
    }
    let resources =
        ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources.toml");
    Some(Fixture {
        settings: ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim.toml"),
        map: ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map.toml"),
        economy: ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy.toml"),
        resources,
        units: ruage_data::load_units_file(&units).expect("units.toml"),
        combat: ruage_data::load_combat_file(&repo("config/combat.toml")).expect("combat.toml"),
        techs: ruage_data::load_techs_file(&repo("config/gamedata/techs.toml")).expect("techs"),
        effects: ruage_data::load_effects_file(&repo("config/gamedata/effects.toml"))
            .expect("effects"),
        civs: ruage_data::load_civs_file(&repo("config/gamedata/civs.toml")).expect("civs"),
    })
}

fn world(fx: &Fixture, seed: u64) -> World {
    let mut world = World::new(&fx.settings, seed).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    world.load_combat(&fx.combat);
    world
        .load_research(&fx.techs, &fx.effects)
        .expect("research tables");
    world.load_civs(&fx.civs).expect("civs");
    world
}

/// Play the skirmish to the end of its script and report the world plus every step's outcome.
fn play(fx: &Fixture, seed: u64) -> (World, Runner) {
    let scenario = load_scenario_file(&repo("config/skirmish.toml")).expect("skirmish.toml");
    let mut world = world(fx, seed);
    let mut runner = Runner::new(&scenario, world.tick_rate_hz()).expect("runnable");
    let ticks = (scenario.duration_seconds * f64::from(world.tick_rate_hz())).round() as u64;
    for _ in 0..ticks {
        runner.due(&mut world);
        world.step().expect("step");
    }
    (world, runner)
}

#[test]
fn both_sides_field_an_army() {
    let Some(fx) = fixture() else { return };
    let (world, _) = play(&fx, 7);
    let infantry = fx.economy.role["militia"];
    for player in [1u8, 2u8] {
        let count = world
            .entities()
            .iter()
            .filter(|entity| entity.player == player && entity.unit == Some(infantry))
            .count();
        assert!(
            count >= 1,
            "player {player} never fielded infantry: the skirmish is one-sided"
        );
    }
}

#[test]
fn the_two_sides_fight() {
    let Some(fx) = fixture() else { return };
    let (world, _) = play(&fx, 7);
    // A fight leaves marks: somebody's unit is hurt, or a building's health is down. Which one is
    // whichever got there first, so the assertion is about damage rather than about who won.
    let infantry = fx.economy.role["militia"];
    let hurt = world.entities().iter().any(|entity| {
        let max = entity
            .unit
            .and_then(|unit| world.max_hp_of(entity.player, unit))
            .unwrap_or(entity.hp);
        !matches!(entity.kind, EntityKind::Resource(_)) && entity.hp < max
    });
    let dead = world
        .entities()
        .iter()
        .filter(|entity| entity.unit == Some(infantry))
        .count()
        < 4;
    assert!(
        hurt || dead,
        "the two armies never met: nothing was damaged and nothing died"
    );
}

#[test]
fn the_skirmish_only_refuses_what_the_fight_has_taken() {
    let Some(fx) = fixture() else { return };
    let (_, runner) = play(&fx, 7);
    let refusals: Vec<String> = runner
        .outcomes
        .iter()
        .filter_map(|(_, outcome)| match outcome {
            Outcome::Refused(reason) => Some(reason.clone()),
            _ => None,
        })
        .collect();
    // A live match can lose the thing a later step names - that is the skirmish working, and the runner
    // says so rather than skipping silently. What must not happen is a *script* error: a name that never
    // resolved, a unit that was never available, a selector nobody implements.
    let surprises: Vec<&String> = refusals
        .iter()
        .filter(|reason| {
            !(reason.contains("has no villager")
                || reason.contains("has no finished")
                || reason.contains("is not available yet"))
        })
        .collect();
    assert!(
        surprises.is_empty(),
        "the skirmish hit a script error: {surprises:?}"
    );
}

#[test]
fn the_skirmish_is_deterministic() {
    let Some(fx) = fixture() else { return };
    let (a, _) = play(&fx, 7);
    let (b, _) = play(&fx, 7);
    assert_eq!(
        a.state_hash(),
        b.state_hash(),
        "two runs of one script diverged"
    );
}
