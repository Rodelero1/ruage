//! **How far a blow reaches**: box to box, or centre to centre (D142).
//!
//! Our strike test compares the distance between two **centres** with the dat's `range`, so a 4x4 town centre is hit from
//! its middle and every building target is effectively two tiles nearer than it looks. The original measures between the
//! dat's **boxes** - a unit is its size box, a building its footprint - which is also what `openaok` found and what the
//! prior-art thread reported. This file holds the rule still while the arithmetic changes underneath it.
//!
//! Written before the change (D7, SPEC §11).

use std::path::PathBuf;

use ruage_sim::{Command, EntityId, EntityKind, World};

mod common;

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// A bare map: two town centres far apart, so nothing else can interfere with a single measured distance.
const MAP: &str = r#"
schema = 1
[map]
width = 32
height = 32
terrain = [
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
]
[[map.town_center]]
player = 1
x = 2
y = 2
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
    let settings = common::present(&repo("config/sim.toml"), ruage_data::load_sim_file);
    let map = common::must(
        "load_map_str",
        ruage_data::load_map_str(map_text, "range.toml", &resources),
    );
    let economy = common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let units = common::must("load_units_file", ruage_data::load_units_file(&units_file));
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
    Some(world)
}

fn spawn(world: &mut World, id: u32, player: u8, role: &str, tile: (i64, i64)) -> EntityId {
    let scale = world.scale();
    let unit = world.role_unit(role).unwrap_or_else(|| panic!("{role}"));
    let id = EntityId(id);
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                ruage_fixed::Fixed::from_raw(tile.0 * scale),
                ruage_fixed::Fixed::from_raw(tile.1 * scale),
            ),
            speed: ruage_fixed::Fixed::ZERO,
            hp: 30,
            amount: 0,
        })
        .expect("a unit can be placed");
    id
}

/// The enemy town centre's own entity (player 2's, at (20, 20): a 4x4 footprint, so its wall is at x = 24).
fn enemy_town_center(world: &World) -> EntityId {
    let centre = world.start_town_center_unit().expect("a town centre role");
    world
        .entities()
        .iter()
        .find(|entity| {
            entity.player == 2
                && entity.kind == EntityKind::Building
                && entity
                    .unit
                    .is_some_and(|unit| world.is_family(unit, centre))
        })
        .expect("the enemy town centre")
        .id
}

/// **A shot at a building is measured to its wall, not to its middle** (D142).
///
/// The skirmisher (range 4.0, radius 0.2) stands at tile (26, 22), six tiles from the town centre's **anchor** at (20, 20)
/// and **1.8** from the wall its 4x4 footprint puts at x = 24. Centre to centre that is 6.3 tiles: out of range by more
/// than two. Box to box it is 1.8: comfortably inside. The same fact, read two ways, which is why this test states the
/// distance rather than asserting a boolean alone.
#[test]
fn a_shot_at_a_building_is_measured_to_its_wall() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let archer = spawn(&mut world, 9_600, 1, "skirmisher", (26, 22));
    world.step().expect("the spawn lands");
    let target = enemy_town_center(&world);

    let scale = world.scale();
    let from = world.position(archer).expect("a position");
    let to = world.position(target).expect("the centre's position");
    let (dx, dy) = (to.0.raw() - from.0.raw(), to.1.raw() - from.1.raw());
    let centre = ((dx * dx + dy * dy) as f64).sqrt() / scale as f64;
    assert!(
        centre > 6.0 && centre < 6.6,
        "the fixture is meant to stand 6.3 tiles from the centre: it is {centre:.2}"
    );

    let _ = scale;
    assert!(
        world.in_strike_range(archer, target),
        "the range is 4.0 tiles; the skirmisher stands {centre:.1} from the middle and 1.8 from the wall"
    );
}

/// **Two units are still measured the way they always were**: their boxes are their radii, so a pair 3.5 tiles apart is in
/// a 4.0 range and a pair 5 apart is not. The box change must not move what was already right.
#[test]
fn a_shot_at_a_unit_is_still_the_distance_between_them() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let archer = spawn(&mut world, 9_610, 1, "skirmisher", (10, 10));
    let near = spawn(&mut world, 9_611, 2, "skirmisher", (13, 11));
    let far = spawn(&mut world, 9_612, 2, "skirmisher", (15, 10));
    world.step().expect("the spawns land");

    assert!(
        world.in_strike_range(archer, near),
        "3.2 tiles is inside a 4.0 range"
    );
    assert!(
        !world.in_strike_range(archer, far),
        "5.0 tiles is outside a 4.0 range"
    );
}
