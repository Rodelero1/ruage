//! **The movement decision is a value you can read** (D193, D279).
//!
//! `StepDecision` is instrumentation: every body's step says which rule took it or which rule refused it, so "why is this
//! man standing still" is a value rather than an afternoon of printing. The vocabulary is now the original's (D279), so this
//! file states what each rule is *for* rather than enumerating the enum.
//!
//! Written before the implementation of the trimmed vocabulary.

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, StepDecision as Step, World};

mod common;

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

fn spawn_with(world: &mut World, id: EntityId, player: u8, point: (i64, i64)) -> EntityId {
    let scale = world.scale();
    let unit = world.start_villager_unit().expect("a villager role");
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (Fixed::from_raw(point.0), Fixed::from_raw(point.1)),
            speed: Fixed::parse("0.8", scale).expect("a speed"),
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    id
}

fn open_row(world: &World, span: u16) -> Option<(u16, u16)> {
    for y in 2..62u16 {
        for x in 2..62u16 - span {
            if (0..span).all(|d| world.is_free_ground(x + d, y))
                && !((8..24).contains(&x) && (8..24).contains(&y))
            {
                return Some((x, y));
            }
        }
    }
    None
}

/// **A body standing on the goal tile does not stop the walk.** The walker keeps taking steps: the original answers a
/// collision with the order's own target by stopping *beside* it, and every other body by a pause - never by a freeze.
#[test]
fn a_body_on_the_goal_tile_is_walked_to() {
    let Some(mut world) = common::fresh() else {
        return;
    };
    world.trace_steps();
    let Some((x, y)) = open_row(&world, 4) else {
        return;
    };
    let scale = world.scale();
    let friend_id = next_id(&world);
    let friend = spawn_with(
        &mut world,
        friend_id,
        1,
        (i64::from(x + 2) * scale, i64::from(y) * scale),
    );
    let walker = spawn_with(
        &mut world,
        EntityId(friend.0 + 1),
        1,
        (i64::from(x) * scale, i64::from(y) * scale),
    );
    world.step().expect("the spawns land");
    world
        .submit_checked(Command::Move {
            unit: walker,
            to: (x + 2, y),
        })
        .expect("the walk");

    let mut saw_take = false;
    for _ in 0..400 {
        world.step().expect("a tick");
        if matches!(world.last_step(walker), Some(Step::Take)) {
            saw_take = true;
        }
    }
    assert!(
        saw_take,
        "the walker never took a step toward a body standing on its goal tile; last decision {:?}",
        world.last_step(walker)
    );
}

/// **The younger body yields a contested tile, by pausing.** The original's tie-break is one comparison of the two object
/// handles (`cmp eax,esi / jle`): the higher id is told to pause for 0.25-1.50 s and keeps its path. This is the rule that
/// makes a head-on resolve instead of deadlocking, so both halves are asserted - it pauses, and it still walks when clear.
#[test]
fn the_younger_body_yields_the_contested_tile() {
    let Some(mut world) = common::fresh() else {
        return;
    };
    world.trace_steps();
    let Some((x, y)) = open_row(&world, 8) else {
        return;
    };
    let scale = world.scale();
    let older_id = next_id(&world);
    let older = spawn_with(
        &mut world,
        older_id,
        1,
        (i64::from(x) * scale, i64::from(y) * scale),
    );
    let younger = spawn_with(
        &mut world,
        EntityId(older.0 + 1),
        2,
        (i64::from(x + 3) * scale, i64::from(y) * scale),
    );
    world.step().expect("the spawns land");
    world
        .submit_checked(Command::Move {
            unit: older,
            to: (x + 6, y),
        })
        .expect("the older walks");
    world
        .submit_checked(Command::Move {
            unit: younger,
            to: (x, y),
        })
        .expect("the younger walks");

    let mut saw_a_pause = false;
    let mut saw_a_take = false;
    for _ in 0..400 {
        world.step().expect("a tick");
        match world.last_step(younger) {
            Some(Step::Paused) => saw_a_pause = true,
            Some(Step::Take) => saw_a_take = true,
            _ => {}
        }
    }
    assert!(
        saw_a_pause,
        "the younger body was never told to stand for a collision - the pause is the whole tie-break"
    );
    assert!(
        saw_a_take,
        "and it still walks when nobody is in the way: a pause must not become a freeze"
    );
}
