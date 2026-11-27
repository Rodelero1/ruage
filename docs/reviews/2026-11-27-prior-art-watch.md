# 2026-11-27 - prior-art watch: a roster move in the identity tables, nothing to act on

Documentation only: **20 lines added to `docs/prior-art.md`**. No `crates/`, `config/`, `tools/`, `build/` or `Age2HD/`
file touched; no test, no fixture, no production code. The only due project was `SiegeEngineers/aoc-reference-data`, head
`e39f3dec` -> `73717b3d`.

## Performance

- **The range is one commit and two files**, cross-checked rather than assumed: GitHub's compare API answers
  `status: ahead`, `total_commits: 1`, and a fresh clone's `git log --name-status e39f3dec..73717b3d` names exactly
  `data/players.yaml` and `data/teams.json` for **+8 / -3** (`git show --stat` agrees: 8 insertions, 3 deletions). No
  merge hides in the range - the head is the commit itself.
- **Files at the new head:** `data/players.yaml` **9,162 lines**, `data/teams.json` **807 lines**.
- **The probe's counts** (a column scanner over the two files, not a YAML parser): **672 players**, 666 with an id,
  **84 teams with ids 1-84 contiguous**, **287 rows carrying `team:`**, **16 ids listed by more than one team**,
  **0 listed ids with no player entry**, **0 rows whose `team:` names a team that does not list them**.
- **Cost to the engine: zero.** `cargo clippy --offline --all-targets -- -D warnings` is clean in **12.6 s**; the full
  suite is **228 test binaries, 946 passed, 0 failed, 25 ignored**, exit 0 (`build/test-2026-11-27.log`, gitignored). Both
  numbers are identical to 2026-11-26's, which is what a docs-only change should produce - a docs diff runs no new test.
- **`cargo fmt --check` is red on HEAD**, at `crates/sim/tests/gather.rs:527`, the same `let Some(..) = ..` split the
  last two watches reported. It was landed by `ba0aba8` (D353), another session's commit, in a file this run did not
  edit; left alone and named here instead of reformatted.

## Stability

What was attacked rather than trusted:

1. **"Only the roster files moved."** The compare API's file list and a fresh clone's `git log --name-status` are two
   independent enumerations and agree on 2 files; the earlier ticks' failure mode - a compare payload truncated by a pipe
   - was avoided by writing the response to a file first and checking its size (13,324 bytes).
2. **"Nothing that carries a game value moved."** That is a fact about the **path list**, not a judgement about content:
   the range touches no file that holds a unit, tech, civ, cost or HP. Stated that way in the note rather than as a
   reading of their diff.
3. **"The roster is internally consistent at the new head."** Measured, not inherited: 0 disagreements between a player's
   `team:` and the teams listing them, 0 orphans, 84 contiguous team ids - and the `team:` count rose by exactly the two
   fields this commit adds (285 -> 287).
4. **"The duplicate-id set is unchanged."** Re-measured: still the same six (681, 753, 756, 771, 774, 775). One of them is
   now concrete rather than a count: **753 is both "Thofou" (team 15) and "Gali" (team 53)**, so two team lists carry the
   same id for different people. Pre-existing; named so the next session does not mistake a team list for an identity
   source.
5. **The splice itself.** The new section was inserted by a script that asserts the anchor string occurs **exactly once**
   before writing, and the inserted block was re-read from the file afterwards - the failure this guards against is a
   duplicated heading in a 2,303-line file, which no gate would catch.

**The failure no check covers:** the repo has no foreign key between `players.yaml` and `teams.json`, so consistency is a
point-in-time property, not an invariant. This probe says the two files agree today; the next commit can break it and
nothing on their side will say so.

## The AI review

`make ai-review` ran and answered **"skipped - 0 files, 0 comments, 0 tokens, 0s"**: the recipe's `OCR_EXCLUDE` is
`docs/**,build/**,target/**,config/gamedata/**,**/*.html`, so a docs-only diff cannot reach the reviewer. There were no
findings to accept or refuse, and the note rests on the five checks above instead.

My own errors this run, reported: the first read piped the compare response through `jq`, which is not installed here, and
the next attempt piped it through `head`, which died with `curl: (23) Failure writing output to destination` - the payload
was fine, the pipe was not. This box's cron approval policy refuses an interpreter invoked with an inline statement (and
refuses a recursive directory removal), so the parse went through a written probe script and the clone into a timestamped
directory. And the first field-level look used `grep -B25`, which walks across entry boundaries in a file where entries are
separated by a single line: it printed another player's `team:` for both "zankoku" and "Gabi". Every number above comes
from the replacement scanner, which reads only `^- name:`, `^  id:` and `^  team:`; the three rows it printed were then
re-read in the file.
