# Working in this repository

## The routine every step follows - non-negotiable

Written here because a rule that lives in a conversation is forgotten by the next thread. Hermes loads this file into the
system prompt for any session working in this tree (context files, `AGENTS.md`), and the cron job reads it too. If you are
an agent reading this: **these four steps bracket every increment, and the third one is the one that gets skipped.**

1. **Compact first.** Persist the state to files *before* touching code: the step's compact in `docs/next-step.md`, the
   SPEC version and changelog row, any decision the step rests on in `docs/decisions.md`. A new session, in a new thread,
   with no conversation history, must be able to resume from those files alone.
2. **Implement test-first.** No production code before a test that was watched failing, with the failing run shown
   (SPEC §11, D7). Behaviour-based coverage; no percentage gate. A slice that slips past this is *named* as such in the
   reply, never silently.
3. **Adversarial review - performance and stability, with numbers.** Before the words "done" or "landed":
   measure the change against the previous commit (the probe A/B, same generated data), ask what *else* stands where the
   change stands, hunt the failure no test covers, and report your own errors including a misreported test count. Write the
   review to `docs/reviews/<date>-<topic>.md` with a `## Performance` section holding a **measured** number and a
   `## Stability` section naming what was attacked. **The gate enforces the file's existence**: `.githooks/pre-commit` runs
   `tools/review_check.py` and refuses a commit that changes `crates/`, `config/` or `tools/` without it staged.
4. **Push.** `main` level with `origin` at the end of every step, not at the end of a topic. A local commit is not the
   deliverable, and this box reboots nightly.

**Never assume - research first.** The order of evidence is: this repository's files, the game's own data
(`config/gamedata/`, the install), the manual and the wiki, then a neighbouring open-source project (`librematch/genie`,
`Zaydme/openaok`, `Secttech/openage`) - read for facts, never copied (see the licensing section; genie is AGPL-3.0). When
the evidence is inconclusive, **ask**, and say which part is unknown rather than picking a plausible answer. The owner has
reversed his own stated rule when the data disagreed, so evidence outranks recollection including his.


This tree is shared. A daily cron job (the prior-art watch) and, from time to time, more than one agent
session commit here. The rules below exist because that has already gone wrong once: a scheduled job ran
`git add -A` while another session's work was uncommitted, and the result was a commit called
"docs: what learning from a GPL project does and does not permit" that contained someone else's feature
code.

## Committing

- **Stage files by name.** `git add path/to/file ...`, never `git add -A`, `git add .` or `git commit -a`.
  Another writer's work is not yours to commit, and attributing it to your message is worse than leaving
  it uncommitted.
- **Look before you commit.** `git status --short` first, and confirm every path listed is one you
  changed. Foreign changes stay where they are.
- **Never** `git checkout --`, `git restore`, `git stash` or `git rm` a file you did not edit in this run.
- **Commit each slice as you finish it**, not once at the end of a long run: the shorter the window
  between editing and committing, the less there is for another writer to sweep up.
- **Read before you write.** Prefer targeted edits over rewriting a whole file, and re-read a file
  immediately before rewriting it: another writer may have changed it since you last looked.

## Before any commit

```sh
export PATH=/opt/data/tools/cargo/bin:$PATH
cargo test --offline
cargo clippy --offline --all-targets -- -D warnings
cargo fmt --check
```

The tests in `config/gamedata/`-dependent suites skip themselves when the generated data is absent
(D8: it is produced from the game's own install and is not committed).

## Never

- Write into `Age2HD/` — that is the game install, read-only, and it is not ours.
- Commit a credential, a game data file, or anything under `build/` or `target/`.
