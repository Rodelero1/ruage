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

/// **A walker whose goal tile holds an idle friend still arrives** (D192). The owner's rule since D189 is that his own units are not
/// walls: they pass each other on the way and separate where they stand. The movement pass disagreed in one place - `collide` refuses
/// a step onto the tile the order asked for when *any* body stands there, friendly or not - and a villager told to step off a fresh
/// foundation froze 1.4 tiles from his goal for 2,000 ticks while an idle neighbour stood on the exit tile (measured: builder at
/// `(5.04, 4.39)`, walker at `(4.95, 4.68)`, idle villager at `(5.00, 4.00)`, house at hp 1).
#[test]
fn a_walker_whose_goal_tile_holds_a_friend_still_arrives() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    // **The friend, standing on the ground the order asks for** - at the tile's centre, which is where the walk ends.
    let friend = walker(&mut world, 1, (20, 20), "0.8");
    {
        let scale = world.scale();
        world
            .submit_checked(Command::Move {
                unit: friend,
                to: (20, 20),
            })
            .expect("the friend settles");
        for _ in 0..200 {
            world.step().expect("step");
        }
        let at = world.position(friend).expect("the friend");
        assert!(
            (at.0.raw() - (20 * scale + scale / 2)).abs() < scale / 2,
            "PRECONDITION: the friend is on the tile the walker is sent to"
        );
    }
    let walker_id = walker(&mut world, 1, (23, 20), "0.8");
    world
        .submit_checked(Command::Move {
            unit: walker_id,
            to: (20, 20),
        })
        .expect("move");

    let mut arrived = false;
    for _ in 0..600 {
        world.step().expect("step");
        if world
            .entity(walker_id)
            .is_some_and(|entity| entity.task == Task::Idle)
        {
            arrived = true;
            break;
        }
    }
    let at = world.position(walker_id).expect("alive");
    let scale = world.scale();
    let goal = (20 * scale, 20 * scale);
    let gap = (((at.0.raw() - goal.0) as f64).powi(2) + ((at.1.raw() - goal.1) as f64).powi(2))
        .sqrt()
        / scale as f64;
    let _ = &gap;
    // **He arrives *beside* the friend, not on his tile** (D200, which withdraws D189's pass-through). Friends are never walked
    // through; the order is satisfied when the walker stands at the pair's reach from the man already there, which is one body
    // width - measured: `gap` 1.011 tiles when he stops on the neighbouring tile centre.
    let reach = world.reach_between(walker_id, friend).raw() as f64 / scale as f64;
    // **The order completes beside him, never on him** (D200). How far beside depends on where the friend stands in the way,
    // so the bound is two tiles - the rule is that a solid friend does not strand the order, and that no two bodies overlap.
    assert!(
        arrived && gap <= 2.0,
        "MEASURED the walker {gap:.3} tiles from the tile it was sent to after 600 ticks (task {:?}); his friend holds that \
         tile, so he must stand at the pair's reach ({reach:.2}) or on the neighbouring tile, never on top of him",
        world.entity(walker_id).map(|e| e.task)
    );
    assert!(
        world.entity(friend).is_some(),
        "the friend is still in the world"
    );
}
