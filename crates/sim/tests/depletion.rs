//! What a tree leaves behind when its wood is gone - the owner's question (2026-10-29).
//!
//! In the game a tree is an obstacle while it stands, and the tile it stood on is **walkable once it is chopped**: the
//! forest edge recedes and villagers can walk through the gap. This file states both rules, and the first one is parked
//! with `#[ignore]` because the engine does not do it yet - which is the point of writing it down: a parked test that is
//! *run* says exactly how far off we are (`cargo test -- --ignored`).

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

/// A tree on **open ground**, so nothing about the answer depends on the map's forest terrain: a tree the villager can
/// reach, with one tick's worth of wood in it.
fn a_small_tree(world: &mut World, at: (i32, i32)) -> EntityId {
    let wood = world.slot_of("wood").expect("a wood column");
    let _ = wood;
    let id = next_id(world);
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id,
            player: 0,
            kind: EntityKind::Resource(wood),
            unit: None,
            position: (
                Fixed::from_raw(i64::from(at.0) * scale),
                Fixed::from_raw(i64::from(at.1) * scale),
            ),
            speed: Fixed::ZERO,
            // A tree is a thing that must be **felled**: its hit points are what make it an obstacle as well as a source
            // (the dat gives a forest tree 20). A fixture that spawned a tree with none was asking the engine about a
            // node that stands nowhere.
            hp: 20,
            // Enough for one trip by the villager's cap: the tree must run out inside the test's lifetime.
            amount: 10,
        })
        .expect("a tree can be placed");
    world.step().expect("one tick");
    id
}

fn a_villager_near(world: &mut World, at: (i32, i32)) -> EntityId {
    let villager = world.role_unit("villager").expect("a villager role");
    let pace = world
        .entities()
        .iter()
        .find(|entity| {
            entity.player == 1 && entity.unit.is_some_and(|u| world.is_family(u, villager))
        })
        .map(|entity| entity.speed)
        .expect("a villager with a speed");
    let id = next_id(world);
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: (
                Fixed::from_raw(i64::from(at.0) * scale),
                Fixed::from_raw(i64::from(at.1) * scale),
            ),
            speed: pace,
            hp: 25,
            amount: 0,
        })
        .expect("a villager can be placed");
    world.step().expect("one tick");
    id
}

/// A tree that still holds wood is an obstacle: a unit walks around it, not through it.
///
/// *This one is expected to pass for the wrong reason on the current map* - the trees stand on `F` terrain, which the
/// walk grid blocks by itself - so it is written on **open ground** to ask the engine the real question: does a
/// **resource entity** block its own tile? It does not, which is the second half of why a chopped tree leaves no gap:
/// there was never a wall there in the first place.
#[test]

fn a_standing_tree_blocks_its_tile() {
    let Some(mut world) = fresh() else {
        return;
    };
    let tree = a_small_tree(&mut world, (30, 30));
    let entity = world.entity(tree).expect("the tree exists");
    assert!(
        world.entity(tree).is_some_and(|e| e.amount.raw() > 0),
        "the tree starts with wood: {:?}",
        entity.amount.raw()
    );
    assert!(
        !world.tile_walkable(30, 30),
        "a tree should block the tile it stands on"
    );
}

/// Once a tree is chopped there is nothing left there: the entity is gone and its tile can be walked through.
#[test]

fn a_chopped_tree_leaves_no_obstacle() {
    let Some(mut world) = fresh() else {
        return;
    };
    let tree = a_small_tree(&mut world, (30, 30));
    let villager = a_villager_near(&mut world, (29, 30));
    world
        .submit_checked(Command::Gather {
            villager,
            node: tree,
        })
        .expect("the villager takes the tree");

    let mut emptied = false;
    for _ in 0..900 {
        world.step().expect("stepping");
        let spent = world
            .entity(tree)
            .is_none_or(|entity| entity.amount.raw() <= 0);
        if spent {
            emptied = true;
            break;
        }
    }
    assert!(
        emptied,
        "the tree should have been gathered out inside 45 s"
    );
    // Removal waits until nobody is working the node - a worker must not lose the tree it is carrying from - so give the
    // engine the few ticks that takes rather than demanding it the instant the wood runs out.
    for _ in 0..1_200 {
        if world.entity(tree).is_none() {
            break;
        }
        world.step().expect("stepping");
    }
    assert!(
        world.entity(tree).is_none(),
        "a spent tree is gone from the world, not left standing as an invisible wall"
    );
    assert!(
        world.tile_walkable(30, 30),
        "the tile a chopped tree stood on can be walked through"
    );
}
