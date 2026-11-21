//! **Step 6: a body that is not moving is an obstacle, even with a route on its books** (`FUN_007f9d80:27-56`: the
//! search inserts an object's grown box unless its id is in the mover's ignore list **and** `[vtbl+0x2a0]` - isMoving -
//! answers non-zero).
//!
//! Written before the implementation (D7, SPEC §11).

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, Task, World};

mod common;

fn fresh() -> Option<World> {
    common::fresh_plain()
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

/// A villager on a tile centre, landed.
fn a_villager(world: &mut World, at: (i64, i64), speed: Fixed) -> EntityId {
    let id = next_id(world);
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
        .expect("a villager on clear ground");
    world.step().expect("the villager lands");
    id
}

/// A stretch of clear ground: a north-south lane with its own tile free, and room east of it for a site.
fn lane(world: &World) -> (i64, i64) {
    for y in 10..52u16 {
        for x in 10..46u16 {
            if (0..4).all(|d| world.is_free_ground(x, y + d))
                && (0..3).all(|d| world.is_free_ground(x + d, y))
            {
                return (i64::from(x), i64::from(y));
            }
        }
    }
    panic!("no clear ground on the map");
}

/// **A body standing at work is an obstacle to the next man's plan.** A builder whose site is already beside it takes no
/// step on any tick and keeps a live route; the search treats "has a route" as "is moving" and certifies legs straight
/// through him.
#[test]
fn a_standing_body_with_a_route_is_planned_round() {
    let Some(mut world) = fresh() else { return };
    let scale = world.scale();
    let (x, y) = lane(&world);
    let builder = a_villager(&mut world, (x, y), Fixed::from_raw(scale * 12 / 10));

    // A site one tile east of the builder: its footprint is beside him, so the Build arm answers "adjacent" from the
    // first tick and the route the order planned is never spent.
    let site = (
        u16::try_from(x + 1).expect("in range"),
        u16::try_from(y).expect("in range"),
    );
    assert!(
        world.site_fits(1, 70, site),
        "the site {site:?} is not legal on this ground"
    );
    world
        .submit_checked(Command::Build {
            villager: builder,
            unit: 70,
            at: site,
        })
        .expect("a house on a site beside the builder");
    world.step().expect("the order lands");

    let standing_at = world.position(builder).expect("the builder");
    let route = world.route_points(builder);
    assert!(
        matches!(
            world.entity(builder).map(|entity| entity.task),
            Some(Task::Build { .. })
        ),
        "the builder is not on a build order: {:?}",
        world.entity(builder).map(|entity| entity.task)
    );
    assert!(
        !route.is_empty(),
        "the builder has no route, so this fixture would not be measuring a body with one"
    );

    // A second man three tiles north of him, ordered three tiles south: the straight line runs through his tile centre.
    let walker = a_villager(&mut world, (x, y - 3), Fixed::from_raw(scale * 8 / 10));
    world
        .submit_checked(Command::Move {
            target: None,
            unit: walker,
            to: (
                u16::try_from(x).expect("in range"),
                u16::try_from(y + 3).expect("in range"),
            ),
        })
        .expect("a walk past the builder");
    world.step().expect("the walk starts");

    let reach = world.reach_between(walker, builder).raw();
    let mut overlap_ticks = 0u32;
    let mut closest = i64::MAX;
    for _ in 0..600 {
        world.step().expect("stepping");
        let (Some(me), Some(him)) = (world.position(walker), world.position(builder)) else {
            break;
        };
        let (dx, dy) = (
            (me.0.raw() - him.0.raw()).abs(),
            (me.1.raw() - him.1.raw()).abs(),
        );
        closest = closest.min(dx.max(dy));
        if dx < reach && dy < reach {
            overlap_ticks += 1;
        }
        if world.route_points(walker).is_empty() {
            break;
        }
    }
    println!(
        "builder stands at {standing_at:?} (tile centre {x},{y}); walker overlapped him for {overlap_ticks} ticks, \
         closest axis gap {closest} raw of reach {reach}"
    );
    assert_eq!(
        overlap_ticks, 0,
        "the second man walked through a body that was standing at work: {overlap_ticks} ticks inside its box \
         (closest axis gap {closest} raw, reach {reach})"
    );
}
