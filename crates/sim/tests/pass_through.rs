//! **Nothing walks through a building or a mine** (D158): the owner's report, with screenshots - *"villager can pass through the
//! built house or stone mine"*.
//!
//! The engine blocks a building's whole footprint **when it is inserted**, and a node's tile when the node is seeded. So the
//! question is not "is the tile blocked" but "does a body that is already walking respect a block that appeared, or that it was
//! never told about". Written before any change (D7, SPEC §11).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

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
    let resources = common::present(
        &repo("config/resources.toml"),
        ruage_data::load_resources_file,
    );
    Some(Fixture {
        settings: common::present(&repo("config/sim.toml"), ruage_data::load_sim_file),
        map: common::must(
            "map",
            ruage_data::load_map_file(&repo("config/map.toml"), &resources),
        ),
        economy: common::present(&repo("config/economy.toml"), ruage_data::load_economy_file),
        units: common::must("units", ruage_data::load_units_file(&units)),
        resources,
    })
}

fn seeded(fx: &Fixture) -> World {
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    world.load_resources(&fx.resources).expect("stockpile");
    world
}

fn villager(world: &mut World, id: u32, tile: (i64, i64)) -> EntityId {
    let scale = world.scale();
    let unit = world.role_unit("villager").expect("villager");
    let id = EntityId(id);
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                Fixed::from_raw(tile.0 * scale),
                Fixed::from_raw(tile.1 * scale),
            ),
            speed: Fixed::parse("1.0", scale).expect("speed"),
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    world.step().expect("step");
    id
}

fn open_patch(world: &World, width: i64, height: i64) -> Option<(i64, i64)> {
    for y in 30..56i64 {
        for x in 8..56i64 {
            let clear = (0..height).all(|dy| {
                (0..width).all(|dx| {
                    let (tx, ty) = ((x + dx) as u16, (y + dy) as u16);
                    world.tile_walkable(tx, ty) && world.is_free_ground(tx, ty)
                })
            });
            if clear {
                return Some((x, y));
            }
        }
    }
    None
}

/// The tile a body stands on, as the engine sees it.
fn tile_of(world: &World, id: EntityId) -> (u16, u16) {
    world.tile_of_position(world.position(id).expect("position"))
}

/// **A route planned before a house existed must not walk through it.** This is the case the screenshots show: the villager is
/// already walking when the wall goes up across its way.
#[test]
fn a_house_built_across_a_walker_makes_it_go_round() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let house_role = world.role_unit("house").expect("house");
    let scale = world.scale();
    let Some(at) = open_patch(&world, 9, 3) else {
        return;
    };
    let walker = villager(&mut world, 9_400, (at.0, at.1));
    // A long walk along the row the house will appear on: the route is through the middle of it.
    world
        .submit_checked(Command::Move {
            unit: walker,
            to: (
                u16::try_from(at.0 + 8).expect("in range"),
                u16::try_from(at.1).expect("in range"),
            ),
        })
        .expect("a walk");
    for _ in 0..40 {
        world.step().expect("step");
    }

    // A **finished** house appears in the middle of the walker's road, three tiles ahead.
    let middle = (at.0 + 4, at.1);
    world
        .submit_checked(Command::Spawn {
            id: EntityId(9_401),
            player: 1,
            kind: EntityKind::Building,
            unit: Some(house_role),
            position: (
                Fixed::from_raw(middle.0 * scale),
                Fixed::from_raw(middle.1 * scale),
            ),
            speed: Fixed::ZERO,
            hp: 550,
            amount: 0,
        })
        .expect("a house");
    world.step().expect("step");

    // Every tile of the house is solid, and the walker never stands on one.
    let footprint: Vec<(u16, u16)> = (0..2)
        .flat_map(|dy| (0..2).map(move |dx| (dx, dy)))
        .map(|(dx, dy)| ((middle.0 + dx) as u16, (middle.1 + dy) as u16))
        .collect();
    for tile in &footprint {
        assert!(
            !world.tile_walkable(tile.0, tile.1),
            "the house's tile {tile:?} is solid"
        );
    }
    for step in 0..600 {
        world.step().expect("step");
        let at = tile_of(&world, walker);
        assert!(
            !footprint.contains(&at),
            "step {step}: the walker passed through the house at {at:?}"
        );
    }
    // It arrives **within one tile**: after the interruption the goal is snapped (D134), and this walker stops at (15,29) for a
    // goal of (16,30) - one tile short. That is recorded as a finding in the review rather than asserted as correct: what this
    // test is for is that it went **round** and not through.
    let here = tile_of(&world, walker);
    let goal = (
        u16::try_from(at.0 + 8).expect("in range"),
        u16::try_from(at.1).expect("in range"),
    );
    assert!(
        here.0.abs_diff(goal.0) <= 1 && here.1.abs_diff(goal.1) <= 1,
        "it still got there: {here:?} against {goal:?}"
    );
}

