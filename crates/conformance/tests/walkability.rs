//! **RULE: what the ground permits.**
//!
//! Source: `zone_spatial_pyramid_check_tile.cpp` and `zone_spatial_pyramid_check_cell_step.cpp` - a cell is walked if it is
//! free, and a diagonal step needs both of the cells it touches to be free, so a route never clips the corner of the wall it
//! was planned round. Occupancy is **counted** in the original's grid (`zone_spatial_grid_link_occupant.cpp` unlinks one
//! occupant at a time), which is why a tile a tree and a wall both stand on stays blocked until both are gone.

use ruage_conformance::{SCALE, centre, ground, tiles_of};

/// RULE: a diagonal may not pass between two blocked tiles.
#[test]
fn a_diagonal_may_not_cut_between_two_blocked_tiles() {
    // (4,4) free, (5,5) free, and the two tiles the diagonal squeezes between blocked.
    let field = ground(12, 12, &[(5, 4), (4, 5)]);
    let route = field
        .route(SCALE, centre(4, 4), centre(5, 5))
        .expect("there is a way, even if not the diagonal");
    let chain = tiles_of((4, 4), &route);
    for pair in chain.windows(2) {
        let (from, to) = (pair[0], pair[1]);
        if from.0 != to.0 && from.1 != to.1 {
            let side_a = (to.0, from.1);
            let side_b = (from.0, to.1);
            assert!(
                field.walkable(side_a.0, side_a.1) || field.walkable(side_b.0, side_b.1),
                "the route cut the corner between {side_a:?} and {side_b:?}: {chain:?}"
            );
        }
    }
}

/// RULE: a line may **end** on a blocked tile - a node's own tile holds the thing being worked - but may not cross one.
#[test]
fn a_line_may_end_on_a_blocked_tile_but_not_cross_one() {
    let field = ground(12, 12, &[(6, 4)]);
    assert!(
        field.line_clear((2, 4), (6, 4)),
        "a line ending on the tile it was sent to is clear"
    );
    assert!(
        !field.line_clear((2, 4), (9, 4)),
        "a line that crosses the blocked tile is not"
    );
}

/// RULE: the map edge is a wall. A search never leaves the map and a line never runs off it.
#[test]
fn the_map_edge_is_a_wall() {
    let field = ground(10, 10, &[]);
    assert!(!field.contains(10, 5), "x = 10 is off a ten-wide map");
    assert!(!field.contains(5, 10), "y = 10 is off a ten-high map");
    // A route whose goal is off the map is not planned at all: there is no ground to stand on.
    assert!(
        field
            .route(SCALE, centre(2, 2), centre(40, 40))
            .is_none(),
        "a goal off the map has no ground"
    );
}

/// RULE: a tile held by two things stays held until both leave.
#[test]
fn a_tile_held_by_two_things_stays_held_until_both_leave() {
    let mut field = ground(8, 8, &[]);
    assert!(field.walkable(3, 3));
    field.block(3, 3);
    field.block(3, 3);
    assert!(!field.walkable(3, 3));
    field.unblock(3, 3);
    assert!(
        !field.walkable(3, 3),
        "one occupant is still standing there"
    );
    field.unblock(3, 3);
    assert!(field.walkable(3, 3), "both are gone");
}

/// RULE: the route never doubles back, and every waypoint is a tile a body may stand on. The **pull** means a hop can be
/// several tiles long - that is what a clear line buys - so the claim is about repeats and about free ground, not about one
/// tile a step.
#[test]
fn a_route_never_repeats_a_tile_and_every_waypoint_is_free() {
    let wall: Vec<(u16, u16)> = (0..=6).map(|y| (6, y)).collect();
    let field = ground(14, 14, &wall);
    let route = field
        .route(SCALE, centre(2, 2), centre(10, 2))
        .expect("a way round exists");
    let chain = tiles_of((2, 2), &route);
    let mut seen = Vec::new();
    for tile in &chain {
        assert!(
            !seen.contains(tile),
            "the route repeats {tile:?}: {chain:?}"
        );
        seen.push(*tile);
    }
    // The last waypoint is the goal itself; every one before it is a tile centre, so every one is free ground.
    for point in &route[..route.len().saturating_sub(1)] {
        let tile = (
            u16::try_from(point.0.raw().div_euclid(SCALE)).unwrap_or(0),
            u16::try_from(point.1.raw().div_euclid(SCALE)).unwrap_or(0),
        );
        assert!(
            field.walkable(tile.0, tile.1),
            "a waypoint sits on blocked ground {tile:?}"
        );
    }
}
