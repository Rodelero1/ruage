//! Ground and arrival: what a tree closes, and where a worker stands.
//!
//! Two of the owner's observations (2026-11-03), written before the implementation (D7):
//!
//! - "Units seem to walk through the forest (each forest tile is blocking passage until tree cut and all wood extracted,
//!   and there is no passage between trees even if touching by corners of the tile)";
//! - "Villagers should stand around the berry bush or stone/gold mine, not on top of it".

use std::path::PathBuf;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

mod common;

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// Nine tiles of grass, two trees touching at a **corner**, a town centre for each side and nothing else: the corner is
/// the whole point, so no other tree may stand in the way of the detour.
const CORNER: &str = r#"
schema = 1
[map]
width = 9
height = 9
terrain = [
  "GGGGGGGGG",
  "GGGGGGGGG",
  "GGGGGGGGG",
  "GGGGGGGGG",
  "GGGGFGGGG",
  "GGGGGFGGG",
  "GGGGGGGGG",
  "GGGGGGGGG",
  "GGGGGGGGG",
]
[[map.resource]]
kind = "wood"
amount = 100
hp = 20
tiles = [[4, 4], [5, 5]]
[[map.town_center]]
player = 1
x = 1
y = 1
[[map.town_center]]
player = 2
x = 7
y = 7
"#;

/// A map with one gold mine - a node with **no hit points**, which is the case the owner saw villagers standing on top of.
const MINE: &str = r#"
schema = 1
[map]
width = 9
height = 9
terrain = [
  "GGGGGGGGG",
  "GGGGGGGGG",
  "GGGGGGGGG",
  "GGGGGGGGG",
  "GGGGGGGGG",
  "GGGGGGGGG",
  "GGGGGGGGG",
  "GGGGGGGGG",
  "GGGGGGGGG",
]
[[map.resource]]
kind = "gold"
amount = 800
hp = 0
tiles = [[4, 4]]
[[map.town_center]]
player = 1
x = 1
y = 1
[[map.town_center]]
player = 2
x = 7
y = 7
"#;

/// A map with one tree and a villager's town centre, for where a worker stands.
const NODE: &str = r#"
schema = 1
[map]
width = 9
height = 9
terrain = [
  "GGGGGGGGG",
  "GGGGGGGGG",
  "GGGGGGGGG",
  "GGGGGGGGG",
  "GGGGFGGGG",
  "GGGGGGGGG",
  "GGGGGGGGG",
  "GGGGGGGGG",
  "GGGGGGGGG",
]
[[map.resource]]
kind = "wood"
amount = 200
hp = 20
tiles = [[4, 4]]
[[map.town_center]]
player = 1
x = 1
y = 1
[[map.town_center]]
player = 2
x = 7
y = 7
"#;

fn loaded(text: &str) -> Option<World> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        return None;
    }
    let resources = common::present(
        &repo("config/resources.toml"),
        ruage_data::load_resources_file,
    );
    let settings = common::present(&repo("config/sim.toml"), ruage_data::load_sim_file);
    let map = common::must(
        "load_map_str",
        ruage_data::load_map_str(text, "fixture.toml", &resources),
    );
    let economy = common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let units = common::must("load_units_file", ruage_data::load_units_file(&units));
    let combat = common::present(&repo("config/combat.toml"), ruage_data::load_combat_file);
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
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

fn spawn_villager(world: &mut World, id: EntityId, x: u16, y: u16) {
    let unit = world.start_villager_unit().expect("a villager role");
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                Fixed::from_raw(i64::from(x) * scale),
                Fixed::from_raw(i64::from(y) * scale),
            ),
            speed: Fixed::from_raw(scale),
            hp: 25,
            amount: 0,
        })
        .expect("a villager can be placed");
}

fn tile_of(world: &World, id: EntityId) -> (i64, i64) {
    let entity = world.entity(id).expect("an entity with a tile");
    (
        entity.position.0.raw().div_euclid(world.scale()),
        entity.position.1.raw().div_euclid(world.scale()),
    )
}

