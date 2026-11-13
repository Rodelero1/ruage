# Working in this repository

## The routine every step follows - non-negotiable

Written here because a rule that lives in a conversation is forgotten by the next thread. Hermes loads this file into the
system prompt for any session working in this tree (context files, `AGENTS.md`), and the cron job reads it too. If you are
an agent reading this: **these four steps bracket every increment, and the third one is the one that gets skipped.**

1. **Compact first.** Persist the state to files *before* touching code: the step's compact in `docs/next-step.md`, the
   SPEC version and changelog row, any decision the step rests on in `docs/decisions.md`. A new session, in a new thread,
   with no conversation history, must be able to resume from those files alone. **The compact's first half is the
   feature's prior implementation** — where it lives now (`file:line`), which tests pin it, what it measures today —
   because a plan written over an unnamed implementation cannot say what it is changing (D203).
2. **Implement test-first.** No production code before a test that was watched failing, with the failing run shown
   (SPEC §11, D7). Behaviour-based coverage; no percentage gate. A slice that slips past this is *named* as such in the
   reply, never silently.
3. **Adversarial review - performance and stability, with numbers.** Before the words "done" or "landed":
   measure the change against the previous commit (the probe A/B, same generated data), ask what *else* stands where the
   change stands, hunt the failure no test covers, and report your own errors including a misreported test count. Write the
   review to `docs/reviews/<date>-<topic>.md` with a `## Performance` section holding a **measured** number and a
   `## Stability` section naming what was attacked. **The gate enforces the file's existence**: `.githooks/pre-commit` runs
   `tools/review_check.py` and refuses a commit that changes `crates/`, `config/`, `tools/`, `Age2HD/` or `build/`
   without it staged. A docs-only change owes the note too, but is not gated: demanding a number from a typo fix teaches
   everyone to write a fake one.
4. **Push.** `main` level with `origin` at the end of every step, not at the end of a topic. A local commit is not the
   deliverable, and this box reboots nightly.

**Never assume - research first.** For any question about how the game behaves, the order of evidence is (D203): the
original game - run it, watch it, read its own data (`config/gamedata/`, the install) and its binary - then a neighbouring
open-source project (`librematch/genie`, `Zaydme/openaok`, `Secttech/openage`) and published sources (the manual, the
wiki, the community tables), read for facts, never copied (see the licensing section; genie is AGPL-3.0). Our own files
are read first for what *we* already decided. When the evidence is inconclusive, **ask**, and say which part is unknown
rather than picking a plausible answer. The owner has reversed his own stated rule when the data disagreed,
so evidence outranks recollection including his.

## When a decision is needed: the owner's attention is the last step, not the first

A question worth his attention is worth work first. The order is fixed (D203), and it is the order he asked for:

1. **The original game first.** How does *the game* deal with this? Run it, watch it, read its own data, read the binary.
   A save, a screenshot, a recording or a byte offset from `Age2HD/` outranks every other source: it is the only source
   that cannot be wrong about AoE2 HD.
2. **Then the second rung of the ladder above** — the adjacent open-source projects and the published sources. Record
   what was found in `docs/prior-art.md`, with the date, because the next session's search starts there.
3. **Then ask — and only then**, in one message: the question, two or three options with their costs, the recommendation
   first, and **what each attempt found, including the attempt that found nothing.** A question with no evidence behind
   it is the failure this rule exists to prevent.

**The question must not expire.** Three mechanisms, doing different jobs:

- **The reply, with the turn ended** — the default. Nothing is waiting, so nothing can time out; he answers hours later,
  in another thread, or after a reboot.
- **A `U<n>` row in the U-register** (`docs/mechanics/README.md`, filed in `docs/decision-areas.toml`'s `[unknown]`
  table, rendered into `docs/site/index.html`) — written *before* the turn ends, and this is what makes the question
  permanent. The box reboots nightly, and a question that lives only in a chat message is gone at 04:00; a question in
  the register is the first thing the next session reads.
- **A `clarify` form** for a choice that blocks the current step and is cheap to answer now. `agent.clarify_timeout` is
  `0` here, so an open form cannot expire unanswered mid-conversation — but a form holds the turn open, so the other two
  are what a decision question uses.


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
