# 2026-11-23 - prior-art watch: openage's C++ tracebacks, nothing to act on

Documentation only: **19 lines added to `docs/prior-art.md`**. No `crates/`, `config/`, `tools/`, `build/` or `Age2HD/` file
touched; no test, no fixture, no production code. The only due project was `SFTtech/openage`, whose head moved from
`f6f87f9a` to `738bfc6c`.

## Performance

- **The range is two commits and two files**, and both numbers are cross-checked rather than assumed: GitHub's compare API
  reports `total_commits: 2` and `files: 2`, and the patch it serves is 175 lines carrying **+53/-34** on
  `openage/cppinterface/exctranslate.pyx` and `exctranslate_tests.pyx`. There is no merge-only commit hiding in it: the head
  `738bfc6c` is PR #1836's merge, and the one substantive commit `0b6e5eccbd` is in the range.
- **The whole change fits in one file read.** `exctranslate.pyx` at that sha is 158 lines, fetched once to place the note's
  citations; nothing else in the repository was opened.
- **Cost to the engine: zero.** No code path in this tree changed. `cargo clippy --offline --all-targets -- -D warnings` is
  clean in **11.0 s**, and the full suite is **222 test binaries, 945 passed, 0 failed, 25 ignored** in about six minutes
  (`build/test-2026-11-23.log`, gitignored). The previous watch reported 220 binaries and 943 passed, so the delta is other
  sessions' tests, not this one's: a docs-only change runs no new test.
- **`cargo fmt --check` is red on HEAD**, at `crates/sim/tests/gather.rs:527` - a `let Some(..) = ..` pattern that wants
  splitting over two lines. It was landed by `ba0aba8` (D353), another session's commit, on a file I did not edit, so I left
  it alone and name it here rather than reformatting someone else's slice. It is not caused by this run.

## Stability

What was attacked:

1. **"Nothing was missed in the range."** The compare response is the authority; it and the served patch agree on commit
   count and file list, and the merge commit's parents explain the author date (14 Nov) against the merge date (22 Nov).
2. **The citations.** Fetched `openage/cppinterface/exctranslate.pyx` at `738bfc6c` and checked each line the note names:
   `:55` `PyCode_NewEmpty`, `:88` `frames.append`, `:155` `make_traceback`, `:157` `with_traceback`. A note that cites a
   line it read out of a diff hunk can cite the wrong number; these were read in the final file.
3. **"No U-number moves."** Read U12's row in `docs/mechanics/README.md` - the only register entry about the tick model, and
   the one a "time" change would touch - and the change does not reach it.
4. **The relevance filter itself.** Both changed files sit inside `cppinterface/`; no content-format, engine-boundary,
   pathing or asset file appears in the range. The claim "nothing on an engine axis" is a fact about the file list, not a
   judgement about the code inside it.

**The failure no check covers:** whether the author intends the hand-built traceback to survive the next CPython release.
It cannot - `PyFrame_New` and `PyCode_NewEmpty` are internals - and the note says so rather than presenting the fix as
durable.

## The AI review

Not run this tick, and the reason is mechanical rather than a judgement: `make ai-review` selects **0 files** for a
docs-only diff (the default `OCR_EXCLUDE` is `docs/**`), which is what happened on 2026-11-22. The note therefore rests on
the four checks above rather than on a reviewer's findings, and says so instead of implying a review happened.
