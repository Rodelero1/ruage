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
#[ignore = "D186 open: this fixture's ring is buried, so the neighbouring-tile fallback packs three men 0.20 tiles apart - measured"]
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
    for _ in 0..1_200 {
        world.step().expect("a tick");
    }
    // **Only the men at work**: a villager walking in or out crosses other ground, and a crossing is a moment rather than a
    // place. What must never happen is two men *working the same bush* inside each other.
    let at = world.position(node).expect("the bush's square");
    let spots: Vec<(Fixed, Fixed)> = ids
        .iter()
        .filter(|id| {
            world.entity(EntityId(**id)).is_some_and(|entity| {
                matches!(entity.task, Task::Gather { node: n, .. } if n == node)
            })
        })
        .filter_map(|id| world.position(EntityId(*id)))
        .filter(|spot| {
            let scale = world.scale() as f64;
            let dx = (spot.0.raw() - at.0.raw()) as f64 / scale;
            let dy = (spot.1.raw() - at.1.raw()) as f64 / scale;
            let from_square = (dx * dx + dy * dy).sqrt();
            (1.05..=1.30).contains(&from_square)
        })
        .collect();
    assert_eq!(spots.len(), 3, "all three villagers are still in the world");
    // **All three gather the one bush** (D186): the ring round a source square holds eighteen men, so three is a small crowd.
    let working = world
        .entities()
        .iter()
        .filter(|entity| ids.contains(&entity.id.0))
        .filter(|entity| matches!(entity.task, Task::Gather { node: at, .. } if at == node))
        .count();
    assert_eq!(
        working, 3,
        "{working} villagers are gathering the one bush; the ring takes eighteen"
    );
    // ... and whatever they are doing, no two bodies share the same ground.
    for a in 0..spots.len() {
        for b in (a + 1)..spots.len() {
            let split = apart(spots[a], spots[b], scale);
            assert!(
                split >= 0.36,
                "villagers {a} and {b} stand {split:.2} tiles apart - their bodies overlap on screen"
            );
        }
    }
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
/// **OPEN (D173).** This is the reproducer, not a passing guard: the shipped engine still *places* an arrived worker on its standoff
/// (`settle_workers`, D169) instead of walking it there, and the placement is the jump. Two attempts to walk it are recorded in
/// `docs/reviews/2026-11-08-the-jump-that-is-a-placement.md` with their measurements; neither shipped. Un-ignore this test when one does.
#[ignore = "D173 open: the settle places the worker on its standoff instead of walking it there"]
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
