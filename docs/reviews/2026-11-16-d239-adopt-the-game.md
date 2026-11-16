# Review: D239 - every pathing divergence adopted from the game, and the register corrected

**What changed, documentation only.** `docs/decisions.md` gains **D239** (the owner's ruling of 2026-11-16: *"Resolve the questions
by adopting from the game. That is the source of truth."*), `docs/decision-areas.toml` files it, `docs/behaviour-diff.md` is
**corrected and extended** (rows 18, 20, 21 amended; new rows **29** and **30**; a section recounting the table), and
`docs/next-step.md` is replaced by the adoption queue **R1-R7** with the old open step archived to
`docs/history/next-step-2026-11-16-t16.md`. No `crates/`, `config/`, `tools/` or `build/` file is touched, so no hash moves and no
test count changes.

## Performance

**Nothing to measure, and no number invented.** No code moved. The two costs this ruling sends into a slice are the ones the
register already carries and neither was re-taken here: the uncapped re-plan price of **3.5 ms/tick against 0.3 ms/tick** (D127,
taken **before** the sub-tile pencil, which is why R6 re-measures rather than inheriting it), and the seat churn's **0.52 tiles**
at tick 756 (D238). R6's review must carry the re-measurement; a figure copied forward from a different search would be the
mistake this file exists to prevent.

## Stability

What was attacked, since the failure mode of a ruling is an untrue premise:

1. **Every corrected verdict re-read in the code, not in the prose.** Row 18's "first write wins" was checked against
   `step.rs:1270-1298` (the skip on `pause_remaining > 0`) and against `setPause`'s whole body in
   `docs/movement-in-the-binary.md` (`0x8aa6d0` writes `path+0x28` with no guard) - the row said `same` and the tree does
   something the original does not. Row 20's "stamped once, never re-tested" was checked against `is_mate`, which **does** re-test
   mutuality by route existence, and against `stander_tiles`, which already drops a body that moved within a tick: the missing
   half is narrower than the row claimed. Row 21 was checked against `step_is_walkable`'s `clearance_at(here, radius)` - the step
   layer has been the game's all along, and only the graph is D117's.
2. **Two rows that did not exist were found by reading the decision order rather than the register.** `collide`'s rule 6
   (`TakeAtRest`) and the `avoiding`-to-obstacle-tiles conversion (`step.rs:1325`) are divergences the sixteen-row batch missed,
   and the second one contradicts a sentence already in this repo's own reference material ("the re-plan avoids **standers**, and
   the classifier is what forgives a man it already knows"). A register that misses rows of its own subject is the defect; both
   are now rows 29 and 30.
3. **Counts recounted from the table, twice, by script.** The morning's section said 7 / 8 / 1 over sixteen rows. Parsing the
   `verdict` column gives **18 movement rows, 17 ruled: 6 same, 11 different, 1 unknown-static**, and **30 rows, 19 ruled** for
   the register. The superseded sentence is marked as superseded rather than silently rewritten, because this file's own rule is
   append-and-correct.
4. **The one inferred mechanism is labelled as an inference.** R3's seat book is his sentence plus the original's absence of
   displacement, **not** a reading; D239 says so, and the slice's first act is a test that pins the appearance. D238's "a seated
   body must not be past" is **withdrawn on the game's own evidence** - a stopped man *is* past under `createdAt >= lastMoveTime` -
   which is the opposite of what D238 concluded from a measurement, and is the honest reading of both.
5. **Nothing was adopted on a guess.** R7's two ports are gated behind reads for a measured reason: the estimate-versus-step-cost
   contradiction (2500/100 against 16) has no reading yet, and porting both would encode one of them wrong.

**Not done, and named rather than hidden:** no fixture was written, because no behaviour changed; the fixtures D239 names (the
walker past a stopped friend, the re-drawn pause, the newcomer with no seat) belong to R1-R3. Row 23 still needs the owner's
recording of forty units ordered to one tile - it is not decidable from anything in the tree.

---

*This file is what `tools/review_check.py` looks for: `## Performance` with a number, `## Stability` with content, staged
with the change. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
