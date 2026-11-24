# Live decisions — hub

**Status: scaffold, not yet migrated.** `docs/decisions.md` (10,206 lines) is still the bank. This directory will become the live truth; nothing here is authoritative until the conflict table is ruled on.

**Bank audit 2026-11-24:** 253 `##` headings (248 unique — D88/D89/D90/D167/D168 each appear twice, a 2026-11-01 numbering collision) + 16 table rows (D1–D16) + suffixed IDs (D24a, D44a, D46a, D50a, D58a, D59a/b, D94a, D98a, D200a, D210b) + split headings ≈ 264 headline rows, 345 indexed rows. Index areas are a keyword heuristic and do **not** read `decision-areas.toml` (see C14).

## Who loads what (agent sessions first)

1. This file — overview, ~2k tokens.
2. `docs/decision-index.md` — one line per bank row (345 rows, ~8k tokens). Grep the bank by ID from here.
3. One `docs/live/<area>.md` only when touching that area. Do not load all areas.
4. `docs/divergence-register.md` — the single live defect/OURS list (D352).
5. `docs/mechanics/` — dat facts + U-unknowns. Never duplicated here.
6. `docs/reviews/INDEX.md` — 314 notes, evidence only, never truth.

## Truth hierarchy (agreed 2026-11-24)

When sources conflict, in this order:

1. **Binary** (`AoK HD.exe`, `FUN_*`, `0x*`) — for behavior.
2. **Dat** (`empires2_x2_p1.dat` via `config/gamedata/`) — for numbers (D5).
3. **Save fixture** (`.aoe2spgame`, `crates/*/tests/`) — for outcomes.
4. **Owner ruling** — recorded as OURS, never as GAME.
5. **Community / prior art** — UNVERIFIED until cross-checked (openage, openaok, genie, `samuelcaldas/age-of-empires-2-decomp`, wiki, talk slides).

End state: zero live contradictions. See `conflict-table.md` for the ones still open.

## Strict row format (every live row)

```
### D<n> — <claim in one clause>
- **Status:** live
- **Conformance:** GAME - <pointer> / OURS - <why + who ruled> / UNVERIFIED - <what would settle it>
- **Source:** <FUN_* / dat field / save / URL / decomp file>
- **Fixture:** <crates/.../tests/foo.rs or explicit gap>
- **Commits to:** <what the engine must do>
```

Rules: no GAME without source pointer. No load-bearing UNVERIFIED. Measurements stay in reviews, only the rule lands here.

## Areas (from `docs/decision-areas.toml`)

Combat, Pathfinding & movement, Economy, Buildings & construction, Production & queues, Techs & civs, Vision & fog, Victory & defeat, UI, Graphics, AI, Simulation core, Data & tooling, Licensing & publication, Project process.

Live files will be `docs/live/<area>.md` with `Decisions / GAME / OURS / UNVERIFIED` sections inside (matrix: overview + detail without loading everything).

## Second-rung sources (2026-11-24 note)

- `samuelcaldas/age-of-empires-2-decomp` (AoC 1.0c / AoK 2.0a / PS2, no licence): behavior shape only, addresses are not HD. Any number needs an HD `FUN_*`. Already used for P11 zone layer (`docs/sources.md`, `docs/pathfinding-conformance.md`); silent on gather/work (see `docs/prior-art.md` 2026-11-17).
- `Zaydme/openaok` (MIT, 1999 AoK, 32.32 fixed at 60 Hz): design precedent (determinism, integer A*, AI interpreter, UI in `docs/ui-from-openaok.md`). Tick and version mismatch vs D16 20 Hz must be named on every cite.

Both are rung 2: below binary/dat/save, above folklore. Read for facts, write our own sentences.

## What is frozen

- `docs/reviews/` — immutable log. Correct by new live row, never by editing a note.
- `docs/decisions.md` — frozen as `docs/archive/` once migration lands. Until then, bank stays read-only except conflict-table rulings.
- Sprawl (`parity-plan.md`, `pathfinding-gap-register.md`, `pathfinding-crosscheck-*.md`, `gates-research.md`, `behaviour-diff.md`, `conformance-sequence.md`, `crosscheck-2026-11-19-rulings.md`, `movement-in-the-binary.md` except as evidence, `building-collision.md`, `press-depth-evidence.md`, `gather-seating-evidence.md`) → archive. Live defect list stays `divergence-register.md`.

## Done criteria (agreed)

1. `conformance_audit.py` 0 pending in `live/` + no GAME-without-source.
2. Every GAME has source, every OURS has owner substanciation.
3. Hub <10k tokens, each area file standalone.
4. `make decision-index / review-index` still pass.
