# Review: bank archived + citation-scope and collision-heuristic fixes (2026-11-24)

Tools + docs-layout change, so gate sections apply. No engine/config change.
Bank bytes untouched by the move (`git mv` only; index regen diff is one header line).

## What landed

- `docs/decisions.md` → `docs/archive/decisions-2026-11-24.md` via `git mv`.
  Repointed: `decisions.py` (DECISIONS, `committed_corpus`, `--decisions` default,
  messages), `decision_index.py`, `conformance_audit.py`, `live_migrate.py`,
  `plan_check.py`, `test_decisions.py` paths, `test_plan_check.py` docstring,
  Makefile comments, `review_index.py` docstrings. `decision-index.md` regen:
  345 rows, byte-identical except the header path — the move is lossless.
- `AGENTS.md`: compact routine and index paragraph point at `docs/live/` +
  frozen bank. `SPEC.md` §12 row, layout, D5 pointer, decision-status line updated.
- Fix 1 (my bug, caught by regen output): `section_titles` counted amendment
  subheadings as collisions — D186 would have rendered D186a–e. Now filters by the
  parser's own `_heading_kind == "decision"`. RED-recorded, GREEN same session.
- Fix 2 (my fallout): generated `docs/live/` excluded from citation scope in
  `_in_scope`, same rationale as the existing `site` exclusion — live collision IDs
  (D88a…) read as dangling otherwise. Archived bank stays in scope.
- `docs/site/index.html` NOT regenerated: the site build exits 1 on 9 pre-existing
  dangling ids (verified identical on the clean tree via `git stash -u`), and it
  writes nothing in that state — so no drift, and fixing those 9 is a separate slice.

## Performance

Full tools suite: 223 tests (+7 this slice-file), 10.6 s. Migration 0.4 s, 364/15/0.
`conformance_audit --check` 0. Index regen one-line diff.

## Stability

Attacked: silent content change in the move (index regen proves byte-identical
content); shrink-detection gap across the move commit (`committed_corpus` reads the
new HEAD path, which the pre-commit HEAD lacks → baseline vacuous for exactly one
commit, then whole again — no false alarm, noted here); over-broad collision
heuristic (false D186a–e, fixed); dangling-scan noise from live IDs (fixed by scope
exclusion); stale `site/index.html` (left as-is, build already red pre-change).
Own errors: the unreachable-else repeat — first the OWNER/TAG branch, then assuming
`--check` output meant the site file wrote (it doesn't on exit 1); two `edit`
oldString misses from drafting too fast. Same 2 pre-existing failures before/after
(areas-wiring, plan dependency tree); `main()` glue still verified by `--check`.
Cargo unavailable; no Rust touched.
