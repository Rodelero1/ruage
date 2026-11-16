//! **The places we stand apart, as tests that must fail.**
//!
//! Each test here asserts the **original's** rule, so it fails on this engine by construction. They are `#[ignore]`d so the
//! suite is green, and running it with `-- --ignored` prints exactly this list:
//!
//! ```sh
//! cargo test -p ruage-conformance -- --ignored
//! ```
//!
//! A test here is a *finding*, not a defect report: some of the differences are decisions (a bounded window, integer costs),
//! others are open slices in `docs/behaviour-diff.md`. Each says which, so the list stays readable as a register rather than
//! a to-do.

use ruage_conformance::{SCALE, at, route_cost, walk};
use ruage_sim::{find_path, short_range};

/// RULE (original): the search answers the **closest point it can reach**, and "no path" is not an answer a mover can use.
///
/// Source: the Klingers deck, slide 96 - *"What if our convex hulls escape our search area? Expand search area once.
/// Otherwise give the closest point path"* - and the PS2 low-level search's own "no low-level path" is reserved for the start
/// being trapped, not for an unreachable goal.
///
/// Ours answers `None`, and the caller compensates with `nearest_reachable` (D134). The difference is real at this layer: a
/// planner that answers nothing hands the mover nothing, and the mover standing still with a live order is the classic
/// stuck unit.
#[test]
#[ignore = "diverges: our search answers None; closest-point is the caller's layer (D134). docs/behaviour-diff.md row 32"]
fn the_original_answers_the_closest_point_not_no_path() {
    let mut walk = walk(12, 12, &[]);
    for dx in -1i32..=1 {
        for dy in -1i32..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            walk.block((6 + dx) as u16, (6 + dy) as u16);
        }
    }
    let answer = find_path(&walk, (1, 1), (6, 6));
    assert!(
        answer.is_some(),
        "the search answered None for a walled-in goal; the original answers the closest reachable point"
    );
}

/// RULE (original): a tile step costs **16**, so ten straight tiles cost 160.
///
/// Source: `FUN_007ffd10` (HD 5.7; `docs/movement-in-the-binary.md`), whose octile form is `table[min] + 16 * (max - min)`.
/// Ours is 10 a cardinal step and 14 a diagonal (`path.rs:16-17`), chosen as algorithm constants while the movement weights
/// are still `-1.0` sentinels (U5/U7, `path.rs:3-6`).
///
/// The **ratio** is within 2% (pinned in `cost_model.rs`), so route shape agrees; the absolute numbers do not, and an
/// acceptance hash that depends on a cost value would disagree with the original's. That is a known, deliberate difference,
/// not a bug - fixed when the weights are measured.
#[test]
#[ignore = "diverges: 10/14 instead of 16-with-octile-table; route shape agrees, absolute costs do not (path.rs:3-6)"]
fn the_originals_step_cost_is_sixteen() {
    let walk = walk(20, 20, &[]);
    let route = find_path(&walk, (2, 10), (12, 10)).expect("open ground");
    assert_eq!(
        route_cost((2, 10), &route),
        160,
        "ten straight tiles cost {} rather than the original's 16 x 10",
        route_cost((2, 10), &route)
    );
}

/// RULE (original): the long route is planned through a **hierarchy**, so a forty-tile walk is ordinary work.
///
/// Source: `zone_spatial_pyramid_search_hierarchical.cpp` / `zone_spatial_pyramid_check_edge.cpp` - a node carries a `level`,
/// coarse nodes span `<< level` tiles, and an edge is refined to a tile on the coarse node's boundary. The PS2 route is a
/// pyramid search; HD's mip-mapped route is the same family (`docs/sources.md`, the 1999 postmortem's three pathfinders).
///
/// Ours declines a short-range window beyond 40 tiles (`short.rs:29-36`) and falls back to the tile chain, which is the
/// bounded choice the owner priced (D211) - the long-route hierarchy is not built. Recorded as a slice, not a defect.
#[test]
#[ignore = "diverges: no hierarchical long route; short_range declines beyond 40 tiles (short.rs:29-36)"]
fn the_original_solves_a_long_walk_in_its_hierarchy() {
    let walk = walk(64, 64, &[]);
    let route = short_range(
        &walk,
        SCALE,
        at(2, 2),
        at(60, 60),
        4_000,
        &Default::default(),
    );
    assert!(
        route.is_some(),
        "the short-range solver declined a 58-tile walk; the original has a hierarchy for exactly this"
    );
}
