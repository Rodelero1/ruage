//! **M3a: the goal is the middle of a tile.**
//!
//! `World::tile()` is a tile's **corner** - the anchor a building's footprint and a spawn position are placed from - and it
//! was also the point a walk aimed at. So every tile order ended at the place where four cells meet: a body stands
//! half a tile off the middle of the tile it was sent to, and when one of those four cells is a wall or a tree, it stands
//! exactly on that obstacle's corner.
//!
//! The owner sees the second half as bodies *in* walls: "villagers went through the house". The first half is subtler and
//! just as visible - click a tile, and everybody piles onto the same corner of it.
//!
//! Written before the implementation (D7). `docs/decisions.md` D114.

use std::path::PathBuf;

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
    map: ruage_data::Map,
    economy: ruage_data::Economy,
    resources: ruage_data::Resources,
    units: ruage_data::Units,
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
        map: common::present(&repo("config/map.toml"), |path| {
            ruage_data::load_map_file(path, &resources)
        }),
        economy: common::present(&repo("config/economy.toml"), ruage_data::load_economy_file),
        resources,
        units: common::must("load_units_file", ruage_data::load_units_file(&units)),
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

fn fresh_id(world: &World) -> u32 {
    world
        .entities()
        .iter()
        .map(|entity| entity.id.0)
        .max()
        .unwrap_or(0)
        + 1
}

/// An empty rectangle of the project map, clear on its west side, away from the seeded base.
fn open_patch(world: &World, width: i64, height: i64) -> Option<(i64, i64)> {
    for y in 26..56i64 {
        for x in 4..46i64 {
            let free = |tx: i64, ty: i64| {
                u16::try_from(tx)
                    .ok()
                    .zip(u16::try_from(ty).ok())
                    .is_some_and(|(tx, ty)| world.is_free_ground(tx, ty))
            };
            let clear = (0..height).all(|dy| (0..width).all(|dx| free(x + dx, y + dy)));
            let approach = (1..=3).all(|back| {
                world.is_free_ground(
                    u16::try_from(x - back).unwrap_or(0),
                    u16::try_from(y).unwrap_or(0),
                )
            });
            if clear && approach {
                return Some((x, y));
            }
        }
    }
    None
}

fn villager(world: &mut World, tile: (i64, i64)) -> EntityId {
    let scale = world.scale();
    let id = EntityId(fresh_id(world));
    let unit = world.role_unit("villager").expect("villager");
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
            speed: Fixed::parse("1.0", scale).expect("a walkable speed"),
            hp: 25,
            amount: 0,
        })
        .expect("spawn villager");
    world.step().expect("step");
    id
}

fn building(world: &mut World, player: u8, role: &str, tile: (i64, i64)) -> EntityId {
    let scale = world.scale();
    let id = EntityId(fresh_id(world));
    let unit = world.role_unit(role).unwrap_or_else(|| panic!("{role}"));
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
            hp: 550,
            amount: 0,
        })
        .expect("spawn building");
    world.step().expect("step");
    id
}

fn position(world: &World, id: EntityId) -> (i64, i64) {
    let entity = world.entity(id).expect("an entity with a position");
    (entity.position.0.raw(), entity.position.1.raw())
}

/// **A walk to a tile ends in the middle of that tile.** Clicking a tile means the middle of it: a body that stops on the
/// corner is half a tile off where the player pointed, which is the difference between "the villager went to the tree" and
/// "the villager went to the corner next to the tree".
#[test]
fn a_walk_to_a_tile_ends_in_the_middle_of_it() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let Some(at) = open_patch(&world, 8, 4) else {
        return;
    };
    let walker = villager(&mut world, (at.0, at.1 + 1));
    let scale = world.scale();
    let goal = (at.0 + 5, at.1 + 1);

    world
        .submit_checked(Command::Move {
            unit: walker,
            to: (
                u16::try_from(goal.0).expect("in range"),
                u16::try_from(goal.1).expect("in range"),
            ),
        })
        .expect("a move across open ground");

    for _ in 0..600 {
        world.step().expect("step");
    }
    let (x, y) = position(&world, walker);
    let (cx, cy) = (goal.0 * scale + scale / 2, goal.1 * scale + scale / 2);
    let (off_x, off_y) = ((x - cx).abs(), (y - cy).abs());
    assert!(
        off_x <= scale / 4 && off_y <= scale / 4,
        "the walker stopped {off_x},{off_y} raw units from the middle of tile {goal:?} (a quarter tile is {})",
        scale / 4
    );
}

/// **A walk to a tile that touches a building does not end on the building's corner.** The tile's own corner is shared
/// with its neighbours; aiming at it puts the body exactly on the wall's corner, which is what a player calls standing
/// inside the building.
#[test]
fn a_walk_to_a_tile_beside_a_house_does_not_end_on_the_house() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let Some(at) = open_patch(&world, 9, 5) else {
        return;
    };
    building(&mut world, 1, "house", at);
    // One tile east of the house's east column: the goal tile's corner is shared with the house.
    let goal = (at.0 + 3, at.1);
    let walker = villager(&mut world, (at.0 - 2, at.1 + 2));
    let scale = world.scale();

    world
        .submit_checked(Command::Move {
            unit: walker,
            to: (
                u16::try_from(goal.0).expect("in range"),
                u16::try_from(goal.1).expect("in range"),
            ),
        })
        .expect("a move beside a house");

    for _ in 0..600 {
        world.step().expect("step");
    }
    let (x, y) = position(&world, walker);
    // Distance to the house tile that shares the goal's corner, as a square (a disc of any radius has to clear it).
    let (hx, hy) = ((goal.0 - 1) * scale, goal.1 * scale);
    let dx = (hx - x).max(0).max(x - (hx + scale));
    let dy = (hy - y).max(0).max(y - (hy + scale));
    let squared = dx * dx + dy * dy;
    assert!(
        squared >= (scale / 2) * (scale / 2),
        "the walker stopped {} raw units from the house's tile - on its corner, not in the middle of the tile it was sent to",
        (squared as f64).sqrt() as i64
    );
}
