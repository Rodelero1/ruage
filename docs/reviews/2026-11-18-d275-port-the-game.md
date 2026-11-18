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
