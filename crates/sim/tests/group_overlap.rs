//! **A group moves as a group** (D207): units ordered together in one tick may walk over each other, and anybody else is
//! solid. The original's rule, read out of the executable and stated by its own developers on the bug forum: the path
//! search avoids only the mover's *moving selection-mates*, and "units give up their formation (and therefore block each
//! other) when: stop button is pressed / click on enemy unit / attack move".
//!
//! Written before the implementation (D7). Our engine has no selection, so the interim rule (owner's call, 2026-11-14) is
//! **orders submitted in the same tick form the group**; an explicit group id on the command comes later.

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

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

/// A villager placed by hand, at a tile **corner** (the engine's spawn convention), with the id chosen by the caller so
/// two spawns in one tick cannot collide on the counter.
fn spawn_villager(world: &mut World, id: EntityId, x: u16, y: u16) {
    let unit = world.start_villager_unit().expect("a villager role");
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                Fixed::from_raw(i64::from(x) * scale),
                Fixed::from_raw(i64::from(y) * scale),
            ),
            speed: Fixed::from_raw(scale / 2),
            hp: 25,
            amount: 0,
        })
        .expect("a villager can be placed");
}

/// An open row on the project map, away from the seeded base, so the fixture is derived from the content rather than
/// hard-coded and cannot rot.
fn open_row(world: &World, span: u16) -> Option<(u16, u16)> {
    let (width, height) = (64u16, 64u16);
    for y in 2..height.saturating_sub(2) {
        for x in 2..width.saturating_sub(span) {
            if (0..span).all(|d| world.is_free_ground(x + d, y)) {
                if (8..24).contains(&x) && (8..24).contains(&y) {
                    continue;
                }
                return Some((x, y));
            }
        }
    }
    None
}

fn distance(world: &World, a: EntityId, b: EntityId) -> i64 {
    let (ax, ay) = world.position(a).expect("a has a position");
    let (bx, by) = world.position(b).expect("b has a position");
    let (dx, dy) = (ax.raw() - bx.raw(), ay.raw() - by.raw());
    dx * dx + dy * dy
}

fn tile_of(world: &World, id: EntityId) -> (i64, i64) {
    let scale = world.scale();
    let (x, y) = world.position(id).expect("the entity has a position");
    (x.raw().div_euclid(scale), y.raw().div_euclid(scale))
}

/// What a run measured: how close the two ever came, and whether each got to its goal.
struct Run {
    closest: i64,
    reach: i64,
    at: ((i64, i64), (i64, i64)),
    tick: u64,
    /// Whether the **higher id** stood a drawn pause at any point (D208): a mate never does, because an exempt collider is
    /// dropped before the tie-break runs.
    paused: bool,
}

fn run(world: &mut World, a: EntityId, b: EntityId, ticks: u32) -> Run {
    let reach = world.reach_between(a, b).raw();
    let mut closest = i64::MAX;
    let mut paused = false;
    for _ in 0..ticks {
        world.step().expect("step");
        if world.position(a).is_some() && world.position(b).is_some() {
            closest = closest.min(distance(world, a, b));
        }
        paused |= world
            .entity(b)
            .is_some_and(|entity| entity.pause_remaining > 0);
    }
    Run {
        closest,
        reach,
        at: (tile_of(world, a), tile_of(world, b)),
        tick: world.tick(),
        paused,
    }
}

/// **Two units ordered in the same tick walk over each other** (D207). The original lets a moving group overlap - "military
/// units moving in groups can go over each other" - and a group that cannot overlap cannot get past itself in a doorway.
/// The measurable form: at some tick the pair is closer than the reach that a solo walker is held to, and both arrive.
#[test]
fn a_group_ordered_together_walks_over_each_other() {
    let Some(mut world) = common::fresh() else {
        return;
    };
    let Some((x, y)) = open_row(&world, 8) else {
        return;
    };
    let a = next_id(&world);
    let b = EntityId(a.0 + 1);
    spawn_villager(&mut world, a, x, y);
    spawn_villager(&mut world, b, x + 6, y);
    world.step().expect("the spawns land");

    // **One tick, two orders**: that is the group, as far as this engine can express one today.
    world.submit(Command::Move {
        unit: a,
        to: (x + 6, y),
    });
    world.submit(Command::Move {
        unit: b,
        to: (x, y),
    });

    let run = run(&mut world, a, b, 2_000);
    let near = |id: EntityId, goal: (u16, u16)| {
        let (tx, ty) = tile_of(&world, id);
        (tx - i64::from(goal.0)).abs() <= 1 && (ty - i64::from(goal.1)).abs() <= 1
    };
    assert!(
        near(a, (x + 6, y)) && near(b, (x, y)),
        "both of a group have to arrive: a at {:?}, b at {:?} after {} ticks",
        run.at.0,
        run.at.1,
        run.tick
    );
    assert!(
        run.closest < run.reach * run.reach,
        "MEASURED the closest the group came was {} raw ({} tiles), and a solo walker is held to {} raw: a group that \
         never gets closer than the reach never walks over itself",
        integer_sqrt(run.closest),
        integer_sqrt(run.closest) as f64 / world.scale() as f64,
        run.reach
    );
}

