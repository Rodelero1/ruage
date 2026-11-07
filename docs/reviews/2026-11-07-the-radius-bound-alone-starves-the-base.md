# The radius bound alone starves the base (D141)

*2026-11-07. Step 2 of four adopted from the prior-art watch. Implemented test-first, measured, **reverted** — and this file
is the measurement, because a rejected adoption with numbers is worth more than a shipped one without.*

## What was adopted

The original's gather search is a BFS bounded by the worker's **line of sight + 5** tiles, reach-testing at most **30**
objects. The digest priced it: *"the cap is a fidelity rule **and** the performance bound; adopting the bound and the cap is
cheap"*.

The **cap** is moot here and always was: D138 removed the reachability test from the picker, so a gather attempt spends
**one** search inside `start_gather`, not thirty. The radius was the whole question.

## The defect the test found (genuine RED)

The pack gives a villager `los = 4.0`, so the bound is **9 tiles**. A fixture with its only wood eighteen tiles away:

```
the only wood is ~18 tiles away and the villager sees nine: the rule must say there is no node rather than
walk him across the map. Refusals: []
```

`Refusals: []` — the rule issued the order without complaint and the villager set off across a twenty-tile map for the last
tree on it. That is the behaviour the bound is meant to stop, and it was real.

## Implementation

- `World::gather_reach(worker)` → `(los + 5)` tiles in raw units, with a **fallback of four tiles** of sight for a pack that
  gives none — "a missing number must not make the search unbounded", which is the failure the rule exists to prevent.
- `nodes_of_slot` gained a `reach` parameter and skipped candidates beyond it; `i64::MAX` stayed available as an explicit
  "unbounded" for a caller that wants it (there is none today).

Two call sites: the economy pass and the script's picker, both passing the worker's own reach.

## The measurement, which is the whole point

36,000 ticks, seed 7, release build, idle box, two runs each:

| | before | bound on |
|---|---|---|
| entities at 1,800 s | **141** | **129** |
| players alive | **1** | **2** |
| repaths / made-room | 188 / 187 | 83 / 164 |
| refusal lines | 574 | **767** |
| hash | `f328ebc8d4d34b78` | `66032535f7d24483` |
| wall | 11.5 s | 7.4 s |

**The match stops resolving.** Before, one side is eliminated inside the thirty minutes; with the bound, both are alive at
the end and the economy has produced twelve fewer entities. The wall clock *falls* because there is less happening — a
faster match that plays worse is exactly the reading a single number would have hidden.

## Why it failed, stated as a design fact rather than an excuse

The bound is not wrong; it is **the wrong half**. Theirs is a BFS from where the worker *stands*, in a game where the AI
keeps re-tasking idle villagers and a player's click is itself the long-range decision. Our `nodes_of_slot` serves **two**
jobs at once: it is the AI's "what should this villager do" and the unit's "which node exactly". Applying a short-range
rule to a long-range decision starves the base the moment the wood within nine tiles is gone — which on our maps is
immediately.

So the corrected shape is a **two-stage** search: the AI picks an *area* (long range, no sight bound), and the unit picks a
node within sight of that area. That is a real design change with its own tests, and it is queued — not guessed at.

## Performance

- With the bound: **7.4 s** for 36,000 ticks against **11.5 s** — but the sim is doing less (129 entities, 83 repaths
  against 141 and 188), so this is not a speed-up; it is a smaller match. Quoting it as a win would be exactly the error the
  project's own pitfalls file warns about.
- The check itself is one integer multiply per candidate node — free. The cost is behavioural, not arithmetic.
- Reverted: the tree is back at `f328ebc8d4d34b78`, 141 entities, 188 repaths, 11.5 s, **726 tests**.

## Stability

- The change was reverted with `git checkout` of the three touched files, and the acceptance run immediately after was
  byte-identical to the baseline (`f328ebc8d4d34b78`, 141 entities, players=1) — so the revert is verified rather than
  assumed.
- The **test** for the bound was reverted with the code: it asserts behaviour this project has decided not to adopt, and a
  test that pins a rejected rule is worse than no test.
- What survives is this document and D141, so the next session does not re-adopt the same half of the rule and rediscover the
  same twelve entities.
