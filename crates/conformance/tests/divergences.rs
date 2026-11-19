//! **Where this port stands apart from the original, named rather than wished for.**
//!
//! D279 ports the original's *shape*: a grid, one A\*, the four collision refusals, one whole step. What it does not carry is
//! everything the original layers **on top** of that shape for speed or precision. Each one is `#[ignore]`d with what it
//! would cost and the decision that parked it, so `cargo test -p ruage-conformance -- --ignored` prints the gap.

use ruage_conformance::{SCALE, centre, ground};

/// RULE: the original searches a **hierarchy** - a node carries a level, a coarse node spans `<< level` tiles, and an edge is
/// refined to a tile on the shared boundary (`zone_spatial_pyramid_search_hierarchical.cpp`,
/// `zone_spatial_pyramid_check_edge.cpp`).
///
/// **Not ported** (D279): this search is a flat A\* over every tile. On the project map (64 x 64) a corner-to-corner search
/// is well under a millisecond, so the hierarchy buys nothing to measure at this size; it becomes worth its complexity on a
/// 480 x 480 map with a hundred units planning per tick.
#[test]
#[ignore = "flat A* on purpose (D279): the hierarchy is a speed layer, priced in docs/next-step.md"]
fn the_original_solves_a_long_walk_in_its_hierarchy() {
    // A 58-tile walk across open ground, which is the case the hierarchy exists for.
    let field = ground(96, 96, &[]);
    let route = field
        .route(SCALE, centre(4, 4), centre(62, 4))
        .expect("a long walk across open ground");
    assert_eq!(route.len(), 1, "an open walk is one waypoint");
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

/// RULE: the original stops expanding when its estimate passes a bound, so a hopeless search costs a bounded number of cells
/// (`zone_spatial_pyramid_search_path.cpp`, the `expansions > h` guard at `0x801861`).
///
/// **Not ported** (D279): no budget. A whole-map search costs what the map costs, and a goal walled in is answered by the
/// nearest cell after the search has settled every reachable tile. On this map that is microseconds; on a full-size map with
/// many hopeless searches per tick it is the first thing to put back.
#[test]
#[ignore = "no expansion budget on purpose (D279): the map is small enough that it does not bind"]
fn the_original_bounds_a_hopeless_search() {
    // A goal inside a sealed room: the search settles the room's outside and stops, however big the map is.
    let mut sealed = Vec::new();
    for x in 8..=12u16 {
        sealed.push((x, 8));
        sealed.push((x, 12));
    }
    for y in 8..=12u16 {
        sealed.push((8, y));
        sealed.push((12, y));
    }
    let field = ground(64, 64, &sealed);
    let route = field
        .route(SCALE, centre(2, 2), centre(10, 10))
        .expect("the search settles outside the room");
    let last = route.last().copied().expect("a route has an end");
    let tile = (
        u16::try_from(last.0.raw().div_euclid(SCALE)).unwrap_or(0),
        u16::try_from(last.1.raw().div_euclid(SCALE)).unwrap_or(0),
    );
    assert!(!sealed.contains(&tile), "the route stops outside the room at {tile:?}");
}
