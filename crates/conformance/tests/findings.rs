//! **RULE: where a click lands, when the ground will not take it.**
//!
//! Source: `zone_spatial_pyramid_search_hierarchical.cpp` (the search returns the best cell it settled on) and
//! `zone_path_eval.cpp` (the click point is what the order asked for, not the tile centre). Together they say: a route's last
//! waypoint is the **clicked point** when the goal tile is reachable, and the settled tile's centre when it is not.

use ruage_conformance::{SCALE, centre, ground};

/// RULE: the last waypoint is the clicked point, not the centre of the goal tile.
#[test]
fn the_route_ends_on_the_point_that_was_clicked() {
    let field = ground(16, 16, &[]);
    let click = (
        ruage_fixed::Fixed::from_raw(7 * SCALE + SCALE / 3),
        ruage_fixed::Fixed::from_raw(9 * SCALE + SCALE / 4),
    );
    let route = field
        .route(SCALE, centre(3, 9), click)
        .expect("open ground is routable");
    assert_eq!(
        route.last().copied(),
        Some(click),
        "the order lands where it pointed, not at a tile centre"
    );
}

/// RULE: a click **inside a wood** lands on the edge of the wood, and the route is real - not empty, not a refusal.
#[test]
fn a_click_into_a_wood_lands_on_the_edge_of_the_wood() {
    // A three-by-three block of blocked tiles with a clear field around it.
    let wood: Vec<(u16, u16)> = (8..=10)
        .flat_map(|y| (8..=10).map(move |x| (x, y)))
        .collect();
    let field = ground(20, 20, &wood);
    let route = field
        .route(SCALE, centre(4, 10), centre(9, 9))
        .expect("the search settles on the wood's edge");
    let last = route.last().copied().expect("a route has an end");
    let tile = (
        u16::try_from(last.0.raw().div_euclid(SCALE)).unwrap_or(0),
        u16::try_from(last.1.raw().div_euclid(SCALE)).unwrap_or(0),
    );
    assert!(
        field.walkable(tile.0, tile.1),
        "the route ends on ground a body can stand on, not in the wood: {tile:?}"
    );
    assert!(
        tile.0.abs_diff(9) <= 2 && tile.1.abs_diff(9) <= 2,
        "and it ends beside the click, not across the map: {tile:?}"
    );
}
