//! **RULE: the search costs what the original's table costs.**
//!
//! Source: `samuelcaldas/age-of-empires-2-decomp`, `zone_spatial_pyramid_find_path.cpp` / `zone_spatial_pyramid_eval_cost.cpp`
//! and the tree's own `docs/movement-in-the-binary.md` (`FUN_008007f0`): a cardinal step accumulates **16**, a diagonal the
//! octile table's **23** (`16 x sqrt2`, rounded), and the estimate is Euclidean tile distance scaled by **25** - a weight of
//! about 1.56, so the search is deliberately not admissible and returns routes slightly longer than the shortest.
//!
//! These are the four numbers a re-port can get wrong silently, so each is asserted against the constant it claims rather
//! than against whatever the engine happens to do today.

use ruage_conformance::{CARDINAL, DIAGONAL, SCALE, ground, octile};

/// RULE: a diagonal is the octile ratio of a cardinal step, not the same and not twice.
#[test]
fn a_diagonal_costs_the_octile_ratio_of_a_cardinal_step() {
    assert_eq!(ruage_sim::Ground::step_cost((4, 4), (5, 4)), CARDINAL);
    assert_eq!(ruage_sim::Ground::step_cost((4, 4), (5, 5)), DIAGONAL);
    assert_eq!(ruage_sim::Ground::step_cost((4, 4), (4, 4)), 0);
    // 23 / 16 = 1.4375, which is `sqrt2` to two decimals: the octile model, not a second cost model.
    assert!(
        (DIAGONAL as f64 / CARDINAL as f64 - std::f64::consts::SQRT_2).abs() < 0.03,
        "the ratio {:.4} is not sqrt2",
        DIAGONAL as f64 / CARDINAL as f64
    );
}

/// RULE: the estimate is Euclidean distance in tiles scaled by 25.
#[test]
fn the_estimate_is_euclidean_distance_scaled_by_twenty_five() {
    // One tile along an axis: 1 x 25.
    assert_eq!(ruage_sim::Ground::estimate((5, 5), (6, 5)), 25);
    // Three-four-five: 5 x 25.
    assert_eq!(ruage_sim::Ground::estimate((5, 5), (8, 9)), 125);
    // Zero distance is zero.
    assert_eq!(ruage_sim::Ground::estimate((5, 5), (5, 5)), 0);
}

/// RULE: no route is cheaper than the octile lower bound - a walk that reaches its goal pays at least what king moves cost.
///
/// Measured over the **search's own tile chain**, not the pulled route: the pull exists to remove slack, and its waypoints are
/// corners rather than steps, so charging it by the step table would measure the wrong thing.
#[test]
fn a_route_is_never_cheaper_than_the_octile_bound() {
    // A wall with one gap, so the route must bend and the bound is a real one rather than a straight line.
    let wall: Vec<(u16, u16)> = (0..=8).map(|y| (8, y)).collect();
    let field = ground(20, 20, &wall);
    let (from, to) = ((4u16, 4u16), (14u16, 4u16));
    let chain = field
        .search(from, to, SCALE, &[])
        .expect("a way round exists");
    let mut total = 0;
    let mut here = from;
    for next in &chain {
        total += ruage_sim::Ground::step_cost(here, *next);
        here = *next;
    }
    assert!(
        total >= octile(from, to),
        "the search paid {total}, below the octile bound {}",
        octile(from, to)
    );
}
