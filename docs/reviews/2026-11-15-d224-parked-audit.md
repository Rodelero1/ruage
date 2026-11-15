# Review: the parked set was audited, not trusted

*Change: the four "live defects" in the gap report were **run** (`--include-ignored`) instead of read. Three are not defects —
`depletion` never carried an `#[ignore]`, `retarget` (D98a) and `carrier_no_pushing` (D204) both pass — and the fourth,
`gather_contact` (D181), was a fixture measuring to a tile's centre instead of its square. One test file corrected, three
un-parked, **no production line changed**. D224.*

## Performance

**Zero simulation cost**: the diff is one test file and three `#[ignore]` removals; `cargo test --workspace` goes from 867
passed / 23 ignored to **870 passed / 20 ignored** for the same wall-clock (the three un-parked fixtures cost **1.19 s, 5.75 s
and 1.09 s**). The audit itself cost the machine the sweep it had never had: `collision_probe` **378 s**, `movement_worth`
**349 s**, `acceptance_economy` **94 s**, everything else under 25 s - about **14 minutes** once, to delete four slices of work
that were not there.

## Stability

- **The risk this change removes is a false work queue.** `docs/next-step.md` listed three defects that do not exist and one
  that was a measurement error; the next session would have "fixed" working code, and the most likely shape of that fix is a
  gate tightened until a correct engine looks wrong - which is exactly what D181's parked sentence proposed.
- **The one real finding is arithmetic, not opinion.** A node occupies a square; a body touching its tile's corner is
  `sqrt(2) x (0.5 + 0.22)` = **0.93** from the centre, and D181 recorded "took while 0.92 tiles from the bush". The fixture now
  asserts the rule the engine states (`distance_to_tile <= body + scale/50`), and it **bites**: with the gate widened to any
  adjacent tile it reports **0.47** tiles off the tile.
- **What is *not* claimed.** The audit did not bisect which change made `retarget` pass, so the un-parked note says "measured
  green today" and not "fixed by X". `retarget`'s and `carrier_no_pushing`'s own parked causes may still describe real faults
  that the fixture no longer exercises - that is a *fixture coverage* question, recorded, not a fix.

Known and accepted:

- **Three `#[ignore]`s were the only thing making the tree look like it had live defects**, so an `#[ignore]` should now be
  treated as a claim to re-measure, not evidence. This is in the skill's hard invariants.
- **The `perf` and `stability` probes were not run in the sweep** (`stability` is four simulated hours), so "every parked test
  is green" is a statement about the deterministic set only.

---

*This file is what `tools/review_check.py` looks for: `## Performance` with a number, `## Stability` with content, staged
with the change. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
