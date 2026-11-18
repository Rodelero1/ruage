# Review: the pathfinding gap register - extra, missing, different, and the plan

*Change: `docs/pathfinding-gap-register.md`. The original's pathfinding model in one table, our divergence split into **nothing
extra / nothing missing / nothing different**, and an ordered plan. **No code changed.***

## Performance

**Nothing executable changed.** The numbers the register is priced with, all measured in this tree today: `main` **131 entities,
`players=1`**, 551 re-paths; the working branch **150 entities, `players=2`**, **149,408** re-paths of which **149,254 are
ground**; the movement suites animals **10/1**, gather **22/0**, gather_motion **6/0**, gather_standoff **1/0**; the conformance
set **36 tests, 35 green, 1 red**; the ground refusals counted by task **29 of 29 `Gather`**.

## Stability

**What was attacked.** Three ways a register like this goes soft, and what the file does about each:

- **Inventing rows.** Every line names its source site or its decision; where a rule is read only from the decomp's *reconstructed
  C* the register says so (the C is lossy - it dropped the `0x4000` stamp test).
- **Hiding a divergence behind "implementation detail".** The four-way split forces the uncomfortable rows into the open: the
  `<=` collision, the pause never re-drawn, the moving-mate re-test, and the ring-vs-box arrival are all *behaviour*, not style.
- **A plan that cannot be checked.** Each step carries the fixture that must fail first and the gate it answers to.

**Found while writing it:** two rows I had treated as settled are not. `TakeAtRest` and rule 3 are both **extra**, and both were
kept for measured reasons that were measured *against our own defect* - deleting them individually made overlap worse because
nothing yet replaces them. The register says so in the row rather than dropping them from the list.

Known and accepted: the plan's step 1 is where the current branch stopped, and the storm is still unexplained there. This document
does not claim to close it; it claims the difference is now counted.
