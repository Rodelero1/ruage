# OPEN - the level floor (D303): let the tier answer at level 1, and judge its routes against the flat search's

**Where this stands:** suite **221 result lines, 939 passed, 0 failed, 23 ignored**; conformance **21 / 0 / 0**; acceptance
`1e1fcc3cf2b05a4c`, 133 entities, one player alive at 48,000, 36,567 re-paths, **~12.0 s** median of three. The *order* to
100% conformance is **`docs/conformance-sequence.md`**; this file is the immediate step and the outstanding forks.

**Landed in the last session:** step 5 as a read (D340), step 6 (D341 - a body is an obstacle unless it moved), step 7
(D342 - the 300 row is a gold or stone mine), step 8 (D343 - the refined waypoint is the crossing the walk's line makes).
All three of 6-8 are bit-identical in the acceptance match, because the levelled tier answers **0 of 61,276** routes on
this map (D338).

## Crosscheck done before this step (2026-11-22) - and it corrected the record, not the code

The descent's polarity was re-read, because the 2026-11-20 entry in `docs/pathfinding-levels-evidence.md` had "corrected"
bit `0x20` of the cell's second byte to "this cell contains an obstacle" - which would have inverted `Levels::descent` and
made the tier run coarsely in clutter and finely in the open. **That correction was backwards.** `FUN_007f0da0` tests the
grown box against the cells that *have* the bit and then **blocks** them (bit `0x10`): blocking a cell that already holds an
obstacle is a no-op, blocking the passable cell the radius now covers is the growth's whole point. `FUN_007f1100` reads the
same bit the same way - it descends through cells without it and stamps the first cell that has it. `FUN_007ee760` refuses
to open a start node whose cell has no passable direction. So the bit means **passable**, the descent stops at the coarsest
**passable** level, and `Levels::descent` is already the game's rule. The evidence file now says so and withdraws the
2026-11-20 wording. No code changed for it.

## The step: the level floor

**Prior implementation.** `Ground::plan_path_with`'s tier arm (`crates/sim/src/ground.rs:616-700`, the gate at
`ground.rs:646`) asks the hierarchy only when the walk is **long** (16 tiles, D306) **and**
`levels.descent(start).min(levels.descent(goal)) >= 2` (D303). `Levels::descent` (`crates/sim/src/levels.rs:463`) walks the
OR-reduced pyramid from level 3 down and stops at the first **passable** cell, flooring at 0; `coarse_route`
(`levels.rs:200`) then searches at `min(descent(start), descent(goal))`. Pinned by `crates/sim/tests/descent.rs` (3 rows),
`coarse_route.rs` (6), `tier_ignores_bodies.rs` (1). Today it answers **0 of 61,276** routes: on this map 127 of 185 long
walks start on a tile whose 4-tile cell is not passable, so the gate refuses.

**The rule, read** (`FUN_007ee980`, `docs/pathfinding-levels-evidence.md`): the binary walks the levels down and floors at
level 0, where the search *is* the tile search - it does not refuse to answer because the start cell is coarse. If the
mid-level attempt fails it falls through to the low-level path (`0xf98`: *"Looking for a LOW LEVEL path (mid-level path
failed)"*). So "no coarse answer" is a fall-through, not a gate.

**The change, one predicate:** `>= 2` becomes `>= 1`. Level 0 is **not** taken: there the coarse search is a
four-directional tile A* whose Manhattan staircase is strictly worse than our octile flat search, which already answers
that case - a labelled deviation, and D338's own numbers price it (level 0 answers 108 routes with a different hash).

**Witness owed, red first:** one long walk on a small seeded map whose **4-tile cell holds a blocker outside the start's
2-tile cell**, so `descent(start) == 1`; the plan's interior waypoints must sit on **2-tile** boundaries (`n * 2 * scale`),
which no tile-centre route can produce. Red on the un-cut tree, where the flat search answers.

**The judgement, and it is the acceptance:** with D341-D343 underneath, at floor 1 the tier answers 13 routes and the
match changes (D338: hash `69a548722252b27e`). The step is only done when the numbers say whether **the tier's route is
better than the flat search's on the same walk** - point count, length, and what the match does (entities, re-paths, wall)
- and if it is not better, the finding is the numbers and the floor stays at 2.

## Also ready, independent of that

- **The level-0 question** (D338): floor 0 answers 108 routes, hash `d700295118db1114`. It is measured in the same run as
  the shadow comparison, so the decision has both numbers even if the answer is "1, not 0".
- **The substitution threshold** (D340): the game's `> 2` against our `REPLAN_TICKS` (20) in `replan_after_ground`, waiting
  for a fixture that stands on an off-grid aim for three ticks.
- **A body on a tile corner blocks no planned tile** (found while staging D341): the search asks `still_inside` of *tile
  centres* (`ground.rs`, `inside_a_body`), so a body standing on a corner - which is where the map's own seeding places
  villagers - excludes nothing. Its own slice; the step-side test is exact, so plan and step disagree there.
- **The mate rule (step 11) is already the game's**: `stamp_group` (`world/step.rs:110-144`) gives mates only to
  **military** units, and a villager is never one. What is left is a *formation model* for soldiers - a gap of its own.
- **The 16-tile floor** (D306): re-test it once the tier answers - it was justified by "the refinement is unread", which
  D343 has just ended.

## Needs the owner

- **An evenly mirrored skirmish**: player 2's barracks order is at 70 s where player 1's is at 320 s, and player 2 is
  eliminated at ~2,350 s in the acceptance match. The imbalance is worth fixing before the next acceptance comparison.

## Parked on what this box does not have

- **P3's inset** (`+0x268`), the **100/600 terrain bits**, the **octile table's values** (`DAT_00afd2a0` is runtime storage),
  the **search window's padding** (D299), and the **straight-line tier** (the caller of `FUN_008000d0`). The **equal-`f` sift
  order** is *not* parked - `FUN_00801e90` is in the corpus and is one read. And the **writer of the collision ignore list** is
  one read: it is what decides when a military formation fills it.
- **The acceptance wall's attribution** (D337): 15.7 s against 9.7 s for the key alone, never profiled per pass.
