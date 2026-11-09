//! **One queue per building** (D157): units and techs wait in the order they were ordered, and **one runs at a time**.
//!
//! The owner: *"When units are trained in a building, there should be a training queue with visible progress. Same for tech
//! research. Units and tech can be queued in the building, but they are performed one after another not in parallel."*
//!
//! Written before the behaviour exists (D7, SPEC §11). Today a building trains a unit **and** researches a tech at the same
//! time - two independent timers - and there is no way to queue a tech at all.

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, QueuedItem, World};

mod common;

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Fixture {
    settings: ruage_data::Sim,
    map: Map,
    economy: Economy,
    units: Units,
    resources: ruage_data::Resources,
    techs: ruage_data::Techs,
    effects: ruage_data::Effects,
    civs: ruage_data::Civs,
}

fn fixture() -> Option<Fixture> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        return None;
    }
    let resources = common::present(
        &repo("config/resources.toml"),
        ruage_data::load_resources_file,
    );
    Some(Fixture {
        settings: common::present(&repo("config/sim.toml"), ruage_data::load_sim_file),
        map: common::must(
            "load_map_file",
            ruage_data::load_map_file(&repo("config/map.toml"), &resources),
        ),
        economy: common::present(&repo("config/economy.toml"), ruage_data::load_economy_file),
        units: common::must("load_units_file", ruage_data::load_units_file(&units)),
        resources,
        techs: common::present(
            &repo("config/gamedata/techs.toml"),
            ruage_data::load_techs_file,
        ),
        effects: common::present(
            &repo("config/gamedata/effects.toml"),
            ruage_data::load_effects_file,
        ),
        civs: common::present(
            &repo("config/gamedata/civs.toml"),
            ruage_data::load_civs_file,
        ),
    })
}

fn seeded(fx: &Fixture) -> World {
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    world
        .load_research(&fx.techs, &fx.effects)
        .expect("the tech and effect tables");
    world.load_civs(&fx.civs).expect("the civ table");
    world
}

/// **Room for what the queue makes.** A fresh match starts at the house cap (5/5), and a capped building's job freezes at
/// "done" until somebody dies - so a queue test that never builds a house tests the cap, not the queue.
fn make_room(world: &mut World, houses: u32) -> u32 {
    let house = world.role_unit("house").expect("house");
    let scale = world.scale();
    let mut placed = 0;
    'outer: for y in 30..56i64 {
        for x in 30..56i64 {
            if placed == houses {
                break 'outer;
            }
            let clear = (0..2)
                .all(|dy| (0..2).all(|dx| world.is_free_ground((x + dx) as u16, (y + dy) as u16)));
            if !clear {
                continue;
            }
            let before = world.population_cap(1);
            world
                .submit_checked(Command::Spawn {
                    id: EntityId(9_900 + placed),
                    player: 1,
                    kind: EntityKind::Building,
                    unit: Some(house),
                    position: (Fixed::from_raw(x * scale), Fixed::from_raw(y * scale)),
                    speed: Fixed::ZERO,
                    hp: 550,
                    amount: 0,
                })
                .expect("a house");
            world.step().expect("step");
            if world.population_cap(1) > before {
                placed += 1;
            }
        }
    }
    world.population_cap(1)
}

/// Loom: 25 s, 50 food, and the town centre's own tech - the cheapest real research in the pack.
const LOOM: u32 = 22;

/// Player 1's villagers, counted from the entity list.
fn villagers(world: &World) -> usize {
    let villager = villager_role(world);
    world
        .entities()
        .iter()
        .filter(|entity| {
            entity.player == 1
                && entity.kind == EntityKind::Unit
                && entity.hp > 0
                && entity.unit == Some(villager)
        })
        .count()
}

fn villager_role(world: &World) -> u32 {
    world.role_unit("villager").expect("villager")
}

fn town_centre(world: &World) -> EntityId {
    world
        .entities()
        .iter()
        .find(|entity| {
            entity.player == 1
                && entity.kind == EntityKind::Building
                && entity.construction.is_none()
        })
        .map(|entity| entity.id)
        .expect("player 1's town centre")
}

