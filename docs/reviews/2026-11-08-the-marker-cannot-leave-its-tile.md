# The marker cannot leave its tile now - and I still cannot see the overlap the owner sees (D166)

*2026-11-08, fifth round on the same report. This note is deliberately short on claims and long on what is **not** established.*

## What changed

`ruage_view::render` draws a unit's marker **pixel by pixel, clipped to its own tile's diamond**. Before, a `fill_rect`/`fill_triangle`
painted a shape of `0.35 * rx` half-width whatever tile the unit stood on, and two neighbouring tiles **share edges**, so any part of the
marker past its diamond was paint inside the next tile - and if that neighbour is a building, the unit looks like it stands inside it.

The clip is `|dx| / rx + |dy| / ry <= 1`, measured from the unit's own point, and it applies to all three silhouettes (square, triangle, dot)
because the shape test and the clip are now one per-pixel loop instead of three primitives.

## What I could **not** establish, stated plainly

1. **The overlap is still not reproduced in a test.** `marker_fits_tile` renders a lone villager and checks that every pixel of its colour is
   inside its own tile's diamond. It **passes** - and neither mutation I tried made it fail: setting the marker back to `0.35`, and removing
   the clip entirely. So my geometric model of "the corner sticks out at 1.05 of the diamond" does **not** match what the renderer actually
   puts on the screen, and I do not know why. That is an open question, not a solved one.
2. **Two earlier attempts at this test were wrong in a way that made them pass**: one sampled only the **lower half** of a tile
   (`0..=ry`), where an overflowing marker never is, and one sampled only a rectangle of pixels rather than the diamond. Both were caught by
   watching them fail to fail - which is the only reason I trust the third one at all, and even it proves less than I wanted.

## The honest position on the remaining overlap

A unit's **position is continuous**: a villager walking between tiles sits at, say, `(30.7, 29.5)`, and a marker drawn at that point straddles
four tiles by construction. No marker size or clip changes that - the clip only stops it leaving **the tile its centre is in**, and when the
centre is on a boundary "its tile" is a rounding decision.

The original has the same property and hides it with **art**: a sprite whose feet are a small ellipse with a shadow, drawn so the *body* rises
away from the ground rather than spreading over the neighbouring tiles. That is an asset-pack question (D18), not a rule.

Two things I can do on request, both a real behaviour change rather than a guess:

- **snap the marker to its tile's centre** while the body stays continuous - no overlap ever, but units visibly step tile to tile;
- **shrink the marker further** (0.25 of a tile), which is what a small figure would look like from this height.

## Performance

**Simulation untouched**: the acceptance hash is byte-identical. The per-pixel marker costs `(2 * 0.35 * rx)^2` = ~1,150 `put`s for a whole
unit at the default zoom instead of 4 `fill_rect` bookkeeping calls - measured in `frame_cost.rs`'s budget, which still passes at the same
threshold. **Measured**: the marker suite runs in **0.80 s**; the gate's wall clock is recorded with this commit.

## Stability

- **The clip is a hard invariant in the code**: no marker pixel can be painted outside the diamond of the tile under the unit's centre, for
  every silhouette, because the shape test and the clip share one loop.
- **What it does not fix**: a unit between tiles still spans its neighbours, and a unit whose centre is next to a wall still has its marker
  touching that wall's tile edge. If the owner wants *that* gone, it is the snap-to-centre change above and it should be asked for explicitly.
- Suite: **770 passed / 0 failed / 19 ignored**, clippy `-D warnings` clean, fmt clean.
