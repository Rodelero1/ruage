//! **A building that arrives over a body moves it out** (D172).
//!
//! Found by D171's spread: a woodcutter settled beside a tree ended up **inside a house** the scenario built round it, because `insert`
//! blocks a building's tiles and never looks at who is standing there. From a blocked tile there is no route out, so that villager was
//! finished for the match - the ten-minute scenario showed it as a refused gather step: *"no walkable route from (22,13) to (13,19): the
//! target is walled off"*.
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

/// Open ground, a town centre far from the test, and nothing else.
const MAP: &str = r#"
schema = 1
[map]
width = 20
height = 16
terrain = [
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
]
[[map.town_center]]
player = 1
x = 2
y = 13
[[map.town_center]]
player = 2
x = 17
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
        ruage_data::load_map_str(map_text, "over.toml", &resources),
    );
    let economy: Economy =
        common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let units: Units = common::must("load_units_file", ruage_data::load_units_file(&units_file));
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
    Some(world)
}

/// **A house lands on the villager's tile.** The villager has to end up outside the footprint and on ground it can walk from - a body
/// left inside a solid footprint has no route out and never works again.
#[test]
fn a_house_built_over_a_villager_moves_it_out() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let scale = world.scale();
    let villager = world.role_unit("villager").expect("villager");
    let house = world.role_unit("house").expect("house");
    let at = (10i64, 8i64);
    world
        .submit_checked(Command::Spawn {
            id: EntityId(9_620),
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: (
                Fixed::from_raw(at.0 * scale + scale / 2),
                Fixed::from_raw(at.1 * scale + scale / 2),
            ),
            speed: Fixed::ZERO,
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    world.step().expect("the villager lands");

    // The house is spawned so that its **2x2 footprint covers the villager's tile**, which is what a player can do with a build order.
    world
        .submit_checked(Command::Spawn {
            id: EntityId(9_621),
            player: 1,
            kind: EntityKind::Building,
            unit: Some(house),
            position: (Fixed::from_raw(at.0 * scale), Fixed::from_raw(at.1 * scale)),
            speed: Fixed::ZERO,
            hp: 550,
            amount: 0,
        })
        .expect("a house");
    world.step().expect("the house lands");

    let position = world.position(EntityId(9_620)).expect("the villager");
    let tile = world.tile_of_position(position);
    assert!(
        world.tile_walkable(tile.0, tile.1),
        "the villager was left on a blocked tile {tile:?} - it is inside the house and has no route out"
    );
    // And it can still be given an order: a body in a pocket it cannot leave is the same bug one step later.
    world
        .submit_checked(Command::Move {
            unit: EntityId(9_620),
            to: (14, 8),
        })
        .expect("a move out of the house");
}
