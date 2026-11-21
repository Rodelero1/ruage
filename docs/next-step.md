# OPEN - step 6 landed (D341: a body is an obstacle unless it moved); next is step 7, the 300 row

**Where this stands:** suite **219 result lines, 937 passed, 0 failed, 22 ignored**; conformance **21 / 0 / 0**; acceptance
`1e1fcc3cf2b05a4c`, 133 entities, **one player alive at 48,000**, 36,567 re-paths, **11.42 s** median of three (the 27%
fall against D339's 15.93 s is not a cost claim: the scenario diverges). The *order* to 100% conformance is
**`docs/conformance-sequence.md`**; this file is the immediate step and the outstanding forks.

**Landed in this session:** step 5 as a read (D340 - the substitution has no stand to answer, because a body never refuses
our mover's step), step 6 (D341 - `standing_bodies` skips a body only when it *has a route and moved on the previous
tick*, the binary's velocity test; a builder standing at work was 19 ticks walked through and is now gone round).

## The next step: 7 - the levelled search's 300 row

**Prior implementation.** The levelled search's per-cell cost is in `crates/sim/src/levels.rs`; the 300 row is applied
where a neighbour is *blocked* (grep `300` there and in `ground.rs`). The sequence's claim is that the game's 300 is not
"a blocked neighbour" but **an object of type 8 or `0x20` on the cell or beside it**.

**The rule, read** (`FUN_007ed280`, reached through `FUN_007f11f0`): the cost rows are keyed by the cell's own flags; a
cell beside a cliff is a *different* row from a cell beside an object. **Still unread:** which of the two flag values
(`8`, `0x20`) is the object and which the cliff, and whether "beside" is the four neighbours or the eight. Two reads.

**Witness owed:** a cliff-adjacent cell costs **10** (not 300); an object-adjacent cell costs **300**. Both cells on one
small scene, so the fixture cannot pass by giving every neighbour the same row.

## Also ready, independent of that

- **The refinement's scan direction** (step 8): the binary scans the span outward from the interpolated point (forward,
  then back), ours from the low end. `FUN_007eeda0` read. Its own A/B; also what would let the 16-tile floor fall
  (D338's re-test).
- **The level floor** (D303): the clause that keeps the tier off in a wooded map. Route-wide; needs its own A/B.
- **The substitution threshold** (D340): the game's `> 2` against our `REPLAN_TICKS` (20) in `replan_after_ground`, waiting
  for a fixture that stands on an off-grid aim for three ticks.
- **A body on a tile corner blocks no planned tile** (found while staging D341): the search asks `still_inside` of *tile
  centres* (`ground.rs`, `inside_a_body`), so a body standing on a corner - which is where the map's own seeding places
  villagers - excludes nothing. Its own slice; the step-side test is exact, so the plan and the step disagree there.
- **The mate rule (step 11) is already the game's**: `stamp_group` (`world/step.rs:110-144`) gives mates only to **military**
  units, and a villager is never one. What is left is a *formation model* for soldiers - a gap of its own.

## Needs the owner

- **Step 7's predicate needs a content field the pack does not carry.** `FUN_007f11f0` is exact: it walks the tile's object
  list and answers 1 only for an object whose type is **8** or **0x20**; `FUN_007ed280` charges **300** to a clear cell
  whose own type test and four orthogonal neighbours' all answer 0. Our pack has no per-unit `type` (the generated
  `units.toml` carries `class`, not that field), and the importer lives in its own repository (D12), so the two values
  cannot be named or emitted from this tree. Options: (a) extend the importer to emit the dat's unit type and key the 300
  row on 8/0x20 exactly; (b) keep the current predicate - *any* blocked orthogonal neighbour, terrain included - and
  record it as a deliberate widening; (c) key it on a property our content vocabulary does have (an object's footprint
  versus terrain) and record the difference. (a) is the game's; (b) is what ships today.
- **An evenly mirrored skirmish**: player 2's barracks order is at 70 s where player 1's is at 320 s. D341 now leaves
  player 2 eliminated by 48,000 as well, so the imbalance is worth fixing before the next acceptance comparison.

## Parked on what this box does not have

- **P3's inset** (`+0x268`), the **100/600 terrain bits**, the **octile table's values** (`DAT_00afd2a0` is runtime storage),
  the **search window's padding** (D299), and the **straight-line tier** (the caller of `FUN_008000d0`). The **equal-`f` sift
  order** is *not* parked - `FUN_00801e90` is in the corpus and is one read. And the **writer of the collision ignore list** is
  one read: it is what decides when a military formation fills it.
- **The acceptance wall's attribution** (D337): 15.7 s against 9.7 s for the key alone, never profiled per pass.
