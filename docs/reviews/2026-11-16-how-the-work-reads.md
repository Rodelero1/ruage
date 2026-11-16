# Two rules about how work is presented, and a skill file that had to obey one of them

*2026-11-16. D246. Docs and a skill only: no `crates/`, no config the engine reads.*

## What was asked

The owner asked for two permanent rules: **replies that read quickly** - no AI slop, no wall of text - and **code that is
simple, readable and maintainable, not overengineered**, with comments that explain what a piece of code is *for*.

They are now in `AGENTS.md` as "How the work reads, and how the code is written (D246)", in `docs/decisions.md` as D246, and
as the last entry of the skill's `references/workflow-rules.md` (with its title in the index in `SKILL.md`).

## Performance

No code path changed, so the measurement is the artefacts:

| what | before | after |
|---|---|---|
| `AGENTS.md` (loaded every session) | 6,796 bytes | **8,335 bytes**, 11,665 under the 20,000-character cap |
| skill `SKILL.md` | **26,719 chars** - over its own 24,000 budget | **20,136 chars**, 51 references |
| the skill's "When to Use" · "the rest of `references/`" | 8,021 · 3,300 chars | 1,900 in `SKILL.md` · 8,280 moved to `references/when-to-look-here.md` |
| `tools/decisions.py --check` | 245 decisions | 252 decisions, exit 0 |

The skill row is the interesting one: it was **2,719 characters over the budget my own checker enforces**, because another
session had been adding to the same file. The rule that says "move a section when you are near the budget" was written in
the file it was being broken in, which is why the check exists and why it refused.

## Stability - what was attacked

1. **Does the new prose survive the context cap?** `AGENTS.md` is 8,335 bytes against a 20,000-character cap; no truncation
   is possible at this size, and the middle of a truncated context file is exactly where a rule disappears silently.
2. **Does the rule reach the prompt?** Same check as for D203: the file passes the context scanner with no blocked
   patterns, so nothing is replaced by a placeholder.
3. **Does the skill still load after a 7 KB move?** `scripts/skill_size.py` passes, 51 references all named, `--self-test`
   6/6. The check caught two real defects in my own edit (below) before either reached a session.
4. **Do the two rules contradict the existing ones?** No: "short" is scoped to presentation, and the review/measurement
   obligations are explicitly unchanged in D246 - a short reply does not excuse a missing measurement.

## My own errors

1. **I numbered the decision D210 without looking.** The corpus had moved on: the other session took D210 through D245 during
   the day, so the heading in `AGENTS.md` and the entry had to be renumbered to D246. `decisions.py --check` would have
   caught a collision on the next run; checking first is cheaper than renumbering four files.
2. **My condensed "When to Use" invented a path.** It pointed at `references/gates-research.md`, which does not exist - the
   survey is `docs/gates-research.md` in the repo. My own size checker refused the file and named it.
3. **The move silently orphaned nine references.** They had been named only in the bullets I was moving; the checker
   refused with nine "named nowhere in SKILL.md" lines and I added them to the by-name list. This is the second time the
   tool has caught a defect in the change that created it.

## Limits

- **The checker cannot read.** It enforces the budget and the index, not the prose; a reply can still be a wall of text
  with every check green.
- **The rule is advice in a file.** What makes it stick is that the owner repeats it when it slips - as he did today.
