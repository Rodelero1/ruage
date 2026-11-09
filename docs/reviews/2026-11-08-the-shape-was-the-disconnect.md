# The window was not lying, exactly - the *shape* was (D159)

*2026-11-08. The owner's hypothesis: *"Then there must be disconnect between headless and gui run."* He was right about the effect and
wrong about the mechanism - and the mechanism is worse than a disconnect, because it is a **shape the engine never claimed**.

## What the engine says, and what the picture said

The walk grid blocks **every tile of a building's footprint** - a house is 2×2 and all four tiles are solid (D152's tests, D158's
tests). The renderer drew a building as **one diamond** of `across × rx` width centred on the block:

```
   tile diamonds of a 2x2 block          the one diamond that was drawn
        /\      /\                          /\
      /    \  /    \                      /    \
      \    /  \    /                     /      \      <- the four corner tiles'
       \/      \/                       /        \        outer halves are NOT
                                        \        /         inside this diamond
                                         \      /
                                          \/
```

The union of four tile diamonds has **four corners a single diamond does not reach**. And it is not only a missing corner: the
diamond's own vertices stick out into tiles it does not own. So for every multi-tile building the picture painted ground the
collision refused, and refused ground the collision allowed - a villager walking past a **corner tile it could not stand on**
read as a villager walking over the house, and a villager on the *drawn* edge was read as "inside" while it was legally beside it.

That is the whole of "headless and gui disagree": one world, two geometries.

## The fix

`render` draws a building **one diamond per footprint tile**, at each tile's centre. The silhouette is now literally the set the
walk grid blocks - the only thing a tile renderer can promise - and the role mark stays at the block's centre.

Cost: a 4×4 town centre is 16 `fill_diamond` calls instead of 1; each call clips to the frame exactly as before, so the frame
cost test (`frame_cost.rs`) still holds its budget at the same zoom. For the whole acceptance scene that is ~150 buildings ×
their footprints, and only the visible ones reach the frame at all.

## The test that settles it, and the mutation that proves the test

`crates/view/tests/drawn_house_size.rs`:

- every tile of a house's footprint has **the building's colour at its centre**;
- every tile around it does **not** - so a villager on a neighbour tile can never be painted inside the house;
- and the test was **proved by mutation**: restoring the single big diamond makes it fail with
  *"the house's own tile (12, 11) is painted as the house"* (the pixel there was grass). That failure is the bug in one line.

Two existing tests had encoded the old shape and were rewritten honestly rather than deleted:

- `a_building_is_drawn_as_the_diamond_its_footprint_covers` → `…_as_the_tiles_its_footprint_covers`: it asserted the top vertex
  was `4 × ry` above the centre (71 px measured against 96 expected, because a stepped top edge is lower than a pointed one). It now
  asserts per-tile centres, the empty ring, and that the bounding-box corner is still not painted.
- `a_remembered_building_is_drawn_dimmer_than_a_watched_one` sampled the pixel at `(20.0, 20.0)` - the point where **four**
  diamonds meet, which belongs to none of them. It samples the tile centre now.

## Performance

**The simulation is untouched**: the headless acceptance hash is **byte-identical**, and the change is in `ruage_view::render` only.
**Measured**: the view suite (20 test binaries) runs in the same order as before, with `footprint.rs` at **2.96 s** and the new
`drawn_house_size` at **1.52 s**, and the whole gate **404,379 ms** of test wall clock for **762 tests**. The per-frame cost is
`footprint area` diamond fills per building instead of one - 16 for a town centre, 4 for a house, 1 for a wall - and `fill_diamond`
clips every scanline to the frame, so the work is proportional to the painted pixels, which is what the old single diamond also
cost for the same area.

## Stability

- **One geometry, one source**: the silhouette comes from `footprint_tiles`, the same function the click-picking code uses, so
  what a player sees, clicks and collides with is one rectangle.
- **The failure mode this removes is silent**: a shape mismatch cannot crash, cannot fail a headless test, and only shows as "the
  game is wrong" in a screenshot. It is now a pixel test with a named opposite.
- **Units are still painted last** and can overlap a building's edge; that is deliberate (a garrisoned villager has to be visible)
  and is why the ring check skips tiles a unit is standing on.
- Suite: **762 passed / 0 failed / 19 ignored**, clippy `-D warnings` clean, fmt clean.
