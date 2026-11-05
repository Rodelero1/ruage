# Review: the minimap had seams, and they were the crossing lines

*The owner, 2026-11-05: "Minimap has 2 crossing lines artifact", with a picture of the minimap. Two straight lines ran across
the diamond like an X.*

## What it was

The minimap was painted **one diamond per tile** (`fill_diamond(point, step, colour)`), with `step` derived from the distance
between two neighbouring tile centres. On a 64-tile map in a 230-pixel box that distance rounds to a non-integer number of
pixels, so neighbouring tile centres land one or three pixels apart, and the diamonds leave **one-pixel gaps** between them.
Those gaps are the panel's own darkground showing through, and because the tile grid is diagonal in this projection the gaps run
along its **two diagonal directions** - which is exactly the pair of crossing lines the owner saw.

**The measurement that proves it, and that I should have been able to make from the picture:** `minimap_seams.rs` counts
panel-coloured pixels *inside* the map diamond for a fully-explored map. Before the fix: **182 of them**. After: **0**.

## The fix, and the wrong ones before it

**The cause, exactly:** the minimap painted a diamond per tile with `ry = step / 2`. On a 64-tile map in a ~230-pixel box the
step between tile centres is **1.82 px**, so `ry = 0.91` - below one pixel - and `fill_diamond` then draws **a single row** per
tile. Tile rows are 1.82 px apart, so roughly every other row was painted: the missing rows are the gaps, and they run along
both diagonals of the grid, which is the pair of crossing lines the owner saw.

**What landed:** `ry` is floored at **1.5 px** (`rx` keeps the full step, as it always did). Nothing else changes: for any
minimap whose step is 3 px or more the radii are identical, so the shipped map draws exactly the work it drew before.

**Two wrong fixes, kept in the record because they were measured:**
1. A **per-pixel fill** - ask every pixel which tile it belongs to, so no seam is possible by construction. Correct, and the
   test passes either way, but it measured **4.4 ms of HUD band against 2.0 ms** in `frame_cost`: `minimap_tile` does a float
   inverse per pixel, 29k times.
2. **Overlapping every diamond by half a pixel** - closed the gap and cost **+1.5 ms**: on a small minimap a larger diamond
   covers a third more pixels, and there are 4,096 of them.

## Performance

**The number this slice is measured by is the seam count**: **182** panel-coloured pixels inside the map diamond before, **0**
after, on a fully-explored 64x64 map - and 0 in a second test that reproduces the owner's own geometry (a 64-tile map in a
231-pixel box, where the tile step is under two pixels).

No timing claim is made, and here is why: for a minimap whose step is 3 px or more the radii are **identical** to what they
were, so the draw count and the pixel count are unchanged - there is nothing added to measure. On the smaller box the floor adds
at most two pixels of height per tile. `frame_cost` estimates the HUD band as the **difference** of two ~10 ms timings, and on
identical binaries it swung between 746 us and 1,847 us in three runs, so it cannot resolve a change this small. Quoting a
percentage from it would be quoting noise, which this project has already been burned by twice.

## Stability

| attack | result |
|---|---|
| a gap between two tiles | 0 of 182 on a 64-tile map (`the_minimap_has_no_gaps_between_its_tiles`), and 0 in the owner's own 231-pixel box (`a_small_minimap_box_has_no_gaps_either`) |
| a pixel outside the map diamond | `minimap_tile` answers `None` and the panel shows through, which is the diamond's shape |
| a box smaller than the map | the same per-pixel mapping, so tiles are sampled rather than skipped |
| the box's border | still drawn after the fill, one pixel outside it, so the map cannot spill onto the panel |
| fog | read per pixel from the simulation's own vision answer, unchanged |
| the minimap's pick | untouched: `minimap_tile` is the same inverse the click already used, so what is drawn and what is picked cannot drift apart |

**A wrong turn of mine, reported:** before finding the seams I spent three attempts arguing the lines must be the **camera view
box** - first as a corner-order bug, then as a clipping bug - and each was ruled out by rendering the HUD and counting the
bright pixels (there were none inside the diamond; the only white was the box's own top and bottom edges). The lesson is the
same one this week keeps teaching: **look at what the picture actually contains** before theorising about what drew it. Counting
pixels in the owner's own screenshot is what finally identified the colour of the lines.

---

*`tools/review_check.py` looks for `## Performance` and `## Stability`; both are above.*
