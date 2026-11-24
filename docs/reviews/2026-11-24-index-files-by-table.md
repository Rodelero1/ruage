# Review: decision index files by authored areas (2026-11-24)

Tools change, so gate sections apply. No engine/config change. Completes C14:
the TOML wins everywhere — live files already filed by table, now the index does too.

## What landed

- `decision_index.py`: deleted the keyword heuristic (`AREAS` + `area_of`);
  `parse()` no longer guesses areas; new `assign_areas` (table lookup, `Unfiled`
  fallback) + `group_entries` (table order, `Unfiled` tail). `main()` loads the
  TOML via the bank parser (single-sourced) and prints the new header line.
- `test_decision_index.py`: replaced the heuristic-pinning test with the TOML
  contract (table wins, unfiled visible + last). RED-recorded (2 errors), GREEN
  same session; the other 5 tests untouched and passing.
- `docs/decision-index.md` regenerated: 345 rows, counts byte-identical
  (337 silent / 5 no-source / 249 no-fixture / 11 superseded), 10 heuristic
  buckets → 15 authored areas, **no `Unfiled` section** — the table covers every
  indexed row, which is itself the verification.

## Performance

Index builds in <1 s; page stays 35k characters (~8k tokens). Full suite: 224 tests
(+1 net), 11.6 s.

## Stability

Attacked: silent re-bucketing (a row landing in a plausible-but-wrong area — the
failure the old scored heuristic was built to avoid, and the new code avoids by
construction: no keywords remain); `Unfiled` rows hidden (they render as their own
trailing section instead); `main()` name slip (`groups` → `grouped`, caught by the
first regen run, fixed); stale header prose (updated). Own error: my replacement
test initially passed `order` to `assign_areas` against its signature (fixed before
GREEN). Same 2 pre-existing failures before/after (areas-wiring, plan dependency
tree); `main()` glue verified by regen output. Cargo unavailable; no Rust touched.
