//! Walls: a building whose tile nobody can walk through.
//!
//! Written before the implementation (D7). Content names which roles block (`[build] blocking_roles`), the
//! same way it names every other thing the engine must not know, so "walls block" is not a class id in the
//! engine. A wall is one tile here, like every other building in this slice.

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, SimError, World};

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

fn place_building(
    world: &mut World,
    player: u8,
    id: u32,
    role: &str,
    tile: (i64, i64),
    hp: i32,
) -> EntityId {
    let scale = world.scale();
    let unit = world.role_unit(role).unwrap_or_else(|| panic!("{role}"));
    let id = EntityId(id);
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind: EntityKind::Building,
            unit: Some(unit),
            position: (
                Fixed::from_raw(tile.0 * scale),
                Fixed::from_raw(tile.1 * scale),
            ),
            speed: Fixed::ZERO,
            hp,
            amount: 0,
        })
        .expect("spawn");
    world.step().expect("step");
    id
}

fn walker(world: &mut World, id: u32, tile: (i64, i64)) -> EntityId {
    let scale = world.scale();
    let id = EntityId(id);
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: world.role_unit("spearman"),
            position: (
                Fixed::from_raw(tile.0 * scale),
                Fixed::from_raw(tile.1 * scale),
            ),
            speed: Fixed::ZERO,
            hp: 40,
            amount: 0,
        })
        .expect("spawn");
    world.step().expect("step");
    id
}

/// A ring of wall around (30,30): the one unambiguous wall this content can express.
fn ring(world: &mut World, role: &str, first_id: u32) -> Vec<EntityId> {
    let mut ids = Vec::new();
    let mut next = first_id;
    for (dy, dxs) in [
        (-1i64, vec![-1i64, 0, 1]),
        (0, vec![-1, 1]),
        (1, vec![-1, 0, 1]),
    ] {
        for dx in dxs {
            ids.push(place_building(
                world,
                1,
                next,
                role,
                (30 + dx, 30 + dy),
                250,
            ));
            next += 1;
        }
    }
    ids
}

#[test]
fn a_wall_of_walls_closes_the_way() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    ring(&mut world, "wall", 9_900);
    let unit = walker(&mut world, 9_950, (26, 30));

    let err = world
        .submit_checked(Command::Move { unit, to: (30, 30) })
        .expect_err("nothing can walk through a closed ring of walls");
    assert!(matches!(err, SimError::NoPath { .. }), "got {err:?}");
}

#[test]
fn knocking_one_segment_down_opens_the_way() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let segments = ring(&mut world, "wall", 9_910);
    let unit = walker(&mut world, 9_951, (26, 30));

    world
        .submit_checked(Command::Delete {
            player: 1,
            entity: segments[3],
        })
        .expect("raze one segment");
    world.step().expect("step");

    world
        .submit_checked(Command::Move { unit, to: (30, 30) })
        .expect("a gap is a way through");
}

#[test]
fn a_house_does_not_block_the_way() {
    // The control: what blocks is content's list, not "a building is in the way". A ring of houses is
    // walk-through in this engine, and that is a decision (D54) rather than an oversight.
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    ring(&mut world, "house", 9_920);
    let unit = walker(&mut world, 9_952, (26, 30));

    world
        .submit_checked(Command::Move { unit, to: (30, 30) })
        .expect("a house is not a wall");
}
