//! **Rule set 3: obstacle avoidance - what the search knows is in the way, and what it does when everything is.**
//!
//! Read from `zone_spatial_pyramid_find_path.cpp` (PS2) and `FUN_007f9d80` (HD 5.7, `docs/movement-in-the-binary.md`): the
//! search's obstacle set is **per search, rebuilt each time, never written into the shared grid** - an object is inserted
//! only if it is a *stander* (velocity zero) and not on the route's ignore list. Our `find_path_avoiding` carries the same
//! idea as a `BTreeSet` parameter (`path.rs:191-203`), so a body is the caller's fact and the map is never poisoned by it.

use std::collections::BTreeSet;

use ruage_conformance::{octile, route_cost, walk};
use ruage_sim::{find_path, find_path_avoiding, find_path_strict, line_clear_avoiding};

/// RULE: a tile the caller names is an obstacle for that search only, and the search routes round it.
///
/// Source: `FUN_007f9d80` inserts standing objects into a **per-search** rectangle list; `zone_spatial_pyramid_find_path`
/// clears its scratch grid before filling it. Ours: `find_path_avoiding`, and the grid it reads is untouched.
#[test]
fn an_avoided_tile_is_an_obstacle_the_grid_does_not_know() {
    let walk = walk(12, 12, &[]);
    let mut avoid = BTreeSet::new();
    avoid.insert((6u16, 6u16));
    let direct = find_path(&walk, (3, 6), (9, 6)).expect("open ground");
    let round =
        find_path_avoiding(&walk, (3, 6), (9, 6), &avoid).expect("one tile is walkable round");
    assert_eq!(direct.len(), 6, "the direct route is six steps: {direct:?}");
    assert!(
        !round.contains(&(6, 6)),
        "the route went through the avoided tile: {round:?}"
    );
    assert!(
        route_cost((3, 6), &round) > octile((3, 6), (9, 6)),
        "avoiding a tile must cost something: {round:?}"
    );
    // The map itself is unchanged: a body is not terrain.
    assert!(
        !walk.blocked(6, 6),
        "the avoid set wrote into the walk layer"
    );
}

/// RULE: the search never writes its obstacle set into the shared grid - two searches with different sets see different
/// worlds, in either order.
///
/// Source: the original's per-search rectangle list, rebuilt for each `FUN_008007f0` call; the same claim on our side is a
/// property of taking `avoid` by reference.
#[test]
fn the_avoid_set_is_per_search() {
    let walk = walk(12, 12, &[]);
    let mut avoid = BTreeSet::new();
    avoid.insert((6u16, 6u16));
    let with = find_path_avoiding(&walk, (3, 6), (9, 6), &avoid).expect("round");
    let without = find_path(&walk, (3, 6), (9, 6)).expect("straight");
    assert!(
        without.contains(&(6, 6)),
        "the plain search detoured: {without:?}"
    );
    assert!(
        !with.contains(&(6, 6)),
        "the avoided search did not detour: {with:?}"
    );
    // And asking again after the plain search still detours.
    let again = find_path_avoiding(&walk, (3, 6), (9, 6), &avoid).expect("round");
    assert_eq!(with, again);
}

/// RULE: an avoided tile is not cut diagonally either - a body's corner is as solid as a wall's.
///
/// Source: `FUN_007f9d80`'s entries are rectangles grown by the mover's radius, so a diagonal past one clips it; our search
/// applies the same both-corners test to `avoid` as to blocked terrain (`path.rs:273-280`).
#[test]
fn an_avoided_tile_cannot_be_cut_diagonally() {
    let walk = walk(8, 8, &[]);
    let mut avoid = BTreeSet::new();
    avoid.insert((2u16, 1u16));
    avoid.insert((1u16, 2u16));
    assert!(
        !line_clear_avoiding(&walk, (1, 1), (2, 2), &avoid),
        "a line was called clear through two avoided corners"
    );
    let route = find_path_avoiding(&walk, (1, 1), (2, 2), &avoid).expect("the long way exists");
    assert!(
        route_cost((1, 1), &route) > 14,
        "the search crossed avoided corners: {route:?}"
    );
}

/// RULE: the goal is allowed to be in the avoid set - a body you were ordered onto is a target, not an obstacle.
///
/// Source: the original's search entry builds its ignore list from **self and the order's target** before any object test
/// (`FUN_008007f0`); ours exempts the goal explicitly (`path.rs:281`).
#[test]
fn the_goal_may_be_in_the_avoid_set() {
    let walk = walk(10, 10, &[]);
    let mut avoid = BTreeSet::new();
    avoid.insert((8u16, 5u16));
    let route = find_path_avoiding(&walk, (2, 5), (8, 5), &avoid).expect("the target is reachable");
    assert_eq!(
        route.last(),
        Some(&(8, 5)),
        "the route stopped short: {route:?}"
    );
}

