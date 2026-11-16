# Review: R7's first half - the cost model read, and my own admissibility error

**What changed, documentation only.** `docs/movement-in-the-binary.md` gains *"The cost model settled (2026-11-16)"*;
`docs/behaviour-diff.md` row **13** is rewritten (verdict still `different`, but for a different reason); `docs/next-step.md`
item **R7** is replaced. No code, no config, no tooling.

## Performance

**Nothing to measure - no code moved**, and no number is claimed as a measurement of this tree. The figure the read produces is a
**ratio**: the heuristic weight is **25** against a step cost of **16**, i.e. **about 1.56**, which is what the original's search
trades path optimality for. That ratio, not a timing, is the point of the read.

## Stability

What was attacked: **my own earlier claim**, which was load-bearing for the queue.

1. **The "contradiction" was in my reading, not in the game.** The register said *"x25 against a 16-per-step cost is not
   admissible in tile units, so either those four arguments are not tiles or 16 is not the per-step cost"*, and R7 was written to
   settle which. Both are right: `FUN_007ffcd0` is Euclidean distance on **tile** coordinates (the same pair that indexes the
   best-cost array at `x*0x1e0 + y`, `0x1e0` = 480 = the map bound), 16 is the step cost (`FUN_007ffc80`, octile table plus
   `16*(max-min)`), and 25 is the heuristic **weight** - written two ways in one function: `*2500/100` and `*0x19`, which are the
   same number. A weighted heuristic is inadmissible **by design**; there was never a puzzle.
2. **The consequence is bigger than the constants, and it is stated rather than glossed.** Adopting this is adopting a search
   that returns slightly longer routes on purpose. That is a behaviour change in every route in every fixture, so row 13 is
   re-verdict-ed as `different in the search's shape` and R7 says the port is one slice with its own measurement.
3. **What remains unread is named, not assumed.** `local_46c` - the quantity the loop compares against `h` before breaking, beside
   a hard `20,000` - and the equal-`f` tie-break, which is frontier order rather than arithmetic. Porting either on a guess would
   be the same mistake this note is about.

**Not done:** no code, no test (nothing to pin yet), and row 27's `AddFuturePath` is untouched.

---

*This file is what `tools/review_check.py` looks for: `## Performance` with a number, `## Stability` with content, staged
with the change. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
