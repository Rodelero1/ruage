//! **Where this port stands apart from the original, named rather than wished for.**
//!
//! D279 ports the original's *shape*: a grid, one A\*, the four collision refusals, one whole step. What it does not carry is
//! everything the original layers **on top** of that shape for speed or precision. Each one is `#[ignore]`d with what it
//! would cost and the decision that parked it, so `cargo test -p ruage-conformance -- --ignored` prints the gap.

use ruage_conformance::{SCALE, centre, ground};

/// RULE: the original searches a **hierarchy** - a node carries a level, a coarse node spans `<< level` tiles, and the route it hands
/// back is the **refined coarse chain**: `FUN_007ee980` walks the parent chain a coarse node at a time and pushes a waypoint per node
/// (`FUN_007edb80` for the node's tile, `FUN_007eeda0` to step toward the next, `FUN_007fd880` to push the point), so the waypoints
/// land on **cell centres of the level the search started at** - `n * 2^level + 0.5` tiles.
///
/// **Not ported yet**: this search is a flat A\* over every tile and answers a long open walk with **one** waypoint. P11 is building the
/// levelled search (`docs/pathfinding-levels-evidence.md`): 11a the level maps and 11b the descent are landed, 11c is the coarse A\* and
/// this refinement.
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
    for point in &points[..points.len() - 1] {
        let x = point.0.raw();
        let y = point.1.raw();
        // A level-3 cell is 8 tiles across, so its centre is `n * 8 + 4` tiles - 4 is half of 8, not the half-tile of a
        // tile centre. My first version of this check used the tile convention and flagged a correct 4.00 as wrong.
        let on_lattice = |raw: i64| (raw - 4 * SCALE) % (8 * SCALE) == 0;
        assert!(
            on_lattice(x) && on_lattice(y),
            "waypoint ({:.2}, {:.2}) is not on the 8-tile lattice the level-3 search works on",
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
/// as grown by the mover's radius, so it can thread a gap narrower than a tile
/// (`zone_spatial_pyramid_check_cell_step.cpp`, and the Forgotten Empires talk's "shrink the unit to a point, expand the
/// obstruction").
///
/// **Not ported** (D279): the search asks tiles, and the body's radius is applied by the **step** (`World::clearance_at`).
/// The consequence is real and visible: a body whose radius will not fit down a one-tile lane is planned through it and
/// refuses its steps there, where the original would have planned round. Every route in this engine is therefore a route a
/// *point* could walk.
#[test]
#[ignore = "tile search on purpose (D279): the radius lives in the step, not in the planner"]
fn the_original_plans_for_the_body_radius() {
    // A one-tile gap: a radius-1/5 body fits, a radius-2/5 body does not.
    let wall: Vec<(u16, u16)> = (0..=12).filter(|y| *y != 6).map(|y| (6, y)).collect();
    let field = ground(14, 14, &wall);
    let route = field
        .route(SCALE, centre(3, 6), centre(9, 6))
        .expect("the gap is a tile wide");
    assert_eq!(route.len(), 1, "the gap is a straight line on the tiles");
}
