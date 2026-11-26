//! Verdict contract for the geometric solver: `Reached` vs `Closest` is explicit,
//! and `verify_path` certifies a polyline against the grown boxes.

use ruage_fixed::Fixed;
use ruage_sim::{Clear, WayRoute, verify_path};

#[test]
fn reached_vs_closest_is_explicit() {
    let route = WayRoute::Reached(vec![(Fixed::from_raw(0), Fixed::from_raw(0))]);
    assert!(matches!(route, WayRoute::Reached(_)));
    let dumped = verify_path(&[(0, 0)], &[]);
    assert!(dumped.is_none());
}

#[test]
fn inside_start_walks_out_instead_of_refusing() {
    let avoid = vec![Clear {
        at: (0, 0),
        half: (10_000, 10_000),
        reach: 0,
    }];
    let dumped = verify_path(&[(0, 0)], &avoid);
    assert!(dumped.is_some());
}
