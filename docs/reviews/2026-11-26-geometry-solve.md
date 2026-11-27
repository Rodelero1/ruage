# Review: re-ray loop, smooth, bounds, no-failure (Task 3, 2026-11-26)

Task 3 of the Full Way plan. Tasks 1-2 signatures untouched (`WayRoute`,
`verify_path`, `ray_hit`, `walk_shape` unchanged except a one-line doc
correction noted below).

## What landed

- `crates/sim/src/geometry.rs`: `solve(scale, from, to, reach, avoid) ->
  WayRoute` - point-walker plus grown rects (per-rect `max(clear.reach,
  reach)`, documented so Task 4's already-grown lists are not grown twice);
  ray to goal, walk hit shape, re-ray per vertex (first vertex in coordinate
  order that sees walk and goal wins, else nearest certified vertex strictly
  closer to goal); brute-force smooth by skipping every waypoint the
  grown-segment test certifies; bounds are search-area rect + expand-once,
  turn count (4 more lefts than rights stops at the hole), and a fuel cap of
  `8 * (boxes + 1) + 8`. Never fails: `Reached` iff the walk stands on the
  goal, else `Closest`. Vertex order is sorted raw coords, rect order slice
  order, never hash order.
- `crates/sim/src/lib.rs`: one-line re-export of `solve` (the brief's file
  list omits it; the test imports `ruage_sim::solve`, so the export is
  required to compile).
- `crates/sim/tests/geometry_solve.rs`: the brief's four tests verbatim plus
  file-local helpers (`point`, `raw_points`, `point_inside_box`,
  `walled_goal`, `surrounded`, `u_boxes`, `open_u_shape`, `closed_hole`,
  `turn_balance`). The hole loop is named from the walls' own inner faces
  (listed counterclockwise) because `walk_shape` returns outer boundaries;
  it certifies clean since every edge lies on a grown face.
- Review Minors from Task 2 judged while in the file: (1) tie-break - the
  review claims an inside origin returns the later index, but the code keeps
  the earlier (`(Some((_, None)), _) => false`, geometry.rs); a scratch probe
  asserted `ray_hit` from inside two overlapping boxes is `Some(0)`, so the
  code already matches the brief and nothing changed. (2) Accepted and fixed:
  the walk doc claimed no multiply/divide while the grid bookkeeping
  multiplies cell indices; one-line correction. (3) Not changed: degenerate
  inflation was already documented where it happens.

## Performance

- Release timing probe (scratch test, deleted before commit, 2000 `solve`
  calls over a 3-box scene): 2069ns/op. Full `ruage-sim` suite wall time
  unchanged within noise (geometry tests 0.00s); the 4-test `geometry_solve`
  binary runs in under 1ms.

## Stability

- Attacked with a scratch harness (deleted before commit): four walks over a
  3-box scene (open reach-around, blocked leg, cross-map, zero-length),
  each solved twice for determinism and each `verify_path`-certified clean;
  goal-inside-a-box answered `Closest` with a clean path; an inside start
  walked out with a clean path. All passed; no failure the harness caught.
- Bounds terminate by construction: every accepted step strictly decreases
  the exact integer squared distance to the goal, and the fuel cap, the
  expand-once flag, and the hole turn-count each force `Closest` when no step
  qualifies - so the loop cannot circle a shape or the map.
- Failure probed on purpose: none found. The two `modifiers` (Town Watch LoS)
  failures and the `world/step.rs:29` clippy error reproduce with this
  slice stashed (clean HEAD); neither file references geometry. `geometry.rs`
  itself is clippy-clean under `-D warnings` (one `type_complexity` error of
  mine fixed by splitting the advance key out); remaining repo-wide clippy
  and rustfmt drift pre-date this slice.
- Own errors this slice: a stray edit ate one newline in `describe` (repaired
  before compiling); the `type_complexity` error above. Both fixed, final
  diff re-tested.
- Known limitation: `Closest` stops at the walk's nearest certified point,
  not a globally nearest reachable point - the contract only promises beside
  the goal, never through a wall.
