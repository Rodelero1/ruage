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
/// **The budget is enforced, and this row says the enforcement does not cost a walk it can afford**: a reachable goal is still
/// reached, whatever the map. The two rows below - the estimate and the 20,000 cap - were `#[ignore]`d from D279 until P11 landed,
/// because a *flat* search could not afford them; long walks now start on the levelled search, so they are live.
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

/// RULE: the budget the binary enforces (`FUN_008007f0`: expansions <= estimate, hard cap 20,000).
///
/// **It was parked on a measurement, and the measurement no longer holds.** A flat search at `h = 25 x tiles` truncated a legitimate
/// detour - a villager one tile from its sheep was answered the best settled node **fifteen tiles east** - so D279 parked this row until
/// the hierarchy existed. P11 landed it (D296-D302): long walks start on the levelled search, whose coarse nodes cover `<< level` tiles,
/// and the truncation is gone.
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
    let (_route, expansions) = field.search_reporting(from, to, SCALE, 0, &[]);
    let budget = ruage_sim::Ground::estimate(from, to);
    assert!(
        expansions <= budget,
        "the search spent {expansions} expansions against a budget of {budget}"
    );
}

/// RULE: **the 20,000 cap is what stops a far search, not its estimate** - `20000 < local_46c` at `0x800d57`, the second
/// clause of the same guard as the row above.
///
/// **The old scene could not see this rule** (D336). It walked a 480 x 480 map corner to corner, whose own estimate is
/// `25 x 674 = 16,850` - already below the cap - and spent **642** expansions; the row asserted `642 <= 20_000` and stayed
/// green with `EXPANSION_CAP` set to `u32::MAX`. The cap binds only when `25 x euclid > 20,000`, i.e. a diagonal walk past
/// ~800 tiles: this map is 1,200 a side (`25 x 1,692 = 42,300` measured) and the goal is sealed, so the cap is the only thing that
/// can stop the search. The assertion is on the **count**, not on a bound recomputed from `estimate`, so a wrong cap fails it.
#[test]
fn the_budget_never_exceeds_twenty_thousand() {
    // A 3x3 ring round the goal: (1198,1198) is walkable, its eight neighbours are not, so the search settles beside it and
    // must spend everything it is allowed to try.
    let mut sealed = Vec::new();
    for dx in -1i64..=1 {
        for dy in -1i64..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            sealed.push((
                u16::try_from(1198 + dx).expect("x fits"),
                u16::try_from(1198 + dy).expect("y fits"),
            ));
        }
    }
    let field = ground(1_200, 1_200, &sealed);
    let (from, to) = ((1u16, 1u16), (1198u16, 1198u16));
    let estimate = ruage_sim::Ground::estimate(from, to);
    assert!(
        estimate > 20_000,
        "the scene must ask for more than the cap, or it tests nothing: estimate {estimate}"
    );
    let (route, expansions) = field.search_reporting(from, to, SCALE, 0, &[]);
    let route = route.expect("a sealed goal is answered, not refused");
    let end = *route.last().expect("a route has an end");
    assert_ne!(end, (to.0, to.1), "the goal is sealed and cannot be reached");
    assert_eq!(
        expansions, 20_000,
        "the cap is the budget on this map: {expansions} of 20,000 (estimate {estimate})"
    );
}

/// RULE: **the bound bites** - a search that runs out of budget answers with a point short of the goal, and spends its budget. This is the
/// row the other two cannot be: they would pass if the search never truncated at all.
#[test]
fn a_search_that_runs_out_of_budget_stops_short_of_the_goal() {
    // A sealed box 40 tiles across, with the goal outside it: the walk is short (so the budget is small - 25 x the few tiles to the
    // wall) but the reachable room inside is 1,600 tiles, far more than the budget can expand.
    let mut wall = Vec::new();
    for t in 0..40u16 {
        wall.push((6 + t, 6));
        wall.push((6 + t, 45));
        wall.push((6, 6 + t));
        wall.push((45, 6 + t));
    }
    let field = ground(64, 64, &wall);
    let (from, to) = ((20u16, 20u16), (20u16, 60u16));
    let (route, expansions) = field.search_reporting(from, to, SCALE, 0, &[]);
    let budget = ruage_sim::Ground::estimate(from, to).min(20_000); // the crate-visible copy of `EXPANSION_CAP`;
    let route = route.expect("a route into the sealed room");
    let end = *route.last().expect("a route has an end");
    assert_ne!(end, to, "the goal is outside the sealed room and cannot be reached");
    assert!(
        (end.0 as i64 - 20).abs() <= 25 && (end.1 as i64 - 20).abs() <= 25,
        "a truncated search must answer inside the room it could expand: it answered {end:?}"
    );
    assert_eq!(
        expansions, budget,
        "the search must spend exactly its budget before answering short: {expansions} of {budget}"
    );
}
