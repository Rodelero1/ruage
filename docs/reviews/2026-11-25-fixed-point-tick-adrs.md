# Review: fixed-point and tick ADRs 0365–0367 (2026-11-25)

Docs-only slice. Code frozen; no `crates/`, `config/`, `tools/` touched.

## What landed

- `docs/adr/0365-fixed-point-domain.md` (D9), `0366-scale-grains.md` (D10), `0367-tick-rate.md` (D16). MANIFEST extended.
- Ruling: the tick-loop/state-hash/command-apply shape (v0.17, never numbered) is parked to the simulation-core branch as unnumbered debt — this slice migrates only numbered rows.

## Performance

Zero engine diff by construction: only `docs/adr/` paths staged. Suite not runnable on this box; trunk gate stays owed.

## Stability

Attacked: (1) D9's superseded clause carried as superseded, not silently dropped — ADR 0365 names ADR 0366 as its amendment; (2) D10's measured numbers quoted with their sources (units 110/1021, speed 0.737), not rounded or re-measured; (3) rung-2 boundaries honest — genie's f32 cited from the bank's own record as the G1/G2 boundary, openage's wall-clock cited as contrast-per-D14, U12 marked corroborated-but-unproven rather than promoted; (4) alias uniqueness re-checked.

## Accepted / refused

- Accepted: three drafts, pending owner sign-off.
- Refused: nothing. Prior slices unchanged.
