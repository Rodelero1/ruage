# Review: the owner rules the game's pathing in wholesale, and the queue is written to match

*Change: **D275** ("port everything from the game; optimisation later") plus the queue it forces in `docs/next-step.md`, the
filing of D254-D275 in `docs/decision-areas.toml`, and **D272**, a row that `crates/sim/src/world/movement.rs:572` cited and
the corpus never defined. **No code changed** - this is the "compact first" half of the step, and it owes the note because a
docs change changes what the next session builds.*

## Performance

**Nothing in the tick can move: no file under `crates/`, `config/` or `tools/` was touched.** The numbers the decision and the
queue are *priced against* are unchanged and were re-read from the corpus rather than recalled:

- the long-route search costs **50.5 -> 8.9 us** corner-to-corner after D143, **5.2 -> 2.0 us** mid, **1.2 -> 0.9 us** short;
- the acceptance match runs **188 searches**, about **0.04 s of an 11.5 s** run - which is why porting row 13 is priced as a
  *shape* change and not a speed one;
- the conformance set stands at **28 green / 5 red** (`docs/pathfinding-conformance.md`), and the register's movement rows
  13-36 are the port list.

The one place this note could be wrong is the `3.5 ms/tick against 0.3` that justified `MAX_GROUND_REPLANS = 2` in D127/D135:
D267 withdrew it as a reason, and D275 now withdraws it as a licence. It stays in the corpus as a measurement to beat, not as
a rule, and the slice that deletes the cap re-measures it.

## P0 - the termination guard, read (the first slice, done inside this step)

The queue's first item was a read with no code, and it closed: `FUN_008007f0`'s break is **`expansions > h`** (the heuristic,
`25 x Euclidean`) with an absolute cap of **20,000**, and both jumps land on the epilogue an **empty frontier** uses - so the
original's search is **best-effort by construction**: when the budget runs out it answers what it has, which is the mechanism
behind *"otherwise give the closest point path"*. Verified at the site (`0x800c58` zeroes the counter, `0x80177a` increments
it, `0x800d40`-`0x800d5d` compare, `0x801855`/`0x80185b` are the shared tail) and written up in
`docs/movement-in-the-binary.md`. **My earlier claim that this was "a unit mismatch in the guard or a misread variable" is
withdrawn** - it is a budget, and the odd pairing is the design. What remains unread is what the epilogue *writes* and the
heap's equal-`f` tie-break; P1 carries both.

## P1 - measured before building: the cost model is already in `main`

The queue put the cost model second, so the first thing checked was what the tree already has. `crates/sim/src/path.rs`
**already carries the ported model** - `CARDINAL = 16`, `DIAGONAL = 23`, `ESTIMATE_WEIGHT = 25` with `h = Euclidean x 25` -
landed in `16853ac` (*"wip: follow the game - the search answers the closest point, and 16/23 with the weighted estimate"*),
and it already settles on the **closest point** rather than answering `None`. Two of the register's red rows were therefore
green before the slice began, and the doc that said otherwise (`docs/pathfinding-conformance.md`'s *"28 green, 5 red"*) was
stale in two directions: the set has since been reorganised into seven files, and the current run is different.

**Measured, `make conformance`, 2026-11-18: 15 tests, 14 green, 1 red.** The red is
`divergences::the_original_solves_a_long_walk_in_its_hierarchy` - `short_range` declines a 58-tile walk at its 40-tile window,
which is the levelled search we do not have. That is now the next step, and it is the only thing in the set that fails.

**A claim of mine withdrawn, from P0's own write-up.** P0 said the guard's two breaks jump to *"the same epilogue the exhausted
frontier uses"* and therefore that a budget-exhausted search returns what it has. Reading the target shows it **pushes a node
onto the frontier** (`0x801861`-`0x80189d`, then the sift at `0x801e90`), not a return - so the termination *rule* is settled
and the answer's *shape* is not. The budget is therefore **not ported**, and the doc says so rather than the inference.

## P1 corrected - the count I first published was a floor, not a total

The first run of the set was `make conformance`, which **stops at the first failing binary**. It printed 15 tests, 14 green,
1 red, and that is what I reported. Re-running with `--no-fail-fast` gives **36 tests: 34 green, 2 red** - four of the seven
test files had not run, and one of them held a second red.

**The hidden red, and it was informative**: `walkability::open_ground_costs_the_octile_distance_and_no_more` asserted that an
open-ground route costs **exactly** the octile distance. That is the rule of an **admissible** A\*, and the ported search is
not admissible - it carries the game's weight of 25, so it returns longer routes on purpose (measured: **509** against an
octile of **473**, ratio 1.076). The fixture was rewritten to the game's guarantee, `octile <= cost <= octile x 25/16`, with
`FUN_008007f0` cited in its header and the measurement in its doc comment; it passes, and the set is now **36 tests, 35 green,
1 red** - the levelled search.

