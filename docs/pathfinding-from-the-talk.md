# The short-range pathfinder, from the horse's mouth

Source: **Raymi Klingers (Engineering Director, Forgotten Empires), "25+ Years of Pathfinding Problems with C++"**, Meeting C++
2025 — slides at `meetingcpp.com/mcpp/slides/2025/25+ Years of Pathfinding Problems442903.pdf` (104 pages, no licence
restriction on reading; nothing here is copied code, it is the algorithm their team documented publicly). Found through
Hermes1's research thread in this server, which also established that **no AoE2 source is public** and that the decompilation
repo (AoC 1.0c, all rights reserved) must be read for behaviour, never copied — so a first-party talk is the right source.

The talk's own summary of the *original* algorithm (pages 21–38):

1. Find the obstructions between A and B. **Units and buildings are axis-aligned rectangles in AoE.**
2. Shrink A to a point and expand every other obstruction by A's radius (the Minkowski trick).
3. Build convex hulls of the rectangles by gift wrapping.
4. Are A and B in the same hull? If not, **shoot cardinal rays to escape**; on hitting an obstruction, shoot sideways; on
   hitting a hull, walk its edge.
5. At every vertex, shoot a ray at the goal.
6. Repeat 4–5 until the goal is reachable, up to **64 iterations** or until there are no unique vertices left.
7. Brute-force smooth the result into the final path.

And then what they replaced it with, which is the part worth copying (pages 82–96) — the **new way**:

1. Shrink ourselves to a point, expand the obstructions.
2. **Shoot a ray toward the goal.** Hit something → the hit point is where the edge walk starts; hit nothing → the path *is*
   the straight line.
3. Walk edges: **turn right at the end of an edge, turn left at an intersection**, walking the union of overlapping rectangles
   (merging rectangles does not need a full concave hull).
4. At every vertex, go back to step 2 — ray at the goal, walk again if it hits.
5. Brute-force smooth.

Their reasons, which are exactly our constraints:

- **It needs only `> < == + -`.** No multiplication in the walk, so it can run in fixed point. That is the same reason this
  project keeps `f64` out of the tick — `ruage-fixed` is already the right arithmetic.
- **Precision is what breaks it**, not logic: nearly-colinear points, SIMD ruining 80-bit x87 extended precision, and 64-bit
  ports changing rounding. Their "fix" was to stay in operations that cannot lose precision.
- **Bounds**: "whoops, we edge-walked the whole map" → add rectangles around the search area; and **count turns** (four more
  rights = still in the open, four more lefts = in a hole) to give the walk a cheap terminal condition.
- Results on their side: average 2× faster, old worst case 40×, new worst case 10% faster than the original.

## What this means for us, concretely

Our problem is the same one, one layer down: `plan_path` is an A* over tiles, and `step_towards` walks toward tile centres, so
a unit's drawn path is a staircase of 45°/cardinal segments (the owner has now asked three times; the third was a screenshot
with a red straight line against a line of blue squares).

The mapping is unusually clean:

| the talk | this repository |
| --- | --- |
| axis-aligned rectangle per unit/building | `World::solid_tiles` / `footprint_area` already produce one |
| unit radius | `World::body_radius` |
| fixed point, `> < == + -` only | `ruage-fixed`; `integer_sqrt` and `round_div` in `sim` |
| long-range path | `plan_path` (A* over the tile grid) — stays, it is the far-away route |
| short-range path | **missing** — this is what the talk describes |

So the work is one new module, not a rewrite:

1. `short_range(from, to, radius)` → a polyline: expand the rectangles of everything near the segment (they are already
   axis-aligned, `solid_tiles` gives the tiles), ray-cast to the goal, and if it hits, walk the union's edge and re-ray at each
   vertex, exactly steps 2–4 above. Fixed point throughout.
2. `plan_path` keeps producing tile waypoints for the *long* route; the unit requests a short-range path to the next waypoint
   (or to the final goal when it is close), so the drawn line is straight with corners rounded by real geometry.
3. `step_towards` then moves along a polyline of points, not tile centres — and `reaches` (already in) ends a leg by radius.
4. The bounce rule the talk states for AoE2 (page 9): **"units bump into others, stop and repath around. No pushing allowed."**
   That is a decision for the movement pass: our `push_units_off` is the opposite behaviour, and the owner has not been asked.

Bounds: a rectangle around `from`/`to` plus four turns of slack, and a hard iteration cap (the talk uses 64).

## Why this is a whole step and not a patch

The three failures of my proportional-step attempt all come from moving the *step* without the *path*: a villager stopped dead
at the start of a walk because the step and `collide` disagree about fractional positions, and arrival was an equality until
it was a radius. This algorithm replaces the thing that was wrong — "walk toward the next tile centre" — instead of bending it.
D109 records that, and this document is the design the next session starts from.
