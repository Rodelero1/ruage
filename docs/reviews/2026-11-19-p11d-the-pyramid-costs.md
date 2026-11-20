# Review: what the pyramid costs, and why the switch (11d) does not ship yet (D300)

*Adversarial performance review of P11 before wiring it in. The probe is `crates/sim/tests/levels_cost.rs`; every number below is printed
by it on the project-sized map (64×65) over 200 runs.*

## Performance

The numbers, from `crates/sim/tests/levels_cost.rs -- --nocapture`:

| operation | cost |
| --- | --- |
| build the four level maps (`Levels::of`) | **543 µs** |
| the descent (`Levels::descent`) | **129 ns** |
| the coarse A\* (`coarse_route`, a 56-tile diagonal) | **139 µs** (15 cells) |
| **the flat tile search on the same walk** | **11.4 µs** (1 waypoint) |

**The levelled search as built is ~60× slower than the flat one**, and the reason is not the search: it is the **543 µs pyramid build**,
which the current API does per call. The binary does not pay that - it holds the pyramid and only re-stamps it per search
(`FUN_007f1100`, which clears with `& 0xf` at the end of `FUN_007ee980`), so its cost per search is the **cell** A\* alone, over cells 8
tiles wide.

## What that means for 11d

**Wiring `coarse_route` into `Ground::route` today would be a 60× regression on every long walk**, and the acceptance run plans hundreds
of routes. So 11d is **not** the switch; it is the piece the switch needs:

1. **a cached pyramid** on `Ground`, rebuilt or incrementally stamped when a tile is blocked or cleared (`Ground::block` / `unblock`
   are the only mutators, so the invalidation is a dirty flag, not a scan);
2. **the coarse search measured again** against 11.4 µs - 139 µs for 15 cells is still 12× the flat search, and the suspects are the
   O(n) frontier insert and the per-neighbour `step_cost` (four map lookups each). A heap would fix the first but break D299's
   ordering, so the honest fix is a bounded insertion (the frontier is tens of nodes) or a bucketed list;
3. only then the switch, with the full suite and acceptance as the gate.

## Stability

**This is the review doing its job**: the hierarchy is *correct* (D299's fixtures, and the conformance row that had been parked since
D279, all green), and it is **not yet affordable**. Shipping it because it is correct would have cost the match on the one number the
game is judged by, which is the failure mode `docs/reviews/2026-11-16-*` records for D249's reachable-region sweep.

**The measured cost is also the honest answer to the original question** - why the binary's search can afford a tight budget: its coarse
node covers 8×8 tiles, its pyramid is persistent, and its per-search work is the cell A\* alone.

## One more of my own errors

The probe's first fixture declared `height = 64` over **65** written rows - the third hand-counted map error this session, each caught by
the loader's own check. The fixture now sets the height from the row count.

## Suite

`crates/sim/tests/levels_cost.rs`: **1 passed / 0.14 s**; the workspace suite is unchanged (no production code touched by this commit).
