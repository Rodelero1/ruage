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

#[allow(dead_code)] // a shared test helper that not every test in the file uses
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
#[allow(dead_code)] // a shared test helper that not every test in the file uses
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

/// A villager sent to a tree at the edge of the map's forest **gathers wood**.
///
/// This is the fault the acceptance match died of, in one assertion: the forest tiles are `F` terrain, so a villager
/// cannot stand on the tree's own tile. It walks up beside the tree, `at_node` is false for ever - the check demanded
/// the exact node position - and it stands there for the rest of the match with an empty hand. The dump at 1,400 s shows
/// exactly that: three villagers parked at x = 19.4 beside trees at x = 20, and 8,500 of the map's 9,000 wood untouched.
#[test]
fn a_woodcutter_works_the_edge_of_the_forest() {
    let Some(mut world) = fresh() else {
        return;
    };
    let wood = world.slot_of("wood").expect("a wood column");
    // The map's own forest starts at x = 20; take the tree nearest the town centre and send the first villager at it.
    let villager = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && matches!(entity.kind, EntityKind::Unit))
        .map(|entity| entity.id)
        .expect("a villager");
    let from = world.position(villager).expect("a position");
    let tree = world
        .entities()
        .iter()
        .filter(|entity| entity.kind == EntityKind::Resource(wood))
        .min_by_key(|entity| {
            let dx = entity.position.0.raw() - from.0.raw();
            let dy = entity.position.1.raw() - from.1.raw();
            dx * dx + dy * dy
        })
        .map(|entity| entity.id)
        .expect("a tree");
    world
        .submit_checked(Command::Gather {
            villager,
            node: tree,
        })
        .expect("the nearest tree can be ordered");

    for _ in 0..1_200 {
        world.step().expect("stepping");
    }
    let stock = world.stock(1, wood);
    assert!(
        stock > 0,
        "the villager should be chopping the tree it was sent to, not standing beside it: wood {stock}"
    );
}
