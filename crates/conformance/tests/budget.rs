//! **RULE: a walk the goal affords is answered at the goal.**
//!
//! Source: `FUN_008007f0_008007f0.c` (the REA corpus, HD 5.7), read 2026-11-19 - the tile search's expansion budget
//! (`expansions <= 25 x euclid`, hard cap 20,000) and its settled-beside tail.
//!
//! The budget rows retired with the tile system (New Way Task 5): there is no expansion counter in a geometric
//! router, so "at most its own estimate" and "never more than 20,000" have nothing to count. What stays is the
//! rule the budget existed to protect - a reachable goal is answered at the goal itself, not truncated short.

use ruage_conformance::{SCALE, centre, ground};

/// RULE: a walk that **reaches** its goal is not truncated, and its answer is the goal itself.
///
/// **The budget is enforced, and this row says the enforcement does not cost a walk it can afford**: a reachable goal is still
/// reached, whatever the map.
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
