# Review: decision-areas.toml hygiene — 29 retired ids removed (2026-11-28)

Docs-only slice. Code frozen.

## What landed

- Removed 29 orphan `[area]` lines from `docs/decision-areas.toml`: D4, D5, D6, D8, D9, D10, D12, D13, D15, D16, D17, D18, D21, D198, D296–D310 (incl. D300/D301).
- Every one is retired with a manifest trace (ADRs 0001–0016); none is defined by the bank anymore (`tools/decisions.py` parser: corpus 335, mapping now 335).
- `python3 tools/live_migrate.py --check`: 335 placed, zero problems (was 29 REFUSED lines). Full regen: 15 area files byte-identical, `decision-index.md` byte-identical (328 rows) — the entries mapped to nothing, so only the stderr noise changed.

## Performance

Zero engine diff. No suite re-run (no `crates/`, `config/`, `tools/`, `Age2HD/`, `build/` touched).

## Stability

Attacked: (1) orphan list derived from the parser itself (`corpus.decisions` vs mapping keys), not hand-picked — exact set of 29; (2) cross-checked each against `docs/adr/MANIFEST.md` — all signed/retired, D13's deletion noted in ADR 0003's row; (3) pending range untouched (D1–D3, D7, D11, D14, D19, D20, D23+ still mapped); (4) `--check` green, regen diff empty.
- Own errors this slice: 0.

## Accepted / refused

- Accepted: pure deletion, no re-filing. Next: owner compacts `docs/next-step.md` for the crowd/standoff tranche.
