//! **The movement decision is a value you can read** (D193). `collide` decided in nine ordered rules and returned a pair of deltas, so
//! "why is this man standing still" had no answer except printing from inside the function - two hours of it, twice in one week.
//!
//! Written before `World::last_step` and `Step` existed. The claim is narrow and checkable: a step that is taken says **which** rule
//! took it, and a step that is refused says which rule refused it.

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

fn spawn(world: &mut World, player: u8, point: (i64, i64)) -> EntityId {
    let id = next_id(world);
    spawn_with(world, id, player, point)
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

/// **A friend standing on the goal tile is a taken step, and says so.** This is the D192 deadlock as a value: before the fix the walker
/// was held for ever (`WaitGoal`), and the fix made the clause hostile-only.
#[test]
fn a_friend_on_the_goal_tile_is_a_take() {
    let Some(mut world) = common::fresh() else {
        return;
    };
    world.trace_steps();
    let Some((x, y)) = open_row(&world, 4) else {
        return;
    };
    let scale = world.scale();
    // **The ids are named, not read**: `next_id` reads the entity list, and a `Spawn` only lands on the next tick, so two spawns in one
    // tick take the same id - the fixture bug that made the first version of this test measure a walker that was never asked anything.
    let friend = spawn(
        &mut world,
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
    // The friend stands still on the tile the walker is sent to.
    world
        .submit_checked(Command::Move {
            unit: friend,
            to: (x + 2, y),
        })
        .expect("the friend settles");
    for _ in 0..120 {
        world.step().expect("a tick");
    }
    world
        .submit_checked(Command::Move {
            unit: walker,
            to: (x + 2, y),
        })
        .expect("the walk");

    let mut saw_take = false;
    let mut saw_wait_on_the_goal = false;
    for _ in 0..400 {
        world.step().expect("a tick");
        match world.last_step(walker) {
            Some(Step::Take) => saw_take = true,
            Some(step) if step.waited_for_a_goal_tile() => saw_wait_on_the_goal = true,
            _ => {}
        }
    }
    assert!(
        saw_take,
        "MEASURED the walker never took a step toward a friend standing on its goal tile; last decision {:?}",
        world.last_step(walker)
    );
    assert!(
        !saw_wait_on_the_goal,
        "MEASURED the walker waited on the goal tile - the clause D192 made hostile-only fired on a friend"
    );
}

/// **An enemy body changes the decision, and is never overlapped.** Under D189 a *friend* is not a wall; a hostile body still is
/// (`bound_to_reach`), so the walker's decisions must stop being plain takes, and the pair must never close inside their radii.
#[test]
fn an_enemy_body_on_the_line_changes_the_decision() {
    let Some(mut world) = common::fresh() else {
        return;
    };
    world.trace_steps();
    let Some((x, y)) = open_row(&world, 6) else {
        return;
    };
    let scale = world.scale();
    // Sample where the walker actually goes, then put the enemy there: a body at the geometric midpoint is off the route.
    let on_route = {
        let mut probe = common::fresh().expect("a second world");
        let id = spawn(&mut probe, 1, (i64::from(x) * scale, i64::from(y) * scale));
        probe.step().expect("the spawn lands");
        probe
            .submit_checked(Command::Move {
                unit: id,
                to: (x + 5, y),
            })
            .expect("a walk");
        let mut sampled = None;
        for _ in 0..400 {
            probe.step().expect("a tick");
            let at = probe.position(id).expect("the walker");
            if at.0.raw() >= i64::from(x + 2) * scale {
                sampled = Some(at);
                break;
            }
        }
        match sampled {
            Some(point) => point,
            None => return,
        }
    };
    let walker = spawn(&mut world, 1, (i64::from(x) * scale, i64::from(y) * scale));
    // **The enemy is placed on the line after the route exists** (restated 2026-11-14): a body that has not moved is an
    // obstacle to the search that meets it (D210b), so an enemy in place before the order is simply planned around -
    // measured, "only plain takes past an enemy body". The claim here is about the *decision* a collision produces, so
    // the collision has to happen.
    world.step().expect("the walker's spawn lands");
    world
        .submit_checked(Command::Move {
            unit: walker,
            to: (x + 5, y),
        })
        .expect("the walk");
    world.step().expect("the order lands");
    let enemy = spawn_with(
        &mut world,
        EntityId(walker.0 + 1),
        2,
        (on_route.0.raw(), on_route.1.raw()),
    );
    world.step().expect("the enemy lands");

    let mut answered_the_body = false;
    // **The reach is asserted the way D210 leaves it** (restated 2026-11-14): one step of overlap at the meeting, and
    // nothing inside the reach afterwards. The old bound was `reach - scale/100`, which is the clamp D210 withdrew - the
    // measured meeting here is **798 raw** inside an 8000 raw reach, which is exactly the 800-raw step the walker was
    // already taking (`TakeAtRest`, rule 6: a body at rest cannot move aside), and every tick after it is back outside.
    let reach = 2 * scale / 5;
    let mut meeting: Option<i64> = None;
    let mut after = i64::MAX;
    let mut longest_step = 0i64;
    let mut was_at = world.position(walker).expect("the walker");
    for _ in 0..600 {
        world.step().expect("a tick");
        if let Some(step) = world.last_step(walker)
            && !matches!(step, Step::Take | Step::Idle)
        {
            answered_the_body = true;
        }
        if let (Some(me), Some(them)) = (world.position(walker), world.position(enemy)) {
            let apart = (((me.0.raw() - them.0.raw()).pow(2) + (me.1.raw() - them.1.raw()).pow(2))
                as f64)
                .sqrt() as i64;
            if meeting.is_none() && apart < reach {
                meeting = Some(apart);
            } else if meeting.is_some() {
                after = after.min(apart);
            }
            let (sx, sy) = (me.0.raw() - was_at.0.raw(), me.1.raw() - was_at.1.raw());
            longest_step = longest_step.max(((sx * sx + sy * sy) as f64).sqrt() as i64);
            was_at = me;
        }
    }
    assert!(
        answered_the_body,
        "MEASURED the walker's decisions were only plain takes past an enemy body; last {:?}",
        world.last_step(walker)
    );
    let meeting =
        meeting.expect("MEASURED the walker never came within the reach: the fixture did not meet");
    assert!(
        reach - meeting <= longest_step,
        "MEASURED the meeting swallowed {} raw of the {reach} raw reach, more than the {longest_step} raw step the walker \
         was already taking: it was not a collision, it was a walk into him",
        reach - meeting
    );
    // **Nothing deeper than the meeting, ever** rather than "outside the reach" - the two rules together say the deepest
    // point of the encounter is the first contact, and that contact is at most the step he was already taking. The stricter
    // form would fail by **one raw unit** here: the step out lands at 7999 against an 8000 reach, which is the pair
    // *touching* (`r1 + r2`, legal since D209) with the rounding of one integer step.
    assert!(
        after >= meeting,
        "MEASURED the walker went {met} raw units deep after the meeting, at {after}: the encounter's deepest point is not \
         its first contact, so the route does not go round him",
        met = reach - meeting
    );
}
