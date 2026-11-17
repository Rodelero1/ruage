//! **A man stands, or he walks** (D252/D255). The owner, 2026-11-17: *"man stands or walks. No acceleration or deceleration"* -
//! and, when the slide was found: *"Follow the game. No sliding."* One step is `speed / tick_rate` raw units, so the observable
//! form of constant speed is that every walking tick covers exactly one step, or none.
//!
//! On every tick a villager is walking, the **distance he covers along his route** is one whole step; the only short tick allowed
//! is the last one, which lands on the goal and may not overshoot it. "Along his route" and not "in a straight line", because a
//! corner is walked *through* inside a tick (`World::fold_the_step`), and the straight line across a fold is shorter than the
//! path walked.
//!
//! Measured before the fix (`step_speed_probe`, 5 villagers, 4,000 ticks): 15 of 20,000 body-ticks moved 0.11-0.95 of a step,
//! 14 of them at a route transit on this very walk - each one a corner whose landing ate the rest of that tick.
//!
//! **Never skipped.** The owner's rule and this fixture stay live: no `#[ignore]`, and no reason to add one - the fold that
//! makes it green is in the mover (`World::fold_the_step`, D252), and it fires only for the walking tasks (`Move`, `Hunt`),
//! because a gather's approach is steered by `settle_workers` as well and folding there let the two passes undo each other.

mod common;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, Task, World};

/// A tile's middle, which is where a walk ordered to a tile ends: `World::tile_centre` is private to the engine.
fn centre(world: &World, (x, y): (u16, u16)) -> (Fixed, Fixed) {
    let scale = world.scale();
    (
        Fixed::from_raw(i64::from(x) * scale + scale / 2),
        Fixed::from_raw(i64::from(y) * scale + scale / 2),
    )
}

/// The distance between two points, in raw units.
fn apart(a: (Fixed, Fixed), b: (Fixed, Fixed)) -> f64 {
    let (dx, dy) = (b.0.raw() - a.0.raw(), b.1.raw() - a.1.raw());
    ((dx * dx + dy * dy) as f64).sqrt()
}

/// A villager at a tile's middle.
fn villager(world: &mut World, id: EntityId, at: (u16, u16)) -> bool {
    let Some(unit) = world.role_unit("villager") else {
        return false;
    };
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: centre(world, at),
            speed: Fixed::parse("0.8", world.scale()).expect("a speed"),
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    world.step().expect("he lands");
    true
}

/// **Every walking tick is a whole step.** A walk across the pack's map from (5,5) to (30,30): the route bends round the
/// forest, so it is several legs - and a leg boundary is exactly where the mover used to land on the corner.
#[test]
fn a_walking_villager_takes_whole_steps() {
    let Some(mut world) = common::fresh() else {
        return;
    };
    let id = EntityId(9_700);
    assert!(villager(&mut world, id, (5, 5)), "a villager");
    world
        .submit_checked(Command::Move {
            unit: id,
            to: (30, 30),
        })
        .expect("a walk order");

    let step = world.entity(id).map_or(0, |entity| {
        entity.speed.raw() / i64::from(world.tick_rate_hz().max(1))
    });
    assert!(step > 0, "the villager has no step to measure");
    let start = centre(&world, (5, 5));
    // An order is applied at the head of the next tick, so the route exists only after one step - and that step is part of
    // the walk, measured from where he stood.
    world.step().expect("the order is applied");
    let route = world.route_points(id);
    // **The ticks the geometry owes this walk**: the route's own length, one step at a time. A corner that costs part of a
    // step costs a whole extra tick, so this number is the sharpest form of "he never slows down".
    let mut route_length = 0.0;
    let mut at = start;
    for point in &route {
        route_length += apart(at, *point);
        at = *point;
    }
    let owed = (route_length / step as f64).ceil() as u32;
    assert!(
        owed > 100,
        "the walk is too short to measure: {owed} ticks owed"
    );

    let mut previous = world.position(id).expect("he stands somewhere");
    let mut previous_route = world.route_points(id);
    let mut walking_ticks = 0u32;
    let mut short_ticks = 0u32;
    let mut first_short: Option<(u64, f64)> = None;
    for _ in 0..1_400 {
        world.step().expect("a tick");
        let Some(here) = world.position(id) else {
            break;
        };
        let after = world.route_points(id);
        // **The walk along the route**: the points the tick spent, then the leg from the last of them to where he stands.
        let spent = previous_route.len().saturating_sub(after.len());
        let mut walked = 0.0;
        let mut point_at = previous;
        for point in previous_route.iter().take(spent) {
            walked += apart(point_at, *point);
            point_at = *point;
        }
        walked += apart(point_at, here);
        previous_route = after;
        previous = here;
        if !matches!(
            world.entity(id).map(|entity| entity.task),
            Some(Task::Move { .. })
        ) {
            continue;
        }
        walking_ticks += 1;
        // A whole step, or a stand (a refused step, a pause, a body in the way).
        if walked <= 1.0 || (walked - step as f64).abs() <= 1.0 {
            continue;
        }
        // **The last step lands**, so it is the remainder of the route and shorter than a step by design.
        if world.path_length(id) == 0 && walked < step as f64 {
            continue;
        }
        short_ticks += 1;
        if first_short.is_none() {
            first_short = Some((world.tick(), walked));
        }
    }

    assert!(
        walking_ticks > 100,
        "the walk only had {walking_ticks} walking ticks, so nothing was measured"
    );
    if let Some((tick, walked)) = first_short {
        panic!(
            "tick {tick}: the villager walked {walked:.0} raw units along his route where one step is {step} - a walk is a \
             whole step or a stand, and {short_ticks} of his {walking_ticks} walking ticks were neither"
        );
    }
    // **And the walk costs the ticks it owes**, not one per corner: the fold is what buys this, and without it a bend costs a
    // whole tick (measured on the same walk: 14 corners, 14 extra ticks).
    assert!(
        walking_ticks <= owed + 2,
        "the walk took {walking_ticks} ticks where {route_length:.0} raw at {step} a step owes {owed} - a corner is \
         costing whole ticks"
    );
}

/// **A short walk still lands on its goal exactly.** The fold must not overshoot the last point, and arrival is an equality
/// at the task layer: `position == goal`.
#[test]
fn a_short_walk_lands_on_its_goal() {
    let Some(mut world) = common::fresh() else {
        return;
    };
    let id = EntityId(9_701);
    assert!(villager(&mut world, id, (20, 20)), "a villager");
    world
        .submit_checked(Command::Move {
            unit: id,
            to: (22, 22),
        })
        .expect("a short walk");
    let goal = centre(&world, (22, 22));
    let mut last = world.position(id).expect("he stands somewhere");
    for _ in 0..200 {
        world.step().expect("a tick");
        let Some(here) = world.position(id) else {
            break;
        };
        last = here;
        if world.path_length(id) == 0 {
            break;
        }
    }
    assert_eq!(
        last,
        goal,
        "the walk ended at ({}, {}) instead of its goal ({}, {})",
        last.0.raw(),
        last.1.raw(),
        goal.0.raw(),
        goal.1.raw()
    );
}
