# OPEN - the adversarial review landed; four live rows are new and P13 is the one that changes every route

**State, measured this session (D334, `blunt-port`):** suite **933 / 0 / 19** (933 / 0 / 20 once the new `#[ignore]`d probe is counted), conformance **21 / 0 / 0** over seven binaries, acceptance `e248486298910e28` (127 entities,
1,727 re-paths, **10.04 s median of five on an idle box**). The review is
`docs/reviews/2026-11-21-pathfinding-conformance-adversarial.md`; the correction row is D334; the register is
`docs/parity-plan.md`, which now carries **P13-P16**.

**What the review changed in what we believe, not in what we run.** No engine line moved. Four claims in our own notes were
wrong and are corrected in place:

- the binary's frontier is ordered by `g + octile`; the `25 x euclid` we ported is its **expansion budget** (`FUN_008007f0`
  177, 198, 312-316). Five documents said otherwise.
- the levelled search answers **nothing** in a match: 121 of 121 long walks carry a stander and the `avoid.is_empty()` gate
  never lets the coarse search run; lift the gate and 6 answer, with the acceptance hash unchanged.
- the 20,000 cap has no witness: its row spends **642** of 20,000 and stays green with the cap set to `u32::MAX`.
- the body rule is a **disc**, the game's is a **box**: **4,741 of 218,920 body-steps (2.17%)** end in a pose whose boxes overlap
  and whose discs do not.

## The queue, in order, and what each owes

1. **P13 - the flat search's key: `g + octile`.** The number is measured (28.9 expansions/search and 54% of searches exhausting
   the budget against 11.8 and 5.8%); what it lacks is a **witness**: a scene the two keys answer differently, red on the octile
   key before the change. It changes every route, so it also owes a full suite run and an acceptance A/B. The A/B is already in
   the review: the match itself changes shape (`players=1` -> `players=2`, hash `9d5fea513f3f8efb`), so the witness decides
   whether that is a fix or a different game.
2. **P15 - a scene where the cap binds.** A map about 1,140 tiles a side (so `25 x euclid > 20,000`), a sealed goal, and an
   assertion on the **count** (`expansions == 20_000`), not on a bound recomputed from `estimate`. Cheapest of the four, and it
   retires a row that cannot fail.
3. **P14 - the levelled tier's two gates.** The 16-tile floor (D306) and the `avoid.is_empty()` clause are both ours. The open
   read is whether the binary's coarse search sees standers at all: if it does, the clause goes and the tier starts answering;
   if it does not, the clause is the game's and the row closes as parity with a measurement. Do not remove the clause before
   that read - it would be adopting a rule on the strength of wanting the tier to run.
4. **P16 - the body rule as boxes.** Four `<=` on two AABBs where `refusal` tests one distance. It lands with the arrival
   arithmetic (`reach_between`, `target_contact`, `body_touches`) or the fixtures will not converge, and the probe
   `collision_probe::what_a_box_would_refuse_that_a_disc_takes` is the acceptance witness (it should fall to 0).

## Ready after one short read each

- **P3's inset** - `FUN_00840830`'s `+0x266`/`+0x268`; the inclusive test cannot land without the number, and the one-tile-gap
  fixture (D306) has to stay green through it. Unchanged by this review: our `object_depth > 0` is still the same rule from the
  other side, and `conformance/contact.rs` still cannot see the step.
- **The straight-line tier** (#10 of the crosscheck): the caller of `FUN_008000d0`, blocked by the build mismatch unless the
  caller is inside the corpus (grep `007ef140` callers).
- **P12's per-edge mask**: `docs/pathfinding-conformance.md` reads it as a derived cache and says nothing needs porting;
  `parity-plan.md` says UNREAD. One of the two is stale and the read settles which.

## Needs the owner

- **An evenly mirrored skirmish** (unchanged): player 2's barracks order is at 70 s where player 1's is at 320 s, and the file's
  own comment explains why. A mirror is a designed opening to be measured for affordability, not a tidy.

## What is *not* owed, and must not be redone

- **The octile cost model** (16 / 23 / the table): read, consistent, and confirmed - the 23-per-diagonal is `16 x sqrt2` rounded
  and the table is 23 per minor-axis unit, so `16*max + 7*min` is the same function (the one nuance: the binary caps the minor
  axis at 479, we do not; unreachable on 64 x 64).
- **The budget's answer on exhaustion**: settled this session - the binary pushes the saved best with cost 0 and **returns**
  (`0x801855`-`0x80189d` is the epilogue, followed by the return). `settled_best` is that rule.
- **The stander-in-the-search rule** (`FUN_007f9d80`, standers only, per search, never in the shared grid) and the
  `FUN_00801a20` fail-closed body test: read, matched, and pinned by D324.
- Any further work on the four levels, the descent, the refinement or the cost table: landed, measured, gated.
