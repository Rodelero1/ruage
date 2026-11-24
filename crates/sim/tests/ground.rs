//! **The ground and the search** (D279): the original's own model, ported bluntly.
//!
//! These fixtures pin the replacement for `path.rs` and `short.rs` - a tile grid with counted occupants, one A\* over it
//! with the original's step costs, and the polyline the mover walks. They are deliberately about **shape**, not about
//! numbers a search happens to return: a route that reaches its goal, never crosses a blocked tile, stays put when the
//! goal is walled in, and costs what the original's table says a step costs.

use ruage_data::{Map, Terrain};
use ruage_fixed::Fixed;
use ruage_sim::Ground;

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

/// A point in raw units, from tile coordinates.
fn at(x: i64, y: i64) -> (Fixed, Fixed) {
    (Fixed::from_raw(x * SCALE), Fixed::from_raw(y * SCALE))
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
    // Nothing in the way, so the route is the goal and nothing else: the search's tile chain is pulled to the corners
    // an obstacle forces, and there is no obstacle.
    let ground = ground(16, 16, &[]);
    let route = ground
        .route(SCALE, at(4, 5), at(12, 5))
        .expect("open ground is routable");
    assert_eq!(
        route.len(),
        1,
        "a straight walk needs one waypoint: {route:?}"
    );
    assert_eq!(route[0], at(12, 5), "and it is the point that was clicked");
}

#[test]
fn a_wall_is_walked_round_and_never_crossed() {
    // A wall of blocked tiles from (8,0) down to (8,10), so the only way past it is the gap below.
    let wall: Vec<(u16, u16)> = (0..=10).map(|y| (8, y)).collect();
    let ground = ground(16, 16, &wall);
    let route = ground
        .route(SCALE, at(4, 5), at(12, 5))
        .expect("there is a way round");
    assert!(route.len() > 1, "a wall forces a corner: {route:?}");
    let last = route.last().copied().expect("a route has an end");
    assert_eq!(last, at(12, 5), "the route ends where it was sent");
    // Every waypoint sits on a walkable tile - the route is not a line through the wall.
    for point in &route {
        let x = u16::try_from(point.0.raw() / SCALE).expect("on the map");
        let y = u16::try_from(point.1.raw() / SCALE).expect("on the map");
        assert!(
            ground.walkable(x, y),
            "waypoint {point:?} is on blocked tile ({x},{y})"
        );
    }
}

#[test]
fn a_walled_in_goal_ends_at_the_closest_reachable_tile() {
    // The original's answer to a goal it cannot reach is "come as close as you can", not "no route": the whole of the
    // (10,10) tile's neighbourhood is blocked, so the search settles beside it and the route ends there.
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
    let route = ground
        .route(SCALE, at(4, 4), at(10, 10))
        .expect("the search settles somewhere");
    let last = route.last().copied().expect("a route has an end");
    let (x, y) = (last.0.raw() / SCALE, last.1.raw() / SCALE);
    // Beside the ring: within one tile of it on both axes, and not inside the wall.
    assert!(
        (x - 10).abs() <= 2 && (y - 10).abs() <= 2,
        "the route stops beside the goal, not at it: ended at ({x},{y})"
    );
    assert!(!ring.contains(&(u16::try_from(x).unwrap(), u16::try_from(y).unwrap())));
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
