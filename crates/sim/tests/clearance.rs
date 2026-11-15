//! **A small enough pencil** (D211): a route must be able to bend round a *body* at less than a tile's clearance.
//!
//! The original's search works on continuous rectangles - each body a box of `pos +- radius`, grown by the mover's radius -
//! so it can thread a gap between a man and a wall that is narrower than a tile. Ours builds its graph out of **tile
//! corners**, so the only detours it can express are whole-tile ones: beside a body that stands in a corridor, or next to
//! the walker himself, there is no such detour, the search fails, the fallback draws a straight line, and the walker ghosts
//! through him - measured at 4207 raw against an 8000 reach (D210).
//!
//! Written before the implementation (D7).

use std::collections::BTreeSet;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, Walk, World, short_range_clear};

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
                Fixed::from_raw(i64::from(x) * scale),
                Fixed::from_raw(i64::from(y) * scale),
            ),
            speed: Fixed::from_raw(scale / 2),
            hp: 25,
            amount: 0,
        })
        .expect("a villager can be placed");
}

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

fn tile_of(world: &World, id: EntityId) -> (i64, i64) {
    let scale = world.scale();
    let (x, y) = world.position(id).expect("the entity has a position");
    (x.raw().div_euclid(scale), y.raw().div_euclid(scale))
}

fn gap(world: &World, a: EntityId, b: EntityId) -> i64 {
    let (ax, ay) = world.position(a).expect("a has a position");
    let (bx, by) = world.position(b).expect("b has a position");
    let (dx, dy) = (ax.raw() - bx.raw(), ay.raw() - by.raw());
    dx * dx + dy * dy
}

/// **A walker steps round a man standing in his lane.** The stander never moves, so the walker's route is planned after he
/// stopped and the search must treat him as ground: the way past is a sub-tile curve round his shoulder, not a whole-tile
/// detour, because there is no free tile between him and the lane.
///
/// The assertion is the reach, not the arrival: a walker that ghosts through the man still *arrives*, which is exactly why
/// this could not be caught by an arrival test.
#[test]
fn a_walker_steps_round_a_man_standing_in_his_lane() {
    let Some(mut world) = common::fresh() else {
        return;
    };
    let Some((x, y)) = open_row(&world, 8) else {
        return;
    };
    let walker = next_id(&world);
    let stander = EntityId(walker.0 + 1);
    // The stander sits **on** the lane, two tiles along, and never gets an order.
    spawn_villager(&mut world, walker, x, y);
    spawn_villager(&mut world, stander, x + 2, y);
    world.step().expect("the spawns land");

    world.submit(Command::Move {
        unit: walker,
        to: (x + 5, y),
    });
    world.step().expect("the order lands");

    let reach = world.reach_between(walker, stander).raw();
    let mut closest = i64::MAX;
    for _ in 0..1_200 {
        world.step().expect("step");
        closest = closest.min(gap(&world, walker, stander));
    }
    assert!(
        (tile_of(&world, walker).0 - i64::from(x + 5)).abs() <= 1,
        "the walker has to arrive: he is at {:?}, sent to ({}, {}) - a search that cannot find the shoulder must still \
         reach the goal",
        tile_of(&world, walker),
        x + 5,
        y
    );
    assert!(
        closest >= reach * reach,
        "MEASURED the walker came {} raw from the man standing in his lane, inside the {} raw his footprint needs: the \
         plan has no sub-tile detour, so the fallback drew a straight line through him",
        closest.max(0).isqrt(),
        reach
    );
}

