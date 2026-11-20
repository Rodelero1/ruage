//! A forager whose bushes are finished goes to a **farm**, not to a standstill.
//!
//! The owner's rule - "move to next bush, next tree, next pile of gold..." - names the next *source*, and once the pack's
//! berries are gone the next source of food is a farm, which is a **building**, not a resource node. `nearest_like` only
//! compared resource nodes with resource nodes, so eighteen villagers sat on the empty berry patch at 2,400 s with nothing
//! to do and the acceptance match never spent a stick of wood again.
//!
//! Written before the fix (D7).

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, Task, World};

mod common;

fn fresh() -> Option<World> {
    common::fresh()
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

/// Load a world from a map this fixture owns. `None` only when the generated pack is absent, like `common::fresh`.
fn world_of(map_text: &str) -> Option<World> {
    let units_file = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("config/gamedata/units.toml");
    if !units_file.exists() {
        return None;
    }
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let resources = common::present(
        &root.join("config/resources.toml"),
        ruage_data::load_resources_file,
    );
    let settings = common::present(&root.join("config/sim.toml"), ruage_data::load_sim_file);
    let map = common::must(
        "load_map_str",
        ruage_data::load_map_str(map_text, "retarget.toml", &resources),
    );
    let economy = common::present(
        &root.join("config/economy.toml"),
        ruage_data::load_economy_file,
    );
    let units = common::must("load_units_file", ruage_data::load_units_file(&units_file));
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
    Some(world)
}

fn spawn(world: &mut World, kind: EntityKind, unit: u32, at: (u16, u16), amount: u32) -> EntityId {
    let id = next_id(world);
    let scale = world.scale();
    let pace = Fixed::from_raw(scale / 2);
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind,
            unit: Some(unit),
            position: (
                Fixed::from_raw(i64::from(at.0) * scale),
                Fixed::from_raw(i64::from(at.1) * scale),
            ),
            speed: if kind == EntityKind::Unit {
                pace
            } else {
                Fixed::ZERO
            },
            hp: if kind == EntityKind::Unit { 25 } else { 100 },
            amount,
        })
        .expect("placed");
    world.step().expect("one tick");
    id
}

/// A bush with one load in it, a finished farm beside it: when the bush runs out the villager works the farm, and the
/// food in the bank goes up.
///
/// **Re-measured green on 2026-11-15** (D224). Parked by D98a because the retarget picked another food source (node 103)
/// instead of the adjacent farm with no refusal recorded; the parked fixture now does what it says - the forager ends the
/// bush, walks to the farm, and the food column rises. The parked sentence did not say which change was missing, and this
/// audit did not bisect it: what is claimed is only what was run, `--include-ignored`, today. It is un-parked so the suite
/// keeps it that way.
/// **A forager whose bush is spent does not walk to a farm.** The engine's own answer to "which source is next" is the nearest
/// node of the **same kind** within the worker's reach - the binary's radius is line of sight plus five tiles (D149), which is how a
/// spent tree finds the next tree. Repurposing a man across resource *types* is the **AI's** job: the game ships
/// `sn-retask-gather-amount` / `sn-max-retask-gather-amount` and a technology whose only job is to improve it, `up-retask-gatherers`.
///
/// Owner, 2026-11-19: *"The game code itself should never hardcode such move. That should be handled by AI to repurpose idle
/// villagers. Is it the case?"* - it is.
///
/// **The map has to hold exactly one bush.** The first version of this fixture ran on the seeded map, which carries its own berry
/// patches within reach, so the spent bush's same-kind search found one of them and the test was measuring the map, not the rule
/// (measured: the forager moved to the map's bush `EntityId(105)` at step 2,200).
#[test]
fn a_forager_whose_bush_is_finished_stands_rather_than_walking_to_a_farm() {
    // **One bush, one farm, nothing else to eat** - the whole point is what happens when the same-kind search has nowhere to go.
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
amount = 10
hp = 0
tiles = [[12,10]]
[[map.town_center]]
player = 1
x = 1
y = 1
[[map.town_center]]
player = 2
x = 20
y = 20
"#;
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let food = world.slot_of("food").expect("a food column");
    let farm_role = world.role_unit("farm").expect("a farm role");
    let villager_role = world.role_unit("villager").expect("a villager role");
    let bush = world
        .entities()
        .iter()
        .find(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .map(|entity| entity.id)
        .expect("the one bush");
    // **A finished farm right beside him**: if the engine were willing to cross resource kinds, this is what it would take.
    let farm = spawn(&mut world, EntityKind::Building, farm_role, (11, 9), 250);
    assert!(
        world.entity(farm).is_some_and(|e| e.construction.is_none()),
        "the farm should be finished"
    );
    let villager = spawn(&mut world, EntityKind::Unit, villager_role, (12, 11), 0);
    world
        .submit_checked(Command::Gather {
            villager,
            node: bush,
        })
        .expect("the bush can be worked");

    let mut ever_on_farm = false;
    for _ in 0..3_000 {
        world.step().expect("stepping");
        if world
            .entity(villager)
            .is_some_and(|e| matches!(e.task, Task::Gather { node, .. } if node == farm))
        {
            ever_on_farm = true;
            break;
        }
    }
    let task = world.entity(villager).map(|e| e.task);
    assert!(
        !ever_on_farm,
        "the engine sent the forager to a farm by itself: {task:?} - reassigning across resource types is the AI's job, and the \
         game ships `sn-retask-gather-amount` for it"
    );
    // **And he has nothing left to gather**: no bush in reach, and a farm is not a bush.
    assert!(
        !matches!(task, Some(Task::Gather { .. })),
        "with no bush left in reach the forager must not be gathering anything: {task:?}"
    );
    assert!(
        !matches!(task, Some(Task::Return { .. })),
        "the forager is still delivering to a source he no longer has: {task:?}"
    );
    // **The load he was carrying is banked** - the man stops after his delivery, not before it.
    assert!(
        world.stock(1, food) > 10,
        "the ten food of the bush never reached the stockpile: {}",
        world.stock(1, food)
    );
}
