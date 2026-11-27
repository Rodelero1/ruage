//! **The take-gate: within one step the boxes touch-take, two steps out they do not** (D285/D286).
//!
//! Pins `at_node == (box_gap_sq <= step^2)` at the gather arm before the dead
//! predecessor predicates beside `step.rs:1638` are deleted. One villager (radius
//! 0.2), one bush (half tile), open grass; the step is `speed / tick_rate`.
//! Written before the deletion (D7, SPEC §11).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Sim as Settings, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, Task, World};

mod common;

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// One bush on open grass. `hp = 0`: a bush has no hit points to fell (unlike a
/// tree), so an arrival takes (progress) on the first tick instead of chopping.
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
[[map.resource]]
kind = "food"
amount = 900
hp = 0
tiles = [[12,7]]
[[map.town_center]]
player = 1
x = 4
y = 13
[[map.town_center]]
player = 2
x = 19
y = 2
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
        ruage_data::load_map_str(map_text, "take_gate.toml", &resources),
    );
    let economy: Economy =
        common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let units: Units = common::must("load_units_file", ruage_data::load_units_file(&units_file));
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
    Some(world)
}

fn bush_of(world: &World) -> EntityId {
    world
        .entities()
        .iter()
        .find(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .map(|entity| entity.id)
        .expect("a bush")
}

/// Spawn a villager `gap` raw units outside the bush's box face on the +x side,
/// same y as the bush. `gap` is the box gap: `node_half + body + gap` from the
/// bush centre, so `box_gap_sq == gap^2` on this axis.
fn spawn_at_gap(world: &mut World, id: EntityId, bush: EntityId, gap: i64) {
    let bush_at = world.position(bush).expect("a bush position");
    let scale = world.scale();
    let node_half = scale / 2; // a resource node is one tile wide (D285)
    let body = scale / 5; // a villager's 0.2
    let unit = world.role_unit("villager").expect("a villager role");
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                Fixed::from_raw(bush_at.0.raw() + node_half + body + gap),
                bush_at.1,
            ),
            speed: Fixed::parse("0.8", scale).expect("a speed"),
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    world.step().expect("the villager lands");
}

fn gather_progress(world: &World, id: EntityId) -> i64 {
    world
        .entity(id)
        .and_then(|entity| match entity.task {
            Task::Gather { progress, .. } => Some(progress.raw()),
            Task::Return { .. } => Some(i64::MAX),
            _ => None,
        })
        .expect("a gatherer still on the job")
}

/// **The dead arrival predicates are gone.** The gather arm keeps exactly one
/// arrival rule (`box_gap_sq <= step^2`); the D285 predecessors must not come back.
/// Watches the `decide` source itself: each token below exists only in the dead
/// block (`step.rs` gather arm) or, for `ring_open`, its now-orphaned method.
#[test]
fn dead_arrival_predicates_stay_deleted() {
    const STEP: &str = include_str!("../src/world/step.rs");
    const LIB: &str = include_str!("../src/lib.rs");
    for token in [
        "let touch_tolerance",
        "self.ring_open(",
        "let settle_reach",
        "owned_place",
        "_near_node",
        "_off_place",
        "near_enough",
        "_on_own_place",
        "let beside = target_is_building",
        "ring_squared",
        "here == there",
        "let _ = (touching",
        "let body = self.body_radius(entity.unit)",
        "let squared = gap_x * gap_x + gap_y * gap_y;",
        "let reach = self.reach_raw(entity.unit, node_unit);",
        "let centre = self.tile_centre",
        "let (ring_x, ring_y)",
        "let touching",
    ] {
        assert!(
            !STEP.contains(token),
            "DEAD CODE BACK: the gather arm still contains `{token}` — the box-gap gate is the only arrival rule"
        );
    }
    assert!(
        !LIB.contains("fn ring_open"),
        "DEAD CODE BACK: `ring_open` has no caller left — its only call was the deleted arrival block"
    );
}

/// **Within one step the man takes.** Spawned exactly one step of box gap out,
/// one tick of `Gather` starts progress (`at_node` is `gap <= step`).
#[test]
fn take_gate_arrives_within_one_step() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let scale = world.scale();
    let bush = bush_of(&world);
    // The step the arm asks of: `speed / tick_rate`, the same `div_exact`.
    let hz = i64::from(world.tick_rate_hz().max(1));
    let step = Fixed::parse("0.8", scale).expect("a speed").raw() / hz;
    assert!(step > 0, "the step must be positive at this scale");
    let id = EntityId(9_100);
    spawn_at_gap(&mut world, id, bush, step);
    world
        .submit_checked(Command::Gather {
            villager: id,
            node: bush,
        })
        .expect("a gather order");
    world.step().expect("a tick");
    assert!(
        gather_progress(&world, id) > 0,
        "MEASURED no take after one tick from one step of box gap out: the gate must fire at gap <= step"
    );
}

/// **Two steps out the man walks but does not take.** Same scene at twice the
/// box gap: one tick moves him closer, progress stays zero.
#[test]
fn take_gate_does_not_arrive_two_steps_out() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let scale = world.scale();
    let bush = bush_of(&world);
    let hz = i64::from(world.tick_rate_hz().max(1));
    let step = Fixed::parse("0.8", scale).expect("a speed").raw() / hz;
    assert!(step > 0, "the step must be positive at this scale");
    let id = EntityId(9_101);
    spawn_at_gap(&mut world, id, bush, 2 * step);
    world
        .submit_checked(Command::Gather {
            villager: id,
            node: bush,
        })
        .expect("a gather order");
    world.step().expect("a tick");
    assert!(
        gather_progress(&world, id) == 0,
        "MEASURED a take after one tick from two steps of box gap out: the gate must not fire past one step"
    );
}
