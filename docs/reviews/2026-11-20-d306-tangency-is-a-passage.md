# Review: tangency is a passage, and the radius reaches the searches (D306)

*The owner measured it in HD on 2026-11-20: a **packed trebuchet walks a one-tile hole** in a wall, and a **one-tile corridor in bamboo**, and
the scenario editor even places a packed trebuchet inside one. My inference from the binary - "0.5 radius in a 1.0 gap leaves zero clearance,
and the box ruler collides on `<=`, so no passage" - was **wrong**, and the game is the authority.*

## What the screenshots change

**Tangency is a passage.** A body 1.0 tile wide in a 1.0-tile gap has *zero* clearance and still walks it. That is consistent with the step,
where the depenetration is computed and **never applied** (D290): the game tolerates a body that touches on both sides, so its planner must
be willing to route it. So `fits` is `nearest - reach >= 0`, not `> 0`.

## What landed

- `Ground::fits(x, y, scale, reach)` - the nearest blocked square's distance minus the reach, tangency allowed;
- `search_reporting(..., reach, ...)` asks `fits` for the node and both diagonal side tiles, so the **flat** search is radius-aware;
- `Levels::for_radius(ground, reach)` and `Ground::pyramid_for(reach)`, the cache keyed by reach - one build per radius per ground change;
- tier 2 of `route_with` now runs for **any** reach (the `reach == 0` gate that D305 found is gone), certifies its first and last legs with the
  **walker's** reach, and is gated on `avoid.is_empty()`, `coarse.level >= 2` **and** a walk of at least **16 tiles**;
- D305's finding is therefore closed: the hierarchy does run for units now.

**The distance gate is a deviation and is labelled one.** The binary refines a coarse edge to a **tile on the boundary**
(`FUN_007eeda0`, crosscheck #5), so its short walks get fine answers from the hierarchy. Ours emits cell centres, so a short walk gets a
staircase - measured, 31 points past a single tree. Sixteen tiles (two level-3 cells) is where the flat search's corners stop being the better
answer. When #5 lands, this gate should go.

## Test-first

The witness flipped with the game's answer: `radius_route::both_bodies_take_the_one_tile_gap` asserts **both** a 0.20 and a 0.50 radius body
reach the goal through the gap. It passed on the first run after the change - which is the weaker kind of evidence, and is recorded as such:
the earlier failing version of the same scene (`a radius-0.50 body must NOT be planned through a gap it cannot fit`) is what proves the
fixture discriminates, and it is in the git history.

## Performance

- `coarse_route` **5 passed / 0.01 s**, `radius_route` **1**, `route_shape` **1**, `straight_line` **4 passed / 1.71 s**, `walk_line` **2** -
  all green together, which is what the earlier attempt did *not* manage;
- `fits` costs up to nine blocked lookups per expanded node instead of one; on the probe's 64×65 regime that is ~hundreds of ns per node, and
  the pyramid is built per radius (~21 µs each, once per ground change);
- full workspace suite: **920 passed / 6 failed / 21 ignored**, and the red set is **identical** to before the change (`diff` of the sorted
  names prints nothing);
- clippy clean at `-D warnings`, workspace and all targets.

## Stability

**Three gates now protect the tier**: bodies (`avoid.is_empty()`), granularity (`coarse.level >= 2`) and distance (≥ 16 tiles). Each was added
because a fixture proved the tier was answering where it should not, and each is named in the code with its measurement.

**The risk this change carries** is that a radius-aware search can now refuse a tile the *line tier* would certify, which is the same class of
mismatch that reddened `straight_line` before - now resolved in the other direction (tangency allowed on both sides). If a future fixture shows
a route whose first leg the line tier allowed and the search then cannot use, the two predicates are drifting again.
