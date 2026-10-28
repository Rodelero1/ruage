//! Movement: a unit that has finished its order stops, and stays stopped.
//!
//! Written before the fix (D7), after a villager at a tenth of a tile per second was seen drifting east at
//! roughly twenty times its own speed after its build order ended. Whatever the cause, the contract is simple
//! and worth a test: when the task is idle, the position does not change.

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, Task, World};

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
}

fn walker(world: &mut World, player: u8, tile: (i64, i64), speed: &str) -> EntityId {
    let scale = world.scale();
    let id = EntityId(world.entities().iter().map(|e| e.id.0).max().unwrap_or(0) + 1);
    let unit = world.role_unit("villager").expect("villager");
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                Fixed::from_raw(tile.0 * scale),
                Fixed::from_raw(tile.1 * scale),
            ),
            speed: Fixed::parse(speed, scale).expect("a speed"),
            hp: 25,
            amount: 0,
        })
        .expect("spawn");
    world.step().expect("step");
    id
}

#[test]
fn a_unit_that_arrives_stops_dead() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let id = walker(&mut world, 1, (20, 20), "0.8");
    world
        .submit_checked(Command::Move {
            unit: id,
            to: (23, 20),
        })
        .expect("move");

    // Walk until the order ends.
    let mut arrived = false;
    for _ in 0..400 {
        world.step().expect("step");
        if world
            .entity(id)
            .is_some_and(|entity| entity.task == Task::Idle)
        {
            arrived = true;
            break;
        }
    }
    assert!(arrived, "the villager never finished a three-tile walk");

    let resting = world
        .entity(id)
        .map(|entity| entity.position)
        .expect("alive");
    for _ in 0..20 {
        world.step().expect("step");
    }
    let later = world
        .entity(id)
        .map(|entity| entity.position)
        .expect("alive");
    assert_eq!(
        resting, later,
        "a unit that finished its order kept moving: {resting:?} -> {later:?}"
    );
}

#[test]
fn a_builder_stops_when_the_building_is_finished() {
    // Where the drift was actually seen: a villager that finishes a house and then walks on for ever. The
    // contract is the same as for a move order, which is why the two tests sit together.
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let id = walker(&mut world, 1, (20, 20), "0.1");
    let house = world.role_unit("house").expect("house");
    world
        .submit_checked(Command::Build {
            villager: id,
            unit: house,
            at: (21, 20),
        })
        .expect("build");

    let mut finished = false;
    for _ in 0..8000 {
        world.step().expect("step");
        if world
            .entity(id)
            .is_some_and(|entity| entity.task == Task::Idle)
        {
            finished = true;
            break;
        }
    }
    assert!(finished, "the builder never finished a house");
    let resting = world
        .entity(id)
        .map(|entity| entity.position)
        .expect("alive");
    for _ in 0..20 {
        world.step().expect("step");
    }
    let later = world
        .entity(id)
        .map(|entity| entity.position)
        .expect("alive");
    assert_eq!(
        resting, later,
        "a builder kept walking after the house was finished: {resting:?} -> {later:?}"
    );
}

#[test]
fn a_very_slow_unit_walks_to_where_it_was_sent() {
    // The other half of the same bug report: at a tenth of a tile per second the villager ended up at the map
    // edge instead of its goal. Slow is not a different code path, so this is the test that says so.
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let id = walker(&mut world, 1, (20, 20), "0.1");
    world
        .submit_checked(Command::Move {
            unit: id,
            to: (22, 20),
        })
        .expect("move");

    // Two tiles at a tenth of a tile per second is twenty seconds, plus slack.
    for _ in 0..600 {
        world.step().expect("step");
    }
    let (x, y) = world
        .entity(id)
        .map(|entity| {
            (
                entity.position.0.raw() / world.scale(),
                entity.position.1.raw() / world.scale(),
            )
        })
        .expect("alive");
    assert!(
        (x, y) == (22, 20) || (x - 22).abs() <= 1 && (y - 20).abs() <= 1,
        "a slow walker ended up at {x},{y} instead of next to 22,20"
    );
}