/// **The same, one tile out.** A man standing *beside* the lane - close enough that his shoulder is in the way, far enough
/// that a whole-tile detour exists - must still be cleared, and the walker must not take the long way round the whole
/// block to do it. This is the guard that keeps the sub-tile fix from becoming a licence to route absurdly.
#[test]
fn a_man_beside_the_lane_is_cleared_without_a_detour_round_the_block() {
    let Some(mut world) = common::fresh() else {
        return;
    };
    let Some((x, y)) = open_row(&world, 8) else {
        return;
    };
    let walker = next_id(&world);
    let stander = EntityId(walker.0 + 1);
    spawn_villager(&mut world, walker, x, y);
    // Half a tile off the lane, so his disc overlaps the straight line but the tiles on either side are free.
    let scale = world.scale();
    let unit = world.start_villager_unit().expect("a villager role");
    world
        .submit_checked(Command::Spawn {
            id: stander,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                Fixed::from_raw(i64::from(x + 3) * scale),
                Fixed::from_raw(i64::from(y) * scale + scale / 2),
            ),
            speed: Fixed::from_raw(scale / 2),
            hp: 25,
            amount: 0,
        })
        .expect("a villager can be placed");
    world.step().expect("the spawns land");

    world.submit(Command::Move {
        unit: walker,
        to: (x + 6, y),
    });

    let reach = world.reach_between(walker, stander).raw();
    let mut closest = i64::MAX;
    for _ in 0..1_200 {
        world.step().expect("step");
        closest = closest.min(gap(&world, walker, stander));
    }
    assert!(
        closest >= reach * reach,
        "MEASURED the walker came {} raw from a man standing half a tile off his lane, inside the {} raw reach",
        closest.max(0).isqrt(),
        reach
    );
}

// ---------------------------------------------------------------------------------------------------------------
// **The search itself, with no world under it.** The two fixtures above go through the engine, so they can be
// satisfied by the *tile* the stander occupies rather than by his disc (measured: they are, which is why the
// sub-tile mechanism they were written for was never exercised). These two ask `short_range_clear` directly.
// ---------------------------------------------------------------------------------------------------------------

/// A flat map, a projected into raw units, and the exact segment-to-point test the search owes its callers.
const SCALE: i64 = 20_000;
const RADIUS: i64 = SCALE / 5;

fn flat(width: u16, height: u16) -> Walk {
    let map = ruage_data::Map {
        width,
        height,
        terrain: vec![ruage_data::Terrain::Grass; usize::from(width) * usize::from(height)],
        resources: Vec::new(),
        town_centers: Vec::new(),
    };
    Walk::from_map(&map)
}

fn corner(x: i64, y: i64) -> (Fixed, Fixed) {
    (Fixed::from_raw(x * SCALE), Fixed::from_raw(y * SCALE))
}

/// The squared distance from a point to a segment, in raw units: the nearer end when the perpendicular foot falls
/// outside, otherwise `cross^2 / length` (floored, which can only make the assertion below stricter).
fn segment_gap(a: (i64, i64), b: (i64, i64), centre: (i64, i64)) -> i128 {
    let (abx, aby) = (i128::from(b.0 - a.0), i128::from(b.1 - a.1));
    let (acx, acy) = (i128::from(centre.0 - a.0), i128::from(centre.1 - a.1));
    let length = abx * abx + aby * aby;
    if length == 0 {
        return acx * acx + acy * acy;
    }
    let dot = acx * abx + acy * aby;
    if !(0..=length).contains(&dot) {
        let (bcx, bcy) = (i128::from(centre.0 - b.0), i128::from(centre.1 - b.1));
        return (acx * acx + acy * acy).min(bcx * bcx + bcy * bcy);
    }
    let cross = (abx * acy - aby * acx).abs();
    (cross * cross) / length
}

fn route_stats(points: &[(Fixed, Fixed)], from: (Fixed, Fixed), centre: (i64, i64)) -> (i128, f64) {
    let mut closest = i128::MAX;
    let mut length = 0.0f64;
    let mut previous = (from.0.raw(), from.1.raw());
    for point in points {
        let here = (point.0.raw(), point.1.raw());
        closest = closest.min(segment_gap(previous, here, centre));
        let (dx, dy) = (
            f64::from((here.0 - previous.0) as i32),
            f64::from((here.1 - previous.1) as i32),
        );
        length += (dx * dx + dy * dy).sqrt();
        previous = here;
    }
    (closest, length)
}

