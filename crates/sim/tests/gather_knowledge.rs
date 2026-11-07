//! **A worker only goes where it has looked** (D148).
//!
//! The owner, 2026-11-07: *"I noticed that villagers go across the map to gather berries even when they were not found
//! before. Villagers are supposed to look for resources which are nearby when their current supply is depleted. If nothing
//! is found, they should be idle."*
//!
//! The picker answered "which node is this worker's?" by distance over **every node in the world**, and a resource node the
//! player has never scouted is not a node the player can be sent to: the engine already keeps a per-player fog
//! (`explored`), and a resource node never moves, so "found" is exactly "its tile has been looked at".
//!
//! Written before the change (D7, SPEC §11).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Sim as Settings, Units};
use ruage_fixed::Fixed;
use ruage_sim::{EntityId, EntityKind, Runner, Task, World, load_scenario_str};

mod common;

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// 40x12, one town centre at (3, 6): a bush **two tiles away** and a berry patch **thirty-two tiles away**.
fn map(far_x: u16) -> String {
    format!(
        r#"
schema = 1
[map]
width = 40
height = 12
terrain = [
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
]
[[map.resource]]
kind = "food"
amount = 2
hp = 0
tiles = [[5,6]]
[[map.resource]]
kind = "food"
amount = 500
hp = 0
tiles = [[{far_x},6]]
[[map.town_center]]
player = 1
x = 3
y = 6
[[map.town_center]]
player = 2
x = 37
y = 1
"#
    )
}

/// A rule that asks for food, the way the pack's own opening does: the *engine* chooses the node.
const RULE: &str = r#"
schema = 2
duration_seconds = 40

[[rule]]
player = 1
cooldown_seconds = 1.0
do = "gather"
resource = "food"
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
        ruage_data::load_map_str(map_text, "knowledge.toml", &resources),
    );
    let economy: Economy =
        common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let units: Units = common::must("load_units_file", ruage_data::load_units_file(&units_file));
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
    Some(world)
}

/// The food node whose tile is `(x, y)`, as the engine positions it.
fn food_at(world: &World, x: u16, y: u16) -> EntityId {
    let scale = world.scale();
    let at = (
        Fixed::from_raw(i64::from(x) * scale),
        Fixed::from_raw(i64::from(y) * scale),
    );
    world
        .entities()
        .iter()
        .find(|entity| {
            matches!(entity.kind, EntityKind::Resource(_))
                && entity.position.0.raw().abs_diff(at.0.raw()) < (scale / 2).unsigned_abs()
                && entity.position.1.raw().abs_diff(at.1.raw()) < (scale / 2).unsigned_abs()
        })
        .map(|entity| entity.id)
        .unwrap_or_else(|| panic!("no food node at ({x}, {y})"))
}

/// What `id` holds, in the engine's raw units - `find` is crate-private and a test has only the list.
fn amount_of(world: &World, id: EntityId) -> Option<i64> {
    world
        .entities()
        .iter()
        .find(|entity| entity.id == id)
        .map(|entity| entity.amount.raw())
}

fn play(world: &mut World, seconds: u64) {
    let scenario = load_scenario_str(RULE, "<knowledge>").expect("the scenario parses");
    let mut runner = Runner::new(&scenario, world.tick_rate_hz()).expect("a runnable scenario");
    let ticks = seconds * u64::from(world.tick_rate_hz());
    for _ in 0..ticks {
        runner.due(world);
        world.step().expect("step");
    }
}

/// **The bug the owner reported**: a berry patch across the map, never scouted, was a candidate.
#[test]
fn a_worker_is_never_sent_to_food_it_has_not_found() {
    let Some(mut world) = world_of(&map(35)) else {
        return;
    };
    let far = food_at(&world, 35, 6);
    let held_before = amount_of(&world, far).expect("the far patch");

    play(&mut world, 40);

    // Nobody went: the far patch is untouched and every worker is idle or working the near bush.
    let held_after = amount_of(&world, far);
    assert_eq!(
        held_after,
        Some(held_before),
        "the far berry patch lost food: a worker walked to berries nobody had found"
    );
    let scale = world.scale();
    let near = 20 * scale;
    for entity in world.entities() {
        if entity.kind != EntityKind::Unit || entity.player != 1 {
            continue;
        }
        assert!(
            entity.position.0.raw() < near,
            "entity {} walked to x={:.1}, off across the map for a node it has never seen \
             (task {:?})",
            entity.id.0,
            entity.position.0.raw() as f64 / scale as f64,
            entity.task
        );
        assert!(
            !matches!(entity.task, Task::Gather { node, .. } if node == far),
            "entity {} was ordered onto the unexplored patch",
            entity.id.0
        );
    }
}

/// **The control**: the same far patch, but inside the town centre's sight, so it *has* been found - and then it is a
/// perfectly good place to work. A filter that refused everything would pass the test above and fail this one.
#[test]
fn a_worker_goes_to_food_it_has_found() {
    let Some(mut world) = world_of(&map(8)) else {
        return;
    };
    let far = food_at(&world, 8, 6);
    let held_before = amount_of(&world, far).expect("the near patch");

    play(&mut world, 60);

    let held_after = amount_of(&world, far);
    assert!(
        held_after.is_none_or(|amount| amount < held_before),
        "the patch at (8, 6) is in plain sight and still holds {held_after:?} raw units (it held {held_before}): the \
         filter refuses what it should allow"
    );
}
