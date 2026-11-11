//! **A second villager joins a foundation that is already there** (D186 addendum 3).
//!
//! The owner's `house.aoe2spgame` carries **four** build orders pointing at the same site - `(76, 78)` four times - which is what
//! clicking a half-built house with a crowd looks like. The engine had no gesture for it at all: `Command::Build` is *place*, and
//! a second placement on the same ground is refused (`SiteOccupied`, with the first builder's name), which is right. So joining
//! is its own command, `Command::JoinBuild`, and this is its guard.
//!
//! Written before the change (D7, SPEC §11).

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, Task, World};

mod common;

fn next_id(world: &World) -> EntityId {
    EntityId(world.entities().iter().map(|e| e.id.0).max().unwrap_or(0) + 1)
}

fn villager_near(world: &mut World, at: (i64, i64)) -> EntityId {
    let scale = world.scale();
    let villager = world.role_unit("villager").expect("a villager role");
    let id = next_id(world);
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

/// **Two men, one foundation, and both of them on it.** The second order is a *join*, not a second building.
#[test]
fn a_second_villager_joins_a_foundation_that_is_already_there() {
    let Some(mut world) = common::fresh() else {
        return;
    };
    let house = world.role_unit("house").expect("a house role");
    let (ax, ay) = (18u16, 18u16);
    for dy in 0..6u16 {
        for dx in 0..6u16 {
            common::look_away(&mut world, 1, (ax + dx, ay + dy));
        }
    }
    let first = villager_near(&mut world, (16, 21));
    world
        .submit_checked(Command::Build {
            villager: first,
            unit: house,
            at: (ax + 2, ay + 2),
        })
        .expect("the first builder places the house");
    world.step().expect("the site lands");
    let site = world
        .entities()
        .iter()
        .find(|entity| entity.construction.is_some() && entity.unit == Some(house))
        .map(|entity| entity.id)
        .expect("the site");

    let second = villager_near(&mut world, (22, 21));
    match world.submit_checked(Command::JoinBuild {
        villager: second,
        site,
    }) {
        Ok(()) => {}
        Err(error) => panic!("MEASURED the second builder's order refused: {error}"),
    }
    world.step().expect("the order lands");

    let task = world.entity(second).expect("the second builder").task;
    assert!(
        matches!(task, Task::Build { site: joined } if joined == site),
        "MEASURED the second builder's task as {task:?}; the foundation at ({}, {}) is his own player's and unfinished, so \
         the order joins it",
        ax + 2,
        ay + 2
    );
}
