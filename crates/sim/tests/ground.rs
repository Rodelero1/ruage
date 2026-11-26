//! **The ground and the router** (D279, New Way Task 5): a tile grid with counted occupants, and one
//! geometric router over it.
//!
//! These fixtures pin the replacement for `path.rs` and `short.rs`: the grid the step still stands on, the cost
//! table it still charges, and the polyline the solver walks. They are deliberately about **shape**, not about
//! numbers a search happens to return: a route that reaches its goal, never crosses a blocked area, stops beside
//! the goal when it is walled in, and costs what the original's table says a step costs.

use ruage_data::{Map, Terrain};
use ruage_fixed::Fixed;
use ruage_sim::{Clear, Ground, verify_path};

/// One tile is 20,000 raw units.
const SCALE: i64 = 20_000;

fn open_map(width: u16, height: u16) -> Map {
    Map {
        width,
        height,
        terrain: vec![Terrain::Grass; usize::from(width) * usize::from(height)],
        resources: Vec::new(),
        town_centers: Vec::new(),
    }
}

fn ground(width: u16, height: u16, blocked: &[(u16, u16)]) -> Ground {
    let mut ground = Ground::from_map(&open_map(width, height));
    for (x, y) in blocked {
        ground.block(*x, *y);
    }
    ground
}

/// A point in raw units, from tile coordinates: the tile's **corner**, which is where a tile number starts.
#[allow(dead_code)]
fn at(x: i64, y: i64) -> (Fixed, Fixed) {
    (Fixed::from_raw(x * SCALE), Fixed::from_raw(y * SCALE))
}

/// The **centre** of a tile in raw units, which is where real callers stand and click.
fn centre(x: i64, y: i64) -> (Fixed, Fixed) {
    (
        Fixed::from_raw(x * SCALE + SCALE / 2),
        Fixed::from_raw(y * SCALE + SCALE / 2),
    )
}

/// The blocked tiles as the grown boxes the router certifies against: a bare grid object grown by the mover.
fn grown_boxes(blocked: &[(u16, u16)], reach: i64) -> Vec<Clear> {
    blocked
        .iter()
        .map(|(x, y)| Clear {
            at: (
                i64::from(*x) * SCALE + SCALE / 2,
                i64::from(*y) * SCALE + SCALE / 2,
            ),
            half: (SCALE / 2, SCALE / 2),
            reach,
        })
        .collect()
}

fn raw(route: &[(Fixed, Fixed)]) -> Vec<(i64, i64)> {
    route
        .iter()
        .map(|point| (point.0.raw(), point.1.raw()))
        .collect()
}

#[test]
fn the_originals_step_costs_are_sixteen_and_twenty_three() {
    // The original's cost table (`docs/archive/research/movement-in-the-binary.md`, `FUN_008007f0`): a cardinal step costs 16 and a
    // diagonal 23 - 16 x sqrt2, rounded.
    assert_eq!(Ground::step_cost((4, 4), (5, 4)), 16);
    assert_eq!(Ground::step_cost((4, 4), (5, 5)), 23);
    assert_eq!(Ground::step_cost((4, 4), (4, 4)), 0);
}

#[test]
fn a_clear_walk_is_one_straight_line() {
    // Nothing in the way, so the walk is the goal and nothing else: the solver's straight leg needs no corner,
    // and the standing point is dropped, so the route holds only the waypoints ahead of the body.
    // Centres, like every real caller: a corner-to-corner line runs along grid lines and only grazes faces.
    let ground = ground(16, 16, &[]);
    let route = ground
        .route(SCALE, centre(4, 5), centre(12, 5))
        .expect("open ground is routable");
    assert_eq!(
        route,
        vec![centre(12, 5)],
        "a straight walk is one waypoint, and it is the point that was clicked: {route:?}"
    );
}

