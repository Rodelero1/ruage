# Review: scenario ADRs 0370–0371 (2026-11-25)

Docs-only slice. Code frozen; no `crates/`, `config/`, `tools/` touched.

## What landed

- `docs/adr/0370-scenario-scope.md` (D15, scenario half), `0371-script-harness-split.md` (D198). MANIFEST extended.

## Performance

Zero engine diff by construction: only `docs/adr/` paths staged. Suite not runnable on this box; trunk gate stays owed.

## Stability

Attacked: (1) D15 split honestly — map + opponent clauses migrated, A* clause parked to movement with the reason in both the ADR and the manifest; (2) D198's measured hash (`91572e72c33cee93`) and gate numbers quoted from the row, not re-measured; (3) rung-2 correctly empty for both — no padding citations; (4) alias uniqueness re-checked.

## Accepted / refused

- Accepted: two drafts, pending owner sign-off.
- Refused: nothing.
