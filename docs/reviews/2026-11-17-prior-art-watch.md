# Prior-art watch, 2026-11-17

Docs only: one new dated section in `docs/prior-art.md`. No `crates/`, `config/`, `tools/` or `Age2HD/` change, so
there is no engine number of ours to measure and none is invented. `make ai-review` ran and returned
**"skipped - 0 files"**: the recipe excludes `docs/**`, so a docs-only diff cannot reach the reviewer. The pre-commit
gate is not triggered; this note is owed anyway, and it is short because the tick was small.

## Performance

Nothing in the engine changed, so a probe would time the same binary twice. What was measured is the work behind the
report, on this box (4 GiB / 2 vCPU):

- **The diff:** GitHub's compare API, `a53436be...e39f3dec` — **status `ahead`, 1 commit, 1 file, +2 / −0**.
- **The file:** `data/players.yaml`, **9,159 lines / 159,859 bytes** at the new head, parsed with PyYAML in a scratch
  venv: **672 entries**, **54** of them carrying `youtube_live`, every value ending in `/live`.
- **Our own checks, unchanged by this commit:** `tools` suite **203 tests in 19.3 s** (one more than last run — another
  session's test, not mine); `plan_check` **ok, 84 items / 120 citations / 8 checks**; `decisions.py --check`
  **256 decisions, 14 open questions, 42 with no stated basis, 7 dangling ids** (D115, D122b, D124, D125b, D126, D136,
  D190). Every one of those counts is inherited from a tree this change does not touch.
- **The Rust tree at HEAD is red, and it is not this commit that made it red.** `cargo test --offline` finishes every
  suite except one: `crates/sim/tests/army_walk.rs` `a_crowd_walking_through_a_gap_stays_on_free_ground` panics with
  `NoPath { from: (6, 20), to: (19, 19) }`. `cargo fmt --check` reports a diff at `crates/sim/src/path.rs:109` (two blank
  lines). **Both trace to `16853ac`, another session's commit titled "wip: …", whose `path.rs` row is a 205-line rewrite.** I did
  not touch either file and did not reformat them: another writer's in-flight branch is not mine to finish, and a
  formatter pass over their WIP would hide the state they left it in. `cargo clippy --all-targets -- -D warnings` stops
  on the same file with two `clippy::type_complexity` errors (`path.rs:250`, `path.rs:278`). Three red checks, one
  commit, none of them mine — recorded here because a green claim beside them would be false.

## Stability

What was attacked rather than assumed:

- **"The new key is a schema change this repo did not document."** It is not new: **54 of 672** entries carry
  `youtube_live` at this head and the added one is the 54th, so the commit follows an established shape. The shape is
  also looser than a mirror of `youtube` — **10** entries have `youtube_live` with no `youtube` list and **24** name a
  channel that is not in their `youtube` list — so "it duplicates the sibling field" is not a rule I could have applied
  to judge it, and I did not pretend to.
- **"The duplicate-id set recorded on 2026-11-13 has since changed."** Re-measured on the new head instead of trusted:
  still exactly **six** ids — 681, 753, 756, 771, 774, 775 — across **12** entries. The standing rule (not an identity
  source for those six, usable for the published values we read it for) survives the tick.
- **"A roster commit could still carry a game value."** The two added lines are URLs under one player's entry; nothing
  in the commit touches a unit, tech, civ, cost or HP, which is the only reason this repo is on the list.
- **The foreign-change hazard.** `git status --short` before staging showed **`M docs/prior-art.md` and nothing else**,
  and the generated `docs/site/index.html` was not re-rendered — the trap last run fell into (another session's
  uncommitted D-rows inflating the page) is avoided by not running the renderer at all.

My own errors this run, reported: the first consistency script assumed `players.yaml` was a mapping with a `players`
key and printed `top keys: None` — the file is a **top-level list** of 672 entries, and the counts above come from the
corrected read. `uv pip install --system pyyaml` also failed against a read-only interpreter, so the check ran from a
scratch venv instead of the toolchain Python.
