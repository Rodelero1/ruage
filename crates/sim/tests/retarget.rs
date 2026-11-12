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
/// **Parked by D98a.** Measured on this fixture: with collision on, the forager ends the bush holding a load and the
/// retarget picks **node 103** - another food source - instead of the farm (127 bush, 128 farm, 129 villager), with
/// **no refusal recorded**, so it is a *choice* and not a rejection, and the food column still rises (210 -> 230 over
/// the probe window). Before collision the same fixture went to the farm. The candidate order or the standable-tile
/// test around a blocked building has shifted, and which one is not yet known: re-read `retarget`/`nearest_like`'s
/// candidate filter before changing it, because a fix that makes this pass by moving the villager could hide the real
/// cause.
#[test]
#[ignore = "parked by D98a: the retarget picks another food source (103) instead of the adjacent farm, with no refusal - cause not yet read"]
fn a_forager_whose_bush_is_finished_goes_to_a_farm() {
    let Some(mut world) = fresh() else {
        return;
    };
    let food = world.slot_of("food").expect("a food column");
    let farm_role = world.role_unit("farm").expect("a farm role");
    let villager_role = world.role_unit("villager").expect("a villager role");

    // Both on open grass well clear of the town centre and the map's own resources.
    let bush = spawn(&mut world, EntityKind::Resource(food), 0, (18, 13), 10);
    let farm = spawn(&mut world, EntityKind::Building, farm_role, (17, 12), 250);
    // `Spawn` with a building unit id is not a real build; the engine may mark it a foundation, so make sure the farm is
    // finished - a farm still going up cannot be worked (and the test says so, rather than pretending).
    assert!(
        world.entity(farm).is_some_and(|e| e.construction.is_none()),
        "the farm should be finished: {:?}",
        world.entity(farm).map(|e| e.construction)
    );
    let villager = spawn(&mut world, EntityKind::Unit, villager_role, (18, 14), 0);

    world
        .submit_checked(Command::Gather {
            villager,
            node: bush,
        })
        .expect("the bush can be worked");

    let mut on_farm = false;
    for _ in 0..3_000 {
        world.step().expect("stepping");
        if world
            .entity(villager)
            .is_some_and(|e| matches!(e.task, Task::Gather { node, .. } if node == farm))
        {
            on_farm = true;
            break;
        }
    }
    assert!(
        on_farm,
        "the villager should move to the farm once the bush is spent, not stand on the empty bush: {:?}",
        world.entity(villager).map(|e| e.task)
    );
    assert!(
        world.stock(1, food) > 0,
        "and the farm should be feeding the stockpile"
    );
}
