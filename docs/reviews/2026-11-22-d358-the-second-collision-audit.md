# The second collision audit: the reach has three numbers, and two dead rows still read as live

**Date:** 2026-11-22 · **Branch:** `blunt-port` · **Docs and decisions only** (no `crates/` change)

**The ask:** *"Are there any other colliding decisions?"* — after the movement audit (D356), this one scanned the **whole** bank
(345 rows), not the movement subset.

## What the scan found

1. **The reach: three numbers, none superseding another, one row contradicting itself.**
   - D149: *"Shipped: sight + 8 = 12 tiles."*
   - D150 (same day): *"The shipped constant is the owner's own **5**, not 8."*
   - D175: the dat's own `search_radius`, villager **4.0**.
   - Code today: the dat's number **when the pack carries it**, else sight + 5. This generated pack carries none for unit 83, so the
     live value is **9** — and the acceptance map's gold sits at **9.6** tiles (D354: 220 refusals, gold frozen at 10 with 1,011 on
     the ground).
2. **The assignment line is the actual defect, not the number.** D149.5 draws it (*"an order is not a guess about what a villager
   can notice"*) and the script keeps it for an explicit node (`None`, `crates/script/src/lib.rs:455`) but **not** for a quota rule
   (`Some(reach)`, `:1431`). So the AI-side assignment is bounded by the engine-side eye — the game's AI assigns from the base.
3. **Two dead rulings read as live**: D70 (herding, superseded by D116 only in D116's *title*) and D36 (garrison "any building takes
   any unit", corrected by D38). Both now carry the marker; `docs/decision-index.md` prints them as superseded.

## What was checked and is not a collision

- **D260 vs D206/D251 (turning)**: D260's *"a turn costs ticks"* is **withdrawn inside its own row**, the same evening, on the
  owner's correction (*"I remember turns being instant"*) and on `FUN_008a3270`: the yaw limit feeds a flag, not a distance. The row
  reads correctly and needs nothing.
- **D122 vs D226 (farm reseed)**: not a contradiction - one is the husk and its replant, the other is the queue/auto-reseed
  setting. D226's DE-only part is a divergence, and it is already in the register's OURS section.
- **D38 vs D36 (garrison)**: a supersession, now marked.
- **D228 vs D227 (the 1-damage floor)**: settled by three published sources in D228; no second live answer.

## Performance

- No engine change, so nothing was rebuilt for this slice: the numbers stand as D357 left them (acceptance **153 entities, 138,588
  re-paths, 24.83 s**; suite **946 passed, 0 failed, 25 ignored**).
- The audit's own cost: `decision_index.parse` over **832,998 characters / 345 rows**, and one heuristic polarity scan (600+
  candidate pairs, all of them noise — see the stability note), which is why the verdicts below rest on **reading the rows**, not on
  the scan.

## Stability

- `decisions-check` **0**, `next_step-check` **0**, `review_check` **0** after the edits; the index grew by one row and one
  supersession (D149 → D150), and `D358` carries the precedence.
- **The heuristic was useless and is reported as such**: a sentence-polarity scan produced 104 candidate pairs for damage and 63 for
  "push", almost all of them rows that merely share a noun. It found nothing I did not then confirm by reading; the useful signal was
  *three rows answering one numeric question*, which a human reads off the index in a minute.
- **One of my own edits silently failed and was caught by re-reading**: the first attempt to annotate D149 matched nothing (the
  italic paragraph is line-wrapped differently from my pattern) and wrote the file unchanged. The index showed no
  `superseded by D150`; that is how it was found.
