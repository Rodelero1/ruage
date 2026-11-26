# Review: layout ADRs 0363–0364 (2026-11-25)

Docs-only slice. Code frozen; no `crates/`, `config/`, `tools/` touched.

## What landed

- `docs/adr/0363-four-layers.md` (D17), `0364-target-shape.md` (D18). MANIFEST extended.
- Ruling: toolchain (SPEC §8) and build-recipe rule need no separate ADR — the build rule is inside ADR 0359 (nothing derived is committed) and the toolchain is environment fact retained in SPEC per Q13.

## Performance

Zero engine diff by construction: only `docs/adr/` paths staged. No numbers can move. Suite not runnable on this box (no Rust toolchain); trunk gate stays owed.

## Stability

Attacked: (1) bank rows read in full before writing — D17's paid-down `ResourceKind` line and its still-open `script.rs` line both carried, neither dropped; D18's verified/unverified split kept, unverified part marked `unproven`; (2) rung-2 honesty — decomp module layout and openaok structure marked unread rather than cited; slides correctly out of scope (pathfinding-only role); (3) alias uniqueness — D17/D18 appear once each in headers and manifest.

## Accepted / refused

- Accepted: both drafts, pending owner sign-off. No deletion authorized.
- Refused: nothing. D22 parking from last slice unchanged.
