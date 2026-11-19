//! **The step pass** (D279): one whole step a tick, and the four refusal rules.
//!
//! These fixtures pin the replacement for `world/movement.rs`'s `collide` - the state machine that had ten answers
//! (slide, at-rest, overlap, pause, wait-by-reason ...). The original has four, and this is them: the order's own
//! target, the ground, a body that has not moved since the route was planned, and everything else, which refuses the
//! step and pauses the **higher id** for 0.25-1.50 s.
//!
//! Written before the implementation. Every fixture here is a **scene** the owner can watch, not a rule stated in the
//! abstract: a man walking a clear line, two men swapping places, a man sent at a body, a walker behind a crowd.

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, Task, World};

mod common;

fn fresh() -> Option<World> {
    common::fresh_plain()
}

/// Free ground in the seed's open field: `width` x `height` tiles away from the woods and the base.
fn open_block(world: &World, width: u16, height: u16) -> Option<(u16, u16)> {
    for y in 2..60u16 {
        for x in 2..60u16 {
            if (24..40).contains(&x) && (24..40).contains(&y) {
                continue;
            }
            if (0..height).all(|dy| (0..width).all(|dx| world.is_free_ground(x + dx, y + dy))) {
                return Some((x, y));
            }
        }
    }
    None
}

fn spawn(world: &mut World, tile: (u16, u16), speed: i64) -> EntityId {
    let id = EntityId(world.entities().iter().map(|e| e.id.0).max().unwrap_or(0) + 1);
    let unit = world.start_villager_unit().expect("a villager role");
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                Fixed::from_raw(i64::from(tile.0) * scale),
                Fixed::from_raw(i64::from(tile.1) * scale),
            ),
            speed: Fixed::from_raw(speed),
            hp: 25,
            amount: 0,
        })
        .expect("a villager can be placed");
    world.step().expect("the spawn lands");
    id
}

fn centre(world: &World, tile: (u16, u16)) -> (Fixed, Fixed) {
    let scale = world.scale();
    (
        Fixed::from_raw(i64::from(tile.0) * scale + scale / 2),
        Fixed::from_raw(i64::from(tile.1) * scale + scale / 2),
    )
}

fn gap(a: (Fixed, Fixed), b: (Fixed, Fixed)) -> i64 {
    let (dx, dy) = (b.0.raw() - a.0.raw(), b.1.raw() - a.1.raw());
    let squared = dx * dx + dy * dy;
    let mut root = 0i64;
    while (root + 1) * (root + 1) <= squared {
        root += 1;
    }
    root
}

#[test]
fn a_walk_takes_whole_steps_and_never_a_partial_one() {
    // The owner's rule: *"man stands or walks. No acceleration or deceleration"*. The measurement of it is that every
    // body-tick is either a full `speed / tick_rate` or a stand - never a fraction.
    let Some(mut world) = fresh() else { return };
    let Some(at) = open_block(&world, 10, 2) else {
        return;
    };
    let speed = world.scale(); // one tile a second
    let step = speed / i64::from(world.tick_rate_hz());
    let id = spawn(&mut world, at, speed);
    let goal = (at.0 + 8, at.1);
    world
        .submit_checked(Command::WalkTo {
            unit: id,
            to: centre(&world, goal),
        })
        .expect("the order is accepted");

    let mut walked = 0;
    for _ in 0..400 {
        let before = world.position(id).expect("the walker exists");
        world.step().expect("a tick");
        let after = world.position(id).expect("the walker exists");
        let moved = gap(before, after);
        // **The last step of a walk is the arrival**, and the task layer stops the man within one step of the goal - so the
        // final landing is short by design. Every step before it is whole: the task's own delta is rounded per axis, so
        // "whole" measures within a hair of `step`, and a step spent landing on a corner is an eighth to a half of one.
        let arrived = matches!(world.entity(id).map(|e| e.task), Some(Task::Idle));
        assert!(
            arrived || moved == 0 || moved + world.scale() / 100 >= step,
            "a step is whole or not taken: moved {moved} of a {step} step"
        );
        if moved > 0 {
            walked += 1;
        }
        if arrived {
            break;
        }
    }
    assert!(walked > 100, "he walked: {walked} steps");
}

