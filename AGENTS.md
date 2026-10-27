# Working in this repository

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
