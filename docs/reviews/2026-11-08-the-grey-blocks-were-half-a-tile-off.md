# The grey blocks were half a tile to the right (D160)

*2026-11-08, after D159. The owner: *"Gray fields representing buildings also need to be moved."* He was looking at a real offset: the
ground draws a tile's diamond on the integer point `(x, y)`, and the new per-tile building pass was drawing at `(x + 0.5, y + 0.5)` -
**half a tile down and to the right of the ground it owns**.

## What was wrong

```rust
// terrain, unchanged since the renderer was written
let centre = camera.project((x as f32, y as f32));
frame.fill_diamond(centre, (rx, ry), colour);

// the building pass I added in D159
let at = camera.project((tx as f32 + 0.5, ty as f32 + 0.5));   // <- half a tile off
```

The `+ 0.5` came from thinking in *tile centres*, which is right for a continuous body position and wrong for a diamond grid: a tile is a
diamond **centred on its integer point**, so neighbouring tiles meet edge to edge and a field of them covers the screen with no seams
(the same reason `fill_diamond` exists at all). A building drawn at the tile's continuous centre therefore sits half a diamond
down-right, covering four tiles' quarters instead of its own - which is exactly "the grey fields need to be moved".

## The fix

One line: draw at `camera.project((tx as f32, ty as f32))`, the same point the terrain pass uses, so a building tile and its ground
tile are the same diamond.

## The test that can see it - after one that could not

The D159 test samples a pixel that belongs to a tile, and its **first** version could not see a half-tile shift at all: it sampled the
tile's point moved 55% down, and a diamond shifted half a tile still covers that pixel. That is the trap of pixel tests with a
tolerance: they pass for a picture that is visibly wrong.

The sampling moved to a point **45% of the way to the upper-left vertex** - inside the tile (0.9 of the way to its edge) and outside a
diamond shifted down-right. That version was then **proved by mutation**: restoring `+ 0.5` fails with

```
the house's own tile (11, 10) is painted as the house
  left: [84, 116, 56, 255]   (grass, one quarter of a diamond up-left)
 right: [200, 200, 200, 255]
```

`footprint.rs`'s per-tile test needed the same sampling change, for the same reason - it had inherited the tile-centre convention.

## Performance

**One arithmetic change per painted tile**: `(x + 0.5)` became `x`; no extra work, no extra calls. The simulation is untouched and the
headless acceptance hash is **byte-identical**. **Measured**: `drawn_house_size` 0.84 s with the fix (0.80 s before it - the same test,
the difference is the run), `footprint.rs` 1.60 s, and the whole gate is recorded with this commit.

## Stability

- **One convention, written down**: *a tile is the diamond centred on its integer point.* The renderer now has exactly two projections
  into pixels - `project(tile)` for anything that occupies a tile, `project(position)` for a body between tiles - and both are used
  where they belong.
- **The offset could not crash and could not fail a headless test**: it was a purely visual error, which is why it needed a pixel test
  with a sample point chosen to be *sensitive* to the mistake, not merely inside the shape.
- **A test that passes for the wrong picture is worse than no test**, and this step had one for exactly one commit. Recorded here
  rather than quietly fixed.
- Suite: **762 passed / 0 failed / 19 ignored**, clippy `-D warnings` clean, fmt clean.
