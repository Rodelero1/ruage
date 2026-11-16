//! **Defects this set found. No test here is `#[ignore]`d.**
//!
//! These are not differences from the original - `divergences.rs` holds those. These are places where **our own** stated rule
//! and our code disagreed, found by writing the rule down as a test. A red test here is a defect; it stays red until the
//! defect is fixed, because an ignored test is a decision nobody re-reads.
//!
//! ```sh
//! cargo test --manifest-path crates/conformance/Cargo.toml --test findings
//! ```

use ruage_conformance::walk;
use ruage_sim::nearest_reachable;

/// RULE: the tile answered for an unreachable click is the one **nearest the click**, not the first free tile the search
/// happens to reach.
///
/// Its own doc comment (`path.rs:303-312`) and D134's *"nearest legal standing point"* both say so. Measured 2026-11-16: on
/// empty ground, from (1,1) toward (10,10), the answer is **(2,2)** - the queue is popped best-first by distance to the
/// click, but every tile it pushes is already free, so the **first** pop is a neighbour of the start and it returns before
/// the wavefront has moved.
///
///    open ground, from (1,1) to (10,10)  -> (2,2), one step
///    3x3 wood, from (1,1) to its middle -> (2,2), one step
///    3x3 wood, from (1,7) to its middle -> (2,6), one step
///
/// **A fix exists and is not landed** (D249): sweeping the reachable region answers the right tile, and costs the acceptance
/// match - **155 entities and a win at `45b5bd2` against 125 and no win**, because the script's two unreachable clicks send a
/// villager six tiles instead of one. The fix is a fork for the owner, not a defect to land quietly.
///
/// Why the world suite did not see it: `click_into_the_woods.rs` spawns the villager **one tile** from the wood (1,4 against a
/// wood at x = 3), so "a legal tile within one tile of the block" was satisfied by the bug's own answer.
#[test]
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

/// RULE: the same rule through the case a player actually causes - a click into a wood, from a distance.
///
/// A 3x3 wood at x, y in 6..9 on a 16x16 map, a walker at (1,1): the nearest free tile to the middle (7,7) is a tile of the
/// wood's ring at 5..=9. Measured answer: **(2,2)** - five tiles short, on the wrong side of the map.
#[test]
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
