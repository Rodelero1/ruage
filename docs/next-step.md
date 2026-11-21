# OPEN - steps 1 and 2 of the sequence are landed; step 3 is the tier's gate, and the acceptance wall is a named perf item

**Where this stands (D337, `blunt-port`):** suite **934 / 0 / 20** over 214 result lines, conformance **21 / 0 / 0**,
acceptance `527bf4ebb9520fa0` (132 entities, 41,473 re-paths, **15.70 s** median of three on an idle box; the key-only build is
`9d5fea513f3f8efb` at 9.72-9.88 s). The *order* to 100% conformance, crosschecked against the first-party talk and openage, is
**`docs/conformance-sequence.md`** - read it first; this file is the immediate step and the outstanding forks.

**What the review established** (details in the note and D334): the binary's frontier is ordered by `g + octile` and the
`25 x euclid` is its **expansion budget** (six files said otherwise); the levelled search answers **nothing** in a match
because all 121 long walks carry a stander; the 20,000 cap has no witness; body collision is a **disc** where the game's is a
**box** (2.17% of body-steps sit in the difference).

**What the online crosscheck changed** (`conformance-sequence.md` §2): the levelled tier's `avoid` gate is **backwards** - the
game's coarse search runs *and ignores moving bodies* (`FUN_007ee980`'s body has no stander insertion; openage: the mip-map
"ignores moving objects, e.g. units or wildlife"); HD's polygonal pather is the **legacy hull algorithm** (slides 21-38), not
the DE-era edge walk the skill had listed; and the mate exemption belongs to a **formation**, which villagers never join.

## The next three steps

1. ~~P15 - a witness for the 20,000 cap~~ **DONE (D336)**: 1,200 x 1,200, estimate 42,300, `expansions == 20_000` asserted,
   mutation-proved.
2. ~~P13 - the frontier's key~~ **DONE (D337)**: `g + octile`, witness 312 -> 294 on one tree, and a mover rounding hole fixed
   in the same slice (801.295 raw -> within 1 of the step).
3. **P14 - drop the tier's `avoid` gate, then re-test the 16-tile floor** (M, next). The game's coarse search runs *and ignores
   bodies* (`FUN_007ee980`'s body has no stander insertion; openage: "it ignores moving objects, e.g. units or wildlife"), so the
   gate is backwards. With the gate lifted the tier answers 6 of 121 long walks and the hash did not move *then* - re-measure on
   the new routes, and make the witness a `World::plan_path` walk whose route has a level-3 boundary crossing (the conformance
   row calls `Levels` directly and cannot see a gate).

## The perf item this slice created

- **The acceptance wall: 15.70 s against 9.72-9.88 s key-only, on a different match** (132 entities, 41,473 re-paths in both
  cases; the sqrt helpers account for 0.16 s). Profile the 15.7 s run **per pass** before touching anything, and price the
  per-search allocations at the same time (49 KB a call, ~2.2 GB a match at 45k calls) - the binary's generation-stamped array
  exists to avoid exactly that.

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