/// RULE: when the goal is walled in, the search answers **the closest point it can reach** - not a route through the wall,
/// and not a refusal (the original's rule; D249).
///
/// Source: the Klingers deck, p. 96 - *"expand the search area once, otherwise give the closest point path"* - and the PS2
/// low-level search, whose "no low-level path" is reserved for a *start* boxed in on every side.
#[test]
fn a_walled_in_goal_is_answered_with_the_closest_point() {
    let walk = walk(12, 12, &[]);
    let mut blocked = Vec::new();
    for dx in -1i32..=1 {
        for dy in -1i32..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            blocked.push(((6 + dx) as u16, (6 + dy) as u16));
        }
    }
    let walk = {
        let mut walk = walk;
        for (x, y) in blocked {
            walk.block(x, y);
        }
        walk
    };
    let route = find_path(&walk, (1, 1), (6, 6)).expect("the closest point is an answer");
    let tile = *route.last().expect("a route has an end");
    assert_ne!(
        tile,
        (6, 6),
        "the search walked into a sealed tile: {route:?}"
    );
    // The ring round the seal is at Chebyshev distance 2, and the closest point is on it: the answer is the closest the
    // ground allows, not one step from the start.
    assert_eq!(
        tile.0.abs_diff(6).max(tile.1.abs_diff(6)),
        2,
        "the answer {tile:?} is not on the ring at 2 round the seal"
    );
}

/// RULE: a caller that must insist on the exact tile asks for it, and gets `None` - the strict answer is the closest
/// point's, filtered.
///
/// Source: our own shape, not the game's: thing orders refuse a tree behind a wall (D241 ruling), and the strict search is
/// how they keep that while the planner still answers the closest point. It is a filter over the same search, so the two
/// can never disagree about the map.
#[test]
fn the_strict_search_refuses_a_walled_in_goal() {
    let mut walk = walk(12, 12, &[]);
    for dx in -1i32..=1 {
        for dy in -1i32..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            walk.block((6 + dx) as u16, (6 + dy) as u16);
        }
    }
    assert!(
        find_path_strict(&walk, (1, 1), (6, 6)).is_none(),
        "the strict search answered a route to a sealed tile"
    );
    assert!(
        find_path_strict(&walk, (1, 1), (4, 4)).is_some(),
        "the strict search refused an open tile"
    );
}

/// RULE: the closest point the search settles on is a tile a body may **stand** on, and the same one every run.
///
/// Source: D134 (the owner: *"every click is accepted"*), now answered by the search itself (D249). A blocked tile as the
/// answer would be an order nobody can complete; a choice that varies between runs would break the replay.
#[test]
fn the_closest_point_is_a_standing_tile_and_is_the_same_every_run() {
    let mut walk = walk(12, 12, &[]);
    for dx in -1i32..=1 {
        for dy in -1i32..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            walk.block((6 + dx) as u16, (6 + dy) as u16);
        }
    }
    walk.block(6, 6);
    let route = find_path(&walk, (1, 1), (6, 6)).expect("there is free ground");
    let tile = *route.last().expect("a route has an end");
    assert!(
        !walk.blocked(tile.0, tile.1),
        "the answer is a blocked tile"
    );
    let again = find_path(&walk, (1, 1), (6, 6)).expect("the same ground");
    assert_eq!(route, again, "the closest point changed between two runs");
}

/// RULE: the answer is a tile with a **route** to it - the route's last point is the answer, and every step is adjacent.
///
/// Source: D134's "already planned" (`lib.rs:2396-2424`): `walk_goal` uses the answer as an order's destination and plans to
/// it. A tile without a route would be an order the mover can never complete.
#[test]
fn the_nearest_standing_answer_comes_with_a_route_to_it() {
    let mut blocked = Vec::new();
    for x in 4..12u16 {
        for y in 4..12u16 {
            blocked.push((x, y));
        }
    }
    let walk = walk(20, 20, &blocked);
    let route = find_path(&walk, (1, 1), (8, 8)).expect("the woods have an edge");
    let tile = *route.last().expect("a route has an end");
    assert!(!walk.blocked(tile.0, tile.1));
    let mut here = (1u16, 1u16);
    for step in &route {
        let dx = i64::from(step.0) - i64::from(here.0);
        let dy = i64::from(step.1) - i64::from(here.1);
        assert!(
            dx.abs() <= 1 && dy.abs() <= 1,
            "the route jumps {here:?} -> {step:?}"
        );
        here = *step;
    }
}
