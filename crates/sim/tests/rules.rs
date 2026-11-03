//! Rules: a condition and one action, re-evaluated - the shape the game's own AI uses for a build order
//! (`docs/ai-of-the-game.md`), and the shape that does not rot when the economy's speed changes.
//!
//! Written *after* the implementation, which is recorded in the review rather than hidden: the compile failures these
//! tests first produced only proved the type names. Each test was then shown to bite by mutating the code it covers.

use std::path::PathBuf;

use ruage_fixed::Fixed;
use ruage_sim::{EntityId, EntityKind, Runner, Scenario, Task, World};

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

/// `no_building` means "the player has none of these", not "the pack knows this role": a rule that builds a mill must
/// stop once a mill stands, or it re-orders one every cooldown for the whole match - and pays for it.
#[test]
fn a_rule_with_no_building_stops_once_it_stands() {
    let Some(mut world) = fresh() else {
        return;
    };
    // Town centres exist from the first tick, so this rule's condition is false immediately and can never fire.
    let scenario: Scenario = toml::from_str(
        r#"
schema = 2
duration_seconds = 60.0

[[rule]]
player = 1
cooldown_seconds = 1.0
when = { no_building = "town_center" }
do = "gather"
resource = "food"
"#,
    )
    .expect("a rule the format can read");
    let mut runner = Runner::new(&scenario, 20).expect("a runner");
    for _ in 0..600 {
        runner.due(&mut world);
        world.step().expect("stepping");
    }
    assert_eq!(
        runner.fired_times,
        vec![0],
        "a rule whose building already stands must not fire"
    );
}

/// Place one unit for player 1 at a tile. **Distinct ids**: three spawns that all name `max + 1` are one entity and two
/// refusals - which is exactly how the first version of the army test "proved" `World::army` broken.
fn spawn_unit(world: &mut World, unit: u32, tile: (i64, i64)) -> EntityId {
    let scale = world.scale();
    let id = EntityId(
        world
            .entities()
            .iter()
            .map(|entity| entity.id.0)
            .max()
            .unwrap_or(0)
            + 1,
    );
    world
        .submit_checked(ruage_sim::Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                Fixed::from_raw(tile.0 * scale),
                Fixed::from_raw(tile.1 * scale),
            ),
            speed: Fixed::ZERO,
            hp: 40,
            amount: 0,
        })
        .expect("a unit can be placed");
    world.step().expect("stepping");
    id
}

/// **An army is a count of soldiers, not of units.** A villager is not a soldier; a man-at-arms is, and he counts as
/// his line's root, because content names the line and the dat swaps the record on upgrade.
#[test]
fn an_army_counts_soldiers_by_family_and_not_villagers() {
    let Some(mut world) = fresh() else {
        return;
    };
    let villager = world.role_unit("villager").expect("a villager role");
    let militia = world.role_unit("militia").expect("a militia role");
    let man_at_arms = world.role_unit("man_at_arms").expect("a man-at-arms role");
    let before = world.army(1);
    assert_eq!(before, 0, "the opening seeds villagers, not soldiers");
    spawn_unit(&mut world, villager, (18, 18));
    assert_eq!(world.army(1), before, "a fourth villager is not an army");
    spawn_unit(&mut world, militia, (19, 18));
    spawn_unit(&mut world, militia, (20, 18));
    spawn_unit(&mut world, man_at_arms, (21, 18));
    assert_eq!(
        world.army(1),
        3,
        "two militia and a man-at-arms are three soldiers of one line"
    );
}

/// **"Gather an army first, then attack"** (the owner, 2026-11-03): a rule waits on the soldier count, where
/// `has_unit` fires for a single unit.
#[test]
fn an_army_rule_waits_for_an_army() {
    let Some(mut world) = fresh() else {
        return;
    };
    let scenario: Scenario = toml::from_str(
        r#"
schema = 2
duration_seconds = 60.0

[[rule]]
player = 1
cooldown_seconds = 2.0
when = { army_at_least = 3 }
do = "build"
role = "house"
"#,
    )
    .expect("a rule the format can read");
    let before = houses(&world);
    let mut runner = Runner::new(&scenario, 20).expect("a runner");
    for _ in 0..120 {
        runner.due(&mut world);
        world.step().expect("stepping");
    }
    assert_eq!(runner.fired_times, vec![0], "no soldiers, so no army");
    assert_eq!(houses(&world), before, "and nothing else happened either");

    let militia = world.role_unit("militia").expect("a militia role");
    for n in 0..3 {
        spawn_unit(&mut world, militia, (20 + n, 22));
    }
    for _ in 0..120 {
        runner.due(&mut world);
        world.step().expect("stepping");
    }
    assert!(
        runner.fired_times[0] >= 1,
        "three soldiers are an army, so the rule fires"
    );
}

/// **An attack is a body of soldiers, not the newest one.** The first shape ordered the latest unit of a role, so a
/// mangonel fought alone and eleven soldiers stood at the barracks. Here every soldier of the family gets the order.
#[test]
fn an_attack_rule_sends_the_whole_army() {
    let Some(mut world) = fresh() else {
        return;
    };
    let militia = world.role_unit("militia").expect("a militia role");
    let army: Vec<EntityId> = (0..3)
        .map(|n| spawn_unit(&mut world, militia, (18 + n, 20)))
        .collect();
    let scenario: Scenario = toml::from_str(
        r#"
schema = 2
duration_seconds = 60.0

[[rule]]
player = 1
cooldown_seconds = 5.0
when = { army_at_least = 3 }
do = "attack"
attacker = "militia"
target = "enemy_town_center"
"#,
    )
    .expect("a rule the format can read");
    let mut runner = Runner::new(&scenario, 20).expect("a runner");
    for _ in 0..40 {
        runner.due(&mut world);
        world.step().expect("stepping");
    }
    assert!(runner.fired_times[0] >= 1, "the rule fired");
    for id in &army {
        let task = world.entity(*id).map(|entity| entity.task);
        assert!(
            matches!(task, Some(Task::Attack { .. })),
            "{id:?} was left behind: {task:?}"
        );
    }
}
