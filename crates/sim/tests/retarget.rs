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
/// node of the **same kind** within the worker's reach - the binary's radius is line of sight plus five tiles (D149) and it is what
/// a spent tree uses to find the next tree. Repurposing a man across resource *types* is the **AI's** job, not the engine's: the game
/// ships AI parameters for exactly that (`sn-retask-gather-amount`, `sn-max-retask-gather-amount`) and even a technology that
/// improves it (`up-retask-gatherers`). So with no bush left in reach, the forager **stands**, and the fixture says so.
///
/// Owner, 2026-11-19: *"The game code itself should never hardcode such move. That should be handled by AI to repurpose idle
/// villagers. Is it the case?"* - it is.
#[test]
fn a_forager_whose_bush_is_finished_stands_rather_than_walking_to_a_farm() {
    let Some(mut world) = fresh() else {
        return;
    };
    let food = world.slot_of("food").expect("a food column");
    let farm_role = world.role_unit("farm").expect("a farm role");
    let villager_role = world.role_unit("villager").expect("a villager role");
    let bush = spawn(&mut world, EntityKind::Resource(food), 0, (18, 13), 10);
    let farm = spawn(&mut world, EntityKind::Building, farm_role, (17, 12), 250);
    assert!(
        world.entity(farm).is_some_and(|e| e.construction.is_none()),
        "the farm should be finished"
    );
    let villager = spawn(&mut world, EntityKind::Unit, villager_role, (18, 14), 0);
    world
        .submit_checked(Command::Gather {
            villager,
            node: bush,
        })
        .expect("the bush can be worked");

    // **Long enough for the bush to run out and for any retarget to have happened** - and for a farm beside him to have been taken
    // if the engine were willing to cross kinds.
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
        "the engine sent the forager to a farm by itself: {task:?} - reassigning across resource types is the AI's job, and the\
         game ships `sn-retask-gather-amount` for it"
    );
    assert!(
        !matches!(task, Some(Task::Gather { .. })),
        "with no bush left in reach the forager must not be gathering anything: {task:?}"
    );
}
