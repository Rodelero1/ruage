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
    spawn_unit(world, id, unit, x, y);
}

/// A unit of any role, placed the same way. The group rule is about **soldiers** (D237): the original's path-ignore list
/// filters a formation's members, and villagers are in no formation.
fn spawn_unit(world: &mut World, id: EntityId, unit: u32, x: u16, y: u16) {
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
    /// Whether the **higher id** stood a drawn pause at any point (D210): a mate never does, because an exempt collider is
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

/// **Two *soldiers* ordered in the same tick walk over each other** (D207, narrowed by D237). The original lets a moving
/// group overlap - *"military units moving in groups can go over each other"* - and a group that cannot overlap cannot get
/// past itself in a doorway. The measurable form: at some tick the pair is closer than the reach that a solo walker is held
/// to, and both arrive.
///
/// **The subject was a pair of villagers until D237.** The exemption is the original's path-ignore list, which filters a
/// *formation's* members, and the owner's rule of 2026-11-16 - backed by Forgotten Empires' own documentation, *"villagers,
/// trade carts and fishing ships will not be part of any formation ... each of them takes their own path"* - is that a
/// villager never walks through another. The villager case lives in `crates/sim/tests/overlap.rs`, where it asserts the
/// opposite; this fixture pins the rule for the units it is actually about.
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
    let spearman = world.role_unit("spearman").expect("a spearman role");
    spawn_unit(&mut world, a, spearman, x, y);
    spawn_unit(&mut world, b, spearman, x + 6, y);
    world.step().expect("the spawns land");

    // **One tick, two orders**: that is the group, as far as this engine can express one today.
    world.submit(Command::Move {
        target: None,
        unit: a,
        to: (x + 6, y),
    });
    world.submit(Command::Move {
        target: None,
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
/// for everybody). Each was given its own order, so neither is exempt - and since **D210** a non-exempt collision is
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
        target: None,
        unit: a,
        to: (x + 6, y),
    });
    world.step().expect("a tick between the orders");
    world.submit(Command::Move {
        target: None,
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
    // **They are not mates, and the pause is the evidence** (D210): a mate is dropped from the collision block entirely, so
    // he never pauses. These two were ordered apart, so one of them must stand for the drawn 0.25-1.50 s.
    assert!(
        run.paused,
        "MEASURED a pair ordered apart was never paused at {} ticks: the engine treated separate orders as a group",
        run.tick
    );
}

/// **Stop ends the exemption** (D207): the developer's own list - "units give up their formation (and therefore block
/// other) when: stop button is pressed". A mate whose order is cancelled is a body again.
///
/// **Restated 2026-11-14 to the rule the engine actually has, with the measurement that forced it.** This asserted
/// "never inside the reach", which D210 cannot honour: D210 withdrew the clamp against a body (the binary's only
/// radius-against-radius retraction has its result discarded at the movement site, `0x8a7fe7`), and rule 6 takes a full
/// step into a body **at rest** rather than freezing the mover, because a resting body cannot move aside. So the meeting
/// itself overlaps by the one step he had already begun - measured **272 raw** against an 8000 reach here, and no route
/// can prevent it: he is only told about the man when the candidate lands inside him.
///
/// What must not happen is the *second* half - going on through him. So the guard is: at most one step of overlap **at**
/// the meeting, nothing inside the reach on any tick **after** it, and he still arrives. Before the search was taught to
/// bend round a body in an open window (D211 follow-up) the closest after the meeting was **5012 raw**, and before that
/// the same fixture's route ran straight through him at **4207**.
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
    // **Soldiers, because the rule is about a formation** (D237): with villagers never being mates, a villager pair would
    // satisfy this fixture without ever forming the group it is meant to dissolve.
    let spearman = world.role_unit("spearman").expect("a spearman role");
    spawn_unit(&mut world, a, spearman, x, y);
    spawn_unit(&mut world, b, spearman, x + 6, y);
    world.step().expect("the spawns land");

    world.submit(Command::Move {
        target: None,
        unit: a,
        to: (x + 6, y),
    });
    world.submit(Command::Move {
        target: None,
        unit: b,
        to: (x, y),
    });
    // Let them close, then take b out of the group - when a is one step out. Stopping b on a fixed tick leaves
    // him wherever the walk happened to be, and a clean router bends round a distant stander without ever
    // touching him; stopping him a step ahead forces the one step of overlap the assertion measures, and the
    // round that has to follow it.
    let step = world.scale() / 2 / i64::from(world.tick_rate_hz().max(1));
    loop {
        world.step().expect("step");
        let gap = integer_sqrt(distance(&world, a, b));
        if gap < world.reach_between(a, b).raw() + 2 * step {
            break;
        }
    }
    world.submit(Command::Stop { unit: b });
    world.step().expect("the stop lands");

    let reach = world.reach_between(a, b).raw();
    let mut meeting: Option<i64> = None;
    let mut after = i64::MAX;
    let mut longest_step = 0i64;
    let mut was_at = world.position(a).expect("a has a position");
    for _ in 0..400 {
        world.step().expect("step");
        let gap = distance(&world, a, b);
        if meeting.is_none() && gap < reach * reach {
            meeting = Some(gap);
        } else if meeting.is_some() {
            after = after.min(gap);
        }
        let now = world.position(a).expect("a has a position");
        let (sx, sy) = (now.0.raw() - was_at.0.raw(), now.1.raw() - was_at.1.raw());
        longest_step = longest_step.max(sx * sx + sy * sy);
        was_at = now;
    }
    let meeting = meeting
        .expect("MEASURED the two never came within the reach: the fixture did not meet at all");
    let overlap = reach - integer_sqrt(meeting);
    assert!(
        overlap <= integer_sqrt(longest_step),
        "MEASURED the meeting swallowed {} raw of the {} raw reach, more than the {} raw step he was already taking: \
         he was not walking when the man stopped, he was driven into him",
        overlap,
        reach,
        integer_sqrt(longest_step)
    );
    assert!(
        integer_sqrt(after) >= reach / 4,
        "MEASURED a stopped body was walked over: after the meeting he came {} raw from him, inside the {} raw reach, so \
         the route does not go round the man - it goes through him",
        integer_sqrt(after),
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
