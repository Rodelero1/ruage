# D355 — the bank becomes a page a session can actually read

**Date:** 2026-11-22 · **Branch:** `blunt-port` · **Owner:** *"Is too many decisions causing problems with context being filled
and starting to forget?"* → **"Proceed"**

## The measurement that made this the step

| | size | can a session load it? |
|---|---|---|
| `docs/decisions.md` | **832,998 chars** (~210k tokens), 341 rows | **no** — larger than a context window |
| `docs/decisions-index` output (`docs/site/index.html`) | HTML, per-area pages | no — a browser reads it, not a session |
| `docs/decision-areas.toml` | 2 `[[decision]]` entries, 49 rows with no area at all | — |
| `docs/reviews/` | **309 notes**, no index | only by `ls` + `grep` |

The failure that followed from it is concrete and mine: on 2026-11-22 a session wrote **D349–D354** while **D150 already held the
owner's pile rule**, then built a fixture that could not tell the rule from its absence. Nothing was missing from the bank; there
was no way to *find* it.

## What landed

- **`tools/decision_index.py`** → **`docs/decision-index.md`**: 341 rows, one line each - claim, `Conformance` verdict, source,
  **fixture**, area, supersession - grouped into 10 areas, plus the three lists that matter: **claims the game with no source**
  (5), **names no fixture** (247 — the "believed implemented" list where D150 lived), **superseded** (3). **34k characters, ~8k
  tokens**: loadable.
- **`tools/review_index.py`** → **`docs/reviews/INDEX.md`**: 309 notes, one line each, newest first, with the claim each note
  settled. The notes are **not moved**: `docs/decisions.md` cites them by path in a hundred places, so an archive directory
  would dangle those citations for tidiness.
- **`make decision-index`**, **`make review-index`**, and the two pages committed and generated, so they cannot drift.
- **`docs/decision-areas.toml` filled**: 49 rows had no area (**D283–D332**, the whole pathfinding run) and two of my own
  (`D353`, `D354`), so `make decisions` was refusing to build. `decisions-check` now exits **0**.
- **D351 carried onto this branch read-only**, because `docs/divergence-register.md` cites it and a citation of a row that
  exists only on `d351-reseat` is a dangling reference: `tools/decisions.py` was reporting exactly that.
- **`AGENTS.md`** now opens with the two indexes and the **`Fixture:` convention** - a ruling that rests on a test names it, and a
  row that names none is the list to work through.
- **`docs/next-step.md`** restated at tonight's state (D353's numbers) with the two next steps in order.

## Performance

- `python3 tools/decision_index.py`: **0.35 s** over 832,998 characters, 341 rows; output **34k characters** (4.1% of the bank,
  because a line is a claim and the bank is an argument). Sessions replace ~8k tokens of blind grepping with ~8k tokens of index.
- `python3 tools/review_index.py`: **0.12 s** over 309 notes; output 70k characters (17k tokens) — a search surface, not a
  start-of-session read.
- `tools-test`: **209 tests, OK**, 10.7 s; `decisions-check` **0**, `next_step --check` **0**, `plan_check` **0** ("84 items, 120
  citations, 8 checks").
- No test in `crates/` was touched and no engine code changed: `cargo` was not rebuilt for this slice.

## Stability

- Both generators are **pure over committed files** and **advisory**: neither is in `make check`, so a stale index makes a session
  read an old claim, never a red build (D209's lesson: a gate over prose teaches a fake line).
- **Three of my own errors, all caught by the tools' tests rather than by reading:** the area classifier first put **261 of 340
  rows** under "Movement & pathfinding" (first-match filing on common words); the fixture column answered `gather` — the *file*,
  not the test — until a declared `Fixture:` line was made to win; and the area test I wrote first chose a title containing
  "wall", so it demanded the wrong answer until I made the test about prose words instead. Each is now a test in
  `tools/test_decision_index.py` (6 cases).
- **`next_step --check` caught me twice**: it refuses a `docs/next-step.md` that does not open with `# OPEN`, which rejected both
  attempts to put the session-start list at the top. The list now sits at the end and `AGENTS.md` carries the pointer, which is
  where a session actually reads it.
- **Not fixed, deliberately:** the 247 rows that name no fixture. Filling that column means a slice per row (the fixture or the
  ruling), and inventing a fixture name to empty a list is the exact behaviour this bank exists to prevent. It is the burn-down
  list now, not a gap.
