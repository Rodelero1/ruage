//! **The coarse A\* the levelled search runs** (P11, step 11c).
//!
//! Read from `FUN_007ed280` (the neighbour opener and its cost table, D298) and `FUN_007ee980` (the descent and the chain walk, D297):
//!
//! - the search runs on **one level**, the coarsest whose cell holding the start is usable;
//! - neighbours are the **four directions** (`param_4` = 1, 2, 4, 8), never diagonal;
//! - `g = step cost + parent's g`, where the step cost is `Levels::step_cost` (10 / 300 / 2000);
//! - `h` is the remaining distance **along one axis** (`window_hi - coord` on the axis the move is on), which for a search without the
//!   binary's window is Manhattan between cells;
//! - the frontier is an **insertion-sorted linked list** on `f`, and a new node goes **before** equal `f` (newest first);
//! - a closed cell carries `0x10` and is never reopened;
//! - on arrival the **parent chain** is walked and each node contributes a **cell centre** (`n * 2^level + 0.5`), which is what the
//!   refinement hands to the movement code.
//!
//! **Two deviations, both recorded and both in the safe direction:**
//!
//! 1. **No search window.** The binary clips every expansion to `in_ECX[0x42..0x45]`, a box whose padding it sets somewhere not yet
//!    read. Without it the search can find routes the clipped one would refuse - a superset, never an illegal route.
//! 2. **`h` is Manhattan, not `window_hi - coord`.** With the window removed there is no `window_hi`; Manhattan is the same measure on
//!    a 4-directional grid.
//!
//! Written before the code (D7, SPEC §11).

use ruage_sim::{Ground, Levels};

const OPEN: &str = r#"
schema = 1
[map]
width = 96
height = 49
terrain = [
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
]
[[map.town_center]]
player = 1
x = 1
y = 1
[[map.town_center]]
player = 2
x = 90
y = 45
"#;

fn open() -> Ground {
    let resources = ruage_data::load_resources_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../config/resources.toml"
        ))
        .expect("config/resources.toml"),
        "resources.toml",
    )
    .expect("resources.toml");
    let map = ruage_data::load_map_str(OPEN, "coarse_route.toml", &resources)
        .expect("the fixture map loads");
    Ground::from_map(&map)
}

/// **The 58-tile walk comes back as a coarse chain**: more than one cell, in order from start to goal, each one apart by a single step
/// in one direction - the shape the refinement turns into cell-centre waypoints.
#[test]
fn a_long_open_walk_is_a_chain_of_cells() {
    let ground = open();
    let levels = Levels::of(&ground);
    let route = levels
        .coarse_route((4, 4), (62, 4))
        .expect("an open walk across the map");
    assert_eq!(route.level, 3, "an open start descends to level 3 (D297)");
    assert!(
        route.cells.len() > 1,
        "a 58-tile walk must come back as a chain, not as {} cell(s)",
        route.cells.len()
    );
    assert_eq!(
        route.cells.first().copied(),
        Some((0u16, 0u16)),
        "the first cell is the start's"
    );
    assert_eq!(
        route.cells.last().copied(),
        Some((7u16, 0u16)),
        "the last cell is the goal's"
    );
    for pair in route.cells.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let step = (
            i32::from(b.0) - i32::from(a.0),
            i32::from(b.1) - i32::from(a.1),
        );
        assert!(
            (step.0.abs() + step.1.abs()) == 1,
            "cells {a:?} and {b:?} are not one orthogonal step apart - the search is 4-directional"
        );
    }
}

