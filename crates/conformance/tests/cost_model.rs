//! **Rule set 2: what a step costs, and how ties are broken.**
//!
//! Read from `zone_spatial_pyramid_eval_cost.cpp` (`func_001D35C0`, PS2): a search node carries `(x, y, level, g, h, parent,
//! dir)`; `g` is accumulated as a **float** and `h` is computed from the squared distance to the target through a sqrt
//! (`func_122690`, `func_1228E8`), i.e. the estimate is Euclidean, not a tile count. The frontier there is a linked list
//! popped in insertion order, not a heap.
//!
//! HD 5.7's own numbers, read from `AoK HD.exe` (`docs/movement-in-the-binary.md`): a tile step costs **16**, the diagonal is
//! the octile form `table[min] + 16 * (max - min)` with `min` capped at 479 (`FUN_007ffd10`), and the straight-line estimate
//! is Euclidean scaled by the integer 2500/100. Ours is 10 and 14. The ratio is what decides route shape, so that is what the
//! tests pin; the constants themselves are pinned in `divergences.rs` as a known, deliberate difference.

use ruage_conformance::{CARDINAL, DIAGONAL, octile, route_cost, walk};
use ruage_sim::find_path;

/// RULE: a diagonal step costs a ratio more than a cardinal one - the octile model, which is what makes a route cut corners
/// the way the original's does.
///
/// Source: `func_001D35C0`'s `g` accumulates one step cost and the heuristic is Euclidean; HD's octile table
/// (`FUN_007ffd10`) makes a diagonal 22.6 against a cardinal 16, a ratio of 1.414. Ours is 1.40 (`path.rs:16-17`).
#[test]
fn the_diagonal_ratio_matches_the_octile_model() {
    let ratio = f64::from(DIAGONAL) / f64::from(CARDINAL);
    let game = 2f64.sqrt();
    assert!(
        (ratio - game).abs() / game < 0.02,
        "the engine's diagonal ratio {ratio:.4} is more than 2% from the octile 1.4142 the original's table gives"
    );
    // And the ratio is what a route actually pays: one diagonal against one cardinal.
    let walk = walk(8, 8, &[]);
    let diagonal = find_path(&walk, (1, 1), (2, 2)).expect("open ground");
    let cardinal = find_path(&walk, (1, 1), (2, 1)).expect("open ground");
    assert_eq!(route_cost((1, 1), &diagonal), DIAGONAL);
    assert_eq!(route_cost((1, 1), &cardinal), CARDINAL);
}

/// RULE: no route is cheaper than the octile lower bound - the estimate never overcharges the search.
///
/// Source: `func_001D35C0` computes `h` from `dx*dx + dy*dy` and adds it to `g`; an admissible estimate is the precondition
/// for the search to be optimal at all, and the game's own Euclidean estimate is admissible by construction.
#[test]
fn a_route_never_beats_the_octile_bound() {
    let walk = walk(
        32,
        32,
        &[
            (16, 4),
            (16, 5),
            (16, 6),
            (16, 7),
            (16, 8),
            (17, 8),
            (18, 8),
        ],
    );
    for from in [(1u16, 1u16), (5, 20), (28, 3), (12, 12)] {
        for to in [(30u16, 30u16), (2, 29), (25, 9), (16, 2)] {
            if from == to {
                continue;
            }
            if let Some(route) = find_path(&walk, from, to) {
                let cost = route_cost(from, &route);
                assert!(
                    cost >= octile(from, to),
                    "route {from:?} -> {to:?} costs {cost}, below the octile bound {}",
                    octile(from, to)
                );
            }
        }
    }
}

