//! **Where this port stands apart from the original, named rather than wished for.**
//!
//! D279 ports the original's *shape*: a grid, one A\*, the four collision refusals, one whole step. What it does not carry is
//! everything the original layers **on top** of that shape for speed or precision. Each one is `#[ignore]`d with what it
//! would cost and the decision that parked it, so `cargo test -p ruage-conformance -- --ignored` prints the gap.
//!
//! **Two rows that were parked here have since landed** (the levelled search in D296-D302, the radius in D303-D306) and are now
//! ordinary conformance rows rather than divergences: an `#[ignore]` whose cause is fixed is decoration, and one of them had been
//! counting as a live divergence for two weeks after the engine caught up.
//!
//! New Way Task 5 replaced the tile search with the geometric solver: the hierarchy row below now pins the solver's
//! answer instead of the coarse chain, and the radius row pins grown boxes instead of the search.

use ruage_conformance::{SCALE, centre, ground};

/// RULE: a long walk is answered, end to end, without crossing what blocks it.
///
/// A 58-tile walk across open ground is the case the original's hierarchy exists for; the solver answers it direct:
/// the polyline is verify-clean against the (empty) avoid set and ends on the goal.
#[test]
fn the_original_solves_a_long_walk_in_its_hierarchy() {
    let field = ground(96, 96, &[]);
    let route = field
        .route(SCALE, centre(4, 4), centre(62, 4))
        .expect("a long walk across open ground");
    assert_eq!(
        route.last().copied(),
        Some(centre(62, 4)),
        "the route ends where it was sent: {route:?}"
    );
    let raw: Vec<(i64, i64)> = route
        .iter()
        .map(|point| (point.0.raw(), point.1.raw()))
        .collect();
    assert!(
        ruage_sim::verify_path(&raw, &[]).is_none(),
        "the route is clean: {route:?}"
    );
}

/// RULE: the original plans for **the boundaries of where this unit's radii will go** - its search treats every obstruction
/// as grown by the mover's radius, so a body is planned by its width and not by a point
/// (`zone_spatial_pyramid_check_cell_step.cpp`, and the Forgotten Empires talk's "shrink the unit to a point, expand the
/// obstruction"). **Tangency is a passage**: the owner measured a packed trebuchet walking a one-tile hole in a wall and a
/// one-tile corridor between bamboo (2026-11-20), so a body whose diameter *is* the gap is planned through it.
///
/// **Ported** in D303-D306, **re-ported** onto grown boxes in New Way Task 5: `Ground::route_avoiding` carries the reach
/// into the solver, which grows every box by it, and touching a grown face stays clear. The
/// engine's own witnesses are `radius_route` and `straight_line`.
#[test]
fn the_search_grows_obstacles_by_the_bodys_radius() {
    // A one-tile gap at (6, 6) in a wall down x = 6: a body a tile wide fits with zero clearance, one wider does not.
    let wall: Vec<(u16, u16)> = (0..=12).filter(|y| *y != 6).map(|y| (6, y)).collect();
    let field = ground(14, 14, &wall);
    let (from, to) = (centre(3, 6), centre(9, 6));

    let narrow = field
        .route_avoiding(SCALE, from, to, SCALE / 2, &[], true)
        .expect("a route toward the gap");
    assert_eq!(
        narrow.last().copied(),
        Some(to),
        "a body exactly one tile wide was not planned through a one-tile gap: {narrow:?}"
    );

    let wide = field.route_avoiding(SCALE, from, to, SCALE * 6 / 10, &[], true);
    let reached = wide.as_ref().and_then(|route| route.last().copied()) == Some(to);
    assert!(
        !reached,
        "a body 1.2 tiles wide was planned through a one-tile gap: {wide:?}"
    );
}
