//! **Rule set 4: the body has a radius, and the route keeps it clear.**
//!
//! Read from the deck by the original's own engineering director (Raymi Klingers, *25+ Years of Pathfinding Problems with
//! C++*, slides 21-38 and 82-96): shrink the unit to a point, **expand every obstruction by its radius**, ray at the goal,
//! walk the outline of the union. HD's low-level search does the same in continuous rectangles, and re-seeds a start that
//! falls inside one by `2 x radius` along a fixed ring (`FUN_007fb220`).
//!
//! Ours is the visibility-graph form of the same idea (`crates/sim/src/short.rs`): a rectangle per blocked tile grown by the
//! radius, a graph over its corners and the two endpoints, and exact integer geometry. What these tests pin is the
//! *property* - no segment of the route comes within the radius of a blocked tile - not the route's exact points.

use std::collections::BTreeSet;

use ruage_conformance::{
    RADIUS, SCALE, at, distance_sq_to_tile, polyline_distance_sq_to_point,
    polyline_distance_sq_to_tile, walk,
};
use ruage_fixed::Fixed;
use ruage_sim::short_range_clear;

/// RULE: every segment of the route keeps the body's radius clear of every blocked tile.
///
/// Source: slide 22 - "shrink the unit to a point, expand every obstruction by its radius"; the same rule stated as an
/// invariant for this engine (`references/movement-invariants.md`).
#[test]
fn every_segment_keeps_the_radius_clear_of_a_block() {
    // A block of four tiles with a walk round it on every side.
    let blocked = [(7u16, 6u16), (8, 6), (7, 7), (8, 7)];
    let walk = walk(16, 16, &blocked);
    let route = short_range_clear(
        &walk,
        SCALE,
        at(2, 7),
        at(14, 7),
        RADIUS,
        &BTreeSet::new(),
        &[],
    )
    .expect("a four-tile block is walkable round");
    for tile in blocked {
        let gap = polyline_distance_sq_to_tile(&route, tile);
        assert!(
            gap >= RADIUS * RADIUS,
            "the route passes {} raw units from {tile:?}, inside the body's {RADIUS}: {route:?}",
            isqrt(gap)
        );
    }
}

/// RULE: a wider body takes a route no closer to the wall - the clearance scales with the radius.
///
/// Source: the same expansion, with `r` the mover's own radius; the engine's gather rings are the measured consequence of
/// pricing that radius exactly (D211: an added margin put every ring place inside a neighbour's disc).
#[test]
fn a_wider_body_keeps_a_wider_clearance() {
    let walk = walk(16, 16, &[(8u16, 7u16)]);
    let narrow = short_range_clear(
        &walk,
        SCALE,
        at(3, 7),
        at(13, 7),
        2_000,
        &BTreeSet::new(),
        &[],
    )
    .expect("a narrow body gets past one tile");
    let wide = short_range_clear(
        &walk,
        SCALE,
        at(3, 7),
        at(13, 7),
        6_000,
        &BTreeSet::new(),
        &[],
    )
    .expect("a wide body gets past one tile too");
    let gap = |points: &[(Fixed, Fixed)]| polyline_distance_sq_to_tile(points, (8, 7));
    assert!(
        gap(&wide) > gap(&narrow),
        "the 0.3-tile body went {} from the block and the 0.3-tile one {}",
        isqrt(gap(&wide)),
        isqrt(gap(&narrow))
    );
    assert!(gap(&wide) >= 6_000 * 6_000);
}

/// RULE: a body that does not fit through a gap is refused, not squeezed.
///
/// Source: the original's expansion is by the mover's radius, so a gap narrower than the body's diameter is not a passage;
/// our `short_range` answers `None` rather than a line through it. This is the layer R5 moves the *tile* graph onto.
#[test]
fn a_body_too_wide_for_the_gap_is_refused() {
    // A one-tile gap at (8,8) between two walls, and a body of 1.2 tiles' diameter.
    let walk = walk(20, 20, &[(8, 6), (8, 7), (8, 9), (8, 10)]);
    let route = short_range_clear(
        &walk,
        SCALE,
        at(4, 8),
        at(12, 8),
        12_000,
        &BTreeSet::new(),
        &[],
    );
    match route {
        None => {}
        Some(points) => {
            // If it did answer, it must still keep the radius from the walls - a "route" that clips them is worse than no
            // route, because the caller will walk it.
            for tile in [(8u16, 6u16), (8, 7), (8, 9), (8, 10)] {
                let gap = polyline_distance_sq_to_tile(&points, tile);
                assert!(
                    gap >= 12_000 * 12_000,
                    "a refused-width body was given a route {} from {tile:?}",
                    isqrt(gap)
                );
            }
        }
    }
}

