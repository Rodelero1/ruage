# Both orderings tried, both reported: the geometry is fine and the art is missing (D167)

*2026-11-08, sixth round. The owner: *"issue is still there, diamond is over the passing unit and the unit can get below it when approaching from the
top and cannot reach it when approaching it from the bottom."*

## The measurement, again, and it is the third time it comes out symmetric

A house at `(40,40)`, a villager ordered to the tile **touching** its north wall and another to the tile **touching** its south wall:

```
unit 9981: goal (40,39) reached tile (40,39)  pos (40.50, 39.50)
unit 9982: goal (40,42) reached tile (40,42)  pos (40.50, 42.50)
```

Both arrive **exactly on their goal tile's centre**, one tile from the wall, north and south alike. There is no reachability asymmetry to fix: the
engine lets a unit stand beside a building on any side, and it stops at the tile boundary on both.

The building's paint is also exactly its blocked tiles (`the_drawn_footprint_is_the_blocked_footprint`, `built_house_alignment`).

## So the two orders have now both been tried and both reported

| Ordering | Owner's report |
|---|---|
| units painted **last** (2026-11-04 rule) | *"the villager can pass through the built house"* - a marker overflowed its tile and painted over the wall |
| depth decides (D165) | *"units are below the dark gray"* - a passing unit is hidden under the building's diamond |
| units last again, **marker clipped to its tile** (D166) | this commit |

With the clip in place, "units last" no longer means "units inside walls": a marker cannot paint outside the tile its centre is in. What remains is
a **flat-shape renderer drawing a 0.4-tile square** where the original draws an object with height. A building's grey reaching edge to edge and a
unit's square reaching edge to edge will always meet at the shared edge, and whichever is painted second looks "on top".

## What I did

`layer_of` puts units above buildings again (the owner's 2026-11-04 rule), with the clip from D166 keeping that from meaning anything is painted
inside a wall. Two tests went back to their pre-D165 assertions, and their comments now record **all three** states of this decision rather than only
the last one.

## What would actually end this, and it is not a rule

- **Art with height**: a sprite whose feet are a small ellipse with a shadow, drawn so the body rises away from the ground. That is the original's
  answer and it needs the asset pack (D18), not more ordering rules.
- Until then, the honest options are a **dark outline** around a unit's marker so it reads as in front even over grey, or **snap the marker to its
  tile's centre** so it can never straddle a neighbour. Both are one-line-ish and both change how the game looks, so both are the owner's call.

## Performance

**Simulation untouched**: acceptance hash byte-identical. The layer restore is a match arm; the clip is per pixel and was measured in D166.
**Measured**: the depth suite runs in **0.76 s**; the gate's wall clock is recorded with this commit.

## Stability

- **Six rounds, one lesson**: I have been fixing the picture for a report that is *about the picture*, while the underlying difference is that this
  renderer has no sprites. Rules cannot substitute for height, and each rule I add makes the next report more confusing.
- **What is proven and stays proven**: collision blocks the footprint, the drawn footprint equals the blocked one, approaches are symmetric, a marker
  cannot leave its tile, and the simulation hash is untouched by all of it.
- Suite: **770 passed / 0 failed / 19 ignored**, clippy `-D warnings` clean, fmt clean.
