//! **Gates** (D153/D154): a building that opens for anyone when an ally comes near, and closes only when nothing stands in
//! it.
//!
//! The owner's rules, in his words: *trigger radius — let's try 1* and *enemy standing in open gate will keep the gate open*.
//! The game's own rule, from its manual and the wiki: a gate *"opens automatically if a friendly unit approaches … and only
//! closes again if no unit stands right under the Gate"*, so *"enemy units can also pass or keep the Gate open on purpose"*.
//!
//! So a gate is **not** per-player passability (D54's premise, corrected): open means walkable for everyone, closed means
//! walkable for nobody. Written before the behaviour exists (D7, SPEC §11).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Sim as Settings, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

mod common;

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
            "load_map_file",
            ruage_data::load_map_file(&repo("config/map.toml"), &resources),
        ),
        economy: common::present(&repo("config/economy.toml"), ruage_data::load_economy_file),
        units: common::must("load_units_file", ruage_data::load_units_file(&units)),
        resources,
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

/// A unit of `player` standing on `tile`, at a walking speed, with a plain unit.
fn unit(world: &mut World, player: u8, id: u32, tile: (i64, i64)) -> EntityId {
    let scale = world.scale();
    let villager = world.role_unit("villager").expect("villager");
    let id = EntityId(id);
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: (
                Fixed::from_raw(tile.0 * scale),
                Fixed::from_raw(tile.1 * scale),
            ),
            speed: Fixed::ZERO,
            hp: 25,
            amount: 0,
        })
        .expect("spawn a unit");
    id
}

/// A unit that can actually walk: the same spawn as `unit`, with a speed. A gate's close rule is about where units *go*,
/// and a stationary fixture proved nothing - the first version of the closing test left its walker standing beside the gate.
fn walker_unit(world: &mut World, player: u8, id: u32, tile: (i64, i64)) -> EntityId {
    let scale = world.scale();
    let villager = world.role_unit("villager").expect("villager");
    let id = EntityId(id);
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: (
                Fixed::from_raw(tile.0 * scale),
                Fixed::from_raw(tile.1 * scale),
            ),
            speed: Fixed::parse("1.0", scale).expect("a walkable speed"),
            hp: 25,
            amount: 0,
        })
        .expect("spawn a walking unit");
    id
}

/// A gate standing on `tile`, finished, owned by `player`.
fn gate(world: &mut World, player: u8, tile: (i64, i64), id: u32) -> EntityId {
    let scale = world.scale();
    let gate = world.role_unit("gate").expect("the pack names a gate role");
    let id = EntityId(id);
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind: EntityKind::Building,
            unit: Some(gate),
            position: (
                Fixed::from_raw(tile.0 * scale),
                Fixed::from_raw(tile.1 * scale),
            ),
            speed: Fixed::ZERO,
            hp: 1_375,
            amount: 0,
        })
        .expect("a gate can be placed");
    id
}

/// Whether a unit may walk on this tile: the **walk grid**, not the placement predicate. `is_free_ground` also refuses a
/// tile an entity's anchor stands on, which is the right question for placing a building and the wrong one here - a gate is
/// standing on its own tiles by definition.
fn walkable(world: &World, tile: (u16, u16)) -> bool {
    world.tile_walkable(tile.0, tile.1)
}

/// An empty patch of the project map, away from the seeded base at (16, 16).
fn open_patch(world: &World, width: i64, height: i64) -> Option<(i64, i64)> {
    for y in 26..56i64 {
        for x in 4..50i64 {
            let clear = (0..height)
                .all(|dy| (0..width).all(|dx| walkable(world, ((x + dx) as u16, (y + dy) as u16))));
            if clear {
                return Some((x, y));
            }
        }
    }
    None
}

/// **A gate that nobody has opened is solid.**
#[test]
fn a_closed_gate_blocks_the_way() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let Some(at) = open_patch(&world, 4, 2) else {
        return;
    };
    gate(&mut world, 1, at, 9_000);
    world.step().expect("step");
    assert!(
        !walkable(&world, (at.0 as u16, at.1 as u16)),
        "a closed gate must not be walkable"
    );
}

/// **An allied unit within one tile opens it** (the owner's radius: 1).
#[test]
fn an_allied_unit_within_one_tile_opens_the_gate() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let Some(at) = open_patch(&world, 4, 2) else {
        return;
    };
    gate(&mut world, 1, at, 9_000);
    world.step().expect("step");
    assert!(
        !walkable(&world, (at.0 as u16, at.1 as u16)),
        "it starts closed"
    );

    // Two tiles away: nothing happens - the radius is one.
    unit(&mut world, 1, 9_010, (at.0 - 2, at.1));
    world.step().expect("step");
    assert!(
        !walkable(&world, (at.0 as u16, at.1 as u16)),
        "a unit two tiles away must not open the gate"
    );

    // One tile away: it opens.
    unit(&mut world, 1, 9_011, (at.0 - 1, at.1));
    world.step().expect("step");
    assert!(
        walkable(&world, (at.0 as u16, at.1 as u16)),
        "an allied unit one tile away must open the gate"
    );
}

/// **An enemy standing in the gate keeps it open** (the owner's rule, and the game's: "enemy units can also pass or keep
/// the Gate open on purpose").
#[test]
fn an_enemy_standing_in_the_gate_keeps_it_open() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let Some(at) = open_patch(&world, 4, 2) else {
        return;
    };
    gate(&mut world, 1, at, 9_000);
    let _enemy = unit(&mut world, 2, 9_020, at);
    for _ in 0..10 {
        world.step().expect("step");
    }
    assert!(
        walkable(&world, (at.0 as u16, at.1 as u16)),
        "an enemy standing in the gate must keep it open"
    );
}

/// **And it closes again when the last unit leaves** - no ally nearby, nothing inside.
#[test]
fn the_gate_closes_when_the_last_unit_leaves() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let Some(at) = open_patch(&world, 4, 2) else {
        return;
    };
    gate(&mut world, 1, at, 9_000);
    let walker = walker_unit(&mut world, 1, 9_010, (at.0 - 1, at.1));
    for _ in 0..5 {
        world.step().expect("step");
    }
    assert!(
        walkable(&world, (at.0 as u16, at.1 as u16)),
        "an ally beside the gate opened it"
    );

    // Send it away and let it leave: the gate must close behind it.
    world
        .submit_checked(Command::Move {
            unit: walker,
            to: (
                u16::try_from(at.0 + 6).expect("in range"),
                u16::try_from(at.1).expect("in range"),
            ),
        })
        .expect("a move away");
    for _ in 0..1_600 {
        world.step().expect("step");
    }
    assert!(
        !walkable(&world, (at.0 as u16, at.1 as u16)),
        "with nothing within one tile and nothing inside, the gate must close"
    );
}
