//! **Two blocks that touch at a corner stop a unit** (D190). The owner: *"Block of forest is never walkable through. Ever. Even
//! tiles with tree on it touching by the corner are blocking path."*
//!
//! The pathfinder already refused a diagonal **grid** step between two blocked tiles (D113), and the movement pass did not: it
//! asked the corner question only of a step that crossed a whole tile, and a villager walks 0.04 of a tile a tick, so the question
//! was never asked and a unit slid through the gap one hair at a time.
//!
//! Written before the implementation (D7, SPEC §11).

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

/// Open grass with **two wood tiles on a diagonal**, `(6,6)` and `(7,7)`, and a strip of trees behind them so the gap is the only
/// obvious way through.
const MAP: &str = r#"
schema = 1
[map]
width = 16
height = 16
terrain = [
  "GGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGG",
  "GGGGGGFGGGGGGGGG",
  "GGGGGGGFGGGGGGGG",
  "GGGGGGFGGGGGGGGG",
  "GGGGGGGFGGGGGGGG",
  "GGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGG",
]
[[map.resource]]
kind = "wood"
amount = 200
hp = 200
tiles = [[6,6],[7,7],[6,8],[7,9]]
[[map.town_center]]
player = 1
x = 2
y = 2
[[map.town_center]]
player = 2
x = 13
y = 13
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
        ruage_data::load_map_str(map_text, "corner.toml", &resources),
    );
    let economy: Economy =
        common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let units: Units = common::must("load_units_file", ruage_data::load_units_file(&units_file));
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
    Some(world)
}

fn spawn(world: &mut World, id: EntityId, tile: (i64, i64)) {
    let unit = world.start_villager_unit().expect("a villager role");
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                Fixed::from_raw(tile.0 * scale + scale / 2),
                Fixed::from_raw(tile.1 * scale + scale / 2),
            ),
            speed: Fixed::from_raw(scale * 4 / 5),
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
}

fn tile_of(world: &World, id: EntityId) -> (i64, i64) {
    let at = world.position(id).expect("a position");
    (
        at.0.raw().div_euclid(world.scale()),
        at.1.raw().div_euclid(world.scale()),
    )
}

/// **The gap between two diagonally touching trees is not a door.** A villager sent from one side to the other must go round the
/// wood, and never occupy either tree's tile.
#[test]
fn a_diagonal_gap_between_two_trees_is_not_walkable() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let id = EntityId(9_000);
    // Below-left of the pair, ordered to above-right of it: the straight line goes through the corner they share.
    spawn(&mut world, id, (5, 8));
    world.step().expect("the spawn lands");
    world
        .submit_checked(Command::Move {
            unit: id,
            to: (8, 5),
        })
        .expect("a move order");

    let trees = [(6i64, 6i64), (7, 7), (6, 8), (7, 9)];
    for _ in 0..1_500 {
        world.step().expect("a tick");
        let at = tile_of(&world, id);
        assert!(
            !trees.contains(&at),
            "MEASURED the villager standing on the tree tile {at:?}"
        );
        // **The corner itself**: with trees at (6,6) and (7,7), the shared corner is the lattice point (7, 7) in tile
        // coordinates - and a body passing through it would be inside one of the two.
        let position = world.position(id).expect("a position");
        let (px, py) = (
            position.0.raw() as f64 / world.scale() as f64,
            position.1.raw() as f64 / world.scale() as f64,
        );
        for (tx, ty) in trees {
            let (lo_x, lo_y) = (tx as f64, ty as f64);
            let (hi_x, hi_y) = (lo_x + 1.0, lo_y + 1.0);
            let dx = (lo_x - px).max(0.0).max(px - hi_x);
            let dy = (lo_y - py).max(0.0).max(py - hi_y);
            let clearance = (dx * dx + dy * dy).sqrt();
            assert!(
                clearance >= 0.15,
                "MEASURED the villager {clearance:.3} tiles from the tree at ({tx},{ty}): his 0.2 circle is inside it"
            );
        }
    }
    // He may or may not have found a way round within the time; what he may not do is stand in the wood.
    let arrived = tile_of(&world, id) == (8, 5);
    assert!(
        arrived,
        "MEASURED the villager at {:?} rather than (8,5): a corner gap is not the only way round, and he must still arrive",
        tile_of(&world, id)
    );
}
