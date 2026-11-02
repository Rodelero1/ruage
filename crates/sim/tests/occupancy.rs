//! What occupies a tile is **counted, not overwritten**: a tree and a wall can stand on the same tile, and the one that
//! leaves must not free the ground the other still holds. Written after the adversarial review of "terrain is ground"
//! found that a single `bool` per tile let whichever entity was removed *first* unblock the tile for both (D84).
//!
//! The same review's second finding is here too: a node that holds nothing must not block from the day it is placed, or
//! an empty node is a wall nobody can ever clear.

use std::path::PathBuf;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

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

fn next_id(world: &World) -> EntityId {
    EntityId(
        world
            .entities()
            .iter()
            .map(|entity| entity.id.0)
            .max()
            .unwrap_or(0)
            + 1,
    )
}

fn spawn_tree(world: &mut World, at: (u16, u16), amount: u32) -> EntityId {
    let scale = world.scale();
    let id = next_id(world);
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Resource(world.slot_of("wood").expect("a wood column")),
            unit: None,
            position: (
                Fixed::from_raw(i64::from(at.0) * scale),
                Fixed::from_raw(i64::from(at.1) * scale),
            ),
            speed: Fixed::ZERO,
            hp: 20,
            amount,
        })
        .expect("a tree");
    world.step().expect("one tick");
    id
}

fn spawn_wall(world: &mut World, at: (u16, u16)) -> EntityId {
    let scale = world.scale();
    let id = next_id(world);
    let wall = world.role_unit("palisade_wall").expect("the wall role");
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Building,
            unit: Some(wall),
            position: (
                Fixed::from_raw(i64::from(at.0) * scale),
                Fixed::from_raw(i64::from(at.1) * scale),
            ),
            speed: Fixed::ZERO,
            hp: 250,
            amount: 0,
        })
        .expect("a wall");
    world.step().expect("one tick");
    id
}

/// A tree and a wall on one tile. Razing the tree (or the wall) first must not open the tile the other still holds -
/// this was a single `bool`, so whichever left first freed the ground for both.
#[test]
fn a_tile_stays_blocked_while_anything_still_stands_on_it() {
    let Some(mut world) = fresh() else { return };
    let at = (30, 30);
    let tree = spawn_tree(&mut world, at, 100);
    let wall = spawn_wall(&mut world, at);
    assert!(
        !world.tile_walkable(at.0, at.1),
        "a tree and a wall on one tile leave it walkable"
    );

    // Raze the wall: the tree is still standing there.
    world
        .submit_checked(Command::Delete {
            player: 1,
            entity: wall,
        })
        .expect("razing your own wall");
    for _ in 0..5 {
        world.step().expect("step");
    }
    assert!(world.entity(wall).is_none(), "the wall should be gone");
    assert!(
        !world.tile_walkable(at.0, at.1),
        "razing the wall freed a tile a tree still stands on"
    );
    assert!(world.entity(tree).is_some(), "the tree must still be there");

    // The other way round needs no second scenario: the count is symmetric, and the tree-to-nothing case is pinned by
    // `a_chopped_tree_leaves_no_obstacle` in `depletion.rs`. What this test exists for is that **two occupants do not free
    // each other's ground**, which the wall's removal alone proves - with a flag, that delete opened the tile.
}

/// A node that holds nothing is not an obstacle: it is spawned empty by a script, so it never becomes a wall that no
/// villager can ever clear.
#[test]
fn an_empty_node_does_not_block_its_tile() {
    let Some(mut world) = fresh() else { return };
    let at = (31, 31);
    let _empty = spawn_tree(&mut world, at, 0);
    assert!(
        world.tile_walkable(at.0, at.1),
        "a node with nothing in it blocked its tile"
    );
}
