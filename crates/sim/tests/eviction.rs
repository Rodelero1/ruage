//! **A building placed on a unit's tile must not put the unit inside a tree** (D184, slice 1 of D183).
//!
//! The owner: *"I noticed that scout stopped near the forest, then villager placed there a lumber camp colliding with his position, that pushed scout
//! to the forest. So reproducer: put scout as near to forest as possible, place lumber camp where the scout is, he should immediately jump to the
//! forest."*
//!
//! The ground under a tree is **grass** - trees are objects, not terrain - so a "free ground" test passes for the tree's tile and the eviction lands
//! the scout on top of a trunk. Whatever D183 does later (a foundation that is walkable and the unit walks out), the eviction must never put a body
//! somewhere a body cannot be.
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

/// Grass everywhere, with a wood patch **north-west** of the camp site - the first tile the eviction ring tries.
const MAP: &str = r#"
schema = 1
[map]
width = 18
height = 14
terrain = [
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGFGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
]
[[map.resource]]
kind = "wood"
amount = 100
hp = 20
tiles = [[9,5]]
[[map.town_center]]
player = 1
x = 3
y = 11
[[map.town_center]]
player = 2
x = 16
y = 1
"#;

#[test]
fn a_placed_building_never_evicts_a_unit_into_a_tree() {
    let units_file = repo("config/gamedata/units.toml");
    if !units_file.exists() {
        return;
    }
    let resources = common::present(
        &repo("config/resources.toml"),
        ruage_data::load_resources_file,
    );
    let settings: Settings = common::present(&repo("config/sim.toml"), ruage_data::load_sim_file);
    let map: Map = common::must(
        "load_map_str",
        ruage_data::load_map_str(MAP, "eviction.toml", &resources),
    );
    let economy: Economy =
        common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let units: Units = common::must("load_units_file", ruage_data::load_units_file(&units_file));
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));

    let scale = world.scale();
    let scout = world
        .role_unit("scout")
        .or_else(|| world.role_unit("militia"))
        .expect("a unit role");
    let camp = world
        .role_unit("lumber_camp")
        .or_else(|| world.role_unit("house"))
        .expect("a building role");
    // The scout stands on **(11,6)** - the *far* tile of a 2x2 building anchored at (10,6), which is where the eviction has to look around its own
    // tile rather than the building's anchor.
    world
        .submit_checked(Command::Spawn {
            id: EntityId(9_300),
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(scout),
            position: (
                Fixed::from_raw(11 * scale + scale / 2),
                Fixed::from_raw(6 * scale + scale / 2),
            ),
            speed: Fixed::parse("1.2", scale).expect("a speed"),
            hp: 45,
            amount: 0,
        })
        .expect("a scout");
    world.step().expect("the scout lands");

    // A building placed across the scout's own tile, exactly as the owner described.
    world
        .submit_checked(Command::Spawn {
            id: EntityId(9_301),
            player: 1,
            kind: EntityKind::Building,
            unit: Some(camp),
            position: (Fixed::from_raw(10 * scale), Fixed::from_raw(6 * scale)),
            speed: Fixed::ZERO,
            hp: 600,
            amount: 0,
        })
        .expect("a building on the scout");
    world.step().expect("the building lands");

    let at = world
        .position(EntityId(9_300))
        .expect("the scout is still in the world");
    let tile = world.tile_of_position(at);
    // **Not inside anything**: the tile the scout now stands on holds no tree, no mine, no building.
    let on_a_trunk = world.entities().iter().any(|entity| {
        matches!(entity.kind, EntityKind::Resource(_) | EntityKind::Building)
            && entity.id != EntityId(9_301)
            && world.tile_of_position(entity.position) == tile
    });
    assert!(
        !on_a_trunk,
        "the scout was evicted to {tile:?}, which holds a resource or building - it was pushed into the forest"
    );
    // **And not inside the building that evicted him**: the footprint is blocked the moment it lands (D152).
    assert!(
        world.tile_walkable(tile.0, tile.1),
        "the scout was evicted to {tile:?}, which is inside the building's own footprint"
    );
    assert!(
        world.body_clearance(EntityId(9_300)) >= 0,
        "the scout was evicted to a tile it cannot stand on"
    );
}