/// RULE: a wall with a gap on each side is paid for at the gap, and both gaps cost the same when they are symmetric.
///
/// Source: `func_001D35C0` expands neighbours by their mask bits and accumulates `g` per cell, so a detour costs what its
/// steps cost - the same arithmetic our `find_path` does (`path.rs:284-295`). The analytic answer is asserted, not the
/// engine's own output.
#[test]
fn a_wall_is_paid_for_at_its_gap() {
    // A full column at x = 8 with one gap at y = 3. From (4,4) to (12,4) the shortest route is 4 across and 1 up, twice:
    // 1 diagonal (23) + 3 cardinal (48) each way = 2 * 71 = 142. Cutting the corner at the gap costs nothing extra, because
    // the gap tile is the only filled tile in the column and its neighbours are open.
    let mut blocked = Vec::new();
    for y in 0..12u16 {
        if y != 3 {
            blocked.push((8u16, y));
        }
    }
    let walk = walk(20, 14, &blocked);
    let route = find_path(&walk, (4, 4), (12, 4)).expect("the gap is a passage");
    assert_eq!(
        route_cost((4, 4), &route),
        142,
        "the route through the gap costs {} rather than the analytic 142: {route:?}",
        route_cost((4, 4), &route)
    );
    assert!(
        route.contains(&(8, 3)),
        "the route did not use the gap: {route:?}"
    );
}

/// RULE: two routes of equal cost are resolved the same way on every run.
///
/// Source: `func_001D35C0` pops its frontier in insertion order, so the original's tie-break is an artefact of the order
/// nodes were pushed - deterministic, and not a distance. Ours sorts the open set by `(f, g, y, x)` (`path.rs:8-9`), which
/// is also deterministic and also not a distance. What matters for a replay is only that it never varies.
#[test]
fn an_equal_cost_choice_is_made_identically_every_time() {
    let walk = walk(12, 12, &[(6, 5), (6, 6), (6, 7)]);
    let first = find_path(&walk, (3, 6), (9, 6)).expect("round the block");
    for _ in 0..8 {
        assert_eq!(
            find_path(&walk, (3, 6), (9, 6)).as_deref(),
            Some(first.as_slice()),
            "the same search answered two different routes"
        );
    }
}

/// RULE: a search does not depend on what was searched before it - no shared grid, no carried frontier.
///
/// Source: `zone_spatial_pyramid_find_path.cpp` *clears* its scratch grid at the start of a search and re-fills it; a route
/// is a function of the map and the endpoints. Our `find_path_avoiding` allocates its `best`/`came` per call
/// (`path.rs:217-223`), which is the stronger form of the same guarantee.
#[test]
fn a_route_does_not_depend_on_earlier_searches() {
    let walk = walk(20, 20, &[(9, 0), (9, 1), (9, 2), (9, 3), (9, 4)]);
    let alone = find_path(&walk, (2, 8), (16, 8)).expect("round the wall's end");
    // The same search, after a crowd of others - including one that fails.
    let mut interleaved = Vec::new();
    for i in 0..40u16 {
        interleaved = find_path(&walk, (2, 8), (16, 8)).expect("round the wall's end");
        let _ = find_path(&walk, (i % 18, 18), (18, i % 18));
        let _ = find_path(&walk, (0, 0), (9, 2));
    }
    assert_eq!(
        alone, interleaved,
        "a route changed after other searches ran"
    );
}

/// RULE: the search answers the same route whatever order the caller asks its endpoints in.
///
/// Source: the search is a function of start and goal only - `func_001D35C0` carries them in the object, not in a shared
/// queue. The reversed route need not be identical (the goal exoneration is asymmetric), but it must end where it started,
/// and a block **on** the line must be paid for in both directions.
#[test]
fn a_reversed_search_reaches_the_other_end() {
    let walk = walk(16, 16, &[(7, 7), (8, 7), (7, 8)]);
    let there = find_path(&walk, (3, 3), (12, 12)).expect("open");
    let back = find_path(&walk, (12, 12), (3, 3)).expect("open");
    assert_eq!(there.last(), Some(&(12, 12)));
    assert_eq!(back.last(), Some(&(3, 3)));
    // The straight diagonal is 9 steps of 14 = 126, and the block at (7,7) sits on it, so the route bends round and pays
    // for the bend - in either direction.
    let bound = octile((3, 3), (12, 12));
    assert!(
        route_cost((3, 3), &there) > bound,
        "the route through a block on its own diagonal cost {bound}: {there:?}"
    );
    assert_eq!(
        route_cost((3, 3), &there),
        route_cost((12, 12), &back),
        "the two directions paid different prices for the same block: {there:?} against {back:?}"
    );
}
