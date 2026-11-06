//! What occupies a tile is **counted, not overwritten**: a tree and a wall can stand on the same tile, and the one that
//! leaves must not free the ground the other still holds. Written after the adversarial review of "terrain is ground"
//! found that a single `bool` per tile let whichever entity was removed *first* unblock the tile for both (D84).
//!
//! The same review's second finding is here too: a node that holds nothing must not block from the day it is placed, or
//! an empty node is a wall nobody can ever clear.

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

mod common;

fn fresh() -> Option<World> {
    common::fresh()
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

/// **A building that appears pushes a unit off its ground.** `Command::Spawn` inserted a building over whatever stood
/// there, and a villager left inside a blocked footprint can never plan a route again - A* refuses to start on a blocked
/// tile - so it stands there for the rest of the match. Six `research.rs` tests were red on exactly this shape: a barracks
/// spawned at (14, 14) swallowed the three starting villagers, and every later path search failed from a blocked start.
/// It is the same family as the farm husk: a command-made entity was not quite a map-made one.
#[test]
fn a_spawned_building_pushes_a_unit_off_its_ground() {
    let Some(mut world) = fresh() else { return };
    let villager = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
        .map(|entity| entity.id)
        .expect("a villager");
    let here = world.position(villager).expect("a position");
    let scale = world.scale();
    let anchor = (
        u16::try_from(here.0.raw() / scale).expect("in range"),
        u16::try_from(here.1.raw() / scale).expect("in range"),
    );
    let barracks = world.role_unit("barracks").expect("a barracks role");
    let (across, down) = world.footprint_of(barracks).expect("a footprint");

    let id = next_id(&world);
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Building,
            unit: Some(barracks),
            position: here,
            speed: Fixed::ZERO,
            hp: 1200,
            amount: 0,
        })
        .expect("a barracks on top of the villager");
    world.step().expect("one tick");

    let now = world
        .position(villager)
        .expect("the villager is still alive");
    let tile = (
        u16::try_from(now.0.raw() / scale).expect("in range"),
        u16::try_from(now.1.raw() / scale).expect("in range"),
    );
    let inside = tile.0 >= anchor.0
        && tile.0 < anchor.0 + across as u16
        && tile.1 >= anchor.1
        && tile.1 < anchor.1 + down as u16;
    assert!(
        !inside,
        "the villager was left inside the spawned building's footprint at {tile:?}"
    );
    assert!(
        world.tile_walkable(tile.0, tile.1),
        "and it stands on walkable ground, or no route can ever start"
    );
}

/// **"Beside the building" is about the building's ground, not its anchor tile.** The anchor is a blocked tile in the
/// middle of the diamond: a villager handing in from a town centre's yard is two tiles from it and stands *on* the
/// building. Measuring from the anchor meant the yard counted as "far away", which is the second half of the reason
/// villagers stood holding loads.
#[test]
fn a_villager_in_the_yard_is_on_the_town_centre() {
    let Some(world) = fresh() else { return };
    let centre = world
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Building && entity.player == 1)
        .map(|entity| entity.id)
        .expect("a town centre");
    let scale = world.scale();
    let at = |x: i64, y: i64| (Fixed::from_raw(x * scale), Fixed::from_raw(y * scale));
    assert!(
        world.touches_footprint(at(16, 16), centre),
        "the anchor itself"
    );
    assert!(
        world.touches_footprint(at(18, 18), centre),
        "the far corner of the yard: two tiles from the anchor, and on the building"
    );
    assert!(
        !world.touches_footprint(at(24, 24), centre),
        "eight tiles away is not the town centre"
    );
}
