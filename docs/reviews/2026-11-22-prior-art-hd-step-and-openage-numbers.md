# 2026-11-22 - prior-art watch: HD's step is ~79 ms, and openage types a multiplier

The change is **documentation only**: a dated section in `docs/prior-art.md`, a dated paragraph on U12 in
`docs/mechanics/README.md`, and a correction plus the measurement in `docs/oracle.md`'s recorded-games note. No production
code, no test, no fixture; nothing in `crates/` or `config/` moved. The measurement behind it reads files the install
already had, with the tree's own walker, so it is reproducible in one command:

    python3 tools/recordings.py Age2HD/SaveGame/*.aoe2record

## Performance

Three numbers, because the measurement is the deliverable and its cost is the only thing that changed:

- **The walk is cheap.** An independent O(n) walker over `142146` (2.6 MB, 111,494 ops, 33,880 steps) runs in **444 ms**;
  `tools/recordings.py`'s own describe pass takes **382 ms**. The whole finding costs about half a second on files that
  shipped with the install.
- **My first version of that walker took 29,136 ms** - 66x - because it compared its step times against
  `recordings.iter_ops` *inside* the loop (`mine != theirs[:len(mine)]`, quadratic in the step count). Comparing once after
  the walk gave 444 ms and identical output. The defect was in the harness, not in the numbers.
- **Cost to the engine: zero.** `cargo test --offline` is **220 binaries, 943 passed, 0 failed**; `cargo fmt --check` and
  `cargo clippy --all-targets -D warnings` clean. The previously red `a_crowd_walking_through_a_gap_stays_on_free_ground`
  (`crates/sim/tests/army_walk.rs`) passes here - another session landed the fix, and I did not touch it.

## Stability

What was attacked:

1. **The grammar.** The walk is the one the tree already trusts: every recording consumes to exact EOF, and a stream that
   desynchronises raises rather than returning a partial read. All four files pass.
2. **A second implementation.** Step counts and per-step times from my walker are identical to `tools/recordings.py`'s
   (`33880 == 33880`, `mine == theirs`), so the cadence is not one parser's artifact.
3. **The checksum spacing, predicted then observed.** The meta block's second word is **500**; the checksum blocks land at
   steps 500, 1000, 1500 ... with **no other spacing** (67 blocks in the long file, 9 in `141539`). The two short recordings
   (302 and 122 steps) carry **none** - a negative control against a parse that invents blocks.
4. **Consistency across files.** Four recordings: 12.25, 12.75, 12.64, 12.64 steps/s; medians 78, 78, 79, 79 ms. A reading
   that held for one file would be a file, not a cadence.
5. **The failure nothing covers.** Named in both documents instead of buried: whether one turn runs per sync op or several,
   and **which game speed these four were recorded at**. Nothing in the tree parses the speed field, and U12's own
   2026-11-07 caution is exactly that factor. The claim in the note is therefore "the recording carries each step's ms",
   not "the tick is 79 ms".

## The AI review

`make ai-review` selected **0 files, 0 comments** for this diff - the default `OCR_EXCLUDE` is `docs/**`, and a docs-only
change has no code hunks, so the reviewer had nothing to read. The first invocation, before I set `OCR_ARGS=`, reviewed
`HEAD` instead: the *previous* commit, 2 files and 3 comments about that commit's probe, never this change. This note rests
on the five checks above, not on a reviewer's findings, and I am not claiming otherwise.

## Errors of mine, named

- The 29 s walker above: a quadratic comparison inside a probe.
- My first reading of the health path said we round a health multiplier per application. `scaled_health`
  (`crates/sim/src/lib.rs:260-280`) adds first, multiplies, and rounds **once**, so the corrected claim - that we already
  sit where openage's fix arrives - is the one in the note.
- My first probe reported **0 checksum blocks** in a file that has **67**: a mis-parse in the block branch, caught by
  comparing the walk against `tools/recordings.py`, which agreed to the millisecond.
- `make decisions` fails on **50 pre-existing AREA gaps** (D283-D333 have no row in `docs/decision-areas.toml` - other
  sessions' work in progress) and therefore **wrote nothing**: `docs/site/index.html` still renders the old U12 row. The
  register row itself is updated; the page will follow the next time that check is green.
