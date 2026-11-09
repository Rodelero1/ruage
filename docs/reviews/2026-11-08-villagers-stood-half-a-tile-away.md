# Villagers stood half a tile from their work (D169)

*2026-11-08. The owner, with two screenshots of the original: *"villagers are standing around 1/2 tile away from what they are doing. In the game they
are much closer. Can you research from the game?"*

## What the data says

`config/gamedata/units.toml`, straight from the dat, `radius[0]` - a **half-extent in tiles**:

| Unit | `radius[0]` |
|---|---|
| `VMBAS` (villager) | **0.2** |
| `FOAK` / `FPIN` / `FPAL` (forest trees) | **0.5** |
| `FORAG` (forage bush) | **0.5** |
| `STONM` (stone mine) | **0.5** |

Two discs touch when their centres are `0.2 + 0.5 = **0.7**` of a tile apart. That is the original's standoff, and it is the number the engine
already used for "is this worker at the node" (`reach_raw` = the sum of the radii). What the engine did **not** use it for was **where the walk
ends**: a route's last waypoint was the node's own tile, the pathfinder snapped that back to the neighbouring tile's centre, and the worker stopped
a whole tile from the node. **One tile against 0.7 is the half tile the owner sees** - and the survey of the original's screenshots agrees: a
gatherer stands with its body against the resource, not a stride away from it.

No web source settles this; the number is in the pack, and the pack came from the dat.

## The failing test, watched failing first

`crates/sim/tests/gather_standoff.rs` puts a villager two tiles west of a tree, orders a gather, runs ten seconds and measures the gap:

```
the woodcutter stopped 1.02 tiles from the tree's centre; the two bodies touch at 0.70 - it is standing at the
neighbouring tile's centre instead of at its work
```

After the change the same measurement reads **0.70**, and the tree has lost hit points, so the closer stand is the stand it works from.

## The change, and the two shapes it is **not**

1. **A route that ends 0.7 of a tile out does not finish.** The first attempt made the path's last waypoint the standoff point. A route the
   movement rules never call *arrived* is a worker that never works: thirteen tests went red, including *"a tree is still standing ten seconds in"*
   and a lumberjack still standing at a spent tree. Reverted.
2. **A worker that walks instead of working deadlocks diagonally.** The second attempt returned a *step* toward the node in place of the chop.
   Where two discs touch diagonally is `(0.5, 0.5)` of a tile - inside the node's own blocked tile - so the step is refused and the worker neither
   moves nor works. Reverted.
3. **What shipped: a settle.** On the tick a worker that has finished its walk is at the node, it is placed on the standoff point - once, on the
   axis it came from, and only when that point is free ground. `settle_workers` runs right after the movement pass. It cannot deadlock, because
   it is not a route and not a step.

The node's radius is **content**: `resources.toml` now gives each resource `radius = 0.5` (the dat's `radius_x` for the unit the node stands for),
and `World::load_resources` resolves it into engine units **once**, so no tick reads a float - the `no_float` guard caught the first version and
`load_resources` is now named in its `LOAD_TIME` list with the reason.

Buildings and farms are untouched: their `reach` is at or above a tile, so 0.7 would move nothing, and the settle is gated on a **resource node**.

## Performance

The settle is one comparison per working gatherer per tick, and one placement per arrival - no path search, no extra pass. **Measured**: the
`gather_standoff` suite runs in **0.86 s**; the gate's wall clock is recorded with this commit. Acceptance hash changes, as it must: positions
moved. The new hash is in `SPEC.md` and in the commit message.

## Stability

- **The deadlock is the story.** Two designs that looked simpler both stopped the economy dead, and neither failed loudly: one left a tree
  untouched for ten seconds, the other left a lumberjack standing at a spent tree. Both were caught by tests that already existed, which is the
  argument for having them.
- **A placement is a new kind of write.** It is one call, after the movement pass, gated on free ground, and it does not touch velocity or the
  route - so a settling worker cannot fight the movement rules for the rest of the match.
- Suite: **772 passed / 0 failed / 19 ignored**, clippy `-D warnings` clean, fmt clean.
- **Acceptance, measured**: hash `5703f537f92c0bf2`, **137 entities**, players=2, **144 repaths**, **132 made-room** - against
  `a120ed0721c8768c`, 150 entities, 88 repaths, 100 made-room. The hash *had* to move: positions moved.

## The trade-off, recorded rather than papered over

**Workers now crowd.** Thirteen fewer units in thirty minutes and repaths up from 88 to 144 is not a free win: two villagers sent at the same tree
settle on the same point - the side they approached from - and then push each other around for the match. The fix is to **spread them round the
node** (try the approach axis first, then the opposite, then the two across, and skip a point another worker already holds), which is what the
original does. It is written down here as the next step, not implemented in this one: it needs its own test ("two woodcutters on one tree stand on
different sides"), and a change to the settle rule should not ride in on the back of this one.
