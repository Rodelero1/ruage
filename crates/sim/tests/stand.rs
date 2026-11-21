//! **Where a man stands to work a node** (D325).
//!
//! The owner's invariant 4 is "a working man does not move", and the ring rules (D186, D293) say a gatherer works a node from a tile beside
//! it. Neither says *which* beside. Ours walks the compass from the north-west and takes the first free, reachable tile - so a man
//! approaching from the east walks round the node to its north-west corner and works it from the far side. Measured here: the man's
//! standing tile is the one his own approach is nearest to.
//!
//! Written before the change (D7, SPEC §11).

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

/// A 24x20 field with **one tree** at (10, 9): its whole ring is free ground, so the stand is a choice and not a constraint.
const MAP: &str = r#"
schema = 1
[map]
width = 24
height = 20
terrain = [
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGFGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
]
[[map.resource]]
kind = "wood"
amount = 100
hp = 0
tiles = [[10, 9]]
[[map.town_center]]
player = 1
x = 1
y = 1
[[map.town_center]]
player = 2
x = 22
y = 18
"#;

fn world_of(map_text: &str) -> World {
    let resources = common::present(
        &repo("config/resources.toml"),
        ruage_data::load_resources_file,
    );
    let settings: Settings = common::present(&repo("config/sim.toml"), ruage_data::load_sim_file);
    let map: Map = common::must(
        "load_map_str",
        ruage_data::load_map_str(map_text, "stand.toml", &resources),
    );
    let economy: Economy =
        common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let units: Units = common::must(
        "load_units_file",
        ruage_data::load_units_file(&repo("config/gamedata/units.toml")),
    );
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
    world
}

fn a_villager(world: &mut World, id: u32, tile: (i64, i64)) -> EntityId {
    let villager = world.role_unit("villager").expect("a villager role");
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id: EntityId(id),
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: (
                Fixed::from_raw(tile.0 * scale + scale / 2),
                Fixed::from_raw(tile.1 * scale + scale / 2),
            ),
            speed: Fixed::parse("0.8", scale).expect("a speed"),
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    world.step().expect("the villager lands");
    EntityId(id)
}

/// **The map's one tree**, found rather than assumed: the loader decides where a resource entity sits inside its tile.
fn the_tree(world: &World) -> (EntityId, (i64, i64)) {
    let scale = world.scale();
    let trees: Vec<&ruage_sim::Entity> = world
        .entities()
        .iter()
        .filter(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .collect();
    assert_eq!(trees.len(), 1, "the fixture has exactly one tree");
    let tree = trees[0];
    (
        tree.id,
        (
            tree.position.0.raw().div_euclid(scale),
            tree.position.1.raw().div_euclid(scale),
        ),
    )
}

fn tile_of(world: &World, id: EntityId) -> (i64, i64) {
    let scale = world.scale();
    let entity = world.entity(id).expect("a body");
    (
        entity.position.0.raw().div_euclid(scale),
        entity.position.1.raw().div_euclid(scale),
    )
}

/// **A man walking at a node stands on the side he came from.** He is four tiles due east of the tree; the ring's north-west tile is
/// the first in compass order and the east tile is the nearest, three tiles closer to him. Every ring tile is free, so nothing but the
/// chooser decides it - and the chooser must send him to the east side.
#[test]
fn a_man_who_comes_from_the_east_works_the_tree_from_the_east() {
    let mut world = world_of(MAP);
    let (tree, (tx, ty)) = the_tree(&world);
    let start = (tx + 4, ty);
    let man = a_villager(&mut world, 9_001, start);
    assert_eq!(
        tile_of(&world, man),
        start,
        "the man stands where he was spawned"
    );

    world
        .submit_checked(Command::Gather {
            villager: man,
            node: tree,
        })
        .expect("a gather on open ground is ordered");

    // Long enough to arrive and start taking, short enough that the tree cannot run out.
    for _ in 0..600 {
        world.step().expect("a tick");
    }

    let standing = tile_of(&world, man);
    assert_eq!(
        standing,
        (tx + 1, ty),
        "the man works the tree from ({tx}+1, {ty}) - the side he walked in from - not from another side of the ring"
    );
    assert!(
        world
            .entity(man)
            .is_some_and(|entity| entity.position.0.raw() > 0),
        "the man is still on the map"
    );
}
