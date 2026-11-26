# Review: ray hit and edge-walk core (Task 2, 2026-11-26)

Task 2 of the Full Way plan. Task 1's `WayRoute`/`verify_path` signatures untouched.

## What landed

- `crates/sim/src/geometry.rs`: `ray_hit` (exact rational slab test, first grown-rect
  hit in slice order, `None` = clear) and `walk_shape` (clockwise wall-follow over the
  split-coordinate grid of the hit rect's touch-connected cluster; closed ring, last
  point repeats first). Walk core is compare/add/sub on raw coords only; the one exact
  division (cross-multiplied rationals) lives in ray setup.
- `crates/sim/src/lib.rs`: re-export `ray_hit`, `walk_shape`.
- `crates/sim/tests/geometry_walk.rs`: the brief's two tests verbatim plus the two
  fixtures (`two_boxes_on_x_axis`, `aligned_overlapping_pair`).

## Performance

- Release timing probe (50k `ray_hit`, 5k `walk_shape`, 4-box cluster, scratch test
  deleted after): `ray_hit` 62ns/op, `walk_shape` 348ns/op. Both linear in box count;
  no allocation on the ray path beyond the best-tracked entry.
- Full `ruage-sim` suite wall time unchanged within noise (geometry tests 0.00s).

## Stability

- Attacked with a scratch fuzz harness (deleted before commit, 2000 random ray cases +
  500 random rect sets x every hit index): `ray_hit` agrees with `verify_path` exactly
  (hit implies that box crossed; miss implies the leg certifies clean), and every
  `walk_shape` ring is closed, deterministic across runs, `len >= 4`, and
  `verify_path`-clean.
- Failure the fuzz caught (real bug, fixed): zero-width/height grown rects own no grid
  cells, so an all-degenerate cluster fell back to the hit rect's own corners and cut
  through a member line-box. Fixed by inflating degenerate axes one raw unit for grid
  purposes; the ring still certifies against the true boxes (boundary edges of the
  inflated union cannot cross a true box without that box joining the cluster).
- Probe bug of mine, not shipped: the first fuzz midpoint formula halved the base
  point and sampled off-segment; replaced with the exact endpoint/segment verdict.
- Pre-existing, untouched: `world/step.rs:29` clippy warning (fails `-D warnings`;
  reproduce on this tree) and 2 `modifiers` test failures (Town Watch LoS); neither
  file references geometry. `cargo fmt --check -p ruage-sim` also flags pre-existing
  drift in `gather.rs`/`geometry_verdict.rs` (this toolchain's rustfmt); reverted, not
  mine.
- Own errors this slice: ate `fn describe`'s signature line in one edit (repaired,
  compiles); duplicated `sides` helper (removed); 3 clippy warnings in new code
  (type alias + `is_some_and`, fixed). Final: 0 clippy warnings in new files,
  new files fmt-clean.
- Known limitation (not attacked further): two member rects touching at a single
  corner point join one cluster and the walk traces one outer loop; the ring stays
  verify-clean but may not surround both lobes. Left for Task 3's routing use to judge.
