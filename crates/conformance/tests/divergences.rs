//! **Where this port stands apart from the original, named rather than wished for.**
//!
//! D279 ports the original's *shape*: a grid, one A\*, the four collision refusals, one whole step. What it does not carry is
//! everything the original layers **on top** of that shape for speed or precision. Each one is `#[ignore]`d with what it
//! would cost and the decision that parked it, so `cargo test -p ruage-conformance -- --ignored` prints the gap.
//!
//! **Two rows that were parked here have since landed** (the levelled search in D296-D302, the radius in D303-D306) and are now
//! ordinary conformance rows rather than divergences: an `#[ignore]` whose cause is fixed is decoration, and one of them had been
//! counting as a live divergence for two weeks after the engine caught up.

use ruage_conformance::{SCALE, centre, ground};

/// RULE: the original searches a **hierarchy** - a node carries a level, a coarse node spans `<< level` tiles, and the route it hands
/// back is the **refined coarse chain**: `FUN_007ee980` walks the parent chain a coarse node at a time and pushes a waypoint per node
/// (`FUN_007edb80` for the node's tile, `FUN_007eeda0` to step toward the next, `FUN_007fd880` to push the point).
///
/// **Where a step's waypoint lands** is `FUN_007eeda0`'s answer, not a cell centre (D322): it expands the cell to the tile span it covers,
/// scans that span tile by tile along the direction of travel, and takes **the first tile the step can pass**, so the point sits **on the
/// boundary between the two cells** with the other coordinate at that tile's middle. The first point stays the start cell's centre (the
/// caller discards it - the walk begins where the body stands) and there is no trailing centre: the goal is the caller's own point.
///
/// **Ported** in D296-D302 and D322: `Levels::coarse_route` is the coarse A\* and `Levels::refine` the chain, and the land unit's route
/// takes that tier for walks past the flat search's useful range.
#[test]
fn the_original_solves_a_long_walk_in_its_hierarchy() {
    // A 58-tile walk across open ground, which is the case the hierarchy exists for.
    let field = ground(96, 96, &[]);
    let levels = ruage_sim::Levels::of(&field);
    let route = levels
        .coarse_route((4, 4), (62, 4))
        .expect("a long walk across open ground");
    let points = levels.refine(&route, SCALE);

    // **The observable of a coarse route**: more than one waypoint, every one but the last on the 8-tile lattice the level-3 search
    // works on, and every one of them a cell the search was allowed to enter.
    assert!(
        points.len() > 1,
        "a 58-tile walk came back as {} waypoint(s): the flat search answers one, the levelled one answers the coarse chain",
        points.len()
    );
    // Every point after the first is **a crossing of a level-3 boundary**: the step's own coordinate on the `n * 8` line, the other
    // one at a tile's middle along it. (Before D322 this row asserted cell centres; its own RULE paragraph already cited
    // `FUN_007eeda0` as the thing that decides the point, so the assertion was the stale half.)
    for point in &points[1..] {
        let x = point.0.raw();
        let y = point.1.raw();
        let on_boundary = |raw: i64| raw % (8 * SCALE) == 0;
        let at_middle = |raw: i64| raw % SCALE == SCALE / 2;
        assert!(
            (on_boundary(x) && at_middle(y)) || (at_middle(x) && on_boundary(y)),
            "waypoint ({:.2}, {:.2}) is not a crossing of a level-3 boundary",
            x as f64 / SCALE as f64,
            y as f64 / SCALE as f64
        );
    }
    for cell in &route.cells {
        assert!(
            !levels.blocked(route.level, cell.0, cell.1),
            "the chain runs through blocked cell {cell:?}"
        );
    }
}

/// RULE: the original plans for **the boundaries of where this unit's radii will go** - its search treats every obstruction
/// as grown by the mover's radius, so a body is planned by its width and not by a point
/// (`zone_spatial_pyramid_check_cell_step.cpp`, and the Forgotten Empires talk's "shrink the unit to a point, expand the
/// obstruction"). **Tangency is a passage**: the owner measured a packed trebuchet walking a one-tile hole in a wall and a
/// one-tile corridor between bamboo (2026-11-20), so a body whose diameter *is* the gap is planned through it.
///
/// **Ported** in D303-D306 - this row was `#[ignore]`d as "the radius lives in the step, not in the planner" until then:
/// `Ground::route_avoiding` carries the reach into the search, `Ground::fits` allows zero clearance, and the pyramid is built per radius. The
/// engine's own witnesses are `radius_route` and `straight_line`.
#[test]
fn the_search_grows_obstacles_by_the_bodys_radius() {
    // A one-tile gap at (6, 6) in a wall down x = 6: a body a tile wide fits with zero clearance, one wider does not.
    let wall: Vec<(u16, u16)> = (0..=12).filter(|y| *y != 6).map(|y| (6, y)).collect();
    let field = ground(14, 14, &wall);
    let (from, to) = (centre(3, 6), centre(9, 6));

    let narrow = field
        .route_avoiding(SCALE, from, to, SCALE / 2, &[])
        .expect("a route toward the gap");
    assert_eq!(
        narrow.last().copied(),
        Some(to),
        "a body exactly one tile wide was not planned through a one-tile gap: {narrow:?}"
    );

    let wide = field.route_avoiding(SCALE, from, to, SCALE * 6 / 10, &[]);
    let reached = wide.as_ref().and_then(|route| route.last().copied()) == Some(to);
    assert!(
        !reached,
        "a body 1.2 tiles wide was planned through a one-tile gap: {wide:?}"
    );
}
