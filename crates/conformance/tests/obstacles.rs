//! **RULE: what a goal the ground refuses is answered with, and what is not an obstacle at all.**
//!
//! Source: `zone_spatial_pyramid_search_hierarchical.cpp` - the original's search settles on the best cell it can reach and
//! the unit walks there, which is why every click in the game is accepted even when it lands in a wood. Bodies are **not** in
//! that search: `zone_spatial_pyramid_check_tile.cpp` asks the cell's attributes, and units are answered later, by the
//! collision block (`zone_spatial_entity_step_collision_avoidance.cpp`). D279 adopted that split: the planner knows walls
//! only, and the step pass knows bodies.

use ruage_conformance::{SCALE, centre, ground, tiles_of};

/// RULE: a wall is walked round, never crossed, and the route still reaches the goal.
#[test]
fn a_wall_is_walked_round_and_never_crossed() {
    let wall: Vec<(u16, u16)> = (0..=10).map(|y| (8, y)).collect();
    let field = ground(20, 20, &wall);
    let route = field
        .route(SCALE, centre(4, 5), centre(12, 5))
        .expect("there is a way round");
    assert_eq!(
        route.last().copied(),
        Some(centre(12, 5)),
        "the route ends where it was sent"
    );
    for point in &route {
        let tile = (
            u16::try_from(point.0.raw().div_euclid(SCALE)).unwrap_or(0),
            u16::try_from(point.1.raw().div_euclid(SCALE)).unwrap_or(0),
        );
        assert!(
            !wall.contains(&tile),
            "a waypoint stands on the wall at {tile:?}"
        );
    }
}

/// RULE: a goal walled in is answered with the **closest reachable tile**, not with a refusal. This is the rule that makes
/// every click in the game accepted.
#[test]
fn a_walled_in_goal_is_answered_with_the_closest_tile() {
    let ring = [
        (9u16, 9u16),
        (10, 9),
        (11, 9),
        (9, 10),
        (11, 10),
        (9, 11),
        (10, 11),
        (11, 11),
    ];
    let field = ground(20, 20, &ring);
    let route = field
        .route(SCALE, centre(4, 4), centre(10, 10))
        .expect("the search settles somewhere");
    let last = route.last().copied().expect("a route has an end");
    let (x, y) = (last.0.raw() / SCALE, last.1.raw() / SCALE);
    assert!(
        (x - 10).abs() <= 2 && (y - 10).abs() <= 2,
        "the route stops beside the walled goal, not at it: ended at ({x},{y})"
    );
    let chain = tiles_of((4, 4), &route);
    for tile in &chain {
        assert!(
            !ring.contains(tile),
            "the route entered the wall at {tile:?}"
        );
    }
}

/// RULE: **bodies are not an obstacle the `Ground` knows about.** The planner has no body *layer* - a `Ground` cannot hold
/// one - so a route through a crowd is the same route through open ground. **The planner does carry bodies**, as `Clear`
/// rectangles handed to `route_avoiding` (D282, D331: it plans round every stander), and the collision block is what answers a
/// body the plan could not avoid. The earlier wording here ("the planner has no body layer") was true of `Ground` and false of
/// the planner since D282 (D334).
#[test]
fn the_search_has_no_body_layer() {
    // Two "crowded" tiles are simply not blocked: nothing in the planner can express a unit.
    let field = ground(16, 16, &[]);
    let straight = field
        .route(SCALE, centre(2, 5), centre(12, 5))
        .expect("open ground is routable");
    assert_eq!(
        straight.len(),
        1,
        "a clear line through the crowd is one waypoint: {straight:?}"
    );
}
