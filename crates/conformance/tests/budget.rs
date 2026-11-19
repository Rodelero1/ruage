//! **RULE: the search spends at most its own estimate, and never more than 20,000 expansions.**
//!
//! Source: `FUN_008007f0_008007f0.c` (the REA corpus, HD 5.7), read 2026-11-19:
//!
//! ```c
//! local_468 = FUN_007ffd10(param_1,param_2,param_3,param_4);   // the start's cost
//! fVar24    = (float10)FUN_007ffcd0(param_1,param_2,param_3,param_4);  // Euclidean tiles
//! iVar20    = ((int)fVar24 * DAT_00aad420) / 100;             // 2500/100
//! if (FUN_0074e020() == 1) iVar20 = (int)fVar24 * 0x19;       // ... which is 25
//! while (0 < iVar21) {                                        // frontier not empty
//!   if (((DAT_00aad420 != -1) && (iVar20 < local_46c)) || (20000 < local_46c)) break;
//!   ...expand one node, eight neighbours, then...
//!   local_46c = local_46c + 1;                                // one per expansion
//! }
//! // the tail pushes the best node it settled (cost 0) and the caller walks the parents from it
//! ```
//!
//! So the **estimate is the budget**: a farther goal is allowed more expansions, which is why the same number reads as a
//! distance in one place and a count in the other. When the budget runs out the answer is the **closest node it settled** -
//! the same shape as a goal that cannot be reached - which is what makes a hopeless search cost a bounded number of cells
//! instead of the whole reachable map.

use ruage_conformance::{SCALE, centre, ground};

/// RULE: a hopeless goal costs at most `h` expansions, where `h` is the search's own initial estimate.
///
/// A room sealed inside a 64x64 field: the goal is inside it, so nothing near the room is reachable and an unbudgeted search
/// settles every one of the field's 4,096 tiles. The estimate from (2,2) to the room's inside is a small number of tile
/// steps, so the budget is a small number of nodes - and the answer must still be the closest cell, not the goal.
#[test]
fn a_hopeless_search_is_bounded_by_its_own_estimate() {
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
    let (from, to) = ((2u16, 2u16), (10u16, 10u16));
    let (route, expansions) = field.search_reporting(from, to, SCALE, &[]);
    let route = route.expect("the search settles outside the room");

    // `Ground::estimate` is the same number the binary computes at `0x800d19`: Euclidean tiles x 25.
    let budget = ruage_sim::Ground::estimate(from, to);
    assert!(
        expansions <= budget,
        "the search spent {expansions} expansions against a budget of {budget} (h = 25 x the Euclidean distance)"
    );
    assert!(
        expansions < 4_096,
        "the search settled the whole field ({expansions} expansions): the budget is not binding"
    );
    // The answer is still "as close as you can get": the chain ends outside the sealed room.
    let tile = *route.last().expect("a route has an end");
    let inside = (9..=11).contains(&tile.0) && (9..=11).contains(&tile.1);
    assert!(
        !sealed.contains(&tile) && !inside,
        "the route ends at {tile:?}, which is the room"
    );
}

/// RULE: the cap is 20,000 expansions, whatever the estimate says.
///
/// The estimate scales with the distance, so a corner-to-corner search on a 480x480 map would ask for more than the cap; the
/// binary's second clause (`20000 < local_46c`) is what bounds it. The field here is smaller than the cap allows, so the
/// assertion is the shape of the rule rather than a reachable number: an open walk must not spend more than the cap, and it
/// must not spend more than `h`.
#[test]
fn the_budget_never_exceeds_twenty_thousand() {
    let field = ground(480, 480, &[]);
    let (from, to) = ((1u16, 1u16), (478u16, 478u16));
    let (route, expansions) = field.search_reporting(from, to, SCALE, &[]);
    assert!(route.is_some(), "an open field has a route");
    let budget = ruage_sim::Ground::estimate(from, to).min(20_000);
    assert!(
        expansions <= budget,
        "the search spent {expansions} expansions against a budget of {budget}"
    );
}

/// RULE: a search that **reaches** its goal is not truncated by the budget.
///
/// The budget bounds the *work*, not the answer: a search whose goal is reachable inside `h` expansions must end on the goal
/// itself, or the budget would quietly become a distance limit on every walk.
#[test]
fn a_reachable_goal_is_still_reached() {
    let field = ground(96, 96, &[]);
    let route = field
        .route(SCALE, centre(4, 4), centre(62, 4))
        .expect("a long walk across open ground");
    let last = route.last().copied().expect("a route has an end");
    assert_eq!(
        (last.0.raw().div_euclid(SCALE), last.1.raw().div_euclid(SCALE)),
        (62, 4),
        "the route must end on the goal"
    );
}
