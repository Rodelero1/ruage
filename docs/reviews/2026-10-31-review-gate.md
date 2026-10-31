# Review: making the routine impossible to forget

*Change: `AGENTS.md` carries the four-step routine, `tools/review_check.py` + `.githooks/pre-commit` enforce the third
step, `make review` / `make review-test` run both by hand.*

## Performance

The gate costs **128 ms** per commit (best of 5, `python3 tools/review_check.py` on a staged set; two `git` subprocesses
dominate, the parsing is microseconds). It runs once per commit and never in the test suite, the build, or the tick, so its
budget is "not felt by a human typing a commit message" - met by two orders of magnitude.

What it replaces, measured in the same currency: two skipped reviews in one session (the terrain step and the step-8
selection work), each of which cost a follow-up exchange with the owner to catch. 128 ms per commit is the cheapest
insurance available against that.

## Stability

Attacked deliberately, each case pinned by a test in `tools/test_review_check.py` (11 tests):

- **A code change with no review** -> refused, naming what to write. This is the failure that kept happening.
- **A review with no `## Stability`** -> refused; **no `## Performance`** -> refused; **Performance with prose but no number**
  ("it felt fine") -> refused, because "reviewed" without a measurement is the exact claim this project keeps catching
  itself making.
- **An empty review file** -> refused; **a staged deletion of a review** -> refused, because a deletion lists like an
  addition and would otherwise pass on the filename alone.
- **Docs-only changes** -> allowed, so a typo fix does not teach everyone to paste a fake review to get past the gate.
- **The gate's own file** -> `tools/` is a code prefix, so changing the gate needs a review of the gate. That is
  deliberate: the checker is the thing most likely to rot.

Known and accepted, stated rather than hidden:

- `git commit --no-verify` bypasses every hook. Nothing in a repository can prevent that; the gate's job is to make the
  omission *deliberate* instead of accidental.
- A review note written for an **older** change can be staged beside a new one and will validate. The checker sees no link
  between the note's words and the diff's content, and could not without a reader.
- The number check is a regex for a time unit or a percentage, so `"3 s"` inside a sentence passes. It refuses the honest
  failure ("no number at all"), not the dishonest one - which is a reviewer's job, not a script's.

One defect found in my own script during this review: a `--all` flag that did nothing. Removed rather than left as a lie
about what the tool can do.
