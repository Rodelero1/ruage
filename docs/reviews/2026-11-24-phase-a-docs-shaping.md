# Review: Phase A docs shaping — thin root, citations follow (2026-11-24)

Docs + tools change (comment-only in crates), so gate sections apply.
No engine behavior change. Bank moved by `git mv` (100% rename, bytes then
pointer-maintained — see the frozen-bank note below).

## What landed

- Root: 45 md → 7 (`decision-index`, `divergence-register`, `next-step`,
  `prior-art`, `sources`, `licensing`, `config` + tomls). 34 files →
  `docs/archive/research/`, 7 history snapshots → `docs/archive/history/`,
  2 pending witness drafts → `docs/archive/`, 1 `.swp` deleted.
- Citations: one-shot script rewrote every moved path across tracked files
  **except** `docs/reviews/**` (frozen evidence log — 12 old paths there resolve
  via git history) and SPEC changelog rows (history). Bank + live + mechanics +
  crates comments + tools + Makefile + AGENTS-adjacent pointers all follow.
- `conformance_audit.py` SOURCE patterns accept the `archive/research/` prefix
  (new `test_conformance_audit.py`, 3/3: moved paths sourced, root paths still
  sourced, burn-down still 0 pending). `next_step.py` HISTORY → archive
  (would otherwise recreate `docs/history/` on `--split`; +1 test).
- SPEC live pointers (§12 table, §4/§6/D2/D1/2b) repointed; history rows untouched.
- Hub: where-to-read map, burn-down goal for the register, archive layout.
- `docs/site/index.html` hand-patched with the same pairs (51/51 pure path
  substitution, verified). Its build stays red pre-existing — regen still refused.

## Performance

Rewrite script: 86 files in one pass. Full suite 228 tests (+5), 11.6 s.
Index `--summary` byte-identical counts (345/337/5/249/11); `conformance --check`
exit 0; `live_migrate --check` 364/15/0; regen-consistent live files.

## Stability

Attacked: dangling cites (grep sweep after rewrite; only reviews/ + SPEC-history +
  frozen-bank-original-bytes remain, all declared); stale audit patterns (would have
  flipped sourced rows to pending — covered by the new tests, which failed RED
  first); tooling recreating `docs/history/` (const test); site hand-edit drift
  (next successful regen overwrites; until then the patch is strictly paths).
Caught by verification, fixed same session: (1) D188's derived date corrupted to
  2026-11-24 — the archived filename contains a date and the date regex reads the
  Recovered-record sentence first; fixed by rewording to "the bank was truncated"
  (git history holds the original bytes). Only 1 of 364 sections affected.
(2) Sentence-final `md.` cites dodged the lookahead in site (re-passed).
(3) My test wrongly assumed `audit()` returns [] when clean (it returns findings;
  the check is `needs_verdict`).
Frozen-bank honesty note: "frozen" means no ruling changes — pointer maintenance
  is recorded here, originals in git history. Reviews stay byte-frozen, no exception.
Same 2 pre-existing failures before/after. Cargo unavailable; no Rust touched.
