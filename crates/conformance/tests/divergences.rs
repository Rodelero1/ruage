//! **Rules of the original that this engine does not keep - asserted, never `#[ignore]`d.**
//!
//! A red test here is not a broken build: it is a rule the game keeps and we do not, written down so the difference has a
//! number and a source instead of a memory. A green run of this file is the claim *we keep all of them*.
//!
//! ```sh
//! cargo test --manifest-path crates/conformance/Cargo.toml --test divergences
//! ```
//!
//! Each test names the decision that put us here, and the ruling that would close it. Nothing in this file is fixed by
//! weakening the assertion: it goes green when the engine changes, or when the owner rules that our answer is the one to
//! keep - and then the test asserts the ruled answer and cites the game's as the rejected alternative.

use ruage_conformance::{SCALE, at, route_cost, walk};
use ruage_sim::{find_path, short_range};

/// RULE (original): a tile step costs **16**, so ten straight tiles cost 160.
///
/// Source: `FUN_007ffd10` (HD 5.7; `docs/movement-in-the-binary.md`), whose octile form is `table[min] + 16 * (max - min)`.
/// Ours is 10 a cardinal step and 14 a diagonal (`path.rs:16-17`), chosen as algorithm constants while the movement weights
/// are still `-1.0` sentinels (U5/U7, `path.rs:3-6`).
///
/// The **ratio** is within 2% (pinned in `cost_model.rs`), so route shape agrees; the absolute numbers do not, and an
/// acceptance hash that depends on a cost value would disagree with the original's. **Decision owed (D249, U25):** adopt the
/// game's 16 and octile table now (it changes tie-breaks and every hash) or wait until the movement weights are measured
/// (U5/U7) and take all of them at once.
#[test]
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
/// bounded choice the owner priced (D211) - the long-route hierarchy is not built. **Decision owed (D249, U25):** build the
/// levelled long route (a slice of its own) or keep the bound and the tile-corner fallback.
#[test]
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
