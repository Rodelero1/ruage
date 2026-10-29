//! Retaliation: a struck animal turns on whoever struck it (D70).
//!
//! Written before the implementation (D7). The owner's rules for the animal slice, in order:
//!
//! - meat implies death, and the villager's killing blow lands **on arrival at the drop site**;
//! - only a villager's kill leaves food in the body;
//! - a herdable changes hands by line of sight.
//!
//! None of those matter while a boar is scenery. A boar carries 75 health and a 7-damage attack in the pack and
//! uses neither: being hit is not an order, and nothing gives an animal a task of its own. This test says what
//! "retaliation" means - the animal it was struck by becomes its target - and it is the first time an animal
//! issues itself an order.

use std::path::PathBuf;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, Task, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Fixture {
    settings: ruage_data::Sim,
    map: ruage_data::Map,
    economy: ruage_data::Economy,
    resources: ruage_data::Resources,
    units: ruage_data::Units,
    combat: ruage_data::Combat,
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
    world.load_combat(&fx.combat);
    world
}

fn task_of(world: &World, id: EntityId) -> Option<Task> {
    world
        .entities()
        .iter()
        .find(|entity| entity.id == id)
        .map(|entity| entity.task)
}

fn hp_of(world: &World, id: EntityId) -> i32 {
    world
        .entities()
        .iter()
        .find(|entity| entity.id == id)
        .map_or(0, |entity| entity.hp)
}

// PARKED, not passing. The first attempt is reverted, and what it learned is worth more than the code was:
//
// - a boar **two tiles** from the town centre is shot by the town centre, so the blow that lands is an
//   *arrow*, and an arrow lands with its damage but not with its shooter (`Task::Fly` carries no attacker).
//   An animal therefore cannot retaliate against a building, and the test was never testing melee at all;
// - moving the boar to eight tiles did not change that, which means the town centre reaches further than
//   expected, or the blow came from somewhere else again - either way, the test's premise is unproven;
// - `retaliate` was called from the melee strike path and never fired, and the debug print inside it never
//   appeared, so the melee path is not the one that damaged the boar.
//
// The mechanic still wants doing; it wants a test whose attacker is unambiguous first - two units, no
// buildings in range - rather than a boar parked beside a town centre.
#[ignore = "retaliation: the test's premise is unproven - see the note above (D70)"]
#[test]
fn a_struck_boar_turns_on_whoever_struck_it() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let boar_unit = world.role_unit("boar").expect("a boar role");
    let scale = world.scale();
    let centre = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Building)
        .expect("a town centre")
        .position;
    let villager = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
        .expect("a villager")
        .id;

    // A boar two tiles from the town centre: close enough that the villager can strike it without walking far.
    let boar = EntityId(
        world
            .entities()
            .iter()
            .map(|entity| entity.id.0)
            .max()
            .unwrap_or(0)
            + 1,
    );
    world
        .submit_checked(Command::Spawn {
            id: boar,
            player: 0,
            kind: EntityKind::Unit,
            unit: Some(boar_unit),
            position: (
                // Eight tiles out, deliberately: the town centre shoots what comes near it, and a boar
                // being shot by a building would test the arrow path - which cannot retaliate yet, because
                // an arrow lands with its damage and not with its shooter.
                Fixed::from_raw(centre.0.raw() + 8 * scale),
                Fixed::from_raw(centre.1.raw() + 8 * scale),
            ),
            speed: Fixed::from_raw(scale / 2),
            hp: 75,
            amount: 0,
        })
        .expect("a boar");
    // Orders apply on the next tick, so the boar exists only after one step.
    world.step().expect("one tick");

    world
        .submit_checked(Command::Attack {
            attacker: villager,
            target: boar,
        })
        .expect("the villager can be sent at the boar");

    // The villager has to walk in and land a blow first: retaliation is a reaction, not a stance.
    let mut struck = false;
    for _ in 0..2_000 {
        world.step().expect("stepping");
        if hp_of(&world, boar) < 75 {
            struck = true;
            break;
        }
    }
    assert!(
        struck,
        "the villager must land a blow before anything reacts"
    );

    // Then it turns round: its task names the villager, and it hits back.
    let mut turned = false;
    for _ in 0..200 {
        world.step().expect("stepping");
        if matches!(task_of(&world, boar), Some(Task::Attack { target, .. }) if target == villager)
        {
            turned = true;
        }
        if world
            .entities()
            .iter()
            .any(|entity| entity.id == villager && entity.hp < 25)
        {
            break;
        }
    }
    assert!(
        turned,
        "a struck boar must take the villager as its target, not stand there being hit"
    );
}

#[ignore = "retaliation: parked with the test above (D70)"]
#[test]
fn an_animal_that_was_never_struck_stays_idle() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let boar_unit = world.role_unit("boar").expect("a boar role");
    let scale = world.scale();
    let centre = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Building)
        .expect("a town centre")
        .position;
    let boar = EntityId(
        world
            .entities()
            .iter()
            .map(|entity| entity.id.0)
            .max()
            .unwrap_or(0)
            + 1,
    );
    world
        .submit_checked(Command::Spawn {
            id: boar,
            player: 0,
            kind: EntityKind::Unit,
            unit: Some(boar_unit),
            position: (
                Fixed::from_raw(centre.0.raw() + 8 * scale),
                Fixed::from_raw(centre.1.raw() + 8 * scale),
            ),
            speed: Fixed::from_raw(scale / 2),
            hp: 75,
            amount: 0,
        })
        .expect("a boar");

    for _ in 0..400 {
        world.step().expect("stepping");
    }
    assert!(
        !matches!(task_of(&world, boar), Some(Task::Attack { .. })),
        "a boar nobody has touched has no quarrel with anyone"
    );
}