/// **Two units ordered on different ticks are not a group** (D207, the half that keeps the rule from being pass-through
/// for everybody). Each was given its own order, so neither is exempt - and since **D208** a non-exempt collision is
/// answered by *time*, not geometry: the pair **does** pass each other, and the proof that they were not treated as mates is
/// that the higher id has to **pause**. The old assertion here was "never overlap", which was D189's rule and is now wrong:
/// measured, the pair came 151 raw apart, i.e. they overlapped, exactly as the original lets them.
#[test]
fn two_units_ordered_apart_are_not_a_group_and_the_loser_pauses() {
    let Some(mut world) = common::fresh() else {
        return;
    };
    let Some((x, y)) = open_row(&world, 8) else {
        return;
    };
    let a = next_id(&world);
    let b = EntityId(a.0 + 1);
    spawn_villager(&mut world, a, x, y);
    spawn_villager(&mut world, b, x + 6, y);
    world.step().expect("the spawns land");

    world.submit(Command::Move {
        unit: a,
        to: (x + 6, y),
    });
    world.step().expect("a tick between the orders");
    world.submit(Command::Move {
        unit: b,
        to: (x, y),
    });

    let run = run(&mut world, a, b, 2_000);
    let near = |id: EntityId, goal: (u16, u16)| {
        let (tx, ty) = tile_of(&world, id);
        (tx - i64::from(goal.0)).abs() <= 1 && (ty - i64::from(goal.1)).abs() <= 1
    };
    assert!(
        near(a, (x + 6, y)) && near(b, (x, y)),
        "both have to arrive: a at {:?}, b at {:?} after {} ticks",
        run.at.0,
        run.at.1,
        run.tick
    );
    // **They are not mates, and the pause is the evidence** (D208): a mate is dropped from the collision block entirely, so
    // he never pauses. These two were ordered apart, so one of them must stand for the drawn 0.25-1.50 s.
    assert!(
        run.paused,
        "MEASURED a pair ordered apart was never paused at {} ticks: the engine treated separate orders as a group",
        run.tick
    );
}

/// **Stop ends the exemption** (D207): the developer's own list - "units give up their formation (and therefore block each
/// other) when: stop button is pressed". A mate whose order is cancelled is a body again.
#[test]
fn a_mate_who_is_stopped_becomes_solid() {
    let Some(mut world) = common::fresh() else {
        return;
    };
    let Some((x, y)) = open_row(&world, 8) else {
        return;
    };
    let a = next_id(&world);
    let b = EntityId(a.0 + 1);
    spawn_villager(&mut world, a, x, y);
    spawn_villager(&mut world, b, x + 6, y);
    world.step().expect("the spawns land");

    world.submit(Command::Move {
        unit: a,
        to: (x + 6, y),
    });
    world.submit(Command::Move {
        unit: b,
        to: (x, y),
    });
    // Let them close, then take b out of the group.
    for _ in 0..60 {
        world.step().expect("step");
    }
    world.submit(Command::Stop { unit: b });
    world.step().expect("the stop lands");

    let reach = world.reach_between(a, b).raw();
    let mut closest = i64::MAX;
    for _ in 0..400 {
        world.step().expect("step");
        closest = closest.min(distance(&world, a, b));
    }
    assert!(
        closest >= reach * reach,
        "MEASURED a stopped body was walked over: closest {} raw against a {} raw reach, after the stop button went",
        integer_sqrt(closest),
        reach
    );
}

/// Integer square root, for reporting a squared distance as raw units. Deliberately local: the assertion's message is the
/// only reader, and a `sqrt` in the test would drag a float into a crate that forbids one in state.
fn integer_sqrt(value: i64) -> i64 {
    if value <= 0 {
        return 0;
    }
    let mut root = 0i64;
    let mut bit = 1i64 << 62;
    let mut rest = value;
    while bit > rest {
        bit >>= 2;
    }
    while bit != 0 {
        if rest >= root + bit {
            rest -= root + bit;
            root = (root >> 1) + bit;
        } else {
            root >>= 1;
        }
        bit >>= 2;
    }
    root
}
