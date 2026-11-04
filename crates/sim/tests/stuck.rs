//! **Nothing walks into a tree.** The owner reported it twice (2026-11-04): "militia still get stuck in the forrest", "scout
//! cavalry still gets stuck in gold mine". Both are one bug: `step_is_walkable` treated the *goal* tile as free ground, so a
//! unit ordered at a node walked inside the obstacle - and a blocked tile has no route out, so it stayed there.
//!
//! Written before the implementation (D7, SPEC §11).

use std::path::PathBuf;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn fresh() -> Option<World> {
    if !repo("config/gamedata/units.toml").exists() {
        return None;
    }
    let resources = ruage_data::load_resources_file(&repo("config/resources.toml")).ok()?;
    let settings = ruage_data::load_sim_file(&repo("config/sim.toml")).ok()?;
    let map = ruage_data::load_map_file(&repo("config/map.toml"), &resources).ok()?;
    let economy = ruage_data::load_economy_file(&repo("config/economy.toml")).ok()?;
    let units = ruage_data::load_units_file(&repo("config/gamedata/units.toml")).ok()?;
    let mut world = World::new(&settings, 7).ok()?;
    world.seed_from(&map, &economy, &units).ok()?;
    world.load_resources(&resources).ok()?;
    Some(world)
}

fn next_id(world: &World) -> EntityId {
    EntityId(world.entities().iter().map(|e| e.id.0).max().unwrap_or(0) + 1)
}

fn soldier(world: &mut World, at: (i64, i64)) -> EntityId {
    let unit = world.role_unit("militia").expect("a militia role");
    let scale = world.scale();
    let id = next_id(world);
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (Fixed::from_raw(at.0 * scale), Fixed::from_raw(at.1 * scale)),
            speed: Fixed::from_raw(scale),
            hp: 40,
            amount: 0,
        })
        .expect("a soldier");
    world.step().expect("one tick");
    id
}

/// A blocked tile on the map that has free ground beside it, found rather than written down: the map is content.
fn a_wood_tile(world: &World) -> (u16, u16) {
    for y in 1..63u16 {
        for x in 1..63u16 {
            if world.is_free_ground(x, y) {
                continue;
            }
            if world.is_free_ground(x - 1, y) || world.is_free_ground(x + 1, y) {
                return (x, y);
            }
        }
    }
    panic!("no blocked tile on this map");
}

fn tile_of(world: &World, id: EntityId) -> (u16, u16) {
    world
        .position(id)
        .map(|at| world.tile_of_position(at))
        .expect("a position")
}

/// **A unit ordered at blocked ground stands beside it, never in it.** And once it is there it can still be ordered away:
/// a unit on a blocked tile is a unit the pathfinder cannot start from.
#[test]
fn a_unit_sent_into_a_wood_stands_beside_it() {
    let Some(mut world) = fresh() else { return };
    let wood = a_wood_tile(&world);
    // Start on free ground two tiles west of the obstacle.
    let start = (wood.0.saturating_sub(3), wood.1);
    let id = soldier(&mut world, (i64::from(start.0), i64::from(start.1)));
    world
        .submit_checked(Command::Move { unit: id, to: wood })
        .expect("a move order at a tree");

    for _ in 0..400 {
        world.step().expect("stepping");
    }
    let now = tile_of(&world, id);
    assert!(
        world.is_free_ground(now.0, now.1),
        "the unit stands inside {wood:?} at {now:?}"
    );

    // And it is not stuck: an order away from the obstacle is accepted and taken.
    let away = (world.tile_of_position(world.position(id).unwrap()).0, now.1);
    let _ = away;
    let goal = (now.0.saturating_sub(4), now.1);
    world
        .submit_checked(Command::Move { unit: id, to: goal })
        .expect("a move order away");
    for _ in 0..400 {
        world.step().expect("stepping");
    }
    let after = tile_of(&world, id);
    assert_ne!(
        after, now,
        "a unit beside a tree could not walk away from it: {after:?}"
    );
}
