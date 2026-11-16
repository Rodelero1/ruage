# 2026-11-16 - the movement rows of the behaviour register, ruled

**What changed.** Documentation only: `docs/behaviour-diff.md` gains rows **13-28** (pathfinding and obstacle avoidance) and a
section, *"The movement rows, ruled (2026-11-16)"*. No `crates/`, `config/`, `tools/` or `build/` file is touched; no code
changed, so no hash and no test count moves.

## Performance

**Nothing to measure, and nothing invented.** A register row is a claim about two implementations, not a change to either. The
only numbers in this note are the ones the rows already carry from their own measurements: the uncapped-replan cost **3.5 ms a
tick against 0.3** (D127/D135, the reason a cap exists at all), and the standoff census taken off the owner's saves (0.641 /
0.702 x3 / 0.935 x3 round a tree, mine and bush - D199), which is the evidence row 28 rests on.

## Stability

What was attacked, since a register's failure mode is a wrong verdict:

1. **Every verdict re-checked against the primary source, not against a summary.** Row 13's constants were read in
   `crates/sim/src/path.rs:16-17` (`CARDINAL = 10`, `DIAGONAL = 14`) and in the binary (`FUN_007ffd10`'s
   `table[min] + 16*(max-min)`) in the same session; row 18's implementation was read at `crates/sim/src/world/step.rs:1276-1298`
   rather than inferred from D205, which is why the row says **same** where D205's own text says "not landable" - the pause half
   did land and the decision text had not caught up.
2. **The four-verdict rule applied honestly.** One row is `unknown-static` (the exclusive goal, D52) because the original has no
   counterpart to it and its *effect* cannot be separated from the collision answer without a recording. Calling it "different"
   would have been the more satisfying answer and the less defensible one.
3. **Counts checked twice.** The first draft of the section said "28 rows, 16 ruled" and "nine same, six different"; the table
   actually carries 7 same, 8 different, 1 unknown-static across rows 13-28, for 18 of 28 ruled. The correction is in the file,
   and it is named here because this project has been burned by a misreported count before.
4. **The owner's own statement admitted as evidence.** Rows 24 and 25 rest partly on his sentence of 2026-11-16 (a newcomer
   "will be walking around until he finds a spot for him"), which outranks our reads on the one thing it covers: what the game
   looks like when a crowd forms at a bush. It does not settle the *mechanism* (placement vs collision), and the rows say so.
5. **No code copied.** The corpus is read for facts; nothing from it entered this tree, and no OSS code is quoted beyond
   fragments already recorded in `docs/prior-art.md` under their licences.

**Not done, and named rather than hidden:** no test was written, because no behaviour changed. Rows 24 and 25 are the ones with
a defect behind them; their tests belong to the slice that fixes them (D238's next step), not to this note.
