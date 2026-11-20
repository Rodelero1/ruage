//! **Where a chosen unit goes when you right-click** (the owner, 2026-11-18):
//!
//! *"Any single unit chosen has to go to the exact spot by moving the center of the circle to that exact spot. If spot
//! unreachable, try come as close as possible. If multiple villagers are chosen, then the first comes to the spot and others
//! stand around him."*
//!
//! The single-unit half already holds (`points.rs`); what did not is the crowd: a follower was sent to `tile_centre(slot)` - the
//! middle of a square a ring-search picked, which `slot_for` scans from `(-1,-1)` outwards, so every follower landed **up and to
//! the left** of the click, measured at **24,202 raw (1.21 tiles)**. The owner's rule is a **ring round the first man**, one
//! shoulder-width apart.

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
                Fixed::from_raw(i64::from(x) * scale + scale / 2),
                Fixed::from_raw(i64::from(y) * scale + scale / 2),
            ),
            speed: Fixed::from_raw(scale / 2),
            hp: 25,
            amount: 0,
        })
        .expect("a villager can be placed");
    world.step().expect("the spawn lands");
}

/// An empty stretch of ground, clear of the base: the crowd test must not be about the map's furniture.
fn open_row(world: &World, span: u16) -> Option<(u16, u16)> {
    for y in 4..60u16 {
        for x in 4..(64 - span) {
            if (0..span).all(|d| world.is_free_ground(x + d, y)) && !(8..26).contains(&x) {
                return Some((x, y));
            }
        }
    }
    None
}

fn span_to(world: &World, id: EntityId, to: (Fixed, Fixed)) -> i64 {
    let at = world.position(id).expect("a position");
    let (dx, dy) = (at.0.raw() - to.0.raw(), at.1.raw() - to.1.raw());
    (dx * dx + dy * dy).isqrt()
}

fn apart(world: &World, one: EntityId, two: EntityId) -> i64 {
    span_to(world, one, world.position(two).expect("a position"))
}

/// **A measurement, not a rule**: what the ring costs when a hundred men are ordered to one point.
#[test]
#[ignore = "measurement, not a rule"]
fn what_the_ring_costs_for_a_hundred_orders() {
    let Some(mut world) = common::fresh() else {
        return;
    };
    let Some((x, y)) = open_row(&world, 8) else {
        return;
    };
    let scale = world.scale();
    let spot = (
        Fixed::from_raw(i64::from(x + 4) * scale + 2 * scale / 5),
        Fixed::from_raw(i64::from(y) * scale + 2 * scale / 5),
    );
    let mut crowd = Vec::new();
    for n in 0..120u16 {
        let id = next_id(&world);
        spawn_villager(&mut world, id, x + n % 5, y + n % 3);
        crowd.push(id);
    }
    let start = std::time::Instant::now();
    for id in &crowd {
        let _ = world.submit_checked(Command::WalkTo {
            target: None,
            unit: *id,
            to: spot,
        });
    }
    let placed = start.elapsed();
    for _ in 0..600 {
        world.step().expect("step");
    }
    println!(
        "RING 120 orders: submit {:.3} ms, 600 ticks {:.3} ms",
        placed.as_secs_f64() * 1_000.0,
        start.elapsed().as_secs_f64() * 1_000.0
    );
}

/// **One unit, one click**: the centre of the circle ends on the spot - not on the middle of the square it happens to be in.
#[test]
fn a_single_unit_stops_on_the_spot_it_was_sent_to() {
    let Some(mut world) = common::fresh() else {
        return;
    };
    let Some((x, y)) = open_row(&world, 8) else {
        return;
    };
    let scale = world.scale();
    let walker = next_id(&world);
    spawn_villager(&mut world, walker, x, y);
    // Four tenths of the way into a tile, so a tile-centre goal is 0.4 tiles off in each axis and impossible to miss.
    let spot = (
        Fixed::from_raw(i64::from(x + 5) * scale + 2 * scale / 5),
        Fixed::from_raw(i64::from(y) * scale + 2 * scale / 5),
    );
    world
        .submit_checked(Command::WalkTo {
            target: None,
            unit: walker,
            to: spot,
        })
        .expect("a walk order");
    for _ in 0..800 {
        world.step().expect("step");
    }

    let off = span_to(&world, walker, spot);
    assert!(
        off <= scale / 20,
        "the unit stopped {off} raw units from the spot it was sent to (0.05 of a tile is {})",
        scale / 20
    );
}

/// **Three units, one click**: somebody takes the spot and the others close on it, pressed against him (D289). D271's ring -
/// each follower given a place of his own round the first man - is withdrawn: *"Drop it and let the crowds jam."*
#[test]
fn a_crowd_stands_round_the_unit_that_took_the_spot() {
    let Some(mut world) = common::fresh() else {
        return;
    };
    let Some((x, y)) = open_row(&world, 8) else {
        return;
    };
    let scale = world.scale();
    let spot = (
        Fixed::from_raw(i64::from(x + 4) * scale + 2 * scale / 5),
        Fixed::from_raw(i64::from(y) * scale + 2 * scale / 5),
    );
    let mut crowd = Vec::new();
    // **All three start on the one row the map is known to leave open** - a spawn two tiles off it can be inside a building's
    // footprint, and then the test measures the map instead of the ring.
    for (n, (dx, dy)) in [(0u16, 0u16), (6, 0), (7, 0)].into_iter().enumerate() {
        let id = next_id(&world);
        spawn_villager(&mut world, id, x + dx, y + dy);
        world
            .submit_checked(Command::WalkTo {
                target: None,
                unit: id,
                to: spot,
            })
            .expect("a walk order");
        world.step().expect("the order lands");
        crowd.push(id);
        let _ = n;
    }
    for _ in 0..900 {
        world.step().expect("step");
    }

    // **Somebody takes the spot, and the rest stand pressed behind him** (D289). Every man of the crowd was sent to the same
    // point, so which one arrives is a race, not an assignment; the original's own players describe men standing pressed on one
    // spot, so the claim is "someone is on it and nobody is left across the map".
    let closest = crowd
        .iter()
        .map(|man| span_to(&world, *man, spot))
        .min()
        .expect("a crowd");
    assert!(
        closest <= scale / 20,
        "nobody took the spot: the nearest villager stopped {closest} raw units from it ({})",
        scale / 20
    );
    for other in crowd.iter() {
        let ring = span_to(&world, *other, spot);
        assert!(
            ring <= 2 * scale,
            "a follower stands {ring} raw units from the spot: the crowd must close on the point it was sent to"
        );
    }
    for (n, one) in crowd.iter().enumerate() {
        for two in crowd.iter().skip(n + 1) {
            let reach = world.reach_between(*one, *two).raw();
            let gap = apart(&world, *one, *two);
            assert!(
                gap >= reach,
                "two villagers of one order stand {gap} raw units apart, inside the {reach} their footprints need"
            );
        }
    }
}
