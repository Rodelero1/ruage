# Workers fan out round a node - and the reason they could not was a half tile (D171/D172)

*2026-11-08. The owner answered **A** to "spread workers round the node, or accept the crowding". The spread turned out to be two fixes, because it
walked straight into a framing bug underneath.*

## D171 - the spread

`crates/sim/tests/gather_spread.rs`: one tree, all four sides free, **two villagers sent from the same side**. Failing first:

```
both woodcutters settled on the same spot: they are 0.00 tiles apart, and one of them should have taken another side of the tree
```

Zero, exactly - both stood on the identical raw coordinate. That is also what the acceptance run was showing as 13 lost units and doubled repaths.

`standoff_point` now tries the **four sides** of the node - the axis points at the touching distance - sorted by how well each faces the worker's
approach, taking the first that is **free ground, not held by another body, and reachable from where the worker stands.** The approach side still
wins when it is free, so a lone worker is unchanged (D169's test still measures 0.70).

## D172 - and the four sides were not symmetric, because a node stood on a tile's **corner**

The spread exposed it: after it landed, the ten-minute scenario failed with

```
the step due at 330 s did not run: Refused("no walkable route from (22, 13) to (13, 19): the target is walled off")
```

and a woodcutter was found standing at `(22.00, 13.30)` - a fractional position in a **blocked** tile. The cause: `seed` placed every map node at
`self.tile((x, y))`, which is the tile's **corner** - the anchor a building's footprint is placed from. A body whose radius is half a tile, centred
on a corner, spills a quarter of itself into each of four tiles. So the contact point 0.7 from a node was in a *free* tile on two sides and inside
the node's own blocked tile on the other two: woodcutters could approach a tree from the north and west and not from the south and east.

`seed` now places nodes at `tile_centre`, the same convention D114 gave walk goals and D168 gave every body. That is one line, and it is the same
half-tile mistake in a third place.

## Measured, acceptance

| | D157 baseline | D169 (crowding) | D171 (spread) | **D171+D172** |
|---|---|---|---|---|
| entities | 150 | 137 | 145 | **153** |
| repaths | 88 | 144 | 163 | **24** |
| made-room | 100 | 132 | 156 | **16** |
| hash | `a120ed0721c8768c` | `5703f537f92c0bf2` | `e61c691aa58dde82` | **`6d23d672eb18ef40`** |

**Repaths fell by 73% against the baseline and made-room by 84%.** The crowding never was a gather-rule problem: it was bodies standing on the wrong
half of a tile and pushing each other off it. Fewer, better-placed shoves also put **three more units** in the field than the pre-D169 run.

The match no longer loses player 2 (`players=2`), which the intermediate D171-only build did - so that regression was the corner bug too.

## Fixtures corrected, and why that is not cheating

Seven tests located a node by its **corner position** (`seeding`, `gather_knowledge`, `terrain`, `ui`). They now locate it by its **tile**, or place
their fixture at `x + 0.5`, with the reason in the comment. A fixture at a corner is a node no map can produce; the *tile* is what the map names, and
the centre is how a body occupies it. Two engine sites also compared a raw position to a tile corner and had to learn the same rule: the rally
resolution in `train.rs` (a rally onto a resource had silently become a plain walk) and the view's resource pass, which now draws a node with the
body conversion.

## Performance

The settle is four candidate points per arrival, each a tile lookup, an exact-coordinate body scan and - only when a candidate survives - one
`can_reach` path search. **Measured**: `gather_spread` 0.83 s, `gather_standoff` 0.86 s; the acceptance run's repath and made-room counters *fell*,
so the path work the settle adds is more than paid for by the shoving it removes.

## Stability

- **The reachability check is load-bearing.** Free ground is not reachable ground: without `can_reach`, a settle point inside a forest pocket trapped a
  woodcutter permanently.
- **Seven tests had to change, and each one changed for a stated reason** - the tile is the map's fact, the centre is the body's. None of them was
  loosened to make a number pass.
- Suite: **774 passed / 0 failed / 19 ignored**, clippy `-D warnings` clean, fmt clean, `review_check` 0. Gate 423,291 ms.
