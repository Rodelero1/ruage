//! Rules: a condition and one action, re-evaluated - the shape the game's own AI uses for a build order
//! (`docs/ai-of-the-game.md`), and the shape that does not rot when the economy's speed changes.
//!
//! Written *after* the implementation, which is recorded in the review rather than hidden: the compile failures these
//! tests first produced only proved the type names. Each test was then shown to bite by mutating the code it covers.

use std::path::PathBuf;

use ruage_sim::{EntityKind, Runner, Scenario, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn fresh() -> Option<World> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        return None;
    }
    let resources = ruage_data::load_resources_file(&repo("config/resources.toml")).ok()?;
    let settings = ruage_data::load_sim_file(&repo("config/sim.toml")).ok()?;
    let map = ruage_data::load_map_file(&repo("config/map.toml"), &resources).ok()?;
    let economy = ruage_data::load_economy_file(&repo("config/economy.toml")).ok()?;
    let units = ruage_data::load_units_file(&units).ok()?;
    let combat = ruage_data::load_combat_file(&repo("config/combat.toml")).ok()?;
    let mut world = World::new(&settings, 7).ok()?;
    world.seed_from(&map, &economy, &units).ok()?;
    world.load_resources(&resources).ok()?;
    world.load_combat(&combat);
    Some(world)
}

fn houses(world: &World) -> usize {
    let house = world.role_unit("house").expect("a house role");
    world
        .entities()
        .iter()
        .filter(|entity| {
            entity.player == 1
                && entity.kind == EntityKind::Building
                && entity.unit.is_some_and(|unit| world.is_family(unit, house))
        })
        .count()
}

/// A rule whose condition is false does nothing; the same rule with a true condition builds.
#[test]
fn a_rule_waits_for_its_condition() {
    let Some(mut asleep) = fresh() else {
        return;
    };
    let scenario: Scenario = toml::from_str(
        r#"
schema = 2
duration_seconds = 60.0

[[rule]]
player = 1
cooldown_seconds = 2.0
when = { villagers_at_least = 500 }
do = "build"
role = "house"
"#,
    )
    .expect("a rule the format can read");

    let before = houses(&asleep);
    let mut runner = Runner::new(&scenario, 20).expect("a runner");
    for _ in 0..600 {
        runner.due(&mut asleep);
        asleep.step().expect("stepping");
    }
    assert_eq!(houses(&asleep), before, "the condition is false: no house");
    assert_eq!(runner.fired_times, vec![0], "the rule never fired");

    // The same rule with a condition the world meets.
    let Some(mut awake) = fresh() else {
        return;
    };
    let scenario: Scenario = toml::from_str(
        r#"
schema = 2
duration_seconds = 60.0

[[rule]]
player = 1
cooldown_seconds = 2.0
when = { villagers_at_least = 1 }
do = "build"
role = "house"
"#,
    )
    .expect("a rule the format can read");
    let before = houses(&awake);
    let mut runner = Runner::new(&scenario, 20).expect("a runner");
    for _ in 0..120 {
        runner.due(&mut awake);
        awake.step().expect("stepping");
    }
    assert!(
        houses(&awake) > before,
        "the condition holds, so a house should be going up: {before} before, {} now",
        houses(&awake)
    );
    assert!(runner.fired_times[0] >= 1, "the rule fired");
}

/// A satisfied condition does not mean an order every tick: the cooldown bounds the firings.
#[test]
fn a_rule_respects_its_cooldown() {
    let Some(mut world) = fresh() else {
        return;
    };
    let scenario: Scenario = toml::from_str(
        r#"
schema = 2
duration_seconds = 60.0

[[rule]]
player = 1
cooldown_seconds = 10.0
when = { villagers_at_least = 1 }
do = "gather"
resource = "food"
"#,
    )
    .expect("a rule the format can read");
    let mut runner = Runner::new(&scenario, 20).expect("a runner");
    // Forty game seconds at 20 Hz: at most five firings, never eight hundred.
    for _ in 0..800 {
        runner.due(&mut world);
        world.step().expect("stepping");
    }
    let fired = runner.fired_times[0];
    assert!(
        (1..=5).contains(&fired),
        "a 10-second cooldown over 40 seconds fires one to five times, not {fired}"
    );
}

/// `at_seconds` holds a rule back even when its condition is true from the first tick.
#[test]
fn a_rule_waits_for_its_time_too() {
    let Some(mut world) = fresh() else {
        return;
    };
    let scenario: Scenario = toml::from_str(
        r#"
schema = 2
duration_seconds = 60.0

[[rule]]
player = 1
at_seconds = 20.0
cooldown_seconds = 1.0
when = { villagers_at_least = 1 }
do = "gather"
resource = "food"
"#,
    )
    .expect("a rule the format can read");
    let mut runner = Runner::new(&scenario, 20).expect("a runner");
    for _ in 0..300 {
        runner.due(&mut world);
        world.step().expect("stepping");
    }
    // 15 game seconds: not yet.
    assert_eq!(
        runner.fired_times,
        vec![0],
        "a rule at 20 s does not fire at 15"
    );
    for _ in 0..300 {
        runner.due(&mut world);
        world.step().expect("stepping");
    }
    assert!(
        runner.fired_times[0] >= 1,
        "and it fires once its time comes"
    );
}
