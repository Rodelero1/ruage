# Review: the arrows pan the screen, not the grid

*Change: `Camera::scroll_screen` takes a direction **as the player sees it** and converts it to tile space; the window's
arrow keys call it. The map is drawn as diamonds, so the old `scroll((1.0, 0.0))` moved the picture diagonally - the
owner's report: "panning should be up-down and left-right, since the map is 45 degrees rotated".*

## Performance

Nothing on the tick path: the camera is view state and the conversion is two arithmetic operations per key repeat, so the
pan costs **0 us per frame** - it runs on an input event. Measured frame cost after the change: **505 us a frame at
960x720 with 126 entities** (`frame_cost`, release, `--ignored`), against 505 us before it, which is the probe's noise
floor. The sim does not see the camera at all: `crates/view` reads the simulation and never writes it, so `state_hash`
and the acceptance match are untouched by construction.

## Stability

- **The conversion is exact.** `project` puts a tile at `(dx - dy, dx + dy)`, so a screen step of one is `(+0.5, -0.5)`
  for right and `(+0.5, +0.5)` for down: halves are exact in `f32`, so a pan out and back lands on the same pixel, which
  the test asserts (`the_arrows_pan_the_screen_not_the_grid`).
- **Watched failing first**: with `scroll_screen` implemented as the obvious passthrough to tile space, the test reports
  *"panning right should not move the picture up or down: (320, 264) -> (272, 240)"* - both axes moved, which is the bug.
- The old tile-space `scroll` is kept: it is what a script or a test wants when it means the grid, and the pinning test
  for a resize (`resizing_the_window_keeps_the_camera_on_its_tile`) still passes.

---

*Staged with the change for `tools/review_check.py`. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
