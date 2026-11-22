# OPEN - the tier answers and loses: make the chain competitive, or decide it should not answer at all

**Where this stands:** suite **939 result lines, 938 passed, 0 failed, 23 ignored**; conformance **21 / 0 / 0**; acceptance
`1e1fcc3cf2b05a4c`, 133 entities, one player alive at 48,000, 36,567 re-paths, **~12.0 s** median of three. The *order* to
100% conformance is **`docs/conformance-sequence.md`**; this file is the immediate step and the outstanding forks.

**Landed in the last session (D344, D345):** the descent's polarity re-read and corrected in the evidence file (bit `0x20` of
a cell's second byte means **passable** - the 2026-11-20 "contains an obstacle" correction was backwards, no code changed), and
the measurement that prices the tier against the flat search: `TierShadow`, `TierFunnel`, `compare_tier_routes`, and
`flat_route` extracted from `route_with`'s tail. Floor-level changes were tried and **put back**; their witnesses are parked.

## What the last session established, with numbers

1. **The floor was never the gate.** No floor value (0, 1, 2) moves the acceptance match at all. D338's priced neighbours
   (13 and 108 answers) do not reproduce after D341.
2. **The funnel**: of 91 long walks, 60 floor-refused, 13 got no coarse route (`coarse_route` refuses a goal whose cell at
   the search level is occupied), 17 had a leg refused by a body in the plan's avoid list, 1 by geometry, **0 answered**.
3. **Opening both gates makes the tier answer, and its routes lose**: 14 of 50,022 routes, **172 waypoints against 67** and
   **5,180,218 raw against 4,719,155** (+9.8%) on the same walks, at **13.71 s against 12.16 s** of wall (13% slower), 134
   entities against 133.
4. So the floor is **2**, the chain is certified against bodies again, and both witnesses sit in `docs/parked/`
   (`tier-level-floor.rs`, `tier-certification.rs`) with the un-park condition in their headers.

## Landed this step (D346): the game's route, smoothed

The owner's ruling on U28 was **"follow original game"**, so the floor is the binary's again (nothing refused above level 0)
and the tier's polyline goes through the original's own smoothing pass, `FUN_008acf20` (`move_obj.cpp:0xf56` ->
*"Found a valid mid-level path"* -> the pass -> `0xf69` *"After smoothing"*). Measured on the acceptance map: **194 raw
waypoints -> 57 smoothed**, the tier's answer now has **fewer** waypoints than the flat search's (57 against 76 over the same
16 walks) and **+4.0% ground** (was +9.8% before the pass), the match runs **11.70 s against 12.18 s** and re-paths fall
**36,567 -> 17,743**. Bodies are not in the tier's geometry again: the coarse search and the smoothing are both blind to them
in the binary, and the step's bump/re-plan is what answers a body (D320).

## The one place we still do not follow the game: level 0

`route_with` refuses `descent == 0` and hands those walks to our octile flat search, where the binary would run its own
levelled search at level 0 (the same coarse A* over 1-tile cells). That is 55 of the 86 long walks in the acceptance match.
Elder sibling of this step: measure it by deleting the refusal and comparing - the flat search's octile costs against the
coarse search's four-directional 10/300 rows at tile resolution, both smoothed. Not started.

## Also ready, independent of that

- **The goal-cell refusal in `coarse_route`** (13 long walks): the binary does not refuse a goal whose cell holds an obstacle -
  it refines the last leg to the point and lets the step stop at contact. Opening it was the other half of the funnel and is
  not yet measured on its own.
- **The substitution threshold** (D340): the game's `> 2` against our `REPLAN_TICKS` (20) in `replan_after_ground`, waiting
  for a fixture that stands on an off-grid aim for three ticks.
- **A body on a tile corner blocks no planned tile** (found while staging D341): the search asks `still_inside` of *tile
  centres* (`ground.rs`, `inside_a_body`), so a body standing on a corner excludes nothing. Its own slice; the step-side test
  is exact, so plan and step disagree there.
- **The 16-tile floor** (D306): it was justified by "the refinement is unread", which D343 ended; re-test it only after the
  fork above is decided, or it re-measures a route nobody takes.
- **`make decisions` is broken for D283 onward** (60 decisions have no area in `docs/decision-areas.toml`), so
  `docs/site/index.html` is stale. A filing pass, not a mechanism.

## Needs the owner

- **The fork above** (a / b / c).
- **An evenly mirrored skirmish**: player 2's barracks order is at 70 s where player 1's is at 320 s, and player 2 is
  eliminated at ~2,350 s in the acceptance match, so every acceptance comparison after that point is between two different
  matches.

## Parked on what this box does not have

- **P3's inset** (`+0x268`), the **100/600 terrain bits**, the **octile table's values** (`DAT_00afd2a0` is runtime storage),
  the **search window's padding** (D299), and the **straight-line tier** (the caller of `FUN_008000d0`). The **equal-`f` sift
  order** is *not* parked - `FUN_00801e90` is in the corpus and is one read. And the **writer of the collision ignore list** is
  one read: it is what decides when a military formation fills it.
- **The acceptance wall's attribution** (D337): 15.7 s against 9.7 s for the key alone, never profiled per pass.
