//! **RULE: what the ground permits.**
//!
//! Source: `zone_spatial_pyramid_check_tile.cpp` and `zone_spatial_pyramid_check_cell_step.cpp` - a cell is walked if it is
//! free, and a diagonal step needs both of the cells it touches to be free, so a route never clips the corner of the wall it
//! was planned round. Occupancy is **counted** in the original's grid (`zone_spatial_grid_link_occupant.cpp` unlinks one
//! occupant at a time), which is why a tile a tree and a wall both stand on stays blocked until both are gone.

use ruage_conformance::{SCALE, centre, ground, tiles_of};

/// RULE: a diagonal may not pass between two blocked tiles.
///
/// A body has a radius, so the two grown boxes overlap at the shared corner and seal it: the route goes
/// round, never through the corner point. (A point route threads the exact corner - there is nothing there
/// for a point to touch - which is why this row walks with a villager's body, the way the game does.)
#[test]
fn a_diagonal_may_not_cut_between_two_blocked_tiles() {
    // (4,4) free, (5,5) free, and the two tiles the diagonal squeezes between blocked.
    let field = ground(12, 12, &[(5, 4), (4, 5)]);
    let route = field
        .route_avoiding(SCALE, centre(4, 4), centre(5, 5), ruage_conformance::RADIUS, &[], true)
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

/// RULE: the route never doubles back, and no leg of it passes through blocked ground. A hop can be
/// several tiles long - that is what a clear line buys - so the claim is about repeats and about the grown boxes,
/// not about one tile a step. Waypoints sit on grown faces (which a tile query reads as blocked ground), so the
/// shape the route certifies against is the boxes, not the tiles.
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
    // No point inside a grown box, no leg through one: the wall as the boxes the router certifies against.
    let grown: Vec<ruage_sim::Clear> = wall
        .iter()
        .map(|(x, y)| ruage_sim::Clear {
            at: (i64::from(*x) * SCALE + SCALE / 2, i64::from(*y) * SCALE + SCALE / 2),
            half: (SCALE / 2, SCALE / 2),
            reach: 0,
        })
        .collect();
    let raw: Vec<(i64, i64)> = route
        .iter()
        .map(|point| (point.0.raw(), point.1.raw()))
        .collect();
    assert!(
        ruage_sim::verify_path(&raw, &grown).is_none(),
        "the route crosses the wall: {route:?}"
    );
}
