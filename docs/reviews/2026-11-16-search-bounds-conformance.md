# Review: area 1 of the conformance set - where a search may look

**Change.** `crates/conformance/tests/search_bounds.rs`: three rules read from `zone_spatial_pyramid_test_cell_bounds.cpp`
(`func_001D40E0`) and `zone_spatial_pyramid_test_point.cpp` (`func_001D5B00`) - the bounds are inclusive at both ends with the
maximum clamped to the map, and a tile outside them is dropped rather than costed, so it can never be a waypoint. No engine
code.

## Performance

None to measure and none claimed: three tests over `Walk::contains`, `find_path` and `short_range`, in the conformance crate,
which is its own workspace and not built by the gate. The suite's own figure is unchanged at **0.19 s** for the whole set;
these three are among the fastest in it.

## Stability

**What was attacked.** The bound rule itself: a tile at `width - 1` must be *inside* (inclusive), and a route must not place a
waypoint outside the grid even when both endpoints sit on the edge - the bug SPEC v1.82 records as "corners outside the map".
Both pass, so they are pins rather than findings.

**What is deliberately not asserted, and why.** The window's growth (`radius + extent`, `zone_spatial_pyramid_search_radius`)
and the border cost nudge (`cost_mod = 0` / `-100`) are real rules of the original and **invisible from outside**: the 8x8
block walk is a data structure, and a border cost surfaces only in tie-breaks we expose no window for. Asserting either would
pin an implementation detail; they are named in the test file's header and left untested until an accessor exists, which is
the next slice.

**Owed.** The `#[ignore]`-free register for the not-yet-implemented areas (trade, formations, waypoint substitution) - the
policy the owner ruled - is not in this commit.