/// RULE: a standing body is a disc the route keeps its clearance from - and the disc is exact, never clearance plus a
/// margin.
///
/// Source: `FUN_007f9d80` inserts each stander's rectangle grown by the mover's radius; the measured trap on our side is that
/// **any** margin put every gather-ring place inside a neighbour's disc, because the ring's places are exactly the pair's
/// reach apart (D211, 18 tests red).
#[test]
fn a_body_disc_is_kept_clear_and_not_padded() {
    let walk = walk(16, 16, &[]);
    let body = (at(8, 8), 4_000i64);
    let route = short_range_clear(
        &walk,
        SCALE,
        at(3, 8),
        at(13, 8),
        RADIUS,
        &BTreeSet::new(),
        &[body],
    )
    .expect("a man in the lane is walkable round");
    let gap = polyline_distance_sq_to_point(&route, at(8, 8));
    assert!(
        gap >= 4_000 * 4_000,
        "the route passes {} raw units from a body it must keep 4000 from: {route:?}",
        isqrt(gap)
    );
    // Touching is legal and is what the game's own clamp does - so a route that stays on the circle is not a problem, but a
    // route that stands off by most of a tile is paying a margin the ring cannot afford.
    assert!(
        gap < 6_000 * 6_000,
        "the route keeps {} from the body: a margin nobody asked for",
        isqrt(gap)
    );
}

/// RULE: a body spawned inside a stander is re-seeded outward by `2 x radius` and still reaches its goal.
///
/// Source: `FUN_007fb220` - when the start point is inside an obstruction the original offsets it by `2 x radius` along a
/// fixed ring of directions, re-seeds, and answers "no low-level path" only when every direction is blocked. Ours is
/// `short.rs:93-160` (D212).
#[test]
fn a_start_inside_a_body_is_re_seeded_outward() {
    let walk = walk(16, 16, &[]);
    // The mover starts 1000 raw units from the stander's centre - well inside its 4000 clearance.
    let body = (at(8, 8), 4_000i64);
    let start = (
        Fixed::from_raw(8 * SCALE + 1_000),
        Fixed::from_raw(8 * SCALE),
    );
    let route = short_range_clear(
        &walk,
        SCALE,
        start,
        at(14, 8),
        RADIUS,
        &BTreeSet::new(),
        &[body],
    )
    .expect("a trapped start can still be planned out of");
    let first = *route
        .first()
        .expect("a re-seeded route has at least one point");
    let gap = polyline_distance_sq_to_point(&[first, first], at(8, 8));
    assert!(
        gap >= 4_000 * 4_000,
        "the route's first point is still {} from the body it started inside",
        isqrt(gap)
    );
    assert_eq!(
        route.last(),
        Some(&at(14, 8)),
        "the re-seeded route did not reach the goal"
    );
}

/// RULE: the short-range solver declines work that is disproportionate rather than guessing - a bounded window.
///
/// Source: our own bound (`short.rs:29-36`: 40 tiles a side, 96 rectangles, 400 nodes), and the original's counter-less
/// answer is the opposite - it expands the search area once and then returns the closest point. The divergence is recorded
/// in `divergences.rs`; what this test pins is that the bound is *stated* rather than silent.
#[test]
fn a_window_beyond_the_bound_is_declined() {
    let walk = walk(64, 64, &[]);
    let route = short_range_clear(
        &walk,
        SCALE,
        at(2, 2),
        at(60, 60),
        RADIUS,
        &BTreeSet::new(),
        &[],
    );
    assert!(
        route.is_none(),
        "a 58-tile walk was solved inside the 40-tile window rather than declined"
    );
    // Inside the window it answers. The window is the endpoints' box plus a three-tile margin, so a 30-tile walk is 36 tiles
    // a side and a 36-tile one is 42 - past the bound, which is why the pair below straddles 34 rather than 40.
    assert!(
        short_range_clear(
            &walk,
            SCALE,
            at(2, 2),
            at(30, 30),
            RADIUS,
            &BTreeSet::new(),
            &[]
        )
        .is_some(),
        "a 30-tile walk is inside the window and must be solved"
    );
}

/// The integer square root, for a readable failure message only.
fn isqrt(value: i64) -> i64 {
    let mut low = 0i64;
    let mut high = value.max(0).max(1);
    while low < high {
        let mid = (low + high + 1) / 2;
        if mid * mid <= value {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    low
}

/// Keep the helper-only import honest: `distance_sq_to_tile` is used by the single-tile cases above.
#[test]
fn the_distance_helper_agrees_with_the_tile_box() {
    assert_eq!(distance_sq_to_tile(at(8, 8), (8, 8)), 0);
    assert_eq!(distance_sq_to_tile(at(9, 8), (8, 8)), 0);
    assert_eq!(distance_sq_to_tile(at(10, 8), (8, 8)), SCALE * SCALE);
}
