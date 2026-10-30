//! A forest is a block of trees, and only the trees on its **edge** can be worked: a villager stands on the tree's own
//! tile, and the middle of a 3x3 block has no free neighbour to step in from. In the game the outer trees are chopped
//! first and the forest recedes; a chooser that picks by distance alone walks into the block and gives up.
//!
//! Written before the fix (D7): this is the shape of the acceptance match's wood collapse - 8,500 of 9,000 wood left on
//! the map with 32 villagers alive.

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

fn place(
    world: &mut World,
    kind: EntityKind,
    unit: Option<u32>,
    at: (u16, u16),
    amount: u32,
) -> EntityId {
    let id = next_id(world);
    let scale = world.scale();
    let pace = Fixed::from_raw(scale / 2);
    world
        .submit_checked(Command::Spawn {
            id,
            player: if kind == EntityKind::Unit { 1 } else { 0 },
            kind,
            unit,
            position: (
                Fixed::from_raw(i64::from(at.0) * scale),
                Fixed::from_raw(i64::from(at.1) * scale),
            ),
            speed: if kind == EntityKind::Unit {
                pace
            } else {
                Fixed::ZERO
            },
            hp: if kind == EntityKind::Unit { 25 } else { 0 },
            amount,
        })
        .expect("placed");
    world.step().expect("one tick");
    id
}

/// A villager told to gather wood in a forest block gathers wood: it works the edge, because the middle cannot be
/// entered. The villager is nearest the **centre** tree, so a chooser that only measures distance picks the one node
/// that cannot be worked.
#[test]
fn a_woodcutter_works_the_edge_of_the_forest() {
    let Some(mut world) = fresh() else {
        return;
    };
    let wood = world.slot_of("wood").expect("a wood column");
    // A 3x3 block of trees at (40..42, 40..42), on open grass, and a villager beside its middle tree.
    let mut block = Vec::new();
    for x in 40..=42u16 {
        for y in 40..=42u16 {
            block.push(place(
                &mut world,
                EntityKind::Resource(wood),
                None,
                (x, y),
                100,
            ));
        }
    }
    let villager_unit = world.role_unit("villager").expect("a villager role");
    // Due east of the middle tree, so the middle tree is the nearest node by a clear margin.
    let villager = place(
        &mut world,
        EntityKind::Unit,
        Some(villager_unit),
        (44, 41),
        0,
    );

    // The middle of the block cannot be worked at all: nothing free to step in from.
    let middle = block[4];
    assert!(
        world
            .submit_checked(Command::Gather {
                villager,
                node: middle,
            })
            .is_err(),
        "the middle of a forest block is not workable"
    );

    // Once the villager is free to choose (the share table wants wood), it has to end up on an edge tree and the wood
    // has to move. One second of sharing, then enough time to walk and chop.
    world.set_villager_shares(&[(0, vec![(wood, 100)])]);
    for _ in 0..1_200 {
        world.step().expect("stepping");
    }
    let stock = world.stock(1, wood);
    assert!(
        stock > 0,
        "the villager should be chopping the edge of the forest, not standing in it: wood {stock}"
    );
}
