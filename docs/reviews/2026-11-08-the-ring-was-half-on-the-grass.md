# The ring was half on the grass (D179)

*2026-11-08. The owner, after the body was anchored by its base: **"the circle is supposed to be at the bottom, not waist. The unit is above the
circle."** D178 had put the body's feet on the point, but left the ellipse centred on the same point - so half of it lay on the **grass below** the
unit, which reads as a circle the unit is standing behind instead of a circle round its feet.*

## The measurement

`crates/view/tests/ring_at_the_feet.rs` selects a villager and asks where the ring's **lowest** row is, against the unit's base row. Failing first:

```
the ring's lowest row is 379 and the unit's feet are at 374: the ellipse is 5 px below the feet
```

Five pixels of ellipse on the grass - a quarter of the ring's own 9.6 px half-height, and enough that the whole white oval reads as *under* the body.

## The change

One term: the ring's centre is the point **raised by its own half-height**, so its lowest row is the base and the ellipse embraces the lowest 19 px of
the body. The body and the ring agree about where "the bottom" is.

## The test that had to move with it

`ring.rs`'s *"the ring is a curve and not four straight edges"* samples the ellipse halfway down to tell an ellipse (86% of its half-width there) from a
projected diamond (50%). It computed that point from the **body's** centre, which is no longer the ring's. It now raises its own centre by the same
half-height, with the reason written in the test.

## Performance

**Measured**: the whole gate ran in **421,167 ms** (`gate101.log`), the view's 130 tests in **33.6 s**, the new `ring_at_the_feet` in **0.8 s**. The
change is one subtraction per selected unit per frame - the ellipse's parameter set is unchanged, so `outline_ellipse` does exactly the work it did.
Acceptance is **byte-identical** (`661e970e58bc0c57`, 138 entities, 529 repaths): nothing in the simulation moved.

## Stability

- **The ring's relation to the body is a test now**, in pixels, not a comment: lowest ring row == base row, ±2 px.
- The ring's size is unchanged, so it still never reaches a neighbouring tile and the "ellipse inside the building diamond" complaint of D176 cannot
  return by this route.
- Suite: **785 passed / 0 failed / 20 ignored**, clippy `-D warnings` clean, fmt clean, `review_check` 0.
