//! **Rule set 5: where a search may look.**
//!
//! Read from `zone_spatial_pyramid_test_cell_bounds.cpp` (`func_001D40E0`) and `zone_spatial_pyramid_test_point.cpp`
//! (`func_001D5B00`): a cell is inside iff `min <= c <= max` on both axes - **inclusive at both ends** - with the maxima
//! clamped to the map; a tile outside is dropped and never given a node, so it can never be a waypoint.
//!
//! **What cannot be tested here, and is not pretended otherwise.** The window itself is built in
//! `zone_spatial_pyramid_search_radius.cpp` as `centre +- (radius + extent)` and walked in **8x8 blocks**; the border tiles
//! carry a cost adjustment (`cost_mod = 0` at the low edge, `-100` at the high one). Ours grows the short-range window by a
//! fixed 3-tile margin (`short.rs:29`) and has no border cost. Both facts are real and both are **invisible from outside**:
//! the block walk is a data structure, and a border cost only shows up in tie-breaks we expose no window for. They belong
//! in the review, not in assertions that would pin an implementation detail.

use ruage_conformance::{SCALE, at, walk};
use ruage_sim::{find_path, short_range};

/// RULE: the bounds are inclusive at both ends, and the maximum is the map's last tile.
///
/// Source: `func_001D40E0` - `x < min_x || x > max_x` is out, so `x == max_x` is in; `zone_spatial_pyramid_search_radius`
/// clamps that maximum to the map (`if max_x >= limit_x { max_x = map_max }`). Our `Walk::contains` is `x < width`, which is
/// the same set for a window of `[0, width - 1]`.
#[test]
fn the_bounds_include_the_last_tile_and_exclude_the_one_past_it() {
    let walk = walk(8, 5, &[]);
    assert!(walk.contains(0, 0), "the origin is inside");
    assert!(walk.contains(7, 4), "the last tile is inside: the bound is inclusive");
    assert!(!walk.contains(8, 4), "x == width is outside");
    assert!(!walk.contains(7, 5), "y == height is outside");
}

/// RULE: a tile outside the bounds is never a waypoint - it is dropped, not costed.
///
/// Source: `func_001D5B00`'s first test returns without creating a node for any cell outside `min..=max`; the search's
/// frontier therefore cannot contain one. Our search refuses an off-map goal outright (`path.rs:204`) and never expands to
/// a tile it does not contain (`path.rs:268`).
#[test]
fn a_tile_outside_the_bounds_is_never_a_waypoint() {
    let walk = walk(8, 8, &[]);
    assert!(find_path(&walk, (0, 0), (8, 0)).is_none(), "an off-map goal is refused");
    assert!(find_path(&walk, (0, 0), (0, 8)).is_none());
    let route = find_path(&walk, (0, 0), (7, 7)).expect("the far corner is on the map");
    for tile in &route {
        assert!(walk.contains(tile.0, tile.1), "the route left the map at {tile:?}");
    }
}

/// RULE: a walk that starts and ends on the map's edge stays on the map - the window is clamped, not wrapped or padded.
///
/// Source: the same clamp in `zone_spatial_pyramid_search_radius.cpp`, which pulls `min` up to 0 and `max` down to the map's
/// last tile before the search uses them. Ours: the geometry solver must not place a corner outside the grid (the bug
/// SPEC v1.82 records: "corners outside the map").
#[test]
fn a_walk_along_the_edge_stays_on_the_map() {
    let walk = walk(12, 12, &[]);
    let edge = short_range(&walk, SCALE, at(0, 0), at(11, 0), 4_000, &Default::default())
        .expect("the edge is walkable");
    for point in &edge {
        let x = point.0.raw() / SCALE;
        let y = point.1.raw() / SCALE;
        assert!(
            (0..12).contains(&x) && (0..12).contains(&y),
            "a waypoint sits off the map at raw ({}, {})",
            point.0.raw(),
            point.1.raw()
        );
    }
}
