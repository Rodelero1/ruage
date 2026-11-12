# Clicking the body

**Selection only.** No simulation rule changed: a click now asks the renderer's own geometry instead of the tile grid.

## The defect, measured

A body is drawn at `position - 0.5` with its base on the point (D178) and is `body_half` pixels wide, rising `2 * body_half`
above the point. The picker used `floor(position)` plus the footprint. For a villager standing on a tile corner - a legal, ordinary
position, since movement is free of the grid - those two disagree about three quarters of the body.

- a click **8 px** left of the body's centre, over tile `(9,9)` with the villager at `(10.0, 10.0)`: **nothing selected**.
- two villagers a fifth of a tile apart, click at the overlap: **nothing selected**.

Both are the owner's *"some villagers cannot be even chosen by clicking on the square representing them"*, reproduced without a
screen.

## Performance

Input only: `Selection::click` runs on a mouse press, not per frame. Measured over a **150-entity** snapshot, 20,000 clicks in a
release build: **1.84 us per click** (it tests every body and then sorts only the few that contain the pixel). The simulation is untouched and the acceptance run is **byte-identical**: rebuilt `ruage-headless`,
36,000 ticks, seed 7, `tick=36000 entities=142 players=1 hash=ef7cdba92a83f173 repaths=435 made_room=151` - the same hash as
`main` before this change, which is the proof that nothing in the tick moved.

`ruage-view`: **40 test binaries green**, four of them new; `clippy -D warnings` and `fmt --check` clean.

## Stability

- **The one definition.** `body_half(camera)` replaced the literal `(rx * 0.20).max(2.0)` inside the renderer, so the picker and
  the painter now share the number instead of agreeing by luck. A future sprite only has to replace `body_contains`.
- **Determinism of the answer.** Bodies are ordered by `depth_of` (the renderer's own front-to-back answer), then own-before-theirs,
  then id - so a click cannot depend on the order the snapshot happens to list entities in.
- **What did not change.** A building is still its footprint, which keeps a farm selectable where nobody stands; a resource node
  and "clicking the ground clears" are as they were. `pick_at(world, tile)` - the window's right-click target - is deliberately
  still tile-based: an order names a place, not a body, and changing that needs its own evidence.
- **Transparent pixels** are not handled because there are no sprites yet (D18); when there are, `body_contains` is the one
  function to give a mask to.