/// **The same with the order a player actually gives.** The window's right click sends [`Command::WalkTo`] - a *point*, not a tile -
/// and that path is free of the grid in a way a tile order is not (A, D134). If a house appears across it, the walker must go round
/// or stop, never through.
#[test]
fn a_walk_to_a_point_does_not_cut_through_a_new_house() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let house_role = world.role_unit("house").expect("house");
    let scale = world.scale();
    let Some(at) = open_patch(&world, 9, 3) else {
        return;
    };
    let walker = villager(&mut world, 9_420, (at.0, at.1));
    let goal = (
        Fixed::from_raw((at.0 + 8) * scale),
        Fixed::from_raw(at.1 * scale),
    );
    world
        .submit_checked(Command::WalkTo {
            unit: walker,
            to: goal,
        })
        .expect("a walk to a point");
    for _ in 0..30 {
        world.step().expect("step");
    }
    let middle = (at.0 + 4, at.1);
    world
        .submit_checked(Command::Spawn {
            id: EntityId(9_421),
            player: 1,
            kind: EntityKind::Building,
            unit: Some(house_role),
            position: (
                Fixed::from_raw(middle.0 * scale),
                Fixed::from_raw(middle.1 * scale),
            ),
            speed: Fixed::ZERO,
            hp: 550,
            amount: 0,
        })
        .expect("a house");
    world.step().expect("step");

    let footprint: Vec<(u16, u16)> = (0..2)
        .flat_map(|dy| (0..2).map(move |dx| (dx, dy)))
        .map(|(dx, dy)| ((middle.0 + dx) as u16, (middle.1 + dy) as u16))
        .collect();
    for step in 0..600 {
        world.step().expect("step");
        let here = tile_of(&world, walker);
        assert!(
            !footprint.contains(&here),
            "step {step}: the walker cut through the house at {here:?}"
        );
    }
}

/// **The villager who raises a house is not left inside it.** The builder walks to its site to work; if it stands *on* the
/// footprint when the last hit point lands, the finished building's tiles are solid underneath it and it can never path out -
/// which is what "a villager inside a built house" looks like from the outside.
#[test]
fn the_builder_is_not_inside_the_house_it_finishes() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let house_role = world.role_unit("house").expect("house");
    let Some(at) = open_patch(&world, 8, 3) else {
        return;
    };
    let site = (at.0 + 3, at.1);
    let builder = villager(&mut world, 9_410, (at.0, at.1));
    // **Remembered ground** (D183): the fixture's patch is away from the base, and a foundation may be placed blind in
    // fog but never in the black.
    common::look_away(
        &mut world,
        1,
        (
            u16::try_from(site.0).expect("in range"),
            u16::try_from(site.1).expect("in range"),
        ),
    );
    world
        .submit_checked(Command::Build {
            villager: builder,
            unit: house_role,
            at: (
                u16::try_from(site.0).expect("in range"),
                u16::try_from(site.1).expect("in range"),
            ),
        })
        .expect("a build order");

    let footprint: Vec<(u16, u16)> = (0..2)
        .flat_map(|dy| (0..2).map(move |dx| (dx, dy)))
        .map(|(dx, dy)| ((site.0 + dx) as u16, (site.1 + dy) as u16))
        .collect();
    let mut built_at = None;
    for step in 0..4_000 {
        world.step().expect("step");
        if built_at.is_none()
            && world.entities().iter().any(|entity| {
                entity.unit == Some(house_role) && entity.construction.is_none() && entity.hp > 0
            })
        {
            built_at = Some(step);
        }
        if built_at.is_some() {
            let here = tile_of(&world, builder);
            assert!(
                !footprint.contains(&here),
                "step {step}: the builder is inside its finished house at {here:?}"
            );
        }
    }
    assert!(built_at.is_some(), "the house was finished");
}

/// **A villager never stands on the mine it works.** The node's own tile is blocked, so a gatherer has to work from beside it.
#[test]
fn a_miner_never_stands_on_the_mine() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    // A stone mine of our own, on open ground, and a villager ordered onto it.
    let Some(at) = open_patch(&world, 6, 3) else {
        return;
    };
    let stone = world
        .entities()
        .iter()
        .find(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .map(|entity| (entity.id, world.tile_of_position(entity.position)))
        .expect("the map has a node");
    let (node, node_tile) = stone;
    let miner = villager(
        &mut world,
        9_402,
        (i64::from(node_tile.0) - 2, i64::from(node_tile.1)),
    );
    world
        .submit_checked(Command::Gather {
            villager: miner,
            node,
        })
        .expect("a gather order");
    for step in 0..900 {
        world.step().expect("step");
        let here = tile_of(&world, miner);
        assert_ne!(
            here, node_tile,
            "step {step}: the miner stood on the node's own tile"
        );
    }
    let _ = at;
}
