# Review: the open step compacted - 16,088 bytes to 8,143

**What changed, documentation only.** `docs/next-step.md` is rewritten to one step: the queue R1-R10 with each item's measured
facts, the **single** question waiting on the owner (the entity floor), the carried debts and the hazards. The previous file is
archived verbatim at `docs/history/next-step-2026-11-17-d250.md`. Nothing is deleted from the record - git has it and so does the
archive.

## Performance

**Nothing to measure; the number this file has is a size.** The step file went from **16,088 bytes** (98% of the 16,384 cap, one
rubric away from being refused by `.githooks/pre-commit`) to **8,143 bytes**, i.e. **49%** of the cap, with the whole queue still
in it. `python3 tools/next_step.py --check` is the gate and it passes.

## Stability

What was attacked, because a compact's failure mode is a lost fact:

1. **Every measured number survived the rewrite, checked against the source rather than retyped from memory**: R1's **8,000 raw**
   against 7,200 and the **2,002-tick** deadlock; **155 -> 143** entities and **497 -> 649** repaths; R5's **18** reddened tests;
   R6's **3.5 vs 0.3 ms/tick**; R7's **16 / 25 / ~1.56**; D250's **67.6/68.2 s -> 28.4/28.5 s**, **1540 -> 710** repaths and the win
   at 2,381 s; the seam's **+2.6%**. A compact that gains a digit is worse than a long file.
2. **The queue's numbering is intact, including the split**: R1a/R1b, R2-R9, and R10's park with its two prerequisites named as
   **R1b and R8** - the item the earlier text stated only in prose.
3. **The one question is hoisted to the top and stated once.** The floor (**entities >= 155**) against the control's 154 with no
   win and D250's 137 **with** one was buried four paragraphs down; a fresh session could have spent a slice chasing a floor the
   owner has not ruled on.
4. **No claim was improved while copying.** R7's "the port is not done, the read is" keeps `local_46c` **unread** and the
   equal-`f` tie-break open; R10 stays **parked**, not "in progress".

**Not done:** no code, no test. The hazard list keeps five entries and gains one - that this file is a step, not a log, with the
cap and the command that checks it.
