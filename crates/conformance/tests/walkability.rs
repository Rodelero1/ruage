//! **Rule set 1: what counts as a passage.**
//!
//! Read from `zone_spatial_pyramid_check_tile.cpp` (`func_001D29F0`, PS2) - a tile carries a **four-bit neighbour mask**
//! (N `0x2`, S `0x4`, W `0x1`, E `0x8`), so passability is a property of an *edge between two tiles*, not of a tile's own
//! centre - and from the same module's search, which only ever expands to a neighbour whose mask bit is set.
//!
//! The engine's version of the same rule is in `crates/sim/src/path.rs`: a cardinal step needs its own tile free, and a
//! **diagonal step needs both of the tiles it brushes**: a diagonal that would clip a corner is not a passage.

use ruage_conformance::{DIAGONAL, at, octile, route_cost, walk};
use ruage_sim::{find_path, line_clear};

/// RULE: a diagonal step is legal only if both tiles it brushes are free - the corner of a wall is a wall.
///
/// Source: `zone_spatial_pyramid_check_tile.cpp` (`func_001D29F0`) masks the four edges; the corner case is the pair the
/// mask cannot describe, and the search resolves it the other way - `path.rs:271-280`.
#[test]
fn a_diagonal_may_not_cut_between_two_blocked_tiles() {
    // (2,1) and (1,2) blocked: (1,1) and (2,2) are free, and only touch at the shared corner.
    let walk = walk(8, 8, &[(2, 1), (1, 2)]);
    assert!(
        !line_clear(&walk, (1, 1), (2, 2)),
        "a straight line between two diagonal free tiles clipped both blocked corners and was called clear"
    );
    let route = find_path(&walk, (1, 1), (2, 2)).expect("there is a way round");
    assert!(
        route_cost((1, 1), &route) > DIAGONAL,
        "the route took the one-step diagonal between two blocked corners: {route:?}"
    );
}

/// RULE: two tiles that touch only at a corner are not a passage, even when both are free.
///
/// Source: the same corner rule read the other way round - `find_path`'s diagonal test consults the *intermediate* tiles, so
/// a diagonal between two blocked tiles on either side is refused whatever the endpoints say (`path.rs:271-280`).
#[test]
fn two_free_tiles_in_a_blocked_diagonal_are_not_a_gap() {
    // The blocked pair runs NE-SW; the free pair crosses it NW-SE.
    let walk = walk(9, 9, &[(5, 5), (6, 6)]);
    assert!(
        !line_clear(&walk, (5, 6), (6, 5)),
        "a line through the corner where two blocked tiles meet was called clear"
    );
    let route = find_path(&walk, (5, 6), (6, 5)).expect("the long way round exists");
    assert!(
        route_cost((5, 6), &route) > DIAGONAL,
        "the search crossed the blocked corner: {route:?}"
    );
}

/// RULE: a blocked tile may hold the goal - the node being worked stands on its own tile.
///
/// Source: `zone_spatial_pyramid_find_path.cpp` builds its frontier from passable tiles but treats the *target* cell as
/// reachable when it is the search's goal; our side states it at `path.rs:281` and `path.rs:184-186`.
#[test]
fn the_goal_may_be_a_blocked_tile() {
    let walk = walk(10, 10, &[(7, 5)]);
    let route = find_path(&walk, (2, 5), (7, 5)).expect("a tree is reachable");
    assert_eq!(
        route.last(),
        Some(&(7, 5)),
        "the route stopped short of the blocked goal: {route:?}"
    );
    // Everything before the goal is free ground.
    for tile in &route[..route.len() - 1] {
        assert!(!walk.blocked(tile.0, tile.1), "the route crossed {tile:?}");
    }
}

/// RULE: a blocked start may be left - a worker stands on the tile of the thing it is working.
///
/// Source: the same goal exemption, applied at the other end (`path.rs:210-211`), and the original's separate start
/// re-seed for a body-blocked start (`FUN_007fb220`, `path.rs:93-97`).
#[test]
fn the_start_may_be_a_blocked_tile() {
    let walk = walk(10, 10, &[(2, 5)]);
    let route = find_path(&walk, (2, 5), (8, 5)).expect("a worker can walk off its own tree");
    assert!(
        !route.is_empty(),
        "a route of no steps does not leave the tile"
    );
    assert_eq!(route.last(), Some(&(8, 5)));
}