#[test]
fn two_men_who_swap_places_do_not_deadlock() {
    // The original's answer to a head-on is the pause: the higher id stands for up to a second and a half, and once he
    // has stood, the mover's route (planned before the pause) treats him as *past* and walks on. A deadlock here is the
    // failure this whole model exists to avoid.
    let Some(mut world) = fresh() else { return };
    let Some(at) = open_block(&world, 12, 3) else {
        return;
    };
    let speed = world.scale();
    let a = spawn(&mut world, (at.0, at.1 + 1), speed);
    let b = spawn(&mut world, (at.0 + 10, at.1 + 1), speed);
    let (goal_a, goal_b) = ((at.0 + 10, at.1 + 1), (at.0, at.1 + 1));
    world
        .submit_checked(Command::WalkTo {
            unit: a,
            to: centre(&world, goal_a),
        })
        .expect("a's order");
    world
        .submit_checked(Command::WalkTo {
            unit: b,
            to: centre(&world, goal_b),
        })
        .expect("b's order");

    for _ in 0..600 {
        world.step().expect("a tick");
    }
    // Each man is nearer the other's start than his own: they got past each other.
    let pa = world.position(a).expect("a exists");
    let pb = world.position(b).expect("b exists");
    let start_a = centre(&world, (at.0, at.1 + 1));
    let start_b = centre(&world, (at.0 + 10, at.1 + 1));
    assert!(
        gap(pa, start_b) < gap(pa, start_a),
        "a never got past b: at {pa:?}, needed to close on {start_b:?}"
    );
    assert!(
        gap(pb, start_a) < gap(pb, start_b),
        "b never got past a: at {pb:?}, needed to close on {start_a:?}"
    );
}

#[test]
fn a_man_sent_at_a_body_stops_beside_it() {
    // The order's own target is never walked through: a man sent at another body arrives beside it. This is the one
    // refusal that is not a failure, so it must not be answered with a pause or a re-plan.
    let Some(mut world) = fresh() else { return };
    let Some(at) = open_block(&world, 12, 3) else {
        return;
    };
    let speed = world.scale();
    let sitter = spawn(&mut world, (at.0 + 6, at.1 + 1), speed);
    let walker = spawn(&mut world, (at.0, at.1 + 1), speed);
    // No order for `sitter`: he stands where he was placed.
    world
        .submit_checked(Command::WalkTo {
            unit: walker,
            to: centre(&world, (at.0 + 6, at.1 + 1)),
        })
        .expect("the order is accepted");
    for _ in 0..600 {
        world.step().expect("a tick");
    }
    let there = world.position(sitter).expect("the sitter exists");
    let here = world.position(walker).expect("the walker exists");
    let apart = gap(here, there);
    let reach = world.body_radius_of(walker) + world.body_radius_of(sitter);
    assert!(
        apart >= reach.max(world.scale() / 2),
        "the walker did not walk into the body he was sent at: {apart} apart, reach {reach}"
    );
    assert!(
        apart <= 2 * world.scale(),
        "and he did arrive beside it: {apart} apart"
    );
}

#[test]
fn a_walker_gets_past_a_body_that_stands_in_his_way() {
    // A body at rest is *past* to a route planned after it stopped, so the mover walks on - the original drops a
    // collider that has not moved. The failure this forbids is the freeze: a man standing in a lane for ever.
    let Some(mut world) = fresh() else { return };
    let Some(at) = open_block(&world, 12, 3) else {
        return;
    };
    let speed = world.scale();
    let _idler = spawn(&mut world, (at.0 + 5, at.1 + 1), speed);
    let walker = spawn(&mut world, (at.0, at.1 + 1), speed);
    let goal = (at.0 + 10, at.1 + 1);
    world
        .submit_checked(Command::WalkTo {
            unit: walker,
            to: centre(&world, goal),
        })
        .expect("the order is accepted");
    for _ in 0..800 {
        world.step().expect("a tick");
    }
    let here = world.position(walker).expect("the walker exists");
    assert!(
        gap(here, centre(&world, goal)) <= world.scale(),
        "he reached the far side of a standing body: at {here:?}"
    );
}
