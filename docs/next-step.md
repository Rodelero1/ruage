# OPEN - step 7 landed (D342: the 300 row is a mine); next is step 8, the refinement's scan direction

**Where this stands:** suite **220 result lines, 938 passed, 0 failed, 23 ignored**; conformance **21 / 0 / 0**; acceptance
`1e1fcc3cf2b05a4c`, 133 entities, one player alive at 48,000, 36,567 re-paths, **11.8 s** median of three. The *order* to
100% conformance is **`docs/conformance-sequence.md`**; this file is the immediate step and the outstanding forks.

**Landed in this session:** step 5 as a read (D340 - the substitution has no stand to answer, because a body never refuses
our mover's step), step 6 (D341 - `standing_bodies` skips a body only when it has a route **and** moved on the previous
tick; a builder standing at work was 19 ticks walked through and is now gone round), step 7 (D342 - the 300 row is a cell
beside a **gold or stone mine**, named by the pack with `snug = true`; the match is **bit-identical** because the tier that
prices the row answers nothing - D338).

## The next step: 8 - the refinement's scan direction

**Prior implementation.** The chain's refinement scans the span **from the low end** and takes the first boundary crossing
it meets (`crates/sim/src/levels.rs`, `Levels::refine`: `for k in 0..span { ... break }`).

**The rule, read in full** (`FUN_007eeda0`, HD 5.7 - both halves, `0x7eeda0`-`0x7eef09` and the scan below it). Two
things, and the second is not a scan order:

1. **The point is interpolated, not the low end.** The function is handed the mover's position (`param_6`, `param_7`) and
   a far point (`param_4`, `param_5`); for a boundary crossed along x it computes
   `iVar7 = (iVar4 * param_5) / param_4 + param_7` - the straight line's own crossing of the edge, `iVar4` being the
   distance from the span's edge to the start. Ours has no interpolation at all: it walks the span from `first + 0`.
2. **Then the span is scanned outward from that point.** Forward from the interpolated point to the span's end, keeping
   the first crossing and its distance; then a **backward** pass from the same point, stopping when `iVar10 < 1` (it has
   walked back as far as the forward crossing was ahead) or at a crossing, which then wins. The vertical case is the same
   code on the other axis.

So the port is **two** changes - thread the mover's position and the far point into `refine`, then scan outward from the
interpolated crossing - and the second cannot be watched on its own until the first exists.

**Witness owed:** a step whose span crosses **two** boundaries, where the interpolated crossing is the far one - the
route must leave through the interpolated crossing, and ours leaves through the low one. Small scene, one variable, and
the size is **M**, not the S the sequence calls it.

**Why it matters beyond itself:** this is what would let the 16-tile floor fall (D338's re-test), and with the floor down
the coarse tier starts answering routes - the precondition for D342's row and D337's key to change a match at all.

## Also ready, independent of that

- **The level floor** (D303): the clause that keeps the tier off in a wooded map (`coarse.level >= 2`; 127 of 185 long
  walks start on a tile whose 4-tile cell is unusable). Route-wide, needs its own A/B, and it is the single lever that
  makes D342 and D337 visible in a match.
- **The substitution threshold** (D340): the game's `> 2` against our `REPLAN_TICKS` (20) in `replan_after_ground`, waiting
  for a fixture that stands on an off-grid aim for three ticks.
- **A body on a tile corner blocks no planned tile** (found while staging D341): the search asks `still_inside` of *tile
  centres* (`ground.rs`, `inside_a_body`), so a body standing on a corner - which is where the map's own seeding places
  villagers - excludes nothing. Its own slice; the step-side test is exact, so the plan and the step disagree there.
- **The mate rule (step 11) is already the game's**: `stamp_group` (`world/step.rs:110-144`) gives mates only to **military**
  units, and a villager is never one. What is left is a *formation model* for soldiers - a gap of its own.

## Needs the owner

- **An evenly mirrored skirmish**: player 2's barracks order is at 70 s where player 1's is at 320 s, and player 2 is
  eliminated at ~2,350 s in the acceptance match. The imbalance is worth fixing before the next acceptance comparison.

## Parked on what this box does not have

- **P3's inset** (`+0x268`), the **100/600 terrain bits**, the **octile table's values** (`DAT_00afd2a0` is runtime storage),
  the **search window's padding** (D299), and the **straight-line tier** (the caller of `FUN_008000d0`). The **equal-`f` sift
  order** is *not* parked - `FUN_00801e90` is in the corpus and is one read. And the **writer of the collision ignore list** is
  one read: it is what decides when a military formation fills it.
- **The acceptance wall's attribution** (D337): 15.7 s against 9.7 s for the key alone, never profiled per pass.
