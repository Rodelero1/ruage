# Review: the search was already bounded, and the queue fallback has nowhere to fire

*Change: documentation and one withdrawn fixture. Two debts above the reseed queue were checked against the code instead of
re-read — `T7`'s unbounded search does not exist (`gather_reach` uses the dat's `search_radius`, or sight + 5 tiles), and the
builder's queue fallback has no reachable stuck state in this model, so it was **not** implemented. D225.*

## Performance

**Nothing in the simulation changed**, which is the point of the number: the tree is byte-identical to `d5d920e` apart from
docs (`git diff --stat` on `crates/` is empty), so the gate's **870 passed / 0 failed / 20 ignored** over 190 binaries is the
same measurement, not a new one. The one *cost* this saves is the slice that was about to be spent: an unbounded-search fix
would have changed `nodes_of_slot`'s bound, which every share decision calls per villager per tick — the exact place where a
misread bound shows up as either a villager walking the map (no bound) or a worker refusing a node two tiles away (a bound too
tight), and both shapes have already happened once in this tree.

## Stability

- **The search bound is the original's own field, and it is already in use.** `gather_reach` returns
  `search_radius x scale` when content carries the dat value (a villager is **4** tiles in the dat), and
  `sight + LOOK_AROUND_TILES (5)` otherwise. `nodes_of_slot` skips any node whose squared distance exceeds it. The debt note's
  "walks the whole map" describes neither path.
- **What is missing is a guard, not a rule**: the note's "at most 30 reach tests". With a four-to-seven-tile reach the candidate
  set is a handful, so a cap would be unobservable - and an unobservable cap is exactly the kind of code that cannot be
  mutation-proved.
- **The queue fallback has no trigger, and four candidates were tested rather than argued**: own bodies are not walls to their
  own side (D189); a visible enemy on the footprint is refused at placement (`SiteOccupied`, measured when the fixture was
  written); an unreachable site is refused before the wood is spent; and the one remaining construction — a blind placement in
  fog over an enemy **unit** — is not known to freeze at all, because `settle_blind_sites` cancels only on an enemy *building*.
  Whether that last case stalls is a measurement, not a supposition, and it is not taken here.

Known and accepted:

- **The owner's "2 a" ruling is therefore half-built on purpose**: the queue advances when a site *finishes* (D66), and does not
  advance when a builder is stuck, because nothing in this model can stick him.
- **`T7` stays in the tree with a smaller claim** - the cap, not the bound - so the next reader is not told a mechanism is
  missing when it is present.
- **The withdrawn fixture is gone from the tree**, not left red: `SiteOccupied` at placement was the measurement that killed it.

---

*This file is what `tools/review_check.py` looks for: `## Performance` with a number, `## Stability` with content, staged
with the change. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
