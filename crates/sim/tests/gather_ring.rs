//! **A gatherer stands on a ring 1.156 tiles out, not 0.70** (D186).
//!
//! Measured, not reasoned. The owner's save (`owner-test2.aoe2spgame`, VER 9.4 / 12.5) carries a per-unit order queue at
//! `0x32ceb4` whose *from* points are standing positions. Six woodcutters round one tree sit at **1.105, 1.142, 1.160,
//! 1.170, 1.170, 1.189** tiles from the source square's centre - mean **1.156**, spread **0.084** - and the three closest
//! pairs are **0.404, 0.455, 0.710** apart, the first of which is exactly two villager radii (2 x 0.2): body touching body.
//! **Nine men a square** is the owner's number - *"max 9 per each. 4 on corners and 4 on sides"* - and the file agrees: the
//! sixteen orders over his two adjacent trees came out as **nine and seven**. Eighteen was this reviewer's misreading of the
//! ring's circumference (`2 * pi * 1.156 / 0.404 = 17.97`) as the game's capacity, which the 9 + 7 split refutes.
//!
//! The engine's standoff was `radius[0] + node radius` - 0.2 + 0.5 = **0.70** (D169) - one man per side of the square
//! (D182). That is nearly half a tile inside where the game puts its front row, which is why the owner's photograph and
//! our render do not look alike.
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

/// **The single worker stands on the measured ring**, 1.156 of a tile from the source square's centre - not the 0.70
/// the two bodies' radii add up to.
#[test]
fn a_woodcutter_stands_on_the_measured_ring() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let scale = world.scale();
    let node = the_tree(&world);
    let tree = world.position(node).expect("the tree's position");
    let ids = woodcutters(&mut world, 1);
    for _ in 0..600 {
        world.step().expect("a tick");
    }
    let worker = world.position(ids[0]).expect("the woodcutter");
    let distance = tiles_between(worker, tree, scale);
    assert!(
        (1.05..=1.27).contains(&distance),
        "MEASURED the woodcutter at {distance:.3} tiles from the tree; the ring is 1.156 and D169's standoff was 0.70"
    );
}

/// **Two workers on one tree stand a body apart on the same ring** - 0.404 in the save, which is exactly two radii.
#[test]
fn two_woodcutters_on_one_tree_stand_a_body_apart_on_the_ring() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let scale = world.scale();
    let node = the_tree(&world);
    let tree = world.position(node).expect("the tree's position");
    let ids = woodcutters(&mut world, 2);
    for _ in 0..600 {
        world.step().expect("a tick");
    }
    let first = world.position(ids[0]).expect("the first");
    let second = world.position(ids[1]).expect("the second");
    let (a, b) = (
        tiles_between(first, tree, scale),
        tiles_between(second, tree, scale),
    );
    let apart = tiles_between(first, second, scale);
    assert!(
        (1.05..=1.27).contains(&a) && (1.05..=1.27).contains(&b),
        "MEASURED radii {a:.3} and {b:.3}; both belong on the 1.156 ring"
    );
    assert!(
        apart >= 0.39,
        "MEASURED {apart:.3} tiles between the two; two villager bodies are 0.40 wide"
    );
}

/// **The ring holds nine men**, and the tenth is left standing - the owner's *"max 9 per each"*, and the 9 + 7 split his
/// save names over two adjacent trees.
#[test]
fn the_ring_holds_nine_woodcutters_and_the_tenth_takes_nothing() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let scale = world.scale();
    let node = the_tree(&world);
    let tree = world.position(node).expect("the tree's position");
    let ids = woodcutters(&mut world, 10);
    let mut ever_carried = vec![false; ids.len()];
    for _ in 0..2_000 {
        world.step().expect("a tick");
        for (index, id) in ids.iter().enumerate() {
            if world
                .entity(*id)
                .is_some_and(|entity| entity.carry.raw() > 0)
            {
                ever_carried[index] = true;
            }
        }
    }
    let working = ever_carried.iter().filter(|carried| **carried).count();
    let standing: Vec<(usize, Fixed, Fixed)> = ids
        .iter()
        .enumerate()
        .filter(|(index, _)| !ever_carried[*index])
        .filter_map(|(index, id)| world.position(*id).map(|at| (index, at.0, at.1)))
        .collect();
    assert_eq!(
        working, 9,
        "MEASURED {working} of 10 woodcutters took wood from one tree; the owner's cap is nine a square, and his save's own \
         group is 9 + 7 over two squares"
    );
    assert!(
        !standing.is_empty(),
        "10 woodcutters and a ring of 9: at least one must be left out, but all 10 carried"
    );
    for (index, _, _) in &standing {
        let at = world.position(ids[*index]).expect("a position");
        let distance = tiles_between(at, tree, scale);
        assert!(
            distance > 1.27,
            "MEASURED the standing woodcutter {index} at {distance:.3} tiles from the tree, inside the ring the working \
             men occupy at {scale} raw units a tile"
        );
    }
}