/// **The chain refines to the crossing of each shared edge** (D322, `FUN_007eeda0`). The refinement does not emit the cells'
/// middles: it expands each cell to the tile span it covers and crosses the boundary between two cells **on the first tile of that
/// span the step can pass** - so every waypoint but the first and last sits *on a tile boundary*, at the middle of the crossing
/// tile, and the walker follows the corridor the cells describe instead of hopping between their centres.
#[test]
fn the_chain_refines_to_the_crossing_of_each_shared_edge() {
    let ground = open();
    let levels = Levels::of(&ground);
    let scale = 20_000i64; // the engine's fixed scale, as `ruage_conformance::SCALE`
    let route = levels
        .coarse_route((4, 4), (62, 4))
        .expect("an open walk across the map");
    assert_eq!(route.level, 3, "an open start descends to level 3");
    let points = levels.refine(&route, scale);
    assert_eq!(
        points.len(),
        route.cells.len(),
        "the first cell's centre and one crossing per step; the goal is the caller's"
    );
    // The ends are the cells' own middles: the caller discards the first (the walk starts where the body stands) and replaces the
    // last with the goal.
    let centre = |cell: (u16, u16)| {
        (
            (i64::from(cell.0) * 8 + 4) * scale,
            (i64::from(cell.1) * 8 + 4) * scale,
        )
    };
    let first = route.cells[0];
    assert_eq!(
        (points[0].0.raw(), points[0].1.raw()),
        centre(first),
        "the first point is the start cell's centre"
    );
    for (nth, pair) in route.cells.windows(2).enumerate() {
        let (from, to) = (pair[0], pair[1]);
        let point = points[nth + 1];
        let (x, y) = (point.0.raw(), point.1.raw());
        // A crossing sits **on the boundary** between the two cells and at a tile's middle along it.
        if to.0 != from.0 {
            let boundary = i64::from(to.0.min(from.0) + 1) * 8 * scale;
            assert_eq!(
                x, boundary,
                "crossing {nth} is not on the column boundary between {from:?} and {to:?}"
            );
            assert_eq!(
                (y - scale / 2) % scale,
                0,
                "crossing {nth} is not at a tile's middle along the edge: y = {y}"
            );
        } else {
            let boundary = i64::from(to.1.min(from.1) + 1) * 8 * scale;
            assert_eq!(
                y, boundary,
                "crossing {nth} is not on the row boundary between {from:?} and {to:?}"
            );
            assert_eq!(
                (x - scale / 2) % scale,
                0,
                "crossing {nth} is not at a tile's middle along the edge: x = {x}"
            );
        }
        // And both tiles it crosses are standable: an open map, so every one of them is.
        assert!(
            !levels.blocked(0, to.0 * 8, to.1 * 8),
            "the crossing's far side is blocked"
        );
    }
}

/// **A wall of blocked cells is routed round, not through**: the chain never contains a blocked cell.
#[test]
fn a_wall_is_routed_round() {
    let mut ground = open();
    // A wall down the middle of the walk's own row band, from y 0 to y 7, so level 3 cell (3, 0) is blocked.
    for y in 0..8u16 {
        ground.block(24, y);
    }
    let levels = Levels::of(&ground);
    let route = levels
        .coarse_route((4, 4), (62, 4))
        .expect("a walk round the wall exists");
    for cell in &route.cells {
        assert!(
            !levels.blocked(route.level, cell.0, cell.1),
            "the chain runs through blocked cell {cell:?}"
        );
    }
}

/// **Nothing reachable is `None`, not a panic**: a goal walled off entirely answers `None`.
#[test]
fn a_walled_off_goal_has_no_coarse_route() {
    let mut ground = open();
    // Ring the goal's cell completely.
    for x in 56..=71u16 {
        ground.block(x, 0);
        ground.block(x, 7);
    }
    for y in 0..=7u16 {
        ground.block(56, y);
        ground.block(71, y);
    }
    let levels = Levels::of(&ground);
    assert!(
        levels.coarse_route((4, 4), (62, 4)).is_none(),
        "a goal whose cell is walled off has no coarse route"
    );
}

/// **A long walk the straight line cannot take is answered by the hierarchy** (D301's second tier): the route comes back as a chain of
/// **level-3 edge crossings** (D322 - the cells' middles were the old refinement), not as a per-tile polyline. The control in the same fixture is the open walk, which stays a single waypoint
/// because the line reaches it.
#[test]
fn a_blocked_walk_is_answered_by_the_hierarchy() {
    let mut ground = open();
    // A wall from the top of the map down through the walk's own band, with a gap at the bottom: the line cannot pass, so a search must.
    for y in 0..28u16 {
        ground.block(32, y);
    }
    let scale = 20_000i64;
    let at = |x: i64, y: i64| {
        (
            ruage_fixed::Fixed::from_raw(x * scale + scale / 2),
            ruage_fixed::Fixed::from_raw(y * scale + scale / 2),
        )
    };

    let blocked = ground
        .route(scale, at(4, 4), at(60, 4))
        .expect("a walk round the wall");
    assert!(
        blocked.len() > 1,
        "a blocked walk must come back as a chain, not {} waypoint(s)",
        blocked.len()
    );
    // Every point but the last is **a crossing of a level-3 boundary** (D322): the step's own coordinate sits on the `n * 8` line
    // between two cells and the other one is at a tile's middle along it.
    for point in &blocked[..blocked.len() - 1] {
        let (x, y) = (point.0.raw(), point.1.raw());
        let on_boundary = |raw: i64| raw % (8 * scale) == 0;
        let at_middle = |raw: i64| raw % scale == scale / 2;
        assert!(
            (on_boundary(x) && at_middle(y)) || (at_middle(x) && on_boundary(y)),
            "waypoint ({x}, {y}) is not a crossing of a level-3 boundary"
        );
    }

    // The control: the same walk with no wall is one waypoint, because the straight line reaches it.
    let clear = open()
        .route(scale, at(4, 4), at(60, 4))
        .expect("an open walk");
    assert_eq!(clear.len(), 1, "an open walk is still one waypoint");
}
