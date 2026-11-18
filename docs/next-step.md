# OPEN - the hierarchy is the only red; port it

**Where things stand (2026-11-18, measured).** D275 orders the game's pathing ported in full. Before writing code, the queue's
first two items were checked against the tree and the set, and **both were already done**:

- **the cost model is ported** - `crates/sim/src/path.rs` carries **16** per cardinal step, **23** per diagonal and
  **`h = 25 x Euclidean`** (a weighted A\*, weight ~1.56), landed in `16853ac`; the conformance test
  `cost_model::the_originals_step_cost_is_sixteen` **passes**;
- **the closest point is answered** - the search settles on the tile nearest the goal instead of returning `None`
  (rows 19/26/31), so `nearest_reachable`'s one-step defect went with the helper.

**The set, run today with `--no-fail-fast`: 36 tests, 35 green, 1 red.** (The first run said 15/14/1, which was a floor, not a
total: without the flag cargo stops at the first failing binary, so four of seven files never ran. The second red it hid was
`walkability::open_ground_costs_the_octile_distance_and_no_more`, a fixture that asserted an **optimal** route — the admissible
search's rule, which the ported weighted search does not keep. It now asserts `octile <= cost <= octile x 25/16`, cited to
`FUN_008007f0`, and passes.) The red one left is the whole of the next step:

> `divergences::the_original_solves_a_long_walk_in_its_hierarchy` - *"the short-range solver declined a 58-tile walk; the
> original has a hierarchy for exactly this"*.

`short_range` declines past a **40-tile window** (`short.rs:29-36`) and the long route falls back to the tile chain. The
original plans the long route through a **pyramid**: a node carries a `level`, a coarse node spans `<< level` tiles, and an
edge is refined to a tile on the coarse node's boundary (`zone_spatial_pyramid_search_hierarchical.cpp`,
`..._check_edge.cpp`), with a parent path installed when the low-level search fails (`AddFuturePath`: *"We don't have a
parent path, so we're stopping"*).

**This step, test-first.** A coarse grid over the walk layer; A\* over coarse cells on the same 16/23 costs; each coarse edge
refined to a tile on the boundary shared with the next cell; `short_range` solving each refined segment; a segment that still
fails asks the parent for the next one rather than declining. Fixtures, watched failing first: the red above; one that a
58-tile route **arrives** rather than merely existing; one that a wall across the coarse grid ends in a route that stops at the
wall, never in `None`. Then the acceptance run - hash moves, entities and repaths reported against `entities >= 155` /
`repaths` near 380, **and the match's resolution reported beside them**.

**The rest of D275's queue, in order, and honestly priced:**

2. **the expansion budget** - `expansions > h` (h = 25 x Euclidean), cap 20,000. **Blocked on a read**: the guard's target is a
   *push* onto the frontier (`0x801861`-`0x80189d`), not a return, so what a budget-exhausted search answers is unread. No
   guess goes in.
3. **plan for the radius** (row 21) - the graph layer still judges passage by tiles; `path.rs` has no radius in it.
4. **the walk** - no slide, the waypoint snapped to and spent, the body kept on the route leg by leg (D259/D260). Needs 3.
5. **the collision block's vocabulary** (rows 20, 22, 23, 25, 29, 30, 36) - `TakeAtRest` out, the avoid set out of the search,
   the pause re-drawn, the caps gone, the mate set re-tested on `isMoving`, the order's path vocabulary.
6. **the place model** - a gather place the body fits on, and an approach it can walk (D257).

**Waiting on the owner, and it does not block item 1:** the acceptance floor `entities >= 155`. The control reads **154 and
never resolves**; the D250 build reads **137 and wins at 2,381 s**. Until it is restated every slice reports **both** numbers.
