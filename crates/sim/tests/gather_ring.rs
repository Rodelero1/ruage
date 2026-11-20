//! **A gatherer stands with his circle on the node's tile** (D192; the 1.156 ring it replaced is D186's, and both came from the
//! owner's save).
//!
//! Measured, not reasoned. The owner's save (`owner-test2.aoe2spgame`, VER 9.4 / 12.5) carries a per-unit order queue at
//! `0x32ceb4` whose *from* points are standing positions. Six woodcutters round one tree sit at **1.105, 1.142, 1.160,
//! 1.170, 1.170, 1.189** tiles from the source square's centre - mean **1.156**, spread **0.084** - and the three closest
//! pairs are **0.404, 0.455, 0.710** apart, the first of which is exactly two villager radii (2 x 0.2): body touching body.
//! Those radii are D186's ring, which this file still names where the history matters; the **rule** is D192's - a man's circle
//! touches the tile, which is **0.70** from its centre for a villager and **0.99** where two edges meet, and the ring was the row
//! *behind* the men the owner was looking at.
//!
//! **Nine men a square** is the owner's number from the ring days - *"max 9 per each"* - and D188 dropped every cap: the capacity is
//! the geometry (`2 * pi * 1.156 / 0.404 = 17.97` on a ring, eleven places on a tile's border), never a table.
//! Written before the change (D7, SPEC §11).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Sim as Settings, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

mod common;

/// **How far a body is from the node's tile**, in tiles: his radius (0.20) when he is touching it, which is D192's rule.
fn clearance(world: &World, node: EntityId, position: (Fixed, Fixed)) -> f64 {
    let scale = world.scale();
    world
        .resource_tiles(node)
        .into_iter()
        .map(|(x, y)| {
            ruage_sim::tiles::distance_to_tile(position, (i64::from(x), i64::from(y)), scale) as f64
                / scale as f64
        })
        .fold(f64::MAX, f64::min)
}

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// One tree at (10,10) with room on every side, a source that never runs dry, and the player's town centre well clear.
const MAP: &str = r#"
schema = 1
[map]
width = 22
height = 22
terrain = [
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGFGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
]
[[map.resource]]
kind = "wood"
amount = 50000
hp = 0
tiles = [[10,10]]
[[map.town_center]]
player = 1
x = 1
y = 1
[[map.town_center]]
player = 2
x = 20
y = 20
"#;

fn world_of(map_text: &str) -> Option<World> {
    let units_file = repo("config/gamedata/units.toml");
    if !units_file.exists() {
        return None;
    }
    let resources = common::present(
        &repo("config/resources.toml"),
        ruage_data::load_resources_file,
    );
    let settings: Settings = common::present(&repo("config/sim.toml"), ruage_data::load_sim_file);
    let map: Map = common::must(
        "load_map_str",
        ruage_data::load_map_str(map_text, "ring.toml", &resources),
    );
    let economy: Economy =
        common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let units: Units = common::must("load_units_file", ruage_data::load_units_file(&units_file));
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
    Some(world)
}

fn the_tree(world: &World) -> EntityId {
    world
        .entities()
        .iter()
        .find(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .map(|entity| entity.id)
        .expect("a tree at (10,10)")
}

/// Drop `count` villagers on the tiles ringing the tree, three to five tiles out on every side, and order them all to it.
fn woodcutters(world: &mut World, count: usize) -> Vec<EntityId> {
    let scale = world.scale();
    let villager = world.role_unit("villager").expect("villager");
    let node = the_tree(world);
    let ring: Vec<(i64, i64)> = (-5i64..=5)
        .flat_map(|dy| (-5i64..=5).map(move |dx| (dx, dy)))
        .filter(|(dx, dy)| {
            let far = (*dx).abs().max((*dy).abs());
            (3..=5).contains(&far)
        })
        .collect();
    let mut ids = Vec::new();
    for (index, (dx, dy)) in ring.iter().take(count).enumerate() {
        let id = EntityId(8_000 + index as u32);
        world
            .submit_checked(Command::Spawn {
                id,
                player: 1,
                kind: EntityKind::Unit,
                unit: Some(villager),
                position: (
                    Fixed::from_raw((10 + dx) * scale + scale / 2),
                    Fixed::from_raw((10 + dy) * scale + scale / 2),
                ),
                speed: Fixed::parse("0.8", scale).expect("a speed"),
                hp: 25,
                amount: 0,
            })
            .expect("a villager");
        world.step().expect("the villager lands");
        ids.push(id);
    }
    for id in &ids {
        world
            .submit_checked(Command::Gather {
                villager: *id,
                node,
            })
            .expect("a gather order");
    }
    ids
}

fn tiles_between(a: (Fixed, Fixed), b: (Fixed, Fixed), scale: i64) -> f64 {
    let dx = (a.0.raw() - b.0.raw()) as f64 / scale as f64;
    let dy = (a.1.raw() - b.1.raw()) as f64 / scale as f64;
    (dx * dx + dy * dy).sqrt()
}

/// **The single worker stands with his circle touching the tree** (D192), 0.20 of a tile from its border - which is 0.70 from the
/// tile's centre for a villager. The engine used to put him 1.156 out, on the row behind.
#[test]
fn a_woodcutter_stands_with_his_circle_on_the_tree() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let node = the_tree(&world);
    let ids = woodcutters(&mut world, 1);
    for _ in 0..600 {
        world.step().expect("a tick");
    }
    let worker = world.position(ids[0]).expect("the woodcutter");
    let distance = clearance(&world, node, worker);
    assert!(
        (0.15..=0.30).contains(&distance),
        "MEASURED the woodcutter {distance:.3} tiles from the tree's border; his circle is 0.20 and must touch it"
    );
}

/// **Two workers on one tree both touch it and stand a body apart** - 0.404 apart in the save, which is exactly two radii.
#[test]
fn two_woodcutters_on_one_tree_both_touch_it_and_press_together() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let scale = world.scale();
    let node = the_tree(&world);
    let ids = woodcutters(&mut world, 2);
    for _ in 0..600 {
        world.step().expect("a tick");
    }
    let first = world.position(ids[0]).expect("the first");
    let second = world.position(ids[1]).expect("the second");
    let (a, b) = (
        clearance(&world, node, first),
        clearance(&world, node, second),
    );
    let apart = tiles_between(first, second, scale);
    assert!(
        (0.15..=0.30).contains(&a) && (0.15..=0.30).contains(&b),
        "MEASURED radii {a:.3} and {b:.3}; both belong on the 1.156 ring"
    );
    // **They press** (D293): the seat book that kept them a body apart is gone, so two men on one tree stand pressed - measured
    // **0.000** tiles apart - and both still touch it. The old claim was the book's, not the game's.
    assert!(
        apart < 0.39,
        "MEASURED {apart:.3} tiles between the two: the seat book is gone, so two men on one tree must press together"
    );
}

// **Two fixtures used to live here and are gone with their subject** (D293): `the_tile_holds_as_many_woodcutters_as_touch_it`
// asserted that one tile seats exactly `touching_place_count` men at 0.66 spacing, and `the_owners_own_arrangement_lands_on_the_same_ring`
// asserted that the engine reproduces the 1.156-tile ring six men sat on in the owner's save. Both describe the **seat book**, which the
// engine no longer reads: a gatherer now presses in and takes, and the crowd's capacity is what the bodies allow. The claims that
// survive are carried by `gather_press_in` (two men press in), `gather_touching` (ten asked, everyone working stands against the tile)
// and `gather_contact` (the crowd converges), so nothing is left unmeasured - what was measured here was a rule that is gone.
