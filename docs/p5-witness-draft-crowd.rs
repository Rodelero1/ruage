// **A DRAFT, not compiled by the workspace.** The crowded-goal witness for P5: eight men ordered to one tile. It is the scene
// that finally bites - `ZZREACHED EntityId(9103) tick 94 ...` and `ZZFINISHED tick 94` on the unmodified engine - but it cannot
// separate the rules, because **three** end a walk and only two of them are ours to cut:
//
//   1. `beside` (D200) - cut;
//   2. `finish_blocked_orders` (D200) - cut;
//   3. `reaches(position, to, one step)` - the arrival test itself, which is *generous*: a man **within one step of the goal
//      counts as arrived**, so several men can satisfy it and the order completes legitimately. With 1 and 2 cut the fixture
//      stayed red, and the man it named had REACHED at tick 94 - and had also been `Idle` at 86 in a run before, i.e. something
//      re-issues his order after the engine stood him down. That re-issue is the next thing to find, and it is what the witness
//      needs before P5 can land.
//
// Kept here rather than in `crates/sim/tests/` because a red fixture may not sit in the suite (D249).

//! **An order is never given up by the engine.** The owner, 2026-11-19: *"I want no deliberate differences to original game. If
//! there is any, I want that adjusted to meet the original or cut off."* The evidence (`docs/gather-seating-evidence.md`) says the
//! original never resolves a wedged walk - the game's most-reported bug is villagers *"frozen in place"*, *"stuck blocking each
//! other"* - so the forty-tick `finish_blocked_orders` (D200) and the `beside` completion are ours and go.
//!
//! The scene has to crowd the goal, because a single friend is walked *past*: two men ordered to one tile pass each other (measured
//! - the walker arrived at the goal's centre at tick 120 while his friend was displaced a tile and a half north-west). Eight men
//! converging on one tile do not pass: one arrives, the rest are refused, and the old rule finished all of theirs.
//!
//! Written before the implementation (D7, SPEC §11).

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

/// Open ground, one tree far away so nothing else competes, and the town centres out of reach.
const MAP: &str = r#"
schema = 1
[map]
width = 22
height = 22
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
  "GGGGGGGGGGFGGGGGGFGGGG",
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
kind = "wood"
amount = 50000
hp = 0
tiles = [[10,10]]
[[map.resource]]
kind = "wood"
amount = 50000
hp = 0
tiles = [[17,10]]
[[map.town_center]]
player = 1
x = 1
y = 1
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
    let settings: Settings = common::present(&repo("config/sim.toml"), ruage_data::load_sim_file);
    let map: Map = common::must(
        "load_map_str",
        ruage_data::load_map_str(map_text, "orders.toml", &resources),
    );
    let economy: Economy =
        common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let units: Units = common::must("load_units_file", ruage_data::load_units_file(&units_file));
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
    Some(world)
}

fn spawn(world: &mut World, at: (i64, i64), id: u32) -> EntityId {
    let scale = world.scale();
    let villager = world.role_unit("villager").expect("villager");
    let id = EntityId(id);
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
    id
}

/// **Eight men, one tile: the seven who cannot get there keep their order for ever.**
#[test]
fn a_crowded_walk_is_never_finished() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let goal = (6u16, 14u16);
    // On a ring three to four tiles out, so every man's route is short and they all arrive together.
    let spots: Vec<(i64, i64)> = (-4i64..=4)
        .flat_map(|dy| (-4i64..=4).map(move |dx| (dx, dy)))
        .filter(|(dx, dy)| (3..=4).contains(&(*dx).abs().max((*dy).abs())))
        .collect();
    let mut men = Vec::new();
    for (index, (dx, dy)) in spots.iter().take(8).enumerate() {
        men.push(spawn(
            &mut world,
            (i64::from(goal.0) + dx, i64::from(goal.1) + dy),
            9_100 + index as u32,
        ));
    }
    for man in &men {
        world
            .submit_checked(Command::Move {
                target: None,
                unit: *man,
                to: goal,
            })
            .expect("a walk to the same tile");
    }

    // **Two hundred ticks: five times the old forty-tick window.**
    for tick in 0..200 {
        world.step().expect("stepping");
        for man in &men {
            let task = world.entity(*man).expect("alive").task;
            assert!(
                matches!(task, Task::Move { .. }),
                "at tick {tick} {man:?} was given up by the engine: {task:?} - the original never finishes a wedged walk"
            );
        }
    }

    // They did move: at least one man reached the goal and the others closed on it.
    let scale = world.scale();
    let reached = men
        .iter()
        .filter(|man| {
            let at = world.position(**man).expect("alive");
            (at.0.raw() - i64::from(goal.0) * scale).abs() <= scale / 2
                && (at.1.raw() - i64::from(goal.1) * scale).abs() <= scale / 2
        })
        .count();
    assert_eq!(reached, 1, "exactly one man holds the tile, the rest stand behind him");
}
