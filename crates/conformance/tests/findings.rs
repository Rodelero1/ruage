//! **The defect this set found, and the rule that replaced it. No test here is `#[ignore]`d.**
//!
//! The two tests below were written against `nearest_reachable`, a helper beside the search that answered **one step from
//! the start** instead of the tile nearest the click. The owner's ruling was to follow the game (D249), and the game's
//! planner answers the closest reachable point itself - so the helper is deleted, the rule lives in `find_path`, and these
//! tests now check the search.
//!
//! ```sh
//! cargo test --manifest-path crates/conformance/Cargo.toml --test findings
//! ```

use ruage_conformance::walk;
use ruage_sim::find_path;

/// RULE: an unreachable click is answered by **the reachable tile nearest the click**, not by the first free tile the
/// search happens to reach.
///
/// Source: the Klingers deck, p. 96 (*"otherwise give the closest point path"*); D134's *"nearest legal standing point"*.
/// The defect it replaces: a helper that returned one step from the start, so from (1,1) toward (10,10) it answered (2,2).
#[test]
fn the_closest_point_is_the_tile_nearest_the_click() {
    let walk = walk(20, 20, &[]);
    // Open ground and a free click: the nearest reachable tile to the click is the click.
    let route = find_path(&walk, (1, 1), (10, 10)).expect("open ground is always a path");
    assert_eq!(
        route.last(),
        Some(&(10, 10)),
        "the route stopped short: {route:?}"
    );
}

/// RULE: the same rule through the case a player causes - a click into a wood, from a distance.
///
/// A 3x3 wood at x, y in 6..9 on a 16x16 map, a walker at (1,1): the nearest reachable tile to the middle (7,7) is a tile of
/// the wood's ring at 5..=9. The helper this replaces answered **(2,2)** - five tiles short, beside the walker's feet.
#[test]
fn a_distant_click_into_a_wood_lands_on_the_woods_edge() {
    let mut blocked = Vec::new();
    for x in 6..9u16 {
        for y in 6..9u16 {
            blocked.push((x, y));
        }
    }
    let walk = walk(16, 16, &blocked);
    let route = find_path(&walk, (1, 1), (7, 7)).expect("the wood has an edge");
    let tile = *route.last().expect("a route has an end");
    let on_the_edge = (5..=9).contains(&tile.0) && (5..=9).contains(&tile.1);
    assert!(
        on_the_edge,
        "the answer {tile:?} is not on the wood's edge: the walker was ordered to a tile {} from it",
        tile.0.abs_diff(7).max(tile.1.abs_diff(7))
    );
    assert!(
        route.len() > 1,
        "a distant click into a wood must walk somewhere: {route:?}"
    );
}
