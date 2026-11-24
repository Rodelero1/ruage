// **A DRAFT, not compiled by the workspace** - `docs/` is not a crate. It is the shape the P5 witness has to take, kept here
// because the session that wrote it ran out of time to make it jam: the scene's two orders both arrive, so the fixture measures
// nothing and the engine's own cuts cannot be attributed. What is established, in `docs/parity-plan.md` under P5:
//
//   * a walk onto a tile a body holds cannot be *ordered* - `Command::Move`'s acceptance resolves it to a free neighbour;
//   * a `Command::Spawn` aimed at a held tile is displaced by the placement rules;
//   * so the jam must be made by a body that **walks into** the goal after the walker is under way, and the first thing the next
//     session should do is instrument `work.finished` (step.rs, the `Task::Move` arm) and `end_walk` (motion.rs) with the tick,
//     to attribute the `Idle` that a 200-tick window shows at tick ~144.
//
// The three cuts it is meant to witness, together: `beside`, `finish_blocked_orders`, and the D204 re-task.

//! **An order is never given up by the engine.** The owner, 2026-11-19: *"I want no deliberate differences to original game. If
//! there is any, I want that adjusted to meet the original or cut off."* And on the fork this raised, decided from the binary:
//! *"no engine help: the order stays live and the man stands; only sliding and the pause rule act."*
//!
//! Three rules of ours ended a walk or moved a man, and the original has none of them:
//!
//! - **`beside`** (D200): a walk onto a goal that holds a body was declared finished once the walker came within 1.25 tiles of it.
//! - **`finish_blocked_orders`** (D200): after `STALLED_TICKS` (40) refusals the order was replaced with `Idle`.
//! - **the D204 re-task**: a gatherer refused for the same count was sent to another node of the same kind.
//!
//! Nothing in the binary does any of it: `getMaxVelFwd` (`008a2a30`) returns exactly `0.0` on a pause, the movement pass slides
//! along single axes, and no path replaces an order because a body was in the way. So a jam stands - the order stays live, the
//! man holds his ground, and the player re-tasks.
//!
//! **The scene has to be two orders to one tile**, because `Command::Move`'s acceptance already resolves a click onto a tile a
//! body holds to a free neighbouring tile (measured), and a `Spawn` aimed at the goal is displaced by the placement rules. So the
//! friend is ordered to the same tile from a shorter distance, arrives first, and the walker jams behind him.
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

/// One tree at (10,10) with room on every side, and a second tree far east so the D204 re-task has somewhere to send a man.
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

fn walk(world: &mut World, unit: EntityId, to: (u16, u16)) {
    world
        .submit_checked(Command::Move {
            target: None,
            unit,
            to,
        })
        .expect("a walk to a free tile");
}

/// **A walk that a friend claims first stays live for ever.** Both men are ordered to the same free tile; the nearer one gets
/// there, and the walker behind him is refused from then on - the order is never cancelled, no `beside` completion and no
/// forty-tick finish.
#[test]
fn a_walk_a_friend_claims_first_is_never_finished() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let walker = spawn(&mut world, (8, 10), 9_001);
    walk(&mut world, walker, (13, 10));
    // The friend starts **closer** to the same tile and is ordered a tick later: he gets there first and stands on it.
    let stander = spawn(&mut world, (11, 10), 9_000);
    walk(&mut world, stander, (13, 10));

    for tick in 0..200 {
        world.step().expect("stepping");
        let task = world.entity(walker).expect("the walker").task;
        assert!(
            matches!(task, Task::Move { .. }),
            "at tick {tick} the walk must still be live - the engine never ends a blocked order - not {task:?}"
        );
    }

    // The friend is on the tile and the walker is standing against him, as close as friends allow.
    let (at, to) = (
        world.position(walker).expect("the walker"),
        world.position(stander).expect("the stander"),
    );
    let scale = world.scale();
    assert!(
        at.0.raw() > 8 * scale + scale / 2,
        "the walker must have closed the distance: {} against a start of {}",
        at.0.raw(),
        8 * scale
    );
    let far = ((at.0.raw() - to.0.raw()).pow(2) + (at.1.raw() - to.1.raw()).pow(2)).isqrt();
    assert!(
        far <= scale,
        "he must be standing at his friend, not across the lane: {far} raw units apart"
    );
}

/// **A gatherer is never sent to another node by the engine.** The D204 re-task is the rule being cut: a man the ring refuses
/// stays ordered to the node he was given, as the original does.
#[test]
fn a_gatherer_is_never_retasked_by_the_engine() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let first = world
        .entities()
        .iter()
        .find(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .map(|entity| entity.id)
        .expect("a tree");
    // Twelve men round one tree: more than the comfortable ring, which is what used to trigger the re-task.
    let spots: Vec<(i64, i64)> = (-6i64..=6)
        .flat_map(|dy| (-6i64..=6).map(move |dx| (dx, dy)))
        .filter(|(dx, dy)| (4..=6).contains(&(*dx).abs().max((*dy).abs())))
        .collect();
    let mut men = Vec::new();
    for (index, (dx, dy)) in spots.iter().take(12).enumerate() {
        men.push(spawn(&mut world, (10 + dx, 10 + dy), 9_100 + index as u32));
    }
    for man in &men {
        world
            .submit_checked(Command::Gather {
                villager: *man,
                node: first,
            })
            .expect("a gather order");
    }

    for tick in 0..600 {
        world.step().expect("stepping");
        for man in &men {
            // Only a *node* is measured: the D204 rule sent a refused man to a different one, which shows up as a task
            // naming it. A man the engine could not plan for at all is a different defect, not this fixture's.
            let Some(entity) = world.entity(*man) else {
                continue;
            };
            let node = match entity.task {
                Task::Gather { node, .. } | Task::Return { node, .. } => Some(node),
                _ => None,
            };
            if let Some(node) = node {
                assert!(
                    node == first,
                    "at tick {tick} {man:?} was moved off the node it was given: {node:?} against {first:?}"
                );
            }
        }
    }
}
