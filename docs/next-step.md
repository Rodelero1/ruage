# OPEN - the tier is a stand-in for an unread driver: read the high-level path's builder next

**Where this stands:** suite **225 result lines, 943 passed, 0 failed, 23 ignored**; conformance **21 / 0 / 0**; acceptance
`7782dd38d7557514`, **134 entities**, one player alive at 48,000, 37,204 re-paths, **12.12 s** median of three. The order to
100% conformance is **`docs/conformance-sequence.md`**; this file is the immediate step and the outstanding forks.

## Landed this session, in order

- **D344** - the descent's polarity re-read and corrected in the evidence file (bit `0x20` of a cell's second byte means
  **passable**; the 2026-11-20 "contains an obstacle" correction was backwards, no code changed), plus the instrumentation
  that prices the tier against the flat search: `TierShadow`, `TierFunnel`, `compare_tier_routes`, `flat_route` extracted from
  `route_with`'s tail.
- **D345** - the tier was made to answer and its routes lost on point count and length against the flat search; the floor went
  back to 2 and both witnesses were parked. **Superseded by D346.**
- **D346** - the owner ruled U28 with *"follow original game"*. The missing piece was the binary's **smoothing pass**
  (`move_obj.cpp:0xf56` -> `FUN_008acf20` -> `0xf69` *"After smoothing"*): `Ground::smooth` keeps a last-kept point and a
  candidate, asks the radius-grown segment test whether the straight line is walkable, and deletes the waypoint in between when
  it is. Bodies leave the tier's geometry, as they are absent from the binary's coarse search and its smoothing.
- **D347** - the last gate went: `descent == 0` is answered by the levelled search at 1-tile resolution, not handed to our
  flat search. Measured: **92 of 50,652** routes answered (75 at level 0, 17 coarser), **4,175 raw waypoints -> 637
  smoothed**, **+2.8% ground** against the flat search on the same walks, 134 entities and 12.12 s.
- **D348** - a read, no code: `FUN_007f0be0` sets the search window to a box of half-extent (radius sum + **the caller's
  margin**) around one point. The **mover** passes a margin of **zero** (`path+0x20`, which the allocator initialises), so its
  mid-level search is a one-cell refinement and its ladder is mid-level -> *"Looking for a STRAIGHT LINE path"* -> *"Looking for
  a TILE-BASED path"*, **per waypoint** of a path it already holds. The **order-level** planner forwards its own margin, and the
  call sites read pass **0**, **1.0** and a **per-unit virtual value** - so its window's scale is **unread**.

**What that means for our tier:** it is a stand-in for the **unread high-level driver** that chooses the waypoints, and whether
it is *faithful* to the order-level planner depends on that per-unit margin (`+0x12c`). The surviving +2.8% is the coarse
guide's shape (corners on cell crossings rather than on obstacle vertices), not a defect in `FUN_007ed280`'s cost rows.

## The next step: the margin and the driver (a read, not a slice)

0. **What does the virtual method at vtable `+0x12c` return?** The order-level planner's window is `radius + radius + margin`,
   and the margin is 0 in one call site, `1.0` in another and this virtual call in two more (`FUN_00529590:193`,
   `FUN_0052ff40:333`). Its scale decides whether an order's long route is planned in a few tiles or across the map - and
   therefore whether our whole-walk tier is faithful or a stand-in. **This is the read to do first.**
1. **Who fills the mover's waypoint array** before `FUN_008a4360` asks for a mid-level path? The path object is
   `FUN_007fd4b0`-allocated, filled by `FUN_007fd880`/`FUN_007fd750`; what decides its points is the gap D297 named. The
   candidates: `FUN_007ed600` (masked quadtree neighbour walk), `FUN_007ee600` (the quadtree ray), and **the callers of
   `FUN_008000d0` that pass a non-zero 9th argument** - that argument is the window margin, so a caller asking for a large one
   is asking for a long route and names the high-level planner by elimination.
2. **Then decide the shape**: our tier as a *guide* (its waypoints fed to the straight-line/tile ladder per leg) or as the
   *route* (today). Both are landable and both are measurable on the same 92 walks.
3. **The uncapped coarse search** (D347's OCR finding) is downstream of 1: the binary bounds it with the same window, so the
   margin we now know (mostly 0, sometimes `+3.0`) is the constant a cap would use.

## The owner's report, and it is the next obstacle-avoidance step: bodies pile up

Reported 2026-11-22: *"villagers stack on each other a lot (like 5 in the same spot)"*. Measured on the acceptance scene
(`collision_probe`, 36,000 ticks, seed 7): **the deepest overlapping group is 11 bodies**, **317,041 pairs closer than a fifth
of a tile**, deepest overlap the full footprint (0.400 tiles), and the same census on the tree before D346/D347 reads
349,413 against 349,824 pair-ticks - so **not a regression from today's routing**, and not fixed by it either.

What the tree already knows: the D267 classification (**mates 0, past 43,315, neither 61,071**) says more than half of these
pairs are ones the step should have refused, so it is a defect as well as the crowd rule the owner asked for. Nothing in the
queue targets it: step 11 is formations for soldiers, step 13 is P3's inset (*touching* collides, which addresses grazing).

**The step to take**: read `FUN_00840830`'s caller - is the body test evaluated **before** the *past* check, and does a past
body still refuse a step that lands **inside** it - then a slice with the pile census as its witness (deepest group, and the
count of pairs inside a fifth of a tile, both before and after).

## Also ready, independent of that

- **The goal-cell refusal in `coarse_route`**: the binary does not refuse a goal whose cell holds an obstacle - it refines
  the last leg to the point and lets the step stop at contact. Not measured on its own yet.
- **The substitution threshold** (D340): the game's `> 2` against our `REPLAN_TICKS` (20) in `replan_after_ground`, waiting
  for a fixture that stands on an off-grid aim for three ticks.
- **A body on a tile corner blocks no planned tile** (found while staging D341): the search asks `still_inside` of *tile
  centres* (`ground.rs`, `inside_a_body`), so a small-radius body on a corner can exclude nothing.
- **The 16-tile floor** (D306): it was justified by "the refinement is unread", which D343 ended.
- **Sequence step 9** - the order's path vocabulary (forced waypoint, teleport-to-waypoint, per-path hit); the `ADDmove`
  fields are unread.
- **Sequence step 11** - the mate exemption is the game's for villagers (D237); what is left is a *formation model* for
  soldiers (U27).
- **Sequence step 12** - P11's polygonal short-range pathfinder: a project, not a step.
- **Sequence step 13** - P3's inset (`+0x268`) and the 100/600 terrain rows; the trebuchet's one-tile passage (D306) must stay
  green through it.
- **`make decisions` is broken for D283 onward** (60 decisions have no area in `docs/decision-areas.toml`), so
  `docs/site/index.html` is stale. A filing pass, not a mechanism.

## Needs the owner

- **An evenly mirrored skirmish**: player 2's barracks order is at 70 s where player 1's is at 320 s, and player 2 is
  eliminated at ~1,985 s in the acceptance match, so every comparison after that point is between two different matches.

## Parked on what this box does not have

- **P3's inset**, the **100/600 terrain bits**, the **octile table's values** (`DAT_00afd2a0` is runtime storage), and the
  **straight-line tier** (the caller of `FUN_008000d0`). The **equal-`f` sift order** is *not* parked - `FUN_00801e90` is in
  the corpus and is one read. And the **writer of the collision ignore list** is one read: it decides when a military
  formation fills it.
- **The acceptance wall's attribution** (D337): 15.7 s against 9.7 s for the key alone, never profiled per pass.
