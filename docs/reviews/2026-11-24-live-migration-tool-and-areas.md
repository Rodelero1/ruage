# Review: live migration tool + 15 area files (2026-11-24)

Tools change (`tools/live_migrate.py`, `tools/test_live_migrate.py`), so this note
carries the gate's two sections. Docs generated, no engine/config change.

## What landed

- `tools/live_migrate.py` — test-first (RED recorded: `ModuleNotFoundError`, then
  5/5 GREEN). Reuses `tools/decisions.py` parser + authored `decision-areas.toml`;
  refuses unfiled, keeps duplicate-id sections as amendments, preserves conformance
  pointers verbatim, flags game-claims-without-source as UNVERIFIED-needed.
- `docs/live/*.md` — 15 area files, 364 decisions, 0 refusals,
  15 amendment sections kept, 5 `Conformance needed` flags (matches the burn-down 5).
- `docs/live/README.md` status flipped scaffold → migrated (mechanical, unresolved).

## Performance

Migration runs in **0.4 s** (`--check`: 364 placed / 15 areas). Largest file
`pathfinding-movement.md` 340 KB — the shard a session loads only when touching
movement; hub stays ~2.5k tokens. Full `tools-test`: 216 tests, 10.5 s, my 5 green.

## Stability

Attacked: loss (a decision in no file), doubling (an id in two), silent bucketing
(unfiled under "Other"), merged duplicates (D88a/D88b collapse), dropped conformance
pointers. Guarded: `build_plan` refuses unfiled (test), amendments render both bodies
(test), pointers preserved verbatim (test), no-source game claims flagged (test).
`--check` on the real bank: 0 refusals, and file count reconciles
(364 placed = parser's decision count).
Pre-existing failures, verified on the clean tree via `git stash -u` and unchanged by
this slice: `test_decisions` wires-the-authored-areas (corpus has 9 dangling ids,
`main` returns 1) and `test_plan_check` dependency tree (`ruage-tools/...` path from
the repo split). `main()` glue is untested by unit test — verified by `--check`
output and `git diff --stat` instead, same as `decision_index.py`.
One own error to record: an early `ssh` probe created `/app/.ssh/`; removed, never
staged. Cargo unavailable in this box, so the Rust suites could not run — no Rust
touched.
