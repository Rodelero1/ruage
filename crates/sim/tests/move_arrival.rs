//! **A move order arrives at the tile it named** (D178).
//!
//! The owner, after D177, with a screenshot of a villager stopped short of the yellow destination cross: *"the circle moves a bit elsewhere than it
//! should... the unit above it stops a bit further"*. This walks a villager across open ground to a tile **with nothing near it** and measures how
//! close it gets. If it stops short on empty grass, the clearance rule is refusing steps it should allow.
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

const MAP: &str = r#"
schema = 1
[map]
width = 22
height = 16
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
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
]
[[map.town_center]]
player = 1
x = 2
y = 13
[[map.town_center]]
player = 2
x = 20
y = 1
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
        ruage_data::load_map_str(map_text, "arrival.toml", &resources),
    );
    let economy: Economy =
        common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let units: Units = common::must("load_units_file", ruage_data::load_units_file(&units_file));
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
    Some(world)
}

/// **Open grass, nothing within three tiles**: the villager must finish on the tile it was sent to, not a stride before it.
#[test]
fn a_move_across_empty_ground_lands_on_the_tile_it_was_given() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let scale = world.scale();
    let villager = world.role_unit("villager").expect("villager");
    world
        .submit_checked(Command::Spawn {
            id: EntityId(9_900),
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: (
                Fixed::from_raw(8 * scale + scale / 2),
                Fixed::from_raw(8 * scale + scale / 2),
            ),
            speed: Fixed::parse("0.8", scale).expect("a speed"),
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    world.step().expect("the villager lands");
    world
        .submit_checked(Command::Move {
            unit: EntityId(9_900),
            to: (12, 11),
        })
        .expect("a move");

    for _ in 0..300 {
        world.step().expect("a tick");
    }
    let at = world.position(EntityId(9_900)).expect("the villager");
    // The tile the order named: its centre, in tiles.
    let (want_x, want_y) = (12.5, 11.5);
    let got = (
        at.0.raw() as f64 / scale as f64,
        at.1.raw() as f64 / scale as f64,
    );
    let gap = ((got.0 - want_x).powi(2) + (got.1 - want_y).powi(2)).sqrt();
    assert!(
        gap <= 0.1,
        "the villager finished at ({:.2},{:.2}), {gap:.2} tiles from the tile it was sent to ({want_x},{want_y}) - it stopped short on empty ground",
        got.0,
        got.1
    );
}
