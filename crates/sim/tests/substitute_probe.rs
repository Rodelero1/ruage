//! **The measurement behind D340**: what a walker actually does when its way onward is held by another body.
//!
//! Sequence step 5 was written from the reading that a mover whose waypoint cannot be reached "re-plans for ever, a stuck
//! unit with a live order". This probe says what happens instead - and the two scenes disagree, which is the finding:
//!
//! * beside the seeded base, with the base's villagers walking about, a walker touching another body re-plans **196 times
//!   in 200 ticks** while **never standing** (the body keeps walking; the collide rule *takes* the step and pauses the
//!   other man, D279/D281); so the game's substitution has no stand to answer there;
//! * in a clear lane, the same catch-up produces **0 re-plans in 260 ticks** - the slower man is *past* by the time the
//!   faster one arrives, and the faster one walks through him.
//!
//! It prints rather than asserts: the numbers belong in a review note, and the rule they argue against belongs in
//! `docs/decisions.md` D340. Run with
//! `cargo test -p ruage-sim --test substitute_probe -- --ignored --nocapture`.

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

mod common;

fn next_id(world: &World) -> u32 {
    world
        .entities()
        .iter()
        .map(|entity| entity.id.0)
        .max()
        .unwrap_or(0)
        + 1
}

/// A stretch of clear ground with a lane, away from the seeded base.
fn lane(world: &World) -> ((i64, i64), (i64, i64)) {
    for y in 40..54u16 {
        for x in 30..48u16 {
            if (0..12).all(|d| world.is_free_ground(x + d - 2, y)) {
                return (
                    (i64::from(x), i64::from(y)),
                    (i64::from(x) + 10, i64::from(y)),
                );
            }
        }
    }
    panic!("no clear lane on the map");
}

fn spawn(world: &mut World, at: (i64, i64), speed: Fixed) -> EntityId {
    let id = EntityId(next_id(world));
    let scale = world.scale();
    let villager = world.role_unit("villager").expect("a villager role");
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
            speed,
            hp: 25,
            amount: 0,
        })
        .expect("a spawn on clear ground");
    world.step().expect("one tick");
    id
}

fn order(world: &mut World, id: EntityId, to: (i64, i64)) {
    let _ = world.submit_checked(Command::Move {
        target: None,
        unit: id,
        to: (
            u16::try_from(to.0).expect("in range"),
            u16::try_from(to.1).expect("in range"),
        ),
    });
}

/// **A faster walker catching a slower one going the same way.** Both walk the whole lane; the faster one touches the
/// slower one for the middle of the run.
#[test]
#[ignore = "probe: prints, does not assert"]
fn probe_catch_up() {
    let Some(mut world) = common::fresh_plain() else {
        return;
    };
    let scale = world.scale();
    let ((sx, sy), (gx, gy)) = lane(&world);
    let slow = spawn(&mut world, (sx + 4, sy), Fixed::from_raw(scale * 8 / 10));
    let walker = spawn(&mut world, (sx, sy), Fixed::from_raw(scale * 12 / 10));
    order(&mut world, slow, (gx, gy));
    order(&mut world, walker, (gx, gy));

    let mut re_plans = 0u64;
    let mut stood = 0u32;
    let mut longest_stand = 0u32;
    let mut before = world.position(walker);
    for tick in 0..260 {
        let seen = world.repaths();
        world.step().expect("stepping");
        re_plans += world.repaths() - seen;
        let now = world.position(walker);
        if now == before {
            stood += 1;
            longest_stand = longest_stand.max(stood);
        } else {
            stood = 0;
        }
        before = now;
        if world.route_points(walker).is_empty() {
            println!("the walk ended at tick {tick}");
            break;
        }
    }
    println!(
        "re-plans on the walker's behalf: {re_plans} over 260 ticks; longest stand {longest_stand} ticks; \
         walker at {:?}, slow at {:?}",
        world.position(walker),
        world.position(slow),
    );
}
