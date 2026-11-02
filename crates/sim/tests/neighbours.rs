//! A worker stands *beside* what it works, never on top of it - the owner's report from `make run`.
//!
//! "The villagers upon approaching to the berry bush disappeared as if entering a building. They should stand near the
//! bush while foraging." Two things could produce that, and they have different fixes: the simulation could be moving
//! the villager onto the node's own tile (in which case the drawing is right and the order is wrong), or the drawing
//! could be painting the bush over a villager that is correctly beside it. This file settles the first question without
//! a window; the second is `crates/view`'s.

use std::path::PathBuf;

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

fn tile(world: &World, position: (ruage_fixed::Fixed, ruage_fixed::Fixed)) -> (i64, i64) {
    let scale = world.scale();
    (position.0.raw() / scale, position.1.raw() / scale)
}

/// The villager that gathers from a node must end **next to** it: sharing the node's tile is what makes a worker look
/// as though it vanished into the bush.
#[test]
fn a_gatherer_stands_beside_the_node_and_not_on_it() {
    let Some(mut world) = fresh() else { return };

    let villager = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
        .map(|entity| entity.id)
        .expect("player 1 starts with a villager");
    let villager_tile = tile(
        &world,
        world
            .entities()
            .iter()
            .find(|entity| entity.id == villager)
            .expect("the villager")
            .position,
    );
    // The nearest node, so the walk is short and the test is about where it stops rather than how it got there.
    let node: EntityId = world
        .entities()
        .iter()
        .filter(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .min_by_key(|entity| {
            let (x, y) = tile(&world, entity.position);
            (x - villager_tile.0).abs() + (y - villager_tile.1).abs()
        })
        .map(|entity| entity.id)
        .expect("the map has a resource node");
    let node_tile = tile(
        &world,
        world
            .entities()
            .iter()
            .find(|entity| entity.id == node)
            .expect("the node")
            .position,
    );

    world
        .submit_checked(Command::Gather { villager, node })
        .expect("the order is legal");
    for _ in 0..600 {
        world.step().expect("step");
    }

    let (x, y) = tile(
        &world,
        world
            .entities()
            .iter()
            .find(|entity| entity.id == villager)
            .expect("the villager still exists")
            .position,
    );
    let dx = (x - node_tile.0).abs();
    let dy = (y - node_tile.1).abs();
    assert!(
        dx.max(dy) == 1,
        "the gatherer ended at ({x}, {y}) for a node at {node_tile:?}: it must stand beside it, not on it"
    );
}

/// And it is still there to be seen: a gatherer that leaves the visible set is the other way to "disappear".
#[test]
fn a_gatherer_stays_visible_to_its_owner() {
    let Some(world) = fresh() else { return };
    let villager = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
        .map(|entity| entity.id)
        .expect("player 1 starts with a villager");
    assert!(
        world.visible_to(1, villager),
        "a player can always see their own villager"
    );
}
