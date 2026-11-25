# Review: ADR renumber 0359–0371 → 0001–0013 (2026-11-25)

Docs-only mechanical rename. Q11 amended by owner: ADRs start at 1 in migration order (old bank deleted at the end, so no permanent collision). Code frozen; no `crates/`, `config/`, `tools/` touched.

## What moved

13 files in `docs/adr/` renamed with identical slugs (0001 D8, 0002 D4, 0003 D12, 0004 D21, 0005 D17, 0006 D18, 0007 D9, 0008 D10, 0009 D16, 0010 D5, 0011 D6, 0012 D15, 0013 D198). Titles, in-body cross-references (0003 in 0002, 0001 in 0004/0011, 0005 in 0006, 0008 in 0007) and `MANIFEST.md` paths repointed.

## Performance

Zero engine diff by construction: only `docs/adr/` paths staged. Suite not runnable on this box; trunk gate stays owed.

## Stability

Attacked: `grep` for every old number over `docs/adr/` + `GLOSSARY.md` returns nothing after the fix (one miss caught: 0007's body still said ADR 0366, fixed). Prior review notes keep old numbers deliberately — reviews are frozen history, not live references. Alias uniqueness preserved: each D-id appears once in headers + manifest.

## Accepted / refused

- Accepted: rename as the Q11 amendment requires. Plan file's numbering rule updated in the same step.
- Refused: nothing.
