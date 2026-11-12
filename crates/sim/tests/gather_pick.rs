//! **Who picks the tree, and what it costs** (D138).
//!
//! The script's `do = "gather"` action asks the engine which node to work. The engine already had an answer to "which
//! node" - `World::nodes_of_slot`: distance order, **no reachability test** (a path search per candidate cost 37x the
//! whole run, measured 2026-10-29), failures remembered in `share_failed` and retried after `share_retry_ticks`. The
//! script had its own copy that did the expensive thing, and this file holds the pair to one rule.
//!
//! Written before the change (D7, SPEC §11).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Sim as Settings, Units};
use ruage_fixed::Fixed;
use ruage_script::{Runner, load_scenario_str};
use ruage_sim::{Command, EntityId, EntityKind, World};

mod common;

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// **Two woods, one walled off.** The near block (x 13-15) is across a palisade line at x = 11; the far block (x 5-6)
/// is on the villager's side of it. The near block is nearer to the town centre, so a picker that asks only "how far?"
/// takes it - and a picker that asks "can I walk there?" for every candidate pays a search per tile to find that out.
const MAP: &str = r#"
schema = 1
[map]
width = 20
height = 14
terrain = [
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGFFGGGGGGGGGGGGG",
  "GGGGGFFGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGFFFGGGG",
  "GGGGGGGGGGGGGFFFGGGG",
  "GGGGGGGGGGGGGFFFGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
]
[[map.resource]]
kind = "wood"
amount = 100
hp = 0
tiles = [[13,6],[14,6],[15,6],[13,7],[14,7],[15,7],[13,8],[14,8],[15,8]]
[[map.resource]]
kind = "wood"
amount = 100
hp = 0
tiles = [[5,1],[6,1],[5,2],[6,2]]
[[map.town_center]]
player = 1
x = 9
y = 7
[[map.town_center]]
player = 2
x = 1
y = 12
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
        ruage_data::load_map_str(map_text, "picks.toml", &resources),
    );
    let economy: Economy =
        common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let units: Units = common::must("load_units_file", ruage_data::load_units_file(&units_file));
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
    Some(world)
}

/// A palisade segment, one tile, placed by hand: the wall is a fixture, not something under test.
fn segment(world: &mut World, id: u32, x: u16, y: u16) {
    let scale = world.scale();
    let unit = world.role_unit("palisade_wall").expect("a palisade role");
    world
        .submit_checked(Command::Spawn {
            id: EntityId(id),
            player: 1,
            kind: EntityKind::Building,
            unit: Some(unit),
            position: (
                Fixed::from_raw(i64::from(x) * scale),
                Fixed::from_raw(i64::from(y) * scale),
            ),
            speed: Fixed::ZERO,
            hp: 9_900,
            amount: 0,
        })
        .expect("a segment can be placed");
}

/// The rule, written the way a pack writes one: a question about the world, and one order.
const RULE: &str = r#"
schema = 2
duration_seconds = 30

[[rule]]
player = 1
cooldown_seconds = 1.0
do = "gather"
resource = "wood"
"#;

/// **The picker asks nothing it does not have to** (D138).
///
/// With the near block walled off, three things have to be true at once, and they are what this test is for:
///
/// 1. the picker **takes the near block** - it asks "how far", not "can I walk there", so the refusal that follows is
///    the engine's own (`start_gather` is the authority on whether a node can be worked);
/// 2. the villager **ends up chopping the far block**, so a refusal is not a dead end - the node is remembered and the
///    next firing moves on;
/// 3. the refusals stay **bounded** by the number of walled nodes (nine), not by the number of firings (thirty), so the
///    memory is doing its job rather than the rule retrying the same tree for the whole match.
#[test]
fn a_walled_off_wood_is_tried_once_and_then_left_alone() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    for y in 0..14u16 {
        segment(&mut world, 9_000 + u32::from(y), 11, y);
    }
    world.step().expect("the segments land");

    // What the far block holds **before**, so the assertion is a change rather than a number I guessed: node amounts are
    // in the engine's raw units, not in "wood".
    let far_before = far_wood(&world);

    let scenario = load_scenario_str(RULE, "<picks>").expect("the scenario parses");
    let mut runner = Runner::new(&scenario, world.tick_rate_hz()).expect("a runnable scenario");
    let ticks = u64::try_from((scenario.duration_seconds * f64::from(world.tick_rate_hz())) as i64)
        .expect("a length in ticks");
    for _ in 0..ticks {
        runner.due(&mut world);
        world.step().expect("step");
    }

    // 1. The picker did not pay for a reachability test: the walled node was chosen, and the engine said no.
    assert!(
        !runner.rule_refusals.is_empty(),
        "the picker must take the nearest node and let the engine refuse it; it refused nothing, so it pre-filtered"
    );

    // 2. A refusal is not a dead end: the far block was **worked**, so the picker moved on after the walled one was
    //    remembered. Read from the nodes rather than from the villager's task, because 30 s is long enough for it to be
    //    carrying a load home at the moment the loop ends.
    let far_after = far_wood(&world);
    assert!(
        far_after < far_before,
        "the far block still holds all {far_after} raw units (it held {far_before}): the remembered failure did not move \
         the picker on"
    );

    // 3. Bounded by the walled nodes, not by the firings.
    assert!(
        runner.rule_refusals.len() <= 12,
        "the rule was refused {} times in 30 s for nine walled nodes: something is retrying the same node",
        runner.rule_refusals.len()
    );
}

/// How much wood the **far** block (x <= 6, the villager's side of the wall) still holds, in the engine's raw units.
fn far_wood(world: &World) -> i64 {
    let scale = world.scale();
    let slot = world.slot_of("wood").expect("a wood slot");
    world
        .entities()
        .iter()
        .filter(|entity| entity.kind == EntityKind::Resource(slot))
        .filter(|entity| entity.position.0.raw().div_euclid(scale) <= 6)
        .map(|entity| entity.amount.raw())
        .sum()
}
