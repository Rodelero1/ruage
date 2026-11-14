# Review — `docs/dependency-tree.md` (2026-11-14)

**What changed.** One new document, `docs/dependency-tree.md` (27,489 bytes, 59 items: 9 foundations, 18 simulation rows,
14 debts, 10 missing mechanics, 8 product rows), a pointer from `docs/next-step.md`, and a SPEC changelog row. No Rust, no
config the engine reads, no tool.

## Performance

**No engine number, and the honest one is the file's own:** the document is 27,489 bytes and its probes are eight
`grep`/`ls` invocations; no probe allocates, no probe touches the sim. The one measurement it *cites* rather than makes is
the acceptance run's `hash=c3b10f4b09e17a43 repaths=324 made_room=147`, taken from `docs/next-step.md` as recorded on
2026-11-14, not re-run here.

The number that would make this file expensive later is not written down, on purpose: **statuses rot**. §11's probes exist
so that a session re-decides a row in one command rather than paying for a re-read of 180 decisions.

## Stability

**What was attacked, and what broke.**

1. *Every status claim was checked against code, not against prose.* Four stale claims fell out and are recorded in §4b
   (D152 still lists gates and pushing as missing; `civs.rs`'s header says the team bonus is unapplied when
   `research.rs:221` applies it; `window.rs` says there is no input or HUD; SPEC v1.153 says the standoffs still use the
   ring when D192 switched them).
2. *The debts were chosen by a probe that can fail*, not by reading the U-register: `grep -rn '\b195\b' crates` → no hit,
   `grep -rn DisableTech crates/sim` → no hit, `grep -rn reseed crates` → no hit, `grep -n '1 =>' crates/data/src/techs.rs`
   → `Attribute::Range`. Each is in §11 so the next session can disagree with this file.
3. **My own errors, found while writing and fixed before the commit:**
   - cited `crates/sim/src/script.rs:624` for the gather search; that file **does not exist** — the function is
     `crates/script/src/lib.rs:712` (the U22 row itself is stale in the same way; fixed here, reported, not silently
     copied);
   - cited `civ_barred_unit` at `lib.rs:4113`; it is `:4139`;
   - wrote "18 gather test files" and "40+ view tests"; `ls` says **12** and **36**.
4. *What the document does not settle, stated as such:* the dependency edges for the missing mechanics (M3–M8) are
   assertions from absence (a `grep` with no hit), which is a claim about the engine and not about what the original game
   does. Each is marked absent-with-evidence, not "unnecessary".
5. *The one thing that could make the whole order wrong:* if the owner answers §12 question 1 with "a playable product",
   Phases 0–3 keep their content but Phase 4 moves to the front. That is why §12 exists and why the sequence is written as
   phases with gates rather than as a single numbered list.

## One operational finding, because it is the multi-writer hazard and it bit

While this note was being written, another session in this tree ran `git stash` with untracked files
(`stash@{0}`, "On main: d207-wip") and the stash **swept this document, its review and the `SPEC.md`/`next-step.md` edits
with it** — `git status` went clean and all four files vanished. They were recovered by path (`git checkout stash@{0} --
SPEC.md docs/next-step.md`, `git checkout 'stash@{0}^3' -- docs/dependency-tree.md docs/reviews/...`) and the other writer's
`crates/sim` work was left **in the stash, untouched**. Two rules this confirms: stage by name, and **commit a docs slice
in the same session it is written** — a document that lives only in the worktree is a document another writer's tidy-up
can delete.
