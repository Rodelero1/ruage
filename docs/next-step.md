# OPEN - the hierarchy is the only red; port it

> **2026-11-19, state of `main` (`d4cb9d3`), read this before the plan below.** Landed since the register: the storm is
> split (`docs/reviews/2026-11-19-the-storm-split-by-route-origin.md` - one ground predicate, half of it bodies); the
> short-range **window removal was measured and reverted** (conformance green, acceptance 26.6 s -> 43.38 s, so the bound
> to build is a **segment-test budget**, `docs/reviews/2026-11-19-the-bound-is-the-wrong-kind-reverted.md`); and the
> **game's start rule is in** - re-seed `2 x radius` (`FUN_007fb220`) plus *a body with somewhere to go is not a stander*
> (`isMoving`) - with our one-tile ring deleted (`docs/reviews/2026-11-19-the-games-start-rule-not-ours.md`).
>
> **The one red this leaves is `army_walk::a_crowd_walking_through_a_gap_stays_on_free_ground`** (`NoPath from (6, 20) to
> (19, 19)`), and the measured cause is **when we plan**: each order is planned as it is *submitted*, so the first of
> twelve men is planned while the other eleven have no route and are, by the game's own test, standers. The original plans
> when the order executes, with the group already formed - its search takes the **group object** and ignores its members
> (`FUN_0072cf10`: `+0xd0` count, `+0xd8` array, *no per-member test*, `docs/movement-in-the-binary.md` §"The path ignore
> list"), and its classifier re-tests them on `isMoving`.
>
> **So the next slice is the group, before the plan**: collect the tick's orders, write the group onto every member's
> route, and only then plan each - or submit without a route and plan at the first step in `move_bodies`. The first is
> smaller; the crowd fixture decides it. Nothing of the old window may come back to make it pass.


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

**Owner-ordered 2026-11-18 (D276): the slide and the ground cap go, in this order.**
1. **The order is reversed: substitution and no cap first, then the slide** (D276 amended, measured). The A/B is in: a re-plan's
   fallback prefixed with the body's own tile centre is **bit-identical** on `main` (`b5292af5a6072d57`, 131 entities,
   `players=1`), so it is inert - and the branch that bundled it with the **deleted slide** never resolves (`players=2`, *"not
   enough food: 500 needed, 195 in the bank"* from 2,387 s). With the slide gone, every ground graze becomes a re-plan, the
   route's two attempts are spent, and the man stands in `WaitGround` for ever. **So: build the original's substituted waypoint
   (register row 26) and delete `MAX_GROUND_REPLANS` first; then the slide, which should then cost nothing.** Keep the fallback
   prefix - it is free - as part of that slice. The remaining animals red is `a_live_animal_is_a_gather_source`, the arrival
   measure (ring distance vs the game's box gap), a separate slice.
2. **the expansion budget** - `expansions > h` (h = 25 x Euclidean), cap 20,000. **Blocked on a read**: the guard's target is a
   *push* onto the frontier (`0x801861`-`0x80189d`), not a return, so what a budget-exhausted search answers is unread. No
   guess goes in.
3. **plan for the radius** (row 21) - the graph layer still judges passage by tiles; `path.rs` has no radius in it.
4. **the walk** - no slide, the waypoint snapped to and spent, the body kept on the route leg by leg (D259/D260). Needs 3.
5. **the collision block's vocabulary** (rows 20, 22, 23, 25, 29, 30, 36) - `TakeAtRest` out, the avoid set out of the search,
   the pause re-drawn, the caps gone, the mate set re-tested on `isMoving`, the order's path vocabulary.
6. **the place model** - a gather place the body fits on, and an approach it can walk (D257).

**Owner-ordered 2026-11-18 (D276): the slide and the ground cap go, in this order.**
1. **A/B the two changes on branch `d276-no-slide`.** The fallback prefix (prefix a re-plan's fallback route with the body's own
   tile centre, `step.rs:1409`) takes **animals from 3 red to 1** and leaves the movement sweep green (12 binaries), but the
   acceptance match **stops resolving**: `main` reads **131 entities / players=1 / repaths 551**, the branch **153 / players=2 /
   474**, with *"not enough food: 500 needed, 195 in the bank"* from 2,387 s. **The build contains two changes and they were not
   separated** - the deleted slide and the prefix. So: measure the **prefix alone on `main`**, then **slide + prefix**, and take
   whichever resolves. The remaining red is `a_live_animal_is_a_gather_source` (*"must end up carrying meat"*): he arrives and
   does not work, which is the **arrival measure** (D262: ring distance vs the game's box gap), a separate slice.
2. **the expansion budget** - `expansions > h` (h = 25 x Euclidean), cap 20,000. **Blocked on a read**: the guard's target is a
   *push* onto the frontier (`0x801861`-`0x80189d`), not a return, so what a budget-exhausted search answers is unread. No
   guess goes in.
3. **plan for the radius** (row 21) - the graph layer still judges passage by tiles; `path.rs` has no radius in it.
4. **the walk** - no slide, the waypoint snapped to and spent, the body kept on the route leg by leg (D259/D260). Needs 3.
5. **the collision block's vocabulary** (rows 20, 22, 23, 25, 29, 30, 36) - `TakeAtRest` out, the avoid set out of the search,
   the pause re-drawn, the caps gone, the mate set re-tested on `isMoving`, the order's path vocabulary.
6. **the place model** - a gather place the body fits on, and an approach it can walk (D257).

**Owner-ordered 2026-11-18 (D276): the slide and the ground cap go, in this order.**
1. **the pull inside `short_range_clear` is tile-based, so it certifies legs the disc refuses.** Two probes on branch
   `d276-no-slide` (review `docs/reviews/2026-11-18-d276-the-slide-and-the-trace.md`) moved the answer twice: the sheep
   fixture's routes come **from `short_range_clear`, not the tile fallback**; both are planned from `(300000,300000)`, the
   corner of tile (15,15) where `Spawn` put the body, and the follower is **0.85 tiles** from it when it first refuses. The
   tick-32 re-plan returns a *different* route and then never changes for 270 ticks, so it is not stale - it is wrong by the
   mover's measure: its first leg is clear to a **tile** line test and refused by the **disc** step test at **164 raw**.
   **Next: give the pull the body's radius (`line_clear_body`), change nothing else, and report the animals count and the
   acceptance run.** D261 bundled this with four other changes and measured 3 red; it has never been measured alone. (The
   reverted attempt - prefixing the fallback route with the tile centre - is recorded so it is not tried again.)
2. **the expansion budget** - `expansions > h` (h = 25 x Euclidean), cap 20,000. **Blocked on a read**: the guard's target is a
   *push* onto the frontier (`0x801861`-`0x80189d`), not a return, so what a budget-exhausted search answers is unread. No
   guess goes in.
3. **plan for the radius** (row 21) - the graph layer still judges passage by tiles; `path.rs` has no radius in it.
4. **the walk** - no slide, the waypoint snapped to and spent, the body kept on the route leg by leg (D259/D260). Needs 3.
5. **the collision block's vocabulary** (rows 20, 22, 23, 25, 29, 30, 36) - `TakeAtRest` out, the avoid set out of the search,
   the pause re-drawn, the caps gone, the mate set re-tested on `isMoving`, the order's path vocabulary.
6. **the place model** - a gather place the body fits on, and an approach it can walk (D257).

**Owner-ordered 2026-11-18 (D276): the slide and the ground cap go, in this order.**
1. **Build the original's substituted waypoint, and delete the cap** (D276's amended order, D278's gate). The mover counts
   consecutive refusals on the **same waypoint**; past that wait it **skips the waypoint** instead of standing - the binary's own
   words are *"we won't reach our waypoint either this or the next update, but we have a count of %d, faking it"*
   (`mConsecutiveSubstituteCount`, register row 26) - and a ground refusal no longer spends `MAX_GROUND_REPLANS`. Fixtures: a
   waypoint behind a wall is skipped within N ticks and the following one is reached; an unreachable goal ends with the man
   standing, not looping; `repath_budget`'s "at most two" is rewritten to the game's rule. **Then the acceptance: the win, the
   food wall, the counts.**
2. **Delete the slide** (`StepDecision::Slide`, D253/D255/D256) - one step applied whole; a refusal is a re-plan or a stand. It
   should cost nothing once item 1 lands, and that is the test of item 1. Keep the fallback prefix: measured inert, and free.
3. **The arrival measure** - "at the node" becomes the game's **box gap** within the job's range (`func_002690E0`) instead of a
   ring distance. That is the last animals red.
4. **The levelled search** - the last conformance red.

**The gate (owner, 2026-11-18, D278): the win, the economy, the suite.** `entities >= 155` is retired - it punished winning,
because a beaten side stops training and its count falls. Every slice reports **`players`** (the match must resolve), **whether
the economy hit a resource wall** (*"not enough food: 500 needed, 195 in the bank"* late in a run is a failure), and the entity
and re-path counts as information. `main` reads **131 / players=1** and is the baseline to beat; `d276-no-slide` reads **153 /
players=2** with the food wall and fails.
