# The body stood half a shape too low (D178)

*2026-11-08. The owner, after D177: **"the circle moves a bit elsewhere than it should. See picture with yellow cross being target destination and the
unit above it stops a bit further."** Two complaints in one picture. One of them was not true, and the other was worse than he said.*

## First, the measurement that says the walk is right

`crates/sim/tests/move_arrival.rs` walks a villager across **empty grass** - nothing within three tiles of either end - to the tile `(12,11)` and
measures the gap to that tile's centre. It passes: the villager finishes **within a tenth of a tile** of the tile it was sent to. So the simulation is
not stopping short; a move lands where it was told to.

## Then the thing that was actually wrong

The silhouette was drawn **centred on the point**: rows `-reach ..= +reach` around the unit's position. A body is not a disc around a point, it stands
*on* one - so its visible mass hung a full half-shape **below** the place it stood, and the selection ellipse, which is centred on the point, sat at
its waist. Reproduced first, in the owner's terms:

```
the body's lowest row is at y=388, and the unit stands at y=374 - the silhouette is
centred on the point instead of standing on it
```

Fourteen pixels - a third of a body - of "the unit stops before the cross", with the cross exactly where it should be.

## The change

The body is drawn with its **base on the point**, `rows -2 * reach ..= 0`. That also answers what he asked for two messages earlier, *"it would help
to align the center of the ellipse with the bottom side of the square/triangle"*: the ellipse is centred on the point, and the point is now the body's
base, so the ring encircles the feet instead of the waist.

Two numbers follow from the geometry rather than from taste:

- **half = 0.20 of a tile**, not the old 0.30. The body now rises `2 * half` above the point, and the tile's diamond narrows as it rises: at height
  `2 * half` it is `rx - 4 * half` wide, so a body of half-width `half` fits whole exactly when `5 * half <= rx`. A fifth satisfies that; the first
  attempt at a quarter put the head into the diamond's corner and the test caught it - **"a square's rows are constant: [17, 21, 25, 25, ...]"**,
  the top rows clipped by the tile.
- **The health bar moves above the head**: it was placed at the ring's radius, which is now at the feet.

## Tests that had to learn the new anchor, and why that is honest

- `shapes.rs` centred its scan on the **tile point** while the unit stood at a tile *corner*; with the body no longer straddling the point, the window
  read forty zero rows. Fixed to scan from `drawn_at(entity.position)` - the D168 lesson, again.
- The same test asserted *"a triangle's rows grow"*, which was only true because the window used to be wider than the body and sampled its lower half.
  Now the whole body is sampled and the rows **narrow towards the apex**. The assertion was rewritten with the reason, not deleted.
- `unit_centring` measured the body's **centroid** against the diamond centre - the D168 invariant. The invariant that survives is *the base is on
  the point*, so it now compares the **lowest painted row**, and the horizontal centroid, which still catches a half-tile shift.

## What the owner sees

The villager stands **on** the cross rather than above it, and the white ellipse is round its feet - which is the picture he described wanting
("align the center of the ellipse with the bottom side of the square/triangle") before there was a name for the defect.

## Performance

**Measured.** The whole gate ran in **415,950 ms** (`gate100.log`), and the view's own tests in **33.04 s** for all 128 of them; the new
`feet_on_the_point` is **0.89 s** and the new `move_arrival` **0.82 s**. Drawing is `2 * reach + 1` rows by `2 * reach + 1` columns and `reach` shrank
from `0.30 * rx` to `0.20 * rx` - 19 px instead of 29 - so the per-body pixel loop is **2.3x smaller** than before (1,521 candidate pixels against
3,481), while the body is taller on screen. Acceptance is **byte-identical** - `661e970e58bc0c57`, 138 entities, 529 repaths - because nothing in the
simulation moved.

## Stability

- **Two pixel tests pin the anchor**: the base sits on the point (`feet_on_the_point`, ±2 px), and a settled unit's body no longer lands half a tile
  from its tile (`unit_centring`, now row-based).
- The tile clip still holds: at `5 * half <= rx` the body is inside its own diamond without being cut.
- Clippy caught two leftovers in the new test file (an unused import and an unused variable) **after** the first gate run - recorded rather than
  quietly fixed: the gate log shows `CLIPPY=101` for that run, and a second clippy run reports `CLIPPY_RC=0`.
- Suite: **784 passed / 0 failed / 20 ignored**, clippy `-D warnings` clean, fmt clean, `review_check` 0.
