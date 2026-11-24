# Review: docs live hub + conflict table (2026-11-24)

Docs-only slice. No engine, config, or tooling behavior changed.

## What landed

- `docs/live/README.md` — hub: who loads what, truth hierarchy (binary > dat > save > owner OURS > community), strict row format, 15 areas, frozen sets, done criteria.
- `docs/live/conflict-table.md` — 12 conflicts drafted for owner ruling (C1–C12): standoff (D285/D199 vs ring rows), seat/pile (D289/D293/D294/D357), cost table (D327/D328/D342 vs D298), tier floor (D346–D348), body solidity (D200/D281/D290/D339), defeat chain (D59b), housing (D34/D157 + U24), garrison (D37/D38), gather search (D150/D175 + U22), D240 one-time break, blunt-copy (D279/D330), 5 unsourced GAME rows → UNVERIFIED.

## Performance

Hub is ~2,100 tokens (measured `wc -w docs/live/README.md` = 378 words). Conflict table ~1,100 words. Bank stays 10,206 lines / 345 rows (`decision_index.py --summary`: rows=345 silent=337 game_no_source=5 no_fixture=249 superseded=11). No load-time cost: generators untouched.

## Stability

Attacked: hub citing a source the tree does not have; conflict proposal silently resolving an owner ruling; live files being mistaken for truth before ruling. Guarded: hub header says "scaffold, not yet migrated"; every owner-ruled row (D289/D293/D330/D346) is marked needs-owner; no `docs/decisions.md` row edited, no sprawl file moved, no index regenerated. `conformance_audit.py` still reports pending 0 on the bank (unchanged scope). Failure not covered: a wrong FUN_ pointer copied into the table — mitigated by requiring source + fixture per live row before migration.

Accepted: sprawl audit findings (divergence-register is live, gap-register superseded). Refused: none in this slice.
