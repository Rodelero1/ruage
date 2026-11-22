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

## The fork this needs the owner for

The tier is the game's mechanism (a coarse mip-map route, refined to a crossing per cell edge) and it is losing to our own
flat search on every measure. Three ways forward, and they are a decision, not a measurement:

- **(a) Pull the chain before handing it back.** The binary smooths its waypoint list; our flat route is pulled to the corners
  obstacles force, the tier's is not pulled at all, so the comparison is between a smoothed route and a staircase. This is
  the cheapest experiment: `pull` already exists and its doc says the original smooths the same way. It may recover most of
  the 2.6x waypoints, and the length gap is the part it cannot recover.
- **(b) Price the coarse costs against the octile ones.** `FUN_007ed280`'s rows (10 plain, 100 terrain, 300 beside a mine,
  600 blocked-ish, 2000 for something) are a 4-directional cost model; our flat search is 8-directional octile. A faithful
  4-directional search is *structurally* longer on a diagonal-heavy map. If the game's own coarse route is longer than its
  own flat route, then "the tier is worse" is the game's behaviour and the question becomes fidelity, not quality.
- **(c) Let the tier stay unread in the match.** It costs 14 routines' worth of work (D341-D345) that no match reads. That is
  a legitimate answer only if the coarse route is a fidelity goal; nothing else in the tree needs it.

The recommendation is **(a) first** - it is a small, testable change to the answer we already produce, and if it closes most of
the gap the rest is a fidelity argument the owner can price. (b) needs the coarse costs re-derived against the binary's own
tables before it can be argued at all.

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
