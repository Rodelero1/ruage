# OPEN - the pile is an arrival problem: which rule keeps two men out of one point, read next as the plan's tile rectangle

**Where this stands:** suite **227 result lines, 944 passed, 0 failed, 25 ignored** at D349; conformance **21 / 0 / 0**;
acceptance `7782dd38d7557514`, **134 entities**, 37,204 re-paths, **12.17 s** median of three. The order to 100% conformance
is **`docs/conformance-sequence.md`**; this file is the immediate step and the outstanding forks.

## The owner's report, sized and attributed (2026-11-22)

*"villagers stack on each other a lot (like 5 in the same spot)"*. On the acceptance scene (`collision_probe`, 36,000 ticks,
seed 7): deepest overlapping group **11 bodies**, **317,041** pairs closer than a fifth of a tile, deepest overlap the full
footprint (0.400 tiles) - and only **877** of 349,824 overlapping pair-ticks are *new* on a tick. So the pile is a state the
engine settles into and stays in, entered rarely.

`pile_probe` (new) attributes each new overlap to the exemption the mover held **on that tick**: **mate 0, past 500,
neither 164**, every offender a `Gather`/`Return` pair arriving at the **same raw point** (dx = 0, dy = 7,200 raw against an
8,000 reach). **The pile is the arrival**: a body collision never refuses a step here (D279's port - the step is taken, the
higher id paused, the mover re-plans), which is the binary's own epilogue, and nothing separates two men who stop inside each
other.

**Two rules the binary has on the plan side, and we have neither:**

1. **A stander blocks the tiles its box covers.** `FUN_007f9d80` grows the body's box by the **mover's** radius, clamps it to
   tile indices (`FUN_007f9b70`) and stamps the rectangle blocked (`FUN_007f6ff0`). Our `Ground::inside_a_body` asks whether a
   tile's **centre** is inside that box - the same answer only while a body stands near a tile centre, and **no tile at all**
   for a body on a lattice corner.
2. **The arrival arm needs the collision list to be exactly the target.** `if ((DAT_00de1998 != 1) || (local_dc != 1))` ->
   *"We have %d non-past collisions ... repathing"*; only a list of one entry, the target, reaches *"That collision is with our
   target, so we're done"* (`0x127d`). Ours cuts at contact **before** the body test, so a friend on the contact point does not
   stop the walk.

## The step to take next: the tile rectangle (item 1)

- Prior implementation: `crates/sim/src/ground.rs:1001` `inside_a_body(scale, tile, avoid)` = `clear.still_inside(centre)`, read
  by the search's expansion (`:1164`, `:1178-1179`) and by `leg_is_clear` (`:674`, `:690-691`). The step's own refusal
  (`motion.rs`) is already exact (box against the landing point) and does not use it.
- The change: a tile is blocked when its **square** overlaps the grown box (touching excluded, the same convention as
  `still_inside`). A body at a tile centre then blocks its own tile - the same as today - and a body anywhere else blocks every
  tile its box touches.
- **Fixture first, and it must bite**: a stander on a lattice corner and a walker whose tile path runs past him, asserting the
  walker's own centre never enters the pair's grown box. A first attempt walked the walker along a **tile row** and did not
  bite (his line passed 0.5 of a tile from the corner, outside a 0.4-tile box) - the scene has to put the body on the line.
- Witness: the pile census (deepest group, pairs inside a fifth of a tile, `pile_probe`'s split) plus the acceptance wall and
  re-path count. **Costs more blocked tiles per plan**, so the wall is the number that decides whether it stays.

## Then: the arrival arm (item 2)

- Needs a fixture that fires it: a **fast** walker whose step lands strictly inside the target's grown box while a friend
  stands on the contact point the cut would place him at. Wired and measured without one, it changed the census **not at all**
  (the cut rarely fires at walking speed), so it went back out - see D349.
- Then: skip the cut when `other_body_on(mover, contact, own, target)` - any unit whose grown box covers the contact point,
  except the mover, the target and his **moving mates** (`FUN_007fe010`, *"That's a path ignore unit"*, which neither pauses nor
  counts nor re-paths). A *past* body is **not** skipped: the binary drops it from the count of non-past collisions, not from
  the list.

## Open defects found on the way, with the evidence, no fix landed

- **A walk whose goal is on the mover's own tile answers `NoPath`** when the straight-line shortcut is refused by a body:
  `route_with` reports a search whose start node *is* its goal as no path, and the order is refused
  (`NoPath { from: (15,14), to: (15,14) }`, seen from two builder fixtures under a stricter plan). The shortcut hides it until a
  body overlaps the leg. A fix (`start == goal` -> the point itself) and a fixture were both written and both **dropped**: the
  `WalkTo` fixture took the task layer's own direction and never asked the search, so it could not witness the change.
- **`last_step` is only written when step tracing is on** (`record_step` returns early otherwise), so a mover that plainly
  walked reads as `Idle`. It cost an hour of this session's attribution.

## Also ready, independent of the pile

- **The virtual margin at vtable `+0x12c`** (D348): the order-level planner's search window is `radius + margin`, and the
  margin is 0, 1.0 and a per-unit virtual value at different sites. It decides whether our whole-walk tier is faithful or a
  stand-in.
- **Who fills the mover's waypoint array** before `FUN_008a4360` asks for a mid-level path (D297's gap): `FUN_007ed600` (masked
  quadtree neighbour walk), the quadtree ray `FUN_007ee600`, or the callers of `FUN_008000d0` that pass a non-zero 9th argument
  (that argument is the window margin).
- **The uncapped coarse search** (D347's OCR finding), downstream of the margin: the binary bounds it with the same window.
- **The goal-cell refusal in `coarse_route`**: the binary does not refuse a goal whose cell holds an obstacle - it refines the
  last leg to the point and lets the step stop at contact. Not measured on its own yet.
- **The substitution threshold** (D340): the game's `> 2` against our `REPLAN_TICKS` (20) in `replan_after_ground`.
- **The 16-tile floor** (D306): justified by "the refinement is unread", which D343 ended.
- **Sequence step 9** - the order's path vocabulary (forced waypoint, teleport-to-waypoint, per-path hit); `ADDmove` unread.
- **Sequence step 11** - a *formation model* for soldiers (U27); the mate exemption is the game's for villagers (D237).
- **Sequence step 12** - P11's polygonal short-range pathfinder: a project, not a step.
- **P3's inset** (`+0x268`, gated by the byte at `+0x266`): both boxes are **grown** by it in the query and in the test, so it is
  a separation margin, not an inset - the value is unread.

## Needs the owner

- **The pile's last inch**: with no pushing and no place model (D293, his *"drop it and let the crowds jam"*), two men who share
  one destination can only be kept apart by the plan. If the tile rectangle does not move the census, the options are a place
  model (withdrawn by D293) or accepting the pile as the game's own behaviour - **his call, and the numbers will be in the review
  that comes with it.**
- **An evenly mirrored skirmish**: player 2's barracks order is at 70 s where player 1's is at 320 s, and player 2 is eliminated
  at ~1,985 s in the acceptance match, so every comparison after that point is between two different matches.

## Parked on what this box does not have

- **P3's inset value**, the **100/600 terrain bits**, the **octile table's values** (`DAT_00afd2a0` is runtime storage), and the
  **straight-line tier** (the caller of `FUN_008000d0`). The **equal-`f` sift order** is *not* parked - `FUN_00801e90` is in the
  corpus and is one read.
- **The acceptance wall's attribution** (D337): never profiled per pass.
