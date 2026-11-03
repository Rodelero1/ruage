# Review: an army is a count, not a unit - and a correction to two claims

*Change: `When.army_at_least`, a rule condition on the number of **military** units a player has, counted through
content's `[metrics] military_roles` - the same list the balance harness counts, so a rule that waits for an army and a
metric that reports one cannot disagree. `World::army(player)` is that count. One test:
`rules::an_army_rule_waits_for_an_army`, which fails without the condition (watched: *"no soldiers, so no attack"*).*

## Performance

**The capability costs nothing and the content rule that used it did not survive measurement.** `army()` walks the
entity list once and reads a handful of role roots; nothing on the tick path calls it unless a scenario writes the
condition, and no shipped scenario does. Acceptance match at 36,000 ticks with the condition landed but **no rule using
it**: `players=2`, hash `91a8ad6e2cbd01f6`, 14.8 s, bank 1,914 wood / 416 food / 512 gold at 1,600 s - byte-identical to
the merged tree, which is what "inert by default" means.

**The rule I wrote with it made the match worse and was reverted.** `when = { army_at_least = 8 }` attacking with the
spear line: `players=2`, hash `238724a2bddbb180`, bank 2,276 / 448 / 730, 21 buildings - the army never forms because
the soldiers withdraw (the owner's item 7), so the rule only spends wood. It is content, one hunk, reverted; the
capability is what stays.

## Stability - and a correction I owe

- **Two claims of mine were wrong, and this review is where they are corrected.**
  1. The merge commit `a782a47` and `docs/next-step.md` (`96c8ba2`) say the acceptance match is **won** at hash
     `51992525a3b1c3fb`, bank 1,294/1,209/175. **It is not.** Re-running the merged binary - and a binary built from
     `a782a47` itself, in a clean worktree - gives `players=2`, hash `91a8ad6e2cbd01f6`, on every run. The winning hash
     came from one intermediate run earlier in the session and I could not reproduce it in the merged tree; a claim
     resting on a single unreproduced run should never have been written down as a result.
  2. The same reviews quote "48.4 s" for that run against "19.9 s" for the baseline, which mixed a `make headless`
     figure (build + run) with direct-run figures. Measured properly, back to back, on one box: **14.8 s** for this tree
     against **9.8-10.0 s** for `1a31830` at the same 36,000 ticks, and the baseline itself no longer wins.
- **What is actually true of the merged tree:** the passage rule, the layout, and the node/arrival work are all in and
  tested (100 binaries green); the acceptance match **draws** at `players=2` / `91a8ad6e2cbd01f6`, with a healthy but not
  winning economy (1,914 wood / 512 gold banked at 1,600 s, 24 buildings).
- **Next, and this is now the top of the queue:** find the run that won (`51992525a3b1c3fb`) or accept it is gone. The
  honest starting point is a bisect of the three commits between `97958a8` and the merge, using the hash as the oracle,
  and the first question is whether the *map seed* or a config file changed under me - the binary and the pack both claim
  the same inputs, and the hash disagrees, which is itself a fact worth explaining before anything is built on top.

---

*Staged with the change for `tools/review_check.py`. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
