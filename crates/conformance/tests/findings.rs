//! **Defects this set found. Each one fails, and each one stays red on purpose.**
//!
//! These are not differences from the original - `divergences.rs` holds those. These are places where **our own** stated rule
//! and our code disagree, found by writing the rule down as a test. `#[ignore]`d so a run of the set is green:
//!
//! ```sh
//! cargo test -p ruage-conformance --test findings -- --ignored
//! ```
//!
//! A finding is not fixed here: a behaviour change owes its own slice, its own failing test and its own measurement
//! (`AGENTS.md` step 2 and 3). It is filed, with the number that proves it.

use ruage_conformance::walk;
use ruage_sim::nearest_reachable;

/// FINDING 1: `nearest_reachable` answers **one step from the start**, not the tile nearest the click.
///
/// The rule it is meant to keep is its own doc comment (`path.rs:303-312`): *"the first tile taken off the queue is the
/// reachable tile closest to the click"*, and D134's *"nearest legal standing point"*. Measured 2026-11-16: with no obstacles
/// at all, from (1,1) toward (10,10), the answer is **(2,2)** - the queue is popped best-first by distance to the click, but
/// every tile pushed is already free, so the **first** pop is a neighbour of the start and the search returns before the
/// wavefront has moved a single tile nearer the goal.
///
///    open ground, from (1,1) to (10,10)  -> (2,2), one step
///    3x3 wood, from (1,1) to its middle -> (2,2), one step
///    3x3 wood, from (1,7) to its middle -> (2,6), one step
///
/// Why the world suite does not see it: `click_into_the_woods.rs` spawns the villager **one tile** from the wood (1,4 against
/// a wood at x = 3), so "a legal tile within one tile of the block" is satisfied by the bug's own answer. The contract holds
/// for the case the fixture happens to build; it does not hold in general.
///
/// What a fix owes: the closest *reachable* tile, which means exhausting the search (or stopping when the frontier's distance
/// exceeds the best free tile found), and a tie-break that is about the click rather than `(y, x)` - the geometry the game
/// answers is "the closest point", not "the smallest coordinates". Re-measure the acceptance match afterwards: the fallback
/// only fires on an unreachable click, so the expected movement is small but not nil.
#[test]
#[ignore = "FINDING: answers a neighbour of the start, not the nearest reachable tile (path.rs:314-365); fix owes its own slice"]
fn the_nearest_standing_tile_is_the_one_nearest_the_click() {
    let walk = walk(20, 20, &[]);
    // Open ground and a free far click: the nearest tile to the click is the click itself.
    let (tile, route) = nearest_reachable(&walk, (1, 1), (10, 10)).expect("open ground");
    assert_eq!(
        tile,
        (10, 10),
        "the answer was {tile:?} (route {} steps) instead of the click: the search stops at the start's best neighbour",
        route.len()
    );
}

/// FINDING 1b: the same defect through the case the game actually has to answer - a click into a wood, from a distance.
///
/// A 3x3 wood at x, y in 6..9 on a 16x16 map, a walker at (1,1): the nearest free tile to the middle (7,7) is a tile of the
/// wood's ring at 5..=9. Measured answer: **(2,2)** - five tiles short, on the wrong side of the map.
#[test]
#[ignore = "FINDING: same defect; a distant click into a wood is answered five tiles short of the wood"]
fn a_distant_click_into_a_wood_lands_on_the_woods_edge() {
    let mut blocked = Vec::new();
    for x in 6..9u16 {
        for y in 6..9u16 {
            blocked.push((x, y));
        }
    }
    let walk = walk(16, 16, &blocked);
    let (tile, _) = nearest_reachable(&walk, (1, 1), (7, 7)).expect("the wood has an edge");
    // The ring round the wood: x or y within (5..=9), and not the wood itself.
    let on_the_edge = (5..=9).contains(&tile.0) && (5..=9).contains(&tile.1);
    assert!(
        on_the_edge,
        "the answer {tile:?} is not on the wood's edge: the walker was ordered to a tile {} from it",
        tile.0.abs_diff(7).max(tile.1.abs_diff(7))
    );
}
