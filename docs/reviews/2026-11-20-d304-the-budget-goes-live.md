# Review: the search budget goes live (P1, D304)

*Item 1 of `docs/next-step.md`. The rule was already in the code - what was parked was the proof.*

## Crosscheck, before touching anything

- **The enforcement is already there**: `search_reporting` computes `budget = Self::estimate(start, goal).min(EXPANSION_CAP)`, stops
  **before** expanding a node it cannot afford (`if expanded >= budget { break; }`), and answers from `settled_best` - the closest cell it
  stood on, ties to the cheaper one, the start excluded. That is `FUN_008007f0`'s shape exactly: budget from the Euclidean distance × 25,
  hard cap 20,000 (`20000 < local_46c`), best-settled on exhaustion.
- **What was missing was never the budget - it was the search that could afford it.** D279 measured a flat search truncating a legitimate
  detour ("a villager one tile from its sheep answered the node fifteen tiles east") and parked the two conformance rows rather than
  enforcing a rule that broke routes. P11 removed that cause: long walks start on the levelled search, whose coarse nodes cover `1 << level`
  tiles.

## What changed

- `crates/conformance/tests/budget.rs`: the two rows are **live** - `a_hopeless_search_is_bounded_by_its_own_estimate` and
  `the_budget_never_exceeds_twenty_thousand` - and the three doc comments that said "not enforced yet" are corrected, with the parked
  measurement and why it no longer holds.

## Test-first note, honestly - and the third row that fixes it

This item is unusual: **the witness existed before the work, and the work that satisfied it was a different item** (P11). So there was no
red-then-green here - there was a *green-then-un-parked*, and un-parking two rows admits a vacuous pass: a search that never truncates
satisfies `expansions <= budget` trivially.

**So a third row was written to close that hole**: `a_search_that_runs_out_of_budget_stops_short_of_the_goal` seals a 40×40 room whose exit
is outside it, with a short walk (small budget) and a huge reachable interior, and asserts that the search **spends exactly its budget** and
answers **inside the room**. Mutation-proved: with `budget` replaced by `u32::MAX`, both this row and the estimate row fail -
`the search must spend exactly its budget before answering short: 1498 of 1000` - and with the budget restored, **4 passed**.

## Performance

- `crates/conformance/tests/budget.rs`: **4 passed / 0.00 s**, including the new biting row, and mutation-proved both ways;
- the conformance crate: **20 passed / 1 ignored** on the budget-adjacent binaries, and the row that matters,
  `a_hopeless_search_is_bounded_by_its_own_estimate`, runs in **0.00 s** on a 64×64 map with a sealed 5×5 ring - a hopeless goal whose
  search must hit the bound;
- the workspace suite is unchanged: **918 passed / 6 failed / 21 ignored** (the conformance crate is its own workspace and outside the gate);
- no hot path changed, so no timing beyond the crate's own.

## Stability

**The risk in un-parking a row is a vacuous pass**, and it was not ignored: the biting row (above) is the guard, and the mutation proves it
bites. **What is still weak** is that two of the four rows compute the budget with the same `estimate` the search uses, so a shared arithmetic
error would satisfy them both; the 20,000-cap row is the independent one, and the biting row is the one that would notice the search
ignoring its own budget entirely.
