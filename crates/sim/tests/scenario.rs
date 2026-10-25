//! The step-5 scenario: one scripted match, run to the end, reported tick by tick.
//!
//! Written before the runner (D7).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Sim as Settings, Units};
use ruage_sim::{
    Command, EntityKind, Outcome, Runner, World, load_scenario_file, load_scenario_str,
};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Fixture {
    settings: Settings,
    map: Map,
    economy: Economy,
    units: Units,
    combat: ruage_data::Combat,
}

fn fixture() -> Option<Fixture> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        eprintln!(
            "skipping: {} is not present (generated data, D8)",
            units.display()
        );
        return None;
    }
    Some(Fixture {
        settings: ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim.toml"),
        map: ruage_data::load_map_file(&repo("config/map.toml")).expect("map.toml"),
        economy: ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy.toml"),
        units: ruage_data::load_units_file(&units).expect("units.toml"),
        combat: ruage_data::load_combat_file(&repo("config/combat.toml")).expect("combat.toml"),
    })
}

fn world(fx: &Fixture, seed: u64) -> World {
    let mut world = World::new(&fx.settings, seed).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    world.load_combat(&fx.combat);
    world
}

/// Run a scenario and collect what the world looked like at the end, plus the step outcomes.
fn play(fx: &Fixture, text: &str, seed: u64) -> (World, Runner) {
    let scenario = load_scenario_str(text, "<scenario>").expect("the scenario parses");
    let mut world = world(fx, seed);
    let mut runner = Runner::new(&scenario, world.tick_rate_hz()).expect("a runnable scenario");
    let ticks =
        u64::try_from((scenario.duration_seconds * f64::from(world.tick_rate_hz())).round() as i64)
            .expect("a length in ticks");
    for _ in 0..ticks {
        runner.due(&mut world);
        world.step().expect("step");
    }
    (world, runner)
}

#[test]
fn the_project_scenario_parses_and_is_runnable() {
    let Some(fx) = fixture() else { return };
    let scenario = load_scenario_file(&repo("config/scenario.toml")).expect("config/scenario.toml");
    assert!(
        (scenario.duration_seconds - 600.0).abs() < f64::EPSILON,
        "the step-5 scenario is a ten-minute match, got {} s",
        scenario.duration_seconds
    );
    let mut world = world(&fx, 3);
    let runner = Runner::new(&scenario, world.tick_rate_hz()).expect("runnable");
    assert!(runner.pending(), "there are steps to run");
    let _ = world.step();
}

#[test]
fn a_step_that_cannot_be_resolved_is_reported_not_dropped() {
    let Some(fx) = fixture() else { return };
    // No such villager: player 1 starts with three.
    let text = r#"
schema = 1
duration_seconds = 1.0

[[step]]
at_seconds = 0.1
player = 1
kind = "gather"
villager = 9
resource = "wood"
"#;
    let (_, runner) = play(&fx, text, 3);
    assert_eq!(runner.outcomes.len(), 1, "the step was attempted");
    match &runner.outcomes[0].1 {
        Outcome::Refused(reason) => assert!(reason.contains("no villager 9"), "got: {reason}"),
        other => panic!("expected Refused, got {other:?}"),
    }
}

#[test]
fn a_scripted_order_moves_the_world() {
    let Some(fx) = fixture() else { return };
    let text = r#"
schema = 1
duration_seconds = 60.0

[[step]]
at_seconds = 0.1
player = 1
kind = "gather"
villager = 0
resource = "wood"
"#;
    let (world, runner) = play(&fx, text, 3);
    assert_eq!(runner.outcomes.len(), 1);
    assert_eq!(runner.outcomes[0].1, Outcome::Ran);
    assert!(
        world.stock(1, ruage_data::ResourceKind::Wood) > 0,
        "a minute of gathering should reach the bank"
    );
}

#[test]
fn the_ten_minute_scenario_builds_trains_and_fights() {
    let Some(fx) = fixture() else { return };
    let text = std::fs::read_to_string(repo("config/scenario.toml")).expect("config/scenario.toml");
    let (world, runner) = play(&fx, &text, 7);

    for (seconds, outcome) in &runner.outcomes {
        assert_eq!(
            *outcome,
            Outcome::Ran,
            "the step due at {seconds} s did not run: {outcome:?}"
        );
    }

    // A house and a barracks stand for player 1, finished, not still going up.
    for unit in [70u32, 12] {
        let built = world.entities().iter().any(|entity| {
            entity.player == 1
                && entity.kind == EntityKind::Building
                && entity.unit == Some(unit)
                && entity.construction.is_none()
        });
        assert!(
            built,
            "unit {unit} should be standing and finished by ten minutes"
        );
    }

    // The spearman exists, and something died: the fight happened.
    let spearmen = world
        .entities()
        .iter()
        .filter(|entity| {
            entity.player == 1 && entity.kind == EntityKind::Unit && entity.unit == Some(74)
        })
        .count();
    assert_eq!(spearmen, 1, "one spearman was trained");
    let villagers_left = world
        .entities()
        .iter()
        .filter(|entity| entity.player == 2 && entity.kind == EntityKind::Unit)
        .count();
    assert!(
        villagers_left < 3,
        "the spearman should have killed an opponent villager, {villagers_left} of 3 remain"
    );
}

#[test]
fn the_scenario_is_deterministic() {
    let Some(fx) = fixture() else { return };
    let text = std::fs::read_to_string(repo("config/scenario.toml")).expect("config/scenario.toml");
    let (first, _) = play(&fx, &text, 7);
    let (second, _) = play(&fx, &text, 7);
    assert_eq!(
        first.state_hash(),
        second.state_hash(),
        "two runs of the same script must end in the same state"
    );
    // The seed does not change this match: nothing draws from the RNG streams, and the map is
    // authored. So the second check is on the *script*: drop the last step and the state must differ,
    // or the hash is not measuring the match at all.
    let shortened = text
        .rsplit_once("[[step]]")
        .map(|(head, _)| head.to_owned())
        .expect("the scenario has steps");
    let (other, _) = play(&fx, &shortened, 7);
    assert_ne!(
        first.state_hash(),
        other.state_hash(),
        "a script with one step fewer must end in a different state"
    );
}

#[test]
fn stopping_a_unit_is_always_accepted() {
    let Some(fx) = fixture() else { return };
    let mut world = world(&fx, 3);
    let villager = world
        .entities()
        .iter()
        .find(|e| e.player == 1 && e.kind == EntityKind::Unit)
        .expect("a villager")
        .id;
    world
        .submit_checked(Command::Stop { unit: villager })
        .expect("stopping needs no preconditions");
}
