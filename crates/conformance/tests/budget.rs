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

/// RULE: a search that **reaches** its goal is not truncated, and its answer is the goal itself.
///
/// The budget is read but **not enforced** (see the two `#[ignore]`d rows), so this is the row that holds today: a walk the
/// search can afford ends on the goal, whatever the map.
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

/// RULE: the budget the binary enforces, **not enforced here yet** - and the reason is measured.
///
/// The binary's budget is its own estimate (`expansions > h`, cap 20,000). It can afford to be tight because its search is
/// **levelled**: a coarse node covers `<< level` tiles, so a long walk is a handful of expansions. Ours is flat, one node per
/// tile, and at `h = 25 x tiles` the budget truncates a *legitimate* detour: measured on
/// `retaliation::a_melee_blow_never_becomes_an_arrow`, a villager one tile from its sheep was answered the "best settled" node
/// **fifteen tiles east** and walked into a wall. So the budget lands **with** the hierarchy (`docs/parity-plan.md` P11), or
/// not at all.
#[test]
#[ignore = "the budget needs the levelled search to be affordable (P1 lands with P11); measured, 2026-11-19"]
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
    let (_route, expansions) = field.search_reporting(from, to, SCALE, &[]);
    let budget = ruage_sim::Ground::estimate(from, to);
    assert!(
        expansions <= budget,
        "the search spent {expansions} expansions against a budget of {budget}"
    );
}

/// RULE: the 20,000 cap, **not enforced here yet** for the same reason as the row above.
#[test]
#[ignore = "the budget needs the levelled search to be affordable (P1 lands with P11); measured, 2026-11-19"]
fn the_budget_never_exceeds_twenty_thousand() {
    let field = ground(480, 480, &[]);
    let (from, to) = ((1u16, 1u16), (478u16, 478u16));
    let (_route, expansions) = field.search_reporting(from, to, SCALE, &[]);
    assert!(expansions <= 20_000, "the search spent {expansions}");
}