#[test]
fn a_wall_is_walked_round_and_never_crossed() {
    // A wall of blocked tiles from (8,0) down to (8,10), so the only way past it is the gap below.
    // Centres, like every real caller: a corner-to-corner line runs along grid lines and only grazes faces.
    let wall: Vec<(u16, u16)> = (0..=10).map(|y| (8, y)).collect();
    let ground = ground(16, 16, &wall);
    let route = ground
        .route(SCALE, centre(4, 5), centre(12, 5))
        .expect("there is a way round");
    assert!(route.len() > 1, "a wall forces a corner: {route:?}");
    let last = route.last().copied().expect("a route has an end");
    assert_eq!(last, centre(12, 5), "the route ends where it was sent");
    // The polyline never enters a grown box: no point inside one, no leg through one.
    assert!(
        verify_path(&raw(&route), &grown_boxes(&wall, 0)).is_none(),
        "the route crosses the wall: {route:?}"
    );
}

#[test]
fn a_walled_in_goal_ends_beside_the_goal() {
    // The original's answer to a goal it cannot reach is "come as close as you can", not "no route": the whole of the
    // (10,10) tile's neighbourhood is blocked, so the solver stops beside it and the route settles on the settled
    // tile's centre.
    let ring = [
        (9, 9),
        (10, 9),
        (11, 9),
        (9, 10),
        (11, 10),
        (9, 11),
        (10, 11),
        (11, 11),
    ];
    let ground = ground(20, 20, &ring);
    let centre = |x: i64, y: i64| {
        (
            Fixed::from_raw(x * SCALE + SCALE / 2),
            Fixed::from_raw(y * SCALE + SCALE / 2),
        )
    };
    let route = ground
        .route(SCALE, centre(4, 4), centre(10, 10))
        .expect("the solver stops beside the goal");
    let last = route.last().copied().expect("a route has an end");
    let (x, y) = (last.0.raw() / SCALE, last.1.raw() / SCALE);
    // Beside the ring: within two tiles of it on both axes, and not inside the wall.
    assert!(
        (x - 10).abs() <= 2 && (y - 10).abs() <= 2,
        "the route stops beside the goal, not at it: ended at ({x},{y})"
    );
    assert!(!ring.contains(&(u16::try_from(x).unwrap(), u16::try_from(y).unwrap())));
    // The settled stop is the tile's centre, and the snapped polyline stays clean.
    assert_eq!(
        (last.0.raw() % SCALE, last.1.raw() % SCALE),
        (SCALE / 2, SCALE / 2),
        "the stop is the settled tile's centre: {last:?}"
    );
    assert!(
        verify_path(&raw(&route), &grown_boxes(&ring, 0)).is_none(),
        "the settled route crosses the ring: {route:?}"
    );
}

#[test]
fn a_line_is_clear_only_when_no_blocked_tile_stands_on_it() {
    let ground = ground(16, 16, &[(8, 5)]);
    assert!(
        !ground.line_clear((4, 5), (12, 5)),
        "the blocked tile is on the line"
    );
    assert!(
        ground.line_clear((4, 6), (12, 6)),
        "a clear parallel line is clear"
    );
    // The goal's own tile is the one exemption, everywhere in this engine: a node's tile holds the thing being worked.
    assert!(
        ground.line_clear((4, 5), (8, 5)),
        "a line may end on the tile it was sent to"
    );
}

#[test]
fn an_occupant_left_on_a_tile_frees_it_again() {
    // Counted, not flagged: the same tile can hold a tree and a wall, and an unbalanced pair leaves it blocked for ever.
    let mut ground = ground(8, 8, &[]);
    assert!(ground.walkable(3, 3));
    ground.block(3, 3);
    ground.block(3, 3);
    ground.unblock(3, 3);
    assert!(
        !ground.walkable(3, 3),
        "one occupant is still standing there"
    );
    ground.unblock(3, 3);
    assert!(ground.walkable(3, 3), "both are gone");
}
