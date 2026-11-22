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

## Landed this session (D346, D347): the game's route end to end

The owner's ruling on U28 was **"follow original game"**, and the subsystem now does from the search to the polyline:

- **The level floor is gone** (D347): every long walk asks `coarse_route`, and the descent only *chooses* the level -
  `FUN_007ee980` floors at 0 and searches there, so `descent == 0` is answered by the levelled search at 1-tile resolution,
  not handed to our flat search.
- **The polyline is smoothed** (D346) by the original's own pass: `move_obj.cpp:0xf56` *"Found a valid mid-level path"* ->
  `FUN_008acf20` -> `0xf69` *"After smoothing"*. `Ground::smooth` keeps a last-kept index and a candidate, asks the
  radius-grown segment test whether the straight line is walkable, and deletes the waypoint in between when it is.
- **Bodies are out of the tier's geometry**, as they are in the binary's coarse search and its smoothing; the step's
  past/bump/re-plan (D320) answers a body.

Measured end to end: the tier answers **92 of 50,652** routes, **4,175 raw waypoints -> 637 smoothed**, and on the same 92
walks it is **+2.8% ground** against the flat search (637 waypoints / 65,242,638 raw against 515 / 63,486,927). Acceptance
`7782dd38d7557514`, **134 entities**, re-paths 37,204, wall **12.12 s** median of three.

## What is next, and it is a real gap now

- **The length gap is the coarse cost model, not the geometry.** +2.8% ground survives the smoothing pass, because the
  levelled search moves four-directionally over cells with the `FUN_007ed280` rows (10 plain, 100 terrain, 300 beside a mine,
  600, 2000) while our flat search is octile. The next slice is to price those rows against the octile ones on the same walks
  - if the binary's route is longer than its own flat route, +2.8% is the game and nothing is owed; if not, one of the two
  cost models is wrong. `docs/pathfinding-from-the-talk.md` and the D298 row hold what is read.
- **The coarse search's eight-neighbour question**: `FUN_007ed280` opens four directions (`param_4` 1, 2, 4, 8); whether the
  binary's *tile* search at level 0 uses the same four or its own eight is unread, and it is exactly where the +2.8% lives.
- **`coarse_route` has no expansion cap** (OCR, D347): the frontier is an insertion-sorted `Vec` (O(frontier) per expansion)
  and the `closed` bitmap is allocated per call, so a hopeless walk at level 0 exhausts the whole tile grid before the flat
  search answers. Measured on the acceptance map: **1.68 ms** a plan, 13 walks, 0.2% of the match - and **unbounded on a larger
  map** (quadratic in expansions; 480x480 is ~40x the tiles). The game bounds it with its search window (`in_ECX[0x42..0x45]`,
  padding unread, D299); a cap would be our invention, so the window is the read to do first.
- **`FUN_00840c60`'s flags**: called with `0x40`, its ignore-list branch is off; whether units are in the segment test by
  default is unread, and it decides whether the original's smoothing keeps slightly more waypoints near bodies than ours.

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