**This is the exact trap `AGENTS.md` names**, and I walked into it while quoting the rule: a failure list read off a run that
stopped early is a floor. The count to quote is the one taken with `--no-fail-fast` after `grep -c '^test result'` matches the
number of test binaries (9 here: 7 files, a lib and the doc-tests).

## Stability

**What was attacked, and by which check:**

- **The scope claim.** "Port everything" is only meaningful if the list is finite, so it was rebuilt from
  `docs/behaviour-diff.md` rows 13-36 rather than from memory: 13 (cost model), 20 (mate trigger), 21 (passage), 22 (ground
  re-plan cap), 23 (exclusive goal), 25 (node ring), 26 (substituted waypoint), 27 (two levels), 29 (`TakeAtRest`), 30 (the
  avoid set in the search), 32 (a *thing* whose route fails), 33 (start re-seed), 36 (the path vocabulary). Rows 18, 19, 24
  and 28 are already `same` and are not in the queue.
- **A blocker that had already been cleared.** The draft of this decision carried "row 13 is blocked because 16 against
  2500/100 is not admissible". Reading `docs/movement-in-the-binary.md`'s *"The cost model settled (2026-11-16)"* refutes it:
  **16 is the step cost and 25 is the heuristic weight**, so the original is a weighted A\* (weight ~1.56) that returns longer
  routes on purpose. The frontier is a **heap beside** the stamped array, not one or the other. The one item still unread is
  the termination guard `h < local_46c || 20000 < local_46c`, and it is now step **P0** rather than a footnote.
- **A stale claim in my own skill reference.** `references/movement-pathing.md` still says *"One of the two is wrong; do not
  port the pair until it is settled."* It is settled; the reference now says so, and carries the guard as the open item. A
  reference that blocks a port the owner has ordered is worse than no reference.
- **The corpus's own gate.** `tools/decisions.py --check`: **281 decisions**, 0 unclassified headings, **D272 no longer
  dangling** (it was cited from code and defined nowhere), and the 21 decisions from D254 on now have their area lines - the
  index could not show any of them before, which is exactly the rot D166 exists to catch. `tools/next_step.py --check`:
  **4,553 bytes**, headed by `# OPEN`, after the split moved **16,222 bytes** to `docs/history/next-step-2026-11-18.md`.

**Found and reported rather than fixed:** the `[unknown]` table still files U25 - the four conformance reds - under
*Pathfinding & movement*, and U25's four sub-questions (a-d) have no rows of their own; they are answered inside the table in
`docs/pathfinding-conformance.md`. Naming that gap is a decision away and is not taken here.

Known and accepted: this note has no timing measurement because no executable changed. The queue's own gate
(`entities >= 155` / `repaths` near 380) is **still waiting on the owner** - the control reads 154 and never resolves while the
D250 build reads 137 and wins - so every slice in the queue measures against a floor that may be wrong.

---

*This file is what `tools/review_check.py` looks for: `## Performance` with a number, `## Stability` with content, staged
with the change. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