/// **A unit then a tech**: the tech does not begin until the unit has left.
#[test]
fn a_unit_and_then_a_tech_run_one_after_another() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let centre = town_centre(&world);
    make_room(&mut world, 4);
    let villager = villager_role(&world);
    let loom = LOOM;
    let rate = u64::from(world.tick_rate_hz());

    world
        .submit_checked(Command::Train {
            building: centre,
            unit: villager,
        })
        .expect("a villager");
    world
        .submit_checked(Command::Research {
            building: centre,
            tech: loom,
        })
        .expect("a tech behind it");
    world.step().expect("the first tick starts the unit");

    // The queue reads back as the interface will draw it: one running, one waiting.
    let production = world.production(centre);
    assert_eq!(
        production.waiting,
        vec![QueuedItem::Tech(loom)],
        "the tech waits"
    );
    assert!(
        matches!(production.running, Some((QueuedItem::Unit(u), _, _)) if u == villager),
        "the unit runs first: {:?}",
        production.running
    );

    // Halfway through the villager, the tech has not started: nothing is researching.
    let half = u32::try_from(25 * rate / 2).expect("in range");
    for _ in 0..half {
        world.step().expect("step");
    }
    assert!(
        !world.researched(1).contains(&loom),
        "the tech cannot have landed while the unit is still in the queue"
    );
    let production = world.production(centre);
    assert!(
        matches!(production.running, Some((QueuedItem::Unit(_), done, total)) if done < total),
        "still the unit: {:?}",
        production.running
    );

    // 25 s + 25 s: the tech lands no earlier than 50 s after the two orders.
    let total = u32::try_from(50 * rate).expect("in range");
    for _ in 0..total {
        world.step().expect("step");
    }
    assert!(
        world.researched(1).contains(&loom),
        "the tech lands after both jobs have had their time"
    );
}

/// **A tech then a unit**: the same rule the other way round, which is the case two independent timers got wrong.
#[test]
fn a_tech_and_then_a_unit_wait_their_turn() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let centre = town_centre(&world);
    make_room(&mut world, 4);
    let villager = villager_role(&world);
    let loom = LOOM;
    let rate = u64::from(world.tick_rate_hz());
    let before = villagers(&world);

    world
        .submit_checked(Command::Research {
            building: centre,
            tech: loom,
        })
        .expect("a tech");
    world
        .submit_checked(Command::Train {
            building: centre,
            unit: villager,
        })
        .expect("a unit behind it");
    world.step().expect("step");

    let production = world.production(centre);
    assert!(
        matches!(production.running, Some((QueuedItem::Tech(t), _, _)) if t == loom),
        "the tech runs first: {:?}",
        production.running
    );

    // At 30 s the tech has landed (25 s) and the villager is under way but not out.
    for _ in 0..u32::try_from(30 * rate).expect("in range") {
        world.step().expect("step");
    }
    assert!(world.researched(1).contains(&loom), "the tech landed first");
    assert_eq!(
        villagers(&world),
        before,
        "the villager is still being trained, not out yet"
    );

    // At 25 s + 25 s it is out.
    for _ in 0..u32::try_from(21 * rate).expect("in range") {
        world.step().expect("step");
    }
    let roster: Vec<(u32, Option<u32>)> = world
        .entities()
        .iter()
        .filter(|e| e.player == 1 && e.kind == EntityKind::Unit)
        .map(|e| (e.id.0, e.unit))
        .collect();
    eprintln!(
        "DBG units={roster:?} villagers={} pop={}/{}",
        villagers(&world),
        world.population(1),
        world.population_cap(1)
    );
    assert_eq!(
        villagers(&world),
        before + 1,
        "the villager arrives after the tech's time and its own"
    );
}

/// **Two units and a tech in one queue**: everything runs in order, one at a time, and the whole thing takes the sum of the
/// three times rather than the longest.
#[test]
fn two_units_and_a_tech_take_the_sum_of_their_times() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let centre = town_centre(&world);
    make_room(&mut world, 4);
    let villager = villager_role(&world);
    let loom = LOOM;
    let rate = u64::from(world.tick_rate_hz());
    let before = villagers(&world);

    for order in [
        Command::Train {
            building: centre,
            unit: villager,
        },
        Command::Train {
            building: centre,
            unit: villager,
        },
        Command::Research {
            building: centre,
            tech: loom,
        },
    ] {
        world.submit_checked(order).expect("an order");
    }
    world.step().expect("step");
    assert_eq!(
        world.production(centre).waiting.len(),
        2,
        "two orders wait behind the one running"
    );

    // 25 + 25 + 25 = 75 s of work: at 70 s the tech has not landed.
    for _ in 0..u32::try_from(70 * rate).expect("in range") {
        world.step().expect("step");
    }
    assert!(!world.researched(1).contains(&loom), "not before its turn");
    for _ in 0..u32::try_from(10 * rate).expect("in range") {
        world.step().expect("step");
    }
    assert!(world.researched(1).contains(&loom), "and lands after it");
    assert_eq!(villagers(&world), before + 2, "both villagers are out");
}
