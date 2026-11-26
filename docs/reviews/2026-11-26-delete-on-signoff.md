# Review: delete-on-sign-off + 0007 plain-wording (2026-11-26)

Docs-only slice. Code frozen; no `crates/`, `config/`, `tools/` touched.

## What landed

- Bank (`docs/archive/decisions-2026-11-24.md`): deleted rows for signed ADRs — D4, D8, D12 (+its same-day amendment), D13, D17, D18, D21. D09/D10 untouched (0007/0008 still Draft).
- ADR 0007 draft: two goal-code lines plain-worded (standing-goal sentences). Part of pending sign-off, not a rewrite.
- Manifest: deletion rule noted, D13-deletion recorded, pending row re-ranged precisely (D1–D3, D7, D11, D14, D19, D20, D23–D197, D199–D358).
- Index regen (`make decision-index`): 345 → 344 rows.

## Performance

Zero engine diff by construction. Suite state unchanged from the trunk verdict (522 passed, 2 pre-existing stale-data, 6 ignored) — no re-run, nothing on the tick changed.

## Stability

Attacked: (1) header counts old vs new — 16→12 table rows, 253→250 sections, exactly the 7 deleted; (2) seams read back (D11/D14 table seam, D14 and D22 section seams clean); (3) index verified to reflect the deletion — old index contained only D21 of the seven (checked `git show HEAD`), new index contains none, 345→344 consistent; (4) the "364" number in the old changelog is stale (HEAD index was already 345), not a discrepancy.
- Own error count this slice: 1 (double-blank left at the D11/D14 seam, fixed). Plus 1 false alarm: an off-by-one between two reads that re-verification (grep + clean `git status`) proved was a misread, not a foreign edit.

## Accepted / refused

- Accepted: deletions as the owner rule requires (delete on sign-off).
- Refused: nothing. 0007–0013 still await tranche 3–5 verdicts (0007 now plain-worded for review).