/// **A body alone in an open field is an obstacle.** Nothing blocks the window's tiles, so the search takes its
/// straight-line shortcut - and that shortcut never looked at the bodies, so a man standing in an open field was
/// walked through. Measured in the engine: `group_overlap::a_mate_who_is_stopped_becomes_solid`, closest **5012 raw
/// against the 8000** the pair's footprints need, with the re-plan that was supposed to route round him producing a
/// route whose second leg passes 0.25 tiles from his centre.
///
/// The second assertion is a guard on the fix, not the rule: a 0.4-tile obstacle between two points ten tiles apart
/// cannot lengthen the walk by more than half a tile if the bend is drawn beside him, so two tiles still fails a route
/// that runs to the window's edge and back.
#[test]
fn a_body_alone_in_an_open_field_is_rounded_like_a_wall() {
    let walk = flat(16, 16);
    let scale = SCALE;
    let centre = (
        Fixed::from_raw(7 * scale + scale / 2),
        Fixed::from_raw(7 * scale),
    );
    let clearance = 2 * RADIUS;
    let bodies = [(centre, clearance)];
    let from = corner(2, 7);
    let to = corner(12, 7);

    let points = short_range_clear(&walk, SCALE, from, to, RADIUS, &BTreeSet::new(), &bodies)
        .expect("an open field with one man in it is a path");
    assert_eq!(
        points.last().copied(),
        Some(to),
        "the route must end on the goal, it ends at {:?}",
        points.last()
    );
    let (closest, length) = route_stats(&points, from, (centre.0.raw(), centre.1.raw()));
    let keep = i128::from(clearance) * i128::from(clearance);
    assert!(
        closest >= keep,
        "MEASURED the route passes {} raw from a man it must keep {} raw from: the window holds no blocked tile, so \
         the straight-line shortcut answered before the bodies were read",
        (closest as f64).sqrt(),
        clearance
    );
    let straight = f64::from((to.0.raw() - from.0.raw()) as i32);
    assert!(
        length <= straight + 2.0 * scale as f64,
        "MEASURED the detour is {:.2} tiles for a straight line of {:.2}: that is a walk to the window's edge, not a \
         bend round one man",
        length / scale as f64,
        straight / scale as f64
    );
}

/// **A walker who starts inside a man steps away from him first.** This is the case D212's re-seed exists for, and the
/// one place where "a free candidate" is not enough: a point beyond the man is free, and walking to it means walking
/// through him (measured 3702 raw against 8000 when the far side was allowed). Leaving a disc is legal, entering it is
/// not - so the first waypoint must move **away** from the centre.
#[test]
fn a_walker_starting_inside_a_body_steps_away_before_going_round() {
    let walk = flat(16, 16);
    let scale = SCALE;
    let centre = (Fixed::from_raw(7 * scale), Fixed::from_raw(7 * scale));
    let clearance = 2 * RADIUS;
    let bodies = [(centre, clearance)];
    // 0.35 tiles from the centre: inside the 0.4 the pair's footprints need.
    let from = (
        Fixed::from_raw(7 * scale - 7 * scale / 20),
        Fixed::from_raw(7 * scale),
    );
    let to = corner(12, 7);

    let points = short_range_clear(&walk, SCALE, from, to, RADIUS, &BTreeSet::new(), &bodies)
        .expect("a walker already touching a man can still be given a route round him");
    let first = points
        .first()
        .copied()
        .expect("the re-seed puts a point at the head");
    let (cx, cy) = (centre.0.raw(), centre.1.raw());
    let (sx, sy) = (from.0.raw(), from.1.raw());
    let (fx, fy) = (first.0.raw(), first.1.raw());
    let dot = (fx - sx) * (cx - sx) + (fy - sy) * (cy - sy);
    assert!(
        dot <= 0,
        "MEASURED the first step goes {} raw towards the man's centre, from a point already inside his clearance: a \
         free point on his far side is a destination, not a place to start",
        dot
    );
    let (closest, _) = route_stats(&points, first, (cx, cy));
    let keep = i128::from(clearance) * i128::from(clearance);
    assert!(
        closest >= keep,
        "MEASURED the route from the first waypoint passes {} raw from him, inside the {} raw it must keep",
        (closest as f64).sqrt(),
        clearance
    );
    assert_eq!(points.last().copied(), Some(to), "and it must still arrive");
}

