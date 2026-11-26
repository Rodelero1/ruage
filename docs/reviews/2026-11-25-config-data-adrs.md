# Review: config/data ADRs 0368–0369 (2026-11-25)

Docs-only slice. Code frozen; no `crates/`, `config/`, `tools/` touched.

## What landed

- `docs/adr/0368-dat-authoritative.md` (D5, with U11 farming as applied instance), `0369-config-layer.md` (D6). MANIFEST extended.

## Performance

Zero engine diff by construction: only `docs/adr/` paths staged. Suite not runnable on this box; trunk gate stays owed.

## Stability

Attacked: (1) D5's falsified construction hypothesis carried as falsified (table kept), not dropped — a future reader must not re-try it; (2) U11's 25–65% cost-of-wrong and cheap settler carried verbatim; (3) D6's later paydowns (D45, D50, D137) named as built-on, not absorbed — their rows stay pending; (4) wiki cited only as losing side of a dat comparison, never as authority; (5) alias uniqueness re-checked.

## Accepted / refused

- Accepted: two drafts, pending owner sign-off.
- Refused: nothing.
