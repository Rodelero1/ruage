# OPEN - steps 5-8 landed (D340-D343); next is the level floor (D303), the lever that makes the tier answer

**Where this stands:** suite **221 result lines, 939 passed, 0 failed, 23 ignored**; conformance **21 / 0 / 0**; acceptance
`1e1fcc3cf2b05a4c`, 133 entities, one player alive at 48,000, 36,567 re-paths, **12.0 s** median of three. The *order* to
100% conformance is **`docs/conformance-sequence.md`**; this file is the immediate step and the outstanding forks.

**Landed in this session:** step 5 as a read (D340 - the substitution has no stand to answer: a body never refuses our
mover's step), step 6 (D341 - a body is an obstacle unless it moved on the previous tick; a builder at work was 19 ticks
walked through and is now gone round), step 7 (D342 - the 300 row is a cell beside a **gold or stone mine**, named by the
pack with `snug = true`), step 8 (D343 - the refined waypoint is the crossing the walk's own line makes, then the scan
outward from it).

**All three of 6-8 are bit-identical in the acceptance match**, and for one reason: the levelled tier answers **0 of
61,276** routes on this map (D338), so nothing it prices or refines is ever read. That is the next step's whole subject.

## The next step: the level floor (D303) - let the tier answer

**Prior implementation.** `Ground::plan_path_with`'s tier arm (`ground.rs:640-660`) asks the hierarchy only when
`long_walk` (16 tiles, D306) **and** `levels.descent(start).min(levels.descent(goal)) >= 2`. `Levels::descent` walks the
pyramid from level 3 down and stops at the first level whose cell is usable. On the acceptance map that floor rejects
**127 of 185** long walks: their start tile's 4-tile cell is blocked, so the tier never runs.

**The rule, read** (`FUN_007ee980`, and the corpus's own descent loop): the binary walks the levels down and **floors at
level 0**, where the search is the flat tile search - it does not refuse to answer because the start cell is coarse. Its
mid-level attempts are *tried* and, when they fail, it falls through to the low-level path (`0xf98`: *"Looking for a LOW
LEVEL path (mid-level path failed)"*). So "no coarse answer" is a fall-through, not a gate.

**This is a measurement slice, not a port.** The lever is the `>= 2` floor, and D338 already priced the neighbours of it:
level `>= 1` answers **13** routes, `>= 0` answers **108**, and each rewrites the match (hashes `69a548722252b27e` and
`d700295118db1114`). What has not been measured is the tier with **D342's row and D343's refinement in it**, and whether
a route the tier answers is *better* than the flat search's on the same walk - which is the acceptance, not the hash.

**Witness owed:** one long walk on the seeded map whose tier route differs from the flat route, with the two routes and
their costs side by side; then the match's A/B at floor `>= 1` (13 answers) and at `>= 0` (108). If the tier's routes are
worse, the finding is the numbers and the floor stays.

## Also ready, independent of that

- **The substitution threshold** (D340): the game's `> 2` against our `REPLAN_TICKS` (20) in `replan_after_ground`, waiting
  for a fixture that stands on an off-grid aim for three ticks.
- **A body on a tile corner blocks no planned tile** (found while staging D341): the search asks `still_inside` of *tile
  centres* (`ground.rs`, `inside_a_body`), so a body standing on a corner - which is where the map's own seeding places
  villagers - excludes nothing. Its own slice; the step-side test is exact, so plan and step disagree there.
- **The mate rule (step 11) is already the game's**: `stamp_group` (`world/step.rs:110-144`) gives mates only to **military**
  units, and a villager is never one. What is left is a *formation model* for soldiers - a gap of its own.
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
