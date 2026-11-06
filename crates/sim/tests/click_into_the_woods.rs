//! **Every click is accepted** (D134, the owner 2026-11-06): *"clicking into the middle of the forest ends up with error
//! message. It has to be allowed. Scout cavalry will simply try to close to the point as close as it can without running
//! into trees."*
//!
//! Written before the implementation (D7, SPEC §11). The wood below is nine tiles in a 3x3 block, so its middle tile has
//! **no free neighbour**: that is the case the tile search cannot answer, and today the order is refused instead of being
//! walked to the nearest legal standing tile.

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, SimError, World};

mod common;

/// Nine tiles of grass, a 3x3 wood in the middle, a town centre for each side.
const WOOD: &str = r#"
schema = 1
[map]
width = 9
height = 9
terrain = [
  "GGGGGGGGG",
  "GGGGGGGGG",
  "GGGGGGGGG",
  "GGGFFFGGG",
  "GGGFFFGGG",
  "GGGFFFGGG",
  "GGGGGGGGG",
  "GGGGGGGGG",
  "GGGGGGGGG",
]
[[map.resource]]
kind = "wood"
amount = 100
hp = 20
tiles = [[3,3],[4,3],[5,3],[3,4],[4,4],[5,4],[3,5],[4,5],[5,5]]
[[map.town_center]]
player = 1
x = 0
y = 0
[[map.town_center]]
player = 2
x = 8
y = 8
"#;

fn loaded(text: &str) -> Option<World> {
    let units = common::repo("config/gamedata/units.toml");
    if !units.exists() {
        return None;
    }
    let resources = common::present(
        &common::repo("config/resources.toml"),
        ruage_data::load_resources_file,
    );
    let settings = common::present(&common::repo("config/sim.toml"), ruage_data::load_sim_file);
    let map = common::must(
        "load_map_str",
        ruage_data::load_map_str(text, "wood.toml", &resources),
    );
    let economy = common::present(
        &common::repo("config/economy.toml"),
        ruage_data::load_economy_file,
    );
    let units = common::must("load_units_file", ruage_data::load_units_file(&units));
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
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

/// A click into the middle of a wood: accepted, and the unit walks to the nearest tile it may legally stand on.
#[test]
fn a_click_in_the_middle_of_a_wood_is_accepted_and_walked_to_its_edge() {
    let Some(mut world) = loaded(WOOD) else {
        return;
    };
    let walker = next_id(&world);
    spawn_villager(&mut world, walker, 1, 4);
    world.step().expect("the spawn lands");

    let ordered = world.submit_checked(Command::Move {
        unit: walker,
        to: (4, 4),
    });
    assert!(
        ordered.is_ok(),
        "a click in the middle of a wood is a move order, not an error: {:?}",
        ordered.err()
    );

    for _ in 0..600 {
        world.step().expect("step");
    }
    assert!(
        !matches!(world.last_refusal(), Some((id, SimError::NoPath { .. }, _)) if id == walker),
        "the order was refused with a pathing error instead of walked: {:?}",
        world.last_refusal().map(|(_, error, _)| format!("{error}"))
    );

    let (x, y) = tile_of(&world, walker);
    assert!(
        world.tile_walkable(
            u16::try_from(x).expect("in range"),
            u16::try_from(y).expect("in range")
        ),
        "it stopped on blocked ground at ({x}, {y})"
    );
    assert_ne!((x, y), (1, 4), "it never left its spawn tile");
    // The wood is (3..=5, 3..=5); a legal standing tile that touches it is within one tile of the block.
    let touching = (2..=6).contains(&x) && (2..=6).contains(&y);
    assert!(
        touching,
        "it stopped at ({x}, {y}), which is not the edge of the wood the click was aimed at"
    );
}

/// A **free tile behind a wall**: a courtyard of grass ringed by wood. The click is legal ground, so nothing about it is
/// "blocked" - and it is still unreachable, which is the other half of the owner's rule (*"every click is accepted"*).
const COURT: &str = r#"
schema = 1
[map]
width = 9
height = 9
terrain = [
  "GGGGGGGGG",
  "GGGGGGGGG",
  "GGGFFFGGG",
  "GGGFFFGGG",
  "GGGFFFGGG",
  "GGGFFFGGG",
  "GGGFFFGGG",
  "GGGGGGGGG",
  "GGGGGGGGG",
]
[[map.resource]]
kind = "wood"
amount = 100
hp = 20
tiles = [[3,2],[4,2],[5,2],[3,3],[5,3],[3,4],[5,4],[3,5],[5,5],[3,6],[4,6],[5,6]]
[[map.town_center]]
player = 1
x = 0
y = 0
[[map.town_center]]
player = 2
x = 8
y = 8
"#;

#[test]
fn a_click_inside_a_walled_court_is_accepted_and_walked_to_its_wall() {
    let Some(mut world) = loaded(COURT) else {
        return;
    };
    let walker = next_id(&world);
    spawn_villager(&mut world, walker, 0, 4);
    world.step().expect("the spawn lands");

    let ordered = world.submit_checked(Command::Move {
        unit: walker,
        to: (4, 4),
    });
    assert!(
        ordered.is_ok(),
        "a free tile nobody can reach is still a place to walk towards: {:?}",
        ordered.err()
    );
    for _ in 0..600 {
        world.step().expect("step");
    }
    let (x, y) = tile_of(&world, walker);
    assert_ne!((x, y), (0, 4), "it never left its spawn tile");
    assert!(
        world.tile_walkable(
            u16::try_from(x).expect("in range"),
            u16::try_from(y).expect("in range")
        ),
        "it stopped on blocked ground at ({x}, {y})"
    );
}

/// A click on the tile a unit already stands on: accepted, nothing moves, nothing panics.
#[test]
fn a_click_on_its_own_tile_is_accepted_and_does_nothing() {
    let Some(mut world) = loaded(WOOD) else {
        return;
    };
    let walker = next_id(&world);
    spawn_villager(&mut world, walker, 1, 4);
    world.step().expect("the spawn lands");
    let before = world.position(walker).expect("a position");

    world
        .submit_checked(Command::Move {
            unit: walker,
            to: (1, 4),
        })
        .expect("a click where it stands");
    for _ in 0..20 {
        world.step().expect("step");
    }
    let after = world.position(walker).expect("a position");
    // The spawn stood on the tile's corner and the order's goal is its centre, so "did not move" means **the same tile**,
    // not the same raw position: a tile order that centres a body has done what it was asked.
    let (x0, y0) = (
        before.0.raw().div_euclid(world.scale()),
        before.1.raw().div_euclid(world.scale()),
    );
    let (x1, y1) = (
        after.0.raw().div_euclid(world.scale()),
        after.1.raw().div_euclid(world.scale()),
    );
    assert_eq!(
        (x0, y0),
        (x1, y1),
        "it wandered off the tile it was told to stand on"
    );
}
