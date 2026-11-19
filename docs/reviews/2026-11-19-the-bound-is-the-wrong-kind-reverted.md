# Review: the short-range bound is the wrong *kind* of bound - measured, and reverted (NOT LANDED)

*The conformance set's last red is `divergences::the_original_solves_a_long_walk_in_its_hierarchy`: "the short-range solver
declined a 58-tile walk; the original has a hierarchy for exactly this". This slice was the cheap half of it - and it was
reverted, because it buys the red at a price the acceptance fixture refuses.*

## What the red actually is

`short.rs` refused any window wider than **40 tiles a side** (`MAX_SIDE`). The original has no such bound: the register's
row is *"stop after `h` expansions, never more than 20,000 - and answer the best so far"* (`FUN_008007f0`), a budget on
**work**. So on this 64 x 64 map the divergence is not "no hierarchy" but **a bound of our own of the wrong kind**: a
58-tile walk across open ground builds two nodes (its endpoints) and costs nothing, so declining it for its length is an
invention.

## What it was, and what it cost

Deleting `MAX_SIDE` (keeping `MAX_RECTS` 96 / `MAX_NODES` 400) makes the conformance set **green for the first time - 9
binaries, 0 failures, `--no-fail-fast`** - and restates `clearance::a_window_beyond_the_bound_is_declined` to the pair that
straddles the rectangle budget (121 spaced tiles declined, 36 solved, 58 tiles on open ground solved).

**And it is 1.63x slower:**

| build | `the_acceptance_match_trains_more_villagers`, debug, 48,000 ticks |
|---|---|
| `main` at `741f053` | **26.6 s** (recorded at `a4331de`, engine unchanged since) |
| with `MAX_SIDE` deleted | **43.38 s** (measured) |

**Why**: `MAX_RECTS`/`MAX_NODES` bound the *count* of geometry, not its *cost*. A 58-tile window holding sixty scattered
blocks is under both caps, and the visibility graph then runs its **exact integer segment tests over sixty-tile segments** -
work that grows with the square of the node count *and* with the length of every segment. The 40-tile window was
accidentally acting as a proxy for that cost, which is why removing it exposed the bill.

## Why it is reverted rather than kept

The conformance set is explicitly **not** the gate (D247: it asserts rules the engine does not keep yet). The acceptance
fixture *is* measured against `main`, and this project has already refused two changes for costing more than they bought
(the 12x picker regression, D280; the 5.1x standers-in-the-search, D282 on its own branch). A conformance red and a 26.6 s
match beat a conformance green and a 43.38 s one.

## What the next attempt is, precisely

**Bound the work, not the count.** The analogue of the original's expansion budget is a budget on the solver's own effort:
number of segment tests = `nodes x (nodes - 1) / 2`, each weighted by the segment's tile length, compared against `h x 25`
capped at 20,000 (the game's cap) before the graph is built. A 58-tile walk on open ground is 2 nodes and passes; a window
holding sixty rectangles declines to the tile chain, exactly as today. Fixtures already written for it: the three cases in
`clearance::a_window_beyond_the_bound_is_declined` as restated here, plus the divergence itself.

**Not a hierarchy either way**: this map needs none, and the hierarchy remains a separate row for a map where a flat search
over 4,096 tiles is actually too slow.
