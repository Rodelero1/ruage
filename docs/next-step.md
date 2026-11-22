# OPEN - the pile is an arrival problem: which rule keeps two men out of one point, read next as the plan's tile rectangle

**Where this stands:** suite **227 result lines, 944 passed, 0 failed, 25 ignored** at D349; conformance **21 / 0 / 0**;
acceptance `7782dd38d7557514`, **134 entities**, 37,204 re-paths, **12.17 s** median of three. The order to 100% conformance
is **`docs/conformance-sequence.md`**; this file is the immediate step and the outstanding forks.

**The one list of what we do differently is `docs/divergence-register.md`** (D352): two verdicts only - DEFECT (read, ours,
with the source and the step that closes it) and OURS (deliberate, with what the game does instead). Eleven defects and five
ours as of 2026-11-22.

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

## The step to take next: three ports, then the census (D350)

The tile rectangle alone was built, measured and reverted: the route bends and the walker still clips a body, because the clip
is made in the **refinement**. All three are one slice, and the fixture is already written - it is preserved **outside the tree**
at `/opt/data/cache/scratch/age2hd-pending-fixtures/body_blocks_its_tiles.rs` (restore it into `crates/sim/tests/` first; it is
red on the current tree at 7,677 / 7,612 raw against an 8,000 reach):

1. **The tile rectangle** - `crates/sim/src/ground.rs:1004` `inside_a_body` asks the tile's **square** against the grown box
   instead of its centre (touching excluded). `FUN_007f9d80` grows the stander's box by the mover's radius, clamps it to tile
   indices (`FUN_007f9b70`) and stamps the rectangle blocked (`FUN_007f6ff0`). Measured in place: the plan bends round a corner
   body, and the walker still clips.
2. **The smoothing's bodies** - `ground.rs:975` `smooth` certifies with `leg_is_clear` (bodies, objects, grid) instead of
   `chain_leg` (objects, grid), and takes the `avoid` list from `route_with`. Grounded on `FUN_008acf20` -> `FUN_00840c60`,
   which walks the **object system's** buckets (`+0x8c`, the same `+0x266`/`+0x268` state) and so sees units; the coarse
   *search*'s blindness to bodies (D338) is true of the search, not of the pass after it. Measured in place: the route grows
   from 1 waypoint to 3 and the clip narrows to 350 raw on one axis.
3. **The refinement's tile span** - `crates/sim/src/levels.rs:307` `refine`: `FUN_007eeda0` scans the cell's tile span along
   the direction of travel and takes the first tile where the direction is **passable**, and passable is the tile test that
   consults the non-ignore objects on the tile (`FUN_00801a20`). Ours interpolates the crossing by geometry alone, so a crossing
   lands inside a body while every tile of the chain is open - and the smoothing can only delete waypoints, never insert one.
   **This is the port that closes the clip**; it needs `avoid` passed down from `route_with`.

Then the witness: the pile census (deepest group, pairs inside a fifth of a tile, `pile_probe`'s split) plus the acceptance wall
and re-path count, one variable per commit.

## Then: the arrival arm (item 2)

- Needs a fixture that fires it: a **fast** walker whose step lands strictly inside the target's grown box while a friend
  stands on the contact point the cut would place him at. Wired and measured without one, it changed the census **not at all**
  (the cut rarely fires at walking speed), so it went back out - see D349.
- Then: skip the cut when `other_body_on(mover, contact, own, target)` - any unit whose grown box covers the contact point,
  except the mover, the target and his **moving mates** (`FUN_007fe010`, *"That's a path ignore unit"*, which neither pauses nor
  counts nor re-paths). A *past* body is **not** skipped: the binary drops it from the count of non-past collisions, not from
  the list.

## Open defects found on the way, with the evidence, no fix landed

- **The walk onto the working standoff steps straight through a friend** (2026-11-22, from the census's own print). A worker
  that has arrived on the tile beside his node takes its last 0.3 of a tile *"as far as its speed allows, straight at the
  standoff"* (`step.rs`, D169/D180) - a **task-layer delta**, so `motion.rs`'s refusal and its body rule never see it, and the
  caller checks that the point is free **ground**, not that it is free of bodies. The census shows it happening: two men on node
  `108` close from **7,830 to 5,600 raw** over the ticks one of them works (progress 0 → 1,860), 800 raw a tick, and end inside
  each other. This is the one mechanism on the list that is *not* a plan question - it is a step with no body test - and it should
  be checked for its own slice before the three ports.
- **`Levels::refine` is body-blind** (D350's third port): the crossing it interpolates can land inside a body while every tile
  of the chain is open, and no later pass can repair it.
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

## Next: the gold the economy outruns (D353's consequence)

The drop-site fix (`D353`) made banking cheaper - re-paths **37,204 -> 12,847**, wall time **12.19 -> 10.10 s** on the acceptance
scene - and the scene's outcome went from **a win to a draw**. Measured cause: `player 1 has no gold node to gather` (220
firings), gold **410 -> 10** at 2,400 s, so the army stalls. Two candidate readings, both testable:

1. **The scene's AI never expands to farther gold** - `nearest_like` answers only within the tiles it scans, and once the near
   gold is gone it answers `None`. Test: give player 1 a gold node 12 tiles out and see whether the economy finds it.
2. **The mining-camp rule stopped firing** - if miners now drop at the town centre, the camp-spacing rule (`[build]
   drop_site_spacing`) may never trigger, and mining is the resource whose walk is longest. Test: count camps built per run
   before/after.

Answer with a measurement, not a choice; the scene's outcome is a metric this tree tracks ("a won match became a draw" has
already cost one slice, see `apply_switches`).