/// **No passage between two trees that touch only at a corner.** The tiles are (4,4) and (5,5); a villager at (4,5) is
/// ordered to (5,4), which is one diagonal step away *through the corner*. It may not take it: the walk is around.
/// The stored route is the evidence: every leg of it certifies clean against the two grown boxes, so the walk never
/// threads the corner point - and it arrives, having walked round instead of through.
#[test]
fn two_trees_touching_at_a_corner_leave_no_gap() {
    let Some(mut world) = loaded(CORNER) else {
        return;
    };
    let walker = next_id(&world);
    spawn_villager(&mut world, walker, 4, 5);
    world.step().expect("the spawn lands");
    world
        .submit_checked(Command::Move {
            target: None,
            unit: walker,
            to: (5, 4),
        })
        .expect("a move through the corner");
    world.step().expect("the order lands");
    // The route the order planned, before the walker spends it: clean against both grown boxes.
    let scale = world.scale();
    let grown: Vec<ruage_sim::Clear> = [(4, 4), (5, 5)]
        .iter()
        .map(|(x, y)| ruage_sim::Clear {
            at: (
                i64::from(*x) * scale + scale / 2,
                i64::from(*y) * scale + scale / 2,
            ),
            half: (scale / 2, scale / 2),
            reach: world.body_radius_of(walker),
        })
        .collect();
    let raw: Vec<(i64, i64)> = world
        .route_points(walker)
        .iter()
        .map(|point| (point.0.raw(), point.1.raw()))
        .collect();
    assert!(!raw.is_empty(), "the order plans a route, not a refusal");
    assert!(
        ruage_sim::verify_path(&raw, &grown).is_none(),
        "the route threads the corner: {raw:?}"
    );

    let mut on_a_tree = Vec::new();
    for tick in 0..600 {
        world.step().expect("step");
        let now = tile_of(&world, walker);
        if now == (4, 4) || now == (5, 5) {
            on_a_tree.push((tick, now));
        }
    }
    assert!(
        on_a_tree.is_empty(),
        "the walker stood on a tree at {on_a_tree:?}"
    );
    assert_eq!(
        tile_of(&world, walker),
        (5, 4),
        "and it still gets there, round the corner"
    );
}

/// **A worker stands *around* a tree, not on it.** The node's own tile is blocked ground; the order ends on the ring, and
/// the ring is enough to take from it (the weather rule of 2026-10-29).
#[test]
fn a_woodcutter_stands_beside_its_tree_and_still_takes() {
    let Some(mut world) = loaded(NODE) else {
        return;
    };
    let tree = world
        .entities()
        .iter()
        .find(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .map(|entity| entity.id)
        .expect("the tree");
    let villager = next_id(&world);
    spawn_villager(&mut world, villager, 2, 4);
    world.step().expect("the spawn lands");
    world
        .submit_checked(Command::Gather {
            villager,
            node: tree,
        })
        .expect("a gather order");

    let mut on_the_tree = Vec::new();
    let mut carried = 0;
    for tick in 0..1_200 {
        world.step().expect("step");
        let now = tile_of(&world, villager);
        if now == (4, 4) {
            on_the_tree.push(tick);
        }
        carried = carried.max(
            world
                .entity(villager)
                .map_or(0, |entity| entity.carry.raw()),
        );
    }
    assert!(
        on_the_tree.is_empty(),
        "the woodcutter stood on the tree on ticks {on_the_tree:?}"
    );
    let (x, y) = tile_of(&world, villager);
    let beside = (x - 4).abs() <= 1 && (y - 4).abs() <= 1;
    assert!(beside, "it works from the ring: {x} {y}");
    assert!(carried > 0, "and it takes wood from there");
}

/// **A mine is an obstacle, not a floor.** A node with no hit points - a gold or stone mine, a berry bush - used to be
/// walked over, so the miner stood on top of it (`beside` only ever fired for a blocked tile). The tile blocks while the
/// node holds anything, and the miner works from the ring.
#[test]
fn a_miner_stands_beside_its_mine_and_still_takes() {
    let Some(mut world) = loaded(MINE) else {
        return;
    };
    let mine = world
        .entities()
        .iter()
        .find(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .map(|entity| entity.id)
        .expect("the mine");
    assert!(
        !world.tile_walkable(4, 4),
        "a mine with gold in it should close its tile"
    );
    let villager = next_id(&world);
    spawn_villager(&mut world, villager, 2, 4);
    world.step().expect("the spawn lands");
    world
        .submit_checked(Command::Gather {
            villager,
            node: mine,
        })
        .expect("a gather order");

    let mut on_the_mine = Vec::new();
    let mut carried = 0;
    for tick in 0..1_200 {
        world.step().expect("step");
        if tile_of(&world, villager) == (4, 4) {
            on_the_mine.push(tick);
        }
        // The **highest** load seen, not the last: a worker that has just handed in is carrying nothing, which is not the
        // same as never having carried anything.
        carried = carried.max(
            world
                .entity(villager)
                .map_or(0, |entity| entity.carry.raw()),
        );
    }
    assert!(
        on_the_mine.is_empty(),
        "the miner stood on the mine on ticks {on_the_mine:?}"
    );
    assert!(carried > 0, "and it takes gold from the ring");
}
