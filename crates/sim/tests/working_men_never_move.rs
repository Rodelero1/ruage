//! **A man who is working is never moved by the engine.** The owner, rule 4 (*"a working man must not move"*), restated with the
//! parity order of 2026-11-19: *"I want no deliberate differences to original game."*
//!
//! The seat book re-lays a ring whenever a man at a node is refused (`maybe_respread`, D200/D202), and a newcomer's arrival
//! displaces working men through it. The original places a man **once**, when the order is given: after that his body is solid
//! and nobody shuffles him. So this measures the thing P8 (`docs/archive/research/parity-plan.md`) must hold: with men working one tree, a man
//! who has arrived does not move by a single raw unit.
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

/// One tree at (10,10) with room on every side; the town centres are far enough not to matter.
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
  "GGGGGGGGGGFGGGGGGGGGGG",
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
        ruage_data::load_map_str(map_text, "working.toml", &resources),
    );
    let economy: Economy =
        common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let units: Units = common::must("load_units_file", ruage_data::load_units_file(&units_file));
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
    Some(world)
}

fn the_tree(world: &World) -> EntityId {
    world
        .entities()
        .iter()
        .find(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .map(|entity| entity.id)
        .expect("a tree at (10,10)")
}

/// Six villagers dropped three to five tiles out on every side, all ordered to the tree.
fn woodcutters(world: &mut World, count: usize) -> Vec<EntityId> {
    let scale = world.scale();
    let villager = world.role_unit("villager").expect("villager");
    let node = the_tree(world);
    let ring: Vec<(i64, i64)> = (-5i64..=5)
        .flat_map(|dy| (-5i64..=5).map(move |dx| (dx, dy)))
        .filter(|(dx, dy)| (3..=5).contains(&(*dx).abs().max((*dy).abs())))
        .collect();
    let mut ids = Vec::new();
    for (index, (dx, dy)) in ring.iter().take(count).enumerate() {
        let id = EntityId(8_000 + index as u32);
        world
            .submit_checked(Command::Spawn {
                id,
                player: 1,
                kind: EntityKind::Unit,
                unit: Some(villager),
                position: (
                    Fixed::from_raw((10 + dx) * scale + scale / 2),
                    Fixed::from_raw((10 + dy) * scale + scale / 2),
                ),
                speed: Fixed::parse("0.8", scale).expect("a speed"),
                hp: 25,
                amount: 0,
            })
            .expect("a villager");
        world.step().expect("the villager lands");
        ids.push(id);
    }
    for id in &ids {
        world
            .submit_checked(Command::Gather {
                villager: *id,
                node,
            })
            .expect("a gather order");
    }
    ids
}

/// **A man who has stopped at the node and is working does not move again.** Fourteen men on one tree - more than the seven
/// the comfortable ring seats - because that is the crowd the re-lay exists for (D200/D246: *"a tenth can only be seated if the
/// seated seven shuffle"*).
///
/// Three things legitimately move a man at the tree: his own walk in, the last third of a tile onto the node
/// (`settle_workers`), and nothing else. So the measurement waits for a man to be **in `Gather` and still for twenty ticks** -
/// he has arrived and settled - and from then on any raw unit he moves is the engine moving a working man.
#[test]
fn a_working_man_is_never_moved() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let men = woodcutters(&mut world, 14);
    let node = the_tree(&world);
    let scale = world.scale();
    let tree = world.position(node).expect("the tree");
    // Where the man is, if he is **working** at this tree - `progress > 0` is the same definition D246 uses for a seat that
    // may not be moved - and `None` otherwise. A man queuing with progress 0 is still walking to his place, which is his own
    // step and not the engine's.
    let working_at = |world: &World, id: EntityId| -> Option<(i64, i64)> {
        let entity = world.entity(id)?;
        if !matches!(entity.task, Task::Gather { progress, .. } if progress.raw() > 0) {
            return None;
        }
        let (dx, dy) = (
            entity.position.0.raw() - tree.0.raw(),
            entity.position.1.raw() - tree.1.raw(),
        );
        ((dx * dx + dy * dy).isqrt() <= scale * 3 / 2)
            .then(|| (entity.position.0.raw(), entity.position.1.raw()))
    };

    // Ticks each man has been standing still while gathering.
    let mut still_for = vec![0u32; men.len()];
    let mut previous: Vec<Option<(i64, i64)>> = vec![None; men.len()];
    let mut shuffled: Vec<(EntityId, u64, i64)> = Vec::new();
    let mut measured_ticks = 0u64;
    for tick in 0..1600u64 {
        world.step().expect("stepping");
        for (index, id) in men.iter().enumerate() {
            match working_at(&world, *id) {
                None => {
                    still_for[index] = 0;
                    previous[index] = None;
                }
                Some(at) => {
                    let moved = previous[index].map_or(0, |was| {
                        ((at.0 - was.0).pow(2) + (at.1 - was.1).pow(2)).isqrt()
                    });
                    if moved > 0 {
                        // He moved. If he had been still and working for a second, the engine did it.
                        if still_for[index] >= 20 {
                            shuffled.push((*id, tick, moved));
                        }
                        still_for[index] = 0;
                    } else {
                        still_for[index] += 1;
                        if still_for[index] >= 20 {
                            measured_ticks += 1;
                        }
                    }
                    previous[index] = Some(at);
                }
            }
        }
    }
    assert!(
        measured_ticks > 1000,
        "the fixture has to see men standing still at work for it to measure anything: {measured_ticks} man-ticks"
    );
    assert!(
        shuffled.is_empty(),
        "a working man must not be moved by the engine: {} moved, worst {:?}",
        shuffled.len(),
        shuffled.iter().max_by_key(|(_, _, raw)| *raw)
    );
}
