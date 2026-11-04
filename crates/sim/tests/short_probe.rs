//! What one short-range solve costs. Ignored by default (a measurement, not a rule); run with
//! `--release -- --ignored --nocapture`.

use std::collections::BTreeSet;
use std::time::Instant;

use ruage_fixed::Fixed;
use ruage_sim::{Walk, short_range};

const SCALE: i64 = 20_000;

/// A base-sized window: a ring of buildings and a wood, which is the shape a route in a base actually meets.
fn walk() -> Walk {
    let map = ruage_data::Map {
        width: 48,
        height: 48,
        terrain: vec![ruage_data::Terrain::Grass; 48 * 48],
        resources: Vec::new(),
        town_centers: Vec::new(),
    };
    let mut walk = Walk::from_map(&map);
    // Twelve 3x3 buildings and a 6x6 wood inside a 20-tile window.
    for (bx, by) in [
        (14, 12),
        (20, 12),
        (26, 12),
        (14, 18),
        (26, 18),
        (14, 24),
        (20, 24),
        (26, 24),
    ] {
        for dy in 0..3u16 {
            for dx in 0..3u16 {
                walk.block(bx + dx, by + dy);
            }
        }
    }
    for dy in 0..6u16 {
        for dx in 0..6u16 {
            walk.block(20 + dx, 30 + dy);
        }
    }
    walk
}

#[test]
#[ignore = "measurement, not a rule"]
fn a_short_range_solve_costs_less_than_a_plan() {
    let walk = walk();
    let at = |x: i64, y: i64| (Fixed::from_raw(x * SCALE), Fixed::from_raw(y * SCALE));
    let runs = 500u32;
    let start = Instant::now();
    let mut corners = 0usize;
    for i in 0..runs {
        let to = at(30 + i64::from(i % 3), 8);
        if let Some(points) = short_range(&walk, SCALE, at(10, 20), to, SCALE / 5, &BTreeSet::new())
        {
            corners += points.len();
        }
    }
    let per = start.elapsed().as_micros() as f64 / f64::from(runs);
    println!(
        "short_range: {per:.1} us a solve, {:.1} corners on average, over a window of eight buildings and a wood",
        corners as f64 / f64::from(runs)
    );
    assert!(per < 5_000.0, "a plan should be cheap");
}