// ---------------------------------------------------------------------------------------------------------------
// **A stander is an obstacle to the search that meets it** (D210b). The original inserts an object into its per-search
// rectangle list when it is not ignored and **its velocity is zero** (`FUN_007f9d80`) - so a man who has stopped is
// ground, and a man who is walking is not. The engine's `stander_*` sets read `Entity::last_moved`, and the first version
// of the stamp wrote it on every tick for every body, so *no body that had ever moved* was a stander: measured, a man who
// stopped in front of a walker was walked into at 7728 raw against an 8000 reach, and the plan that should have gone round
// him was never built.
// ---------------------------------------------------------------------------------------------------------------

/// **A man who stopped before the route was planned is routed round, not walked into.** He has *moved* (so the
/// never-moved sentinel does not cover him) and then stopped, which is exactly the case `last_moved` is stamped wrongly
/// for today.
#[test]
fn a_man_who_stopped_before_the_route_was_planned_is_routed_round() {
    let Some(mut world) = common::fresh() else {
        return;
    };
    let Some((x, y)) = open_row(&world, 10) else {
        return;
    };
    let walker = next_id(&world);
    let stopper = EntityId(walker.0 + 1);
    spawn_villager(&mut world, walker, x, y);
    spawn_villager(&mut world, stopper, x + 4, y);
    world.step().expect("the spawns land");

    // The stopper walks away from the walker and then stops, so he ends up on the walker's lane with a history of having
    // moved. Two ticks after the stop are given: "did not move on the last tick" has to be true of him.
    world.submit(Command::Move {
        unit: stopper,
        to: (x + 6, y),
    });
    for _ in 0..80 {
        world.step().expect("step");
    }
    world.submit(Command::Stop { unit: stopper });
    world.step().expect("the stop lands");
    world
        .step()
        .expect("one more, so the stopped man has not moved on the last tick");

    // Now the walker is sent past him, and the route is planned from a world in which he is standing still.
    world.submit(Command::Move {
        unit: walker,
        to: (x + 9, y),
    });
    let reach = world.reach_between(walker, stopper).raw();
    let mut closest = i64::MAX;
    for _ in 0..1_200 {
        world.step().expect("step");
        closest = closest.min(gap(&world, walker, stopper));
    }
    assert!(
        (tile_of(&world, walker).0 - i64::from(x + 9)).abs() <= 1,
        "the walker has to arrive: he is at {:?}, sent to ({}, {})",
        tile_of(&world, walker),
        x + 9,
        y
    );
    assert!(
        closest >= reach * reach,
        "MEASURED the walker came {} raw from a man who had stopped before the route was planned, inside the {} raw their \
         footprints need: the stopped man was not a stander to the plan, so it answered with a line through him",
        closest.max(0).isqrt(),
        reach
    );
}

/// **A man who is walking is not ground.** The other half of the rule, and the one that keeps a busy base from planning
/// round everything that moves: a body that moved on the last tick must not lengthen another body's route.
#[test]
fn a_man_who_is_still_walking_is_not_an_obstacle_to_the_plan() {
    let Some(mut world) = common::fresh() else {
        return;
    };
    let Some((x, y)) = open_row(&world, 10) else {
        return;
    };
    let walker = next_id(&world);
    let mover = EntityId(walker.0 + 1);
    spawn_villager(&mut world, walker, x, y);
    spawn_villager(&mut world, mover, x + 4, y);
    world.step().expect("the spawns land");

    // The other man is under way along the lane when the walker's route is planned.
    world.submit(Command::Move {
        unit: mover,
        to: (x + 8, y),
    });
    for _ in 0..10 {
        world.step().expect("step");
    }
    world.submit(Command::Move {
        unit: walker,
        to: (x + 9, y),
    });
    world.step().expect("the order lands");
    let route = world.route_points(walker);
    assert_eq!(
        route.len(),
        1,
        "MEASURED the plan for a man walking down an open lane took {} waypoints ({route:?}): a body that moved on the \
         last tick is not an obstacle to the search",
        route.len()
    );
}