/// RULE: the map's edge is a wall, and a goal off the map is refused rather than clamped.
///
/// Source: `zone_spatial_pyramid_check_tile.cpp` bounds every neighbour test by `max_x`/`max_y`; our side refuses a tile
/// it does not contain in `Walk::blocked` and in the search's bounds test (`path.rs:204-206`).
#[test]
fn the_map_edge_is_a_wall() {
    let walk = walk(6, 6, &[]);
    assert!(
        find_path(&walk, (0, 0), (6, 0)).is_none(),
        "x = width is off the map"
    );
    assert!(
        find_path(&walk, (0, 0), (0, 6)).is_none(),
        "y = height is off the map"
    );
    assert!(
        find_path(&walk, (0, 0), (5, 5)).is_some(),
        "the corner itself is on the map"
    );
}

/// RULE: a route is a chain of single king moves, never repeating a tile, ending on the goal.
///
/// Source: `zone_spatial_pyramid_eval_cost.cpp` (`func_001D35C0`) only ever inserts the four mask directions from a node, so
/// an emitted path is a walk of adjacent cells; our search's eight-neighbour loop has the same property (`path.rs:255-298`).
#[test]
fn a_route_is_a_chain_of_single_steps_without_repeats() {
    let walk = walk(
        24,
        24,
        &[(10, 5), (10, 6), (10, 7), (10, 8), (11, 8), (12, 8)],
    );
    let route = find_path(&walk, (3, 3), (20, 20)).expect("open corners");
    let mut here = (3u16, 3u16);
    let mut seen = std::collections::BTreeSet::new();
    for step in &route {
        let dx = i64::from(step.0) - i64::from(here.0);
        let dy = i64::from(step.1) - i64::from(here.1);
        assert!(
            dx.abs() <= 1 && dy.abs() <= 1 && (dx != 0 || dy != 0),
            "the route jumps from {here:?} to {step:?}"
        );
        assert!(seen.insert(*step), "the route visits {step:?} twice");
        here = *step;
    }
    assert_eq!(here, (20, 20), "the route does not end on the goal");
}

/// RULE: ground is counted, not flagged - two things may share a tile and the first to leave does not free it.
///
/// Source: not a pathing function but the layer it stands on. `zone_spatial_grid_scan_cross_occupants.cpp` counts occupants
/// per cell rather than testing one flag; our `Walk` counts for the same measured reason (`path.rs:28-31`).
#[test]
fn a_tile_held_by_two_things_stays_held_until_both_leave() {
    let mut walk = walk(4, 4, &[]);
    walk.block(1, 1);
    walk.block(1, 1);
    walk.unblock(1, 1);
    assert!(
        walk.blocked(1, 1),
        "one of two occupants leaving freed the tile"
    );
    walk.unblock(1, 1);
    assert!(
        !walk.blocked(1, 1),
        "the tile stayed blocked after both left"
    );
    // An unbalanced release cannot wrap the count into "blocked by 65,535 things".
    walk.unblock(1, 1);
    assert!(!walk.blocked(1, 1));
}

/// RULE: an unobstructed route costs exactly the octile distance, so the terrain layer invents no cost.
///
/// Source: `zone_spatial_pyramid_eval_cost.cpp` seeds `g = 0` at the start and adds a per-step cost with no terrain term
/// (terrain is either passable or not); the same claim on our side, checked rather than assumed.
#[test]
fn open_ground_costs_the_octile_distance_and_no_more() {
    let walk = walk(30, 30, &[]);
    for (from, to) in [
        ((2u16, 3u16), (25u16, 18u16)),
        ((0, 0), (29, 29)),
        ((5, 20), (5, 1)),
    ] {
        let route = find_path(&walk, from, to).expect("open ground");
        assert_eq!(
            route_cost(from, &route),
            octile(from, to),
            "the open-ground route from {from:?} to {to:?} is not the octile cost: {route:?}"
        );
        assert_eq!(route.last(), Some(&to));
    }
    let _ = at(0, 0);
}
