# Review: the movement layer deleted and re-ported from the original (D279)

*Change: `crates/sim/src/path.rs`, `crates/sim/src/short.rs` and `crates/sim/src/world/movement.rs` are **deleted**
(1,663 lines), and three files replace them: `ground.rs` (the tile grid, one A\*, the line test), `world/motion.rs` (one
whole step a tick, four refusals) and `world/placement.rs` (`push_out_of_buildings`, `place_apart`, moved unchanged). The
ten-answer collision state machine, the short-range visibility graph, the place model, the avoid set, the re-plan budget,
the substituted waypoint and the step fold go with them, and `StepDecision` is 7 variants instead of 10. Every withdrawn
mechanism is named in `docs/next-step.md`; the decision is D279.*

## Performance

**The owner's instruction for this slice was that verification runs are his, so there is no acceptance hash.** What is
measured, all on this tree:

- **a 12x regression in the acceptance fixture**, and it is the headline of this review: `ruage-sim --test
  acceptance_economy` took **26.6 s** earlier in this session and takes **322 s** now (`the_acceptance_match_trains_more_villagers`,
  debug build, 48,000 ticks). Two experiments refute the obvious suspects: with the collision pause disabled entirely it
  is **455 s**, and with every refusal short-circuited it is **316 s** - so neither the pause nor the body scan is the
  cost. The remaining hypothesis, unproven, is **more work per tick because more of the match functions**: gather orders
  now succeed where the closest-tile planner used to refuse them, so more villagers walk, and every step stamps vision.
  **This is not profiled** (no `perf` on this box) and is the first item of the next step.
- **the suite**: the last **complete** run reads **873 passed, 25 failed, 21 ignored across 198 test binaries**. Two
  fixes landed after it (a bare world with no map must still walk, and a walk order is always accepted) and the re-run was
  still going when this note was written, so **25 is an upper bound** - the exact list is in `docs/next-step.md`;
- **conformance**: 13 passed, 3 ignored (the three layers this port does not carry: hierarchy, radius-aware search,
  expansion budget);
- **the code**: 1,663 lines removed against 789 added in the engine's movement layer. The per-tick *work* the pass does is
  smaller by construction - a linear scan of units per moving body, at most one search per man per 20 ticks, no sorted
  body index and no four-searches-a-tick budget - which is why the regression above points at behaviour, not at the pass.

## Stability

**What was attacked, and what bit (every one of these is mine):**

- **The clean-slate delete left the tree uncompilable in 30 places.** Twelve fixtures were deleted with the code they
  pinned (`pathing`, `short_range`, `short_probe`, `clearance`, `movement_worth`, `repath_reasons`, `carrier_no_pushing`,
  and five conformance files); the rest were re-pointed at `Ground` and the new vocabulary.
- **A blocked goal answered `NoPath` instead of the closest tile**, which broke *every* gather order: the search preferred
  the **start** cell when no cell was closer to the goal, so `settled_best == start` returned `None`. The start is no
  longer a candidate.
- **`a_touching_tile_is_enough_to_bank` froze a gatherer for 1,200 ticks.** Two defects: the step refused any landing
  whose *disc* overlapped a blocked tile (28 raw units of rounding wedged him on a bush's corner) and the gather arm's
  `settling_now` gate stood him still while the new settle only sets a route. The disc test is now the **tile** test the
  original's collision block uses, and the gate is gone.
- **A gatherer froze 1.36 tiles from his tree from tick 100 to 800** - `settle_workers` re-issued his reserved place
  every tick, undoing every fallback the step pass made. It now only aims a man who has nowhere to go, and a place the
  body cannot reach falls back to the tile beside it after one refused re-plan.
- **`can_reach` was true for a sealed site**: the search answers a walled-in goal with the closest tile, so "a route
  exists" became meaningless and an unreachable build was paid for. It is now "the route ends within two tiles" (two, not
  one, or every tree in the game is unreachable - that cost `research.rs` fourteen reds in one run).
- **`a_walking_villager_takes_whole_steps` caught three real defects**: the task layer clips its delta at the next
  waypoint (a 287-unit "step" was taken), per-axis rounding made every diagonal step 798 of 800, and a step that crossed
  a waypoint could be refused after moving. The step is now the body's own `speed / tick_rate`, rescaled to that length,
  and a refusal rolls the waypoints back and stands still.

Known and accepted: **no acceptance run** (the owner's call); the three `#[ignore]`d conformance divergences are real
gaps priced in `docs/next-step.md`; the gather ring/standoff is **kept** (placement, not movement); **a crowd passes
through itself** - the original drops the retraction its radius test computes and pauses the loser instead - so a pair of
bodies can come as close as 0.02 tiles while passing, which is why `overlap.rs` now asserts a settled crowd a body apart
rather than no-deepening.

---

*This file is what `tools/review_check.py` looks for: `## Performance` with a number, `## Stability` with content, staged
with the change. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
