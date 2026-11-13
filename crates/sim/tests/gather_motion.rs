//! **A gatherer walks to its work, it does not jump to it** (D173).
//!
//! The owner, after D169-D172, with three screenshots: *"villagers tend to jump towards the bush (I noticed with tree, too), pile up on each other,
//! forage from greater distance ... appear and disappear in rapid succession"*, and *"This is regression of previous step"*.
//!
//! The jump is the **settle**: it moved an arrived worker the last third of a tile in one tick, which is a teleport, not a walk. This test says a
//! body may never move further in one tick than its own speed allows - the rule every other movement in the engine already obeys.
//!
//! Written before the change (D7, SPEC §11).

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
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGFGGGGGGG",
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
tiles = [[10,6]]
[[map.town_center]]
player = 1
x = 2
y = 12
[[map.town_center]]
player = 2
x = 16
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
        ruage_data::load_map_str(map_text, "motion.toml", &resources),
    );
    let economy: Economy =
        common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let units: Units = common::must("load_units_file", ruage_data::load_units_file(&units_file));
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
    Some(world)
}

/// **A settled gatherer stays still.** With the standoff recomputed every tick, a worker that swings between two sides jitters across a
/// tile edge - the flicker the owner photographed. Once it is at its work, its position must not change at all while the load is filling.
#[test]
fn a_settled_gatherer_does_not_jitter() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let scale = world.scale();
    let node = world
        .entities()
        .iter()
        .find(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .map(|entity| entity.id)
        .expect("a tree at (10,6)");
    let villager = world.role_unit("villager").expect("villager");
    world
        .submit_checked(Command::Spawn {
            id: EntityId(9_660),
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: (
                Fixed::from_raw(7 * scale + scale / 2),
                Fixed::from_raw(6 * scale + scale / 2),
            ),
            speed: Fixed::parse("0.8", scale).expect("a speed"),
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    world.step().expect("the villager lands");
    world
        .submit_checked(Command::Gather {
            villager: EntityId(9_660),
            node,
        })
        .expect("a gather order");
    // Long enough to arrive from two tiles away at 0.8 tiles a second, and to be chopping.
    for _ in 0..120 {
        world.step().expect("a tick");
    }
    let settled = world.position(EntityId(9_660)).expect("the villager");
    for tick in 0..60 {
        world.step().expect("a tick");
        let now = world.position(EntityId(9_660)).expect("the villager");
        assert_eq!(
            now, settled,
            "the settled gatherer moved on tick {tick}: {settled:?} -> {now:?} - it is jittering between two working spots"
        );
    }
}

/// **Three villagers ordered onto one bush do not pile up on it** (D171, restated under D182).
///
/// The old shape of this test asserted that all three *gathered* the one bush from separate sides: D171's fan-out gave a node four sides. The
/// owner replaced that rule - *"if there is no space at half diameter from another villager and the resource, try another nearest resource of the
/// same kind"* - so a resource square now holds **one** gatherer and the other two are answered elsewhere (or stand idle when the map has nothing
/// free). What survives from the old test, and is the reason it exists, is that **no two bodies end up on the same ground**.
#[test]
fn three_gatherers_on_one_bush_do_not_pile_up() {
    let Some(mut world) = world_of(MAP_BUSH) else {
        return;
    };
    let scale = world.scale();
    let node = world
        .entities()
        .iter()
        .find(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .map(|entity| entity.id)
        .expect("a bush at (10,6)");
    let villager = world.role_unit("villager").expect("villager");
    let ids = [9_640u32, 9_641, 9_642];
    for (nth, id) in ids.iter().enumerate() {
        let at = (
            6 + i64::try_from(nth).unwrap_or(0) * 2,
            6 + i64::try_from(nth).unwrap_or(0),
        );
        world
            .submit_checked(Command::Spawn {
                id: EntityId(*id),
                player: 1,
                kind: EntityKind::Unit,
                unit: Some(villager),
                position: (
                    Fixed::from_raw(at.0 * scale + scale / 2),
                    Fixed::from_raw(at.1 * scale + scale / 2),
                ),
                speed: Fixed::parse("0.8", scale).expect("a speed"),
                hp: 25,
                amount: 0,
            })
            .expect("a villager");
        world.step().expect("the villager lands");
        world
            .submit_checked(Command::Gather {
                villager: EntityId(*id),
                node,
            })
            .expect("a gather order");
    }

    // **Measured over the run, not sampled at one tick** (D200). Three gatherers cycle - chop, carry, walk to the town
    // centre, walk back - so a single sample catches one of them on the road; the version of this test that sampled tick
    // 1,200 called a carrier a deadlock. Measured on this branch before the assertion was written: all three on the bush on
    // **601 of 1,200** ticks, and the closest pair over the whole run **0.400** - exactly the reach, never inside it.
    let tile = world.resource_tiles(node)[0];
    let mut all_three = 0u32;
    let mut closest = f64::MAX;
    for _ in 0..1_200 {
        world.step().expect("a tick");
        let mut on = 0u32;
        for id in ids {
            let Some(p) = world.position(EntityId(id)) else {
                continue;
            };
            let working = world
                .entity(EntityId(id))
                .is_some_and(|e| matches!(e.task, Task::Gather { node: n, .. } if n == node));
            let clearance = ruage_sim::tiles::distance_to_tile(
                p,
                (i64::from(tile.0), i64::from(tile.1)),
                scale,
            );
            if working && (scale / 8..=scale / 3).contains(&clearance) {
                on += 1;
            }
        }
        if on == 3 {
            all_three += 1;
        }
        for (nth, a) in ids.iter().enumerate() {
            for b in ids.iter().skip(nth + 1) {
                if let (Some(pa), Some(pb)) = (
                    world.position(EntityId(*a)),
                    world.position(EntityId(*b)),
                ) {
                    let dx = (pa.0.raw() - pb.0.raw()) as f64 / scale as f64;
                    let dy = (pa.1.raw() - pb.1.raw()) as f64 / scale as f64;
                    closest = closest.min((dx * dx + dy * dy).sqrt());
                }
            }
        }
    }
    assert!(
        all_three >= 500,
        "all three villagers worked the bush on only {all_three} of 1,200 ticks (measured 601)"
    );
    assert!(
        closest >= 0.39,
        "two villagers came {closest:.3} tiles apart - their bodies overlap on screen (measured 0.400)"
    );
}

/// The same one-tree map with the tree standing for **food** (a bush), which has no hit points and is foraged straight away.
const MAP_BUSH: &str = r#"
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
  "GGGGGGGGGGGGGGGGGG",
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
kind = "food"
amount = 100
hp = 0
tiles = [[10,6]]
[[map.town_center]]
player = 1
x = 2
y = 12
[[map.town_center]]
player = 2
x = 16
y = 1
"#;

fn apart(a: (Fixed, Fixed), b: (Fixed, Fixed), scale: i64) -> f64 {
    let dx = (a.0.raw() - b.0.raw()) as f64 / scale as f64;
    let dy = (a.1.raw() - b.1.raw()) as f64 / scale as f64;
    (dx * dx + dy * dy).sqrt()
}

/// **No tick may move a gatherer further than its own speed.** A teleport is exactly that, and the owner sees it as a jump towards the bush.
///
/// **D173 is closed**: the settle walks a worker to its standoff, one step at a time, so this is a passing guard. It was ignored
/// while the settle still *placed* the worker
/// (`settle_workers`, D169) instead of walking it there, and the placement is the jump. Two attempts to walk it are recorded in
/// `docs/reviews/2026-11-08-the-jump-that-is-a-placement.md` with their measurements; neither shipped. Un-ignore this test when one does.
#[test]
fn a_gatherer_never_moves_further_in_a_tick_than_its_speed() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let scale = world.scale();
    let node = world
        .entities()
        .iter()
        .find(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .map(|entity| entity.id)
        .expect("a tree at (10,6)");
    let villager = world.role_unit("villager").expect("villager");
    world
        .submit_checked(Command::Spawn {
            id: EntityId(9_630),
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: (
                Fixed::from_raw(7 * scale + scale / 2),
                Fixed::from_raw(6 * scale + scale / 2),
            ),
            speed: Fixed::parse("0.8", scale).expect("a speed"),
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    world.step().expect("the villager lands");
    world
        .submit_checked(Command::Gather {
            villager: EntityId(9_630),
            node,
        })
        .expect("a gather order");

    // 0.8 tiles a second at 20 Hz is 0.04 of a tile a tick; a step may not exceed it (with one raw unit of slack for rounding).
    let limit = scale / 20;
    let mut before = world.position(EntityId(9_630)).expect("the villager");
    let mut worst = 0i64;
    for tick in 0..400 {
        world.step().expect("a tick");
        let now = world.position(EntityId(9_630)).expect("the villager");
        let (dx, dy) = (
            (now.0.raw() - before.0.raw()).abs(),
            (now.1.raw() - before.1.raw()).abs(),
        );
        worst = worst.max(dx.max(dy));
        assert!(
            dx <= limit && dy <= limit,
            "the gatherer jumped {dx} raw units in x on tick {tick}, and its speed allows {limit} - it was placed, not walked"
        );
        before = now;
    }
    assert!(worst > 0, "the villager never moved at all");
}

/// **A gatherer who is neither walking nor working asks for a route again** (D200). Measured on the trebuchet fixture: a
/// villager frozen at (16.23,13.80) with a live `Gather` task, a reserved place at (19.80,15.41) and `progress = 0`, for
/// **400,000 ticks** - 3.9 tiles from his work, with no route and nothing that would ever build one. `main` has no such
/// state (it banks the same 400 in 11,359 ticks), so the movement rule introduced it.
#[test]
fn a_gatherer_five_tiles_away_reaches_his_node() {
    let Some(mut world) = world_of(MAP_BUSH) else {
        return;
    };
    let scale = world.scale();
    let node = world
        .entities()
        .iter()
        .find(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .map(|entity| entity.id)
        .expect("a bush at (10,6)");
    let villager = world.role_unit("villager").expect("villager");
    let id = EntityId(9_670);
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: (
                Fixed::from_raw(5 * scale + scale / 2),
                Fixed::from_raw(6 * scale + scale / 2),
            ),
            speed: Fixed::parse("0.8", scale).expect("a speed"),
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    world.step().expect("the villager lands");
    world
        .submit_checked(Command::Gather { villager: id, node })
        .expect("a gather order");
    let tile = world.resource_tiles(node)[0];
    let mut reach = f64::MAX;
    let mut arrived = None;
    for tick in 0..1_500 {
        world.step().expect("a tick");
        let Some(p) = world.position(id) else { break };
        let clearance = ruage_sim::tiles::distance_to_tile(
            p,
            (i64::from(tile.0), i64::from(tile.1)),
            scale,
        );
        reach = reach.min(clearance as f64 / scale as f64);
        if clearance <= scale / 3 {
            arrived = Some(tick);
            break;
        }
    }
    assert!(
        arrived.is_some(),
        "MEASURED he never reached the bush in 1,500 ticks; closest {reach:.2} tiles, at ({:.2},{:.2}); measured frozen on the trebuchet fixture for 400,000 ticks",
        world.position(id).map(|p| p.0.raw() as f64 / scale as f64).unwrap_or(-1.0),
        world.position(id).map(|p| p.1.raw() as f64 / scale as f64).unwrap_or(-1.0)
    );
}

/// **A friend standing still in the path is walked round, not waited on for ever** (D200). The engine asks for a repath round
/// a blocking body only when that body is *not already* in the route's avoid set - and the avoid set's own comment says a
/// collision with a body in it is **past**, "exactly what stops a blocked unit from repathing every tick". Measured on the
/// trebuchet fixture: a villager with a live order, a reserved place and a three-point route sat at `WaitBlocked` for
/// **400,000 ticks** beside a body that never moved, while `main` finished the same work in 11,359.
#[test]
fn a_gatherer_walks_round_a_friend_standing_in_his_path() {
    let Some(mut world) = world_of(MAP_BUSH) else {
        return;
    };
    let scale = world.scale();
    let node = world
        .entities()
        .iter()
        .find(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .map(|entity| entity.id)
        .expect("a bush at (10,6)");
    let villager = world.role_unit("villager").expect("villager");
    let (walker, stander) = (EntityId(9_680), EntityId(9_681));
    for (id, at) in [(walker, (6i64, 6i64)), (stander, (8, 6))] {
        world
            .submit_checked(Command::Spawn {
                id,
                player: 1,
                kind: EntityKind::Unit,
                unit: Some(villager),
                position: (
                    Fixed::from_raw(at.0 * scale + scale / 2),
                    Fixed::from_raw(at.1 * scale + scale / 2),
                ),
                speed: Fixed::parse("0.8", scale).expect("a speed"),
                hp: 25,
                amount: 0,
            })
            .expect("a villager");
        world.step().expect("the villager lands");
    }
    let standing = world.position(stander).expect("the stander");
    world
        .submit_checked(Command::Gather { villager: walker, node })
        .expect("a gather order");
    world.trace_steps();
    let tile = world.resource_tiles(node)[0];
    let mut closest = f64::MAX;
    let mut arrived = None;
    for tick in 0..1_500 {
        world.step().expect("a tick");
        let Some(p) = world.position(walker) else { break };
        let clearance = ruage_sim::tiles::distance_to_tile(
            p,
            (i64::from(tile.0), i64::from(tile.1)),
            scale,
        );
        closest = closest.min(clearance as f64 / scale as f64);
        if clearance <= scale / 3 {
            arrived = Some(tick);
            break;
        }
    }
    assert_eq!(
        world.position(stander),
        Some(standing),
        "the friend was supposed to stand still and was moved instead"
    );
    assert!(
        arrived.is_some(),
        "MEASURED the gatherer never reached the bush in 1,500 ticks; closest {closest:.2} tiles at ({:.2},{:.2}), last step {:?}",
        world.position(walker).map(|p| p.0.raw() as f64 / scale as f64).unwrap_or(-1.0),
        world.position(walker).map(|p| p.1.raw() as f64 / scale as f64).unwrap_or(-1.0),
        world.last_step(walker)
    );
}
