# OPEN - the queue is 13 ordered steps in `docs/conformance-sequence.md`; step 1 is a witness, step 2 changes every route

**Where this stands (D334, `blunt-port`):** the adversarial review landed and is green - suite **933 / 0 / 20**, conformance
**21 / 0 / 0**, acceptance `e248486298910e28` (127 entities, 1,727 re-paths, **10.04 s** median of five on an idle box).
Nothing is half-applied. The *order* to 100% conformance, crosschecked against the first-party talk and openage, is
**`docs/conformance-sequence.md`** - read it first; this file is only the immediate step and the outstanding forks.

**What the review established** (details in the note and D334): the binary's frontier is ordered by `g + octile` and the
`25 x euclid` is its **expansion budget** (six files said otherwise); the levelled search answers **nothing** in a match
because all 121 long walks carry a stander; the 20,000 cap has no witness; body collision is a **disc** where the game's is a
**box** (2.17% of body-steps sit in the difference).

**What the online crosscheck changed** (`conformance-sequence.md` §2): the levelled tier's `avoid` gate is **backwards** - the
game's coarse search runs *and ignores moving bodies* (`FUN_007ee980`'s body has no stander insertion; openage: the mip-map
"ignores moving objects, e.g. units or wildlife"); HD's polygonal pather is the **legacy hull algorithm** (slides 21-38), not
the DE-era edge walk the skill had listed; and the mate exemption belongs to a **formation**, which villagers never join.

## The next three steps, ready now

1. **P15 - a witness for the 20,000 cap** (S, no blocker). A ~1,140-tile sealed map so `25 x euclid > 20,000`, asserting the
   **count** (`expansions == 20_000`), not a bound recomputed from `estimate`. Today the row spends 642 of 20,000 and stays
   green with `EXPANSION_CAP = u32::MAX`.
2. **P13 - the frontier's key `g + octile`** (M). Measured A/B: 28.9 expansions/search and 54% of searches exhausting the
   budget, against 11.8 and 5.8%. It owes a witness - a scene the two keys answer differently, red on the octile key before the
   change - then the full suite and an acceptance run, because it moves every route and the match changes shape
   (`players=1` -> `players=2`).
3. **P14 - drop the tier's `avoid` gate, then re-test the 16-tile floor** (M). With the gate lifted the tier answers 6 of 121
   long walks and the hash does not move, so the witness must be a `World::plan_path` walk whose route *is* the hierarchy's
   (the conformance row calls `Levels` directly and cannot see a gate). The 16-tile floor's removal was measured in D322 as a
   7-point staircase - re-measure after step 2.

## Also ready, independent of those

- **P16 - the body rule as boxes** (L): four `<=` on AABBs, with the arrival arithmetic in the same slice
  (`reach_between`, `target_contact`, `body_touches`). The probe `collision_probe::what_a_box_would_refuse_that_a_disc_takes`
  is the witness (4,741 steps -> 0).
- **P17 - waypoint substitution** (`mConsecutiveSubstituteCount`): a waypoint a re-plan keeps returning is skipped. Nothing in
  the tree does it.
- **A waiting walker is an obstacle**: `standing_bodies` skips any body with a route; the binary's test is velocity zero.
- **The 300 row** (an object of type 8/`0x20` on or beside the cell, not "a blocked neighbour") and **the refinement's scan
  direction** (outward from the interpolated point): both reads are done, each is a small slice.

## Needs the owner

- **The mate rule (sequence step 11)**: villagers never join a formation, so a villager crowd is solid in the game and ours
  walks over itself when ordered in one tick. Either a formation model, or villagers solid with the pause carrying the jams,
  or the current rule kept and recorded as a deviation. This is the only fork left that the sources cannot settle.
- **An evenly mirrored skirmish** (unchanged): player 2's barracks order is at 70 s where player 1's is at 320 s.

## Parked on what this box does not have

- **P3's inset** (`+0x268`), the **100/600 terrain bits**, the **octile table's values** (`DAT_00afd2a0` is runtime storage),
  the **search window's padding** (D299), and the **straight-line tier** (the caller of `FUN_008000d0`). The **equal-`f` sift
  order** is *not* parked - `FUN_00801e90` is in the corpus and is one read.

## What is *not* owed, and must not be redone

- **The octile cost model** (16 / 23 / the table): read, consistent; the 23-per-diagonal is `16 x sqrt2` rounded and the table
  is 23 per minor-axis unit, so `16*max + 7*min` is the same function (the binary caps the minor axis at 479, we do not;
  unreachable on 64 x 64).
- **The budget's answer on exhaustion**: settled - the binary pushes the saved best with cost 0 and **returns**
  (`0x801855`-`0x80189d` is the epilogue, then the return). `settled_best` is that rule.
- **The stander-inserter's own rule** (`FUN_007f9d80`: standers only, per search, grown by the mover's radius) and the
  `FUN_00801a20` fail-closed body test: read and pinned by D324. What is still open is *which* search it is called for.
- Any further work on the four levels, the descent, the refinement or the cost table: landed, measured, gated.
