# Half a tile: the frame mistake under every "the unit is inside the building" report (D168)

*2026-11-08, seventh round. The owner sent a screenshot with three blue squares whose centres sat well inside a building's dark grey and said the
picture contradicted my explanation. It did. This is the defect.*

## The two frames, written down

`Camera::project` puts a **tile's diamond centre on the tile's point**:

```
project((x, y)) = the middle of the diamond labelled (x, y)      =>  tile (x, y) covers  x - 0.5 .. x + 0.5  on screen
```

The simulation's tile `(x, y)` covers `x .. x + 1`, so a unit that has **settled** stands at its tile's centre, `x + 0.5`. Drawn with the raw
position, that lands at `project(x + 0.5, y + 0.5)` - the point where **four diamonds meet**, half a tile below and right of the tile the unit is
actually standing in. A marker of `0.35 * rx` half-width then spills a quarter of itself into each of those four tiles, and if one of them is a
building's solid tile, the marker is painted *inside the building* with its centre visibly within the dark grey. Exactly the screenshot.

## The failing test, watched failing first

`crates/view/tests/unit_centring.rs` places a villager at the centre of tile `(42,42)` and asserts the **centroid of its painted pixels** is at the
centre of that tile's diamond:

```
the villager at the centre of tile (42,42) was drawn at (450.0,398.0) but its tile's diamond is centred at (450.0,374.0)
- off by (0.0,24.0) pixels, half a tile
```

24 px is exactly `ry`, half a diamond's height: the marker was drawn one tile's half below its tile. Mutation-checked after the fix - reverting
`drawn_at` to the identity reproduces the same failure, so the test is load-bearing and not a coincidence.

## The fix

`ruage_view::drawn_at(position) = (position.0 - 0.5, position.1 - 0.5)`, one public function used by **every** place that puts a body on the
screen: the unit marker, the selection ring, the rubber band's hit test and the double-click's on-screen test. Buildings keep their own rule - their
ring is computed from the footprint's tile midpoint, which was already in the right frame.

The marker size also went back to `0.30 * rx` from `0.35`: at `0.30` the square's corner sits at `0.30 + 0.60 = 0.90` of the diamond, inside it with
room, so the D166 clip no longer has to cut the corners of a settled unit's square.

## What was wrong with the six rounds before this

1. **I trusted the tests I had written.** Three of them placed units at *integer* positions - `(10.0, 10.0)` - which is a tile **corner** in the
   simulation's frame and a diamond **centre** in the view's. Every such fixture reproduced the bug's precondition and then measured from the wrong
   point, so it agreed with the code. Real units, after a `Move`, sit at `x + 0.5` (measured: `(40.50, 39.50)`), and that is the case that was
   broken. The fixtures in `yard_drawing_order`, `marker_fits_tile`, `depth`, `ring`, `picking` and `shapes` now spawn at tile centres.
2. **I argued about ordering when the offset was the cause.** Both `layer_of` orderings were tried (D165, D167) and neither could fix a half-tile
   displacement. The overlap survived both because it was never about which was painted second.
3. **I said the picture agreed with the engine.** It did not, and the owner said so twice. "The drawn footprint equals the blocked footprint" was
   true of the **tiles** and never of the **units**.

## Performance

**Simulation untouched**: the gate's acceptance hash is byte-identical - this is `ruage-view` only. The conversion is two subtractions per drawn
body. **Measured**: `unit_centring` runs in **0.82 s**; the whole `ruage-view` suite is unchanged in count and green; the gate's wall clock is
recorded with this commit.

## Stability

- **One function, four call sites.** A second subtraction anywhere would drift again, so `drawn_at` is `pub` and the four sites import it; a
  stray `position.0 - 0.5` outside it would be a review failure.
- **Fixtures at tile centres.** Six test files were corrected to spawn where real units stand. A fixture at an integer position now tests something
  no unit does.
- **The clip is still there** (D166) and still centred on the marker, which for a settled unit *is* its tile's diamond centre.
- Suite: **771 passed / 0 failed / 19 ignored**, clippy `-D warnings` clean, fmt clean.
