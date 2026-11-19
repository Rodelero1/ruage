# OPEN - the 12x is closed, the body collision follows the decomp, and the stall is named (D281)

**Where things stand (2026-11-19, measured).** D279's port is on this branch (`movement-clean-slate`, HEAD `532a997`):
`ground.rs` (grid + one A\\* + the line test), `world/motion.rs` (one whole step a tick, four refusals) and
`world/placement.rs` replace 1,663 deleted lines. D280 closed the **12x regression** it left: `ruage-sim --test
acceptance_economy the_acceptance_match_trains_more_villagers` was **330.97 s** and is **42.62 s** (debug, 48,000 ticks;
`main` is 26.6 s). The picker now asks four cheap questions and a distance of every node, then one place and one route of
**the nearest eight** - and it no longer answers a node with nothing left in it, which had four villagers bouncing between
two spent bushes every tick (7,369 re-tasks by tick 14,400).

**The crowd follows the decomp now (D281, landed) - and the model exposed a stall.** A non-past body collision **re-plans the
mover round the collider's tiles**, the **higher id pauses** 0.25-1.50 s, the epilogue **writes no position** (bodies overlap as
they pass), a collider is *past* only by `path->createdAt >= other->lastMoveTime`, and the path-ignore list is the mover's
**moving** mates - so a stopped body blocks, and D279's *"not going anywhere is past"* exemption is gone. Acceptance fixture
**42.62 s -> 39.59 s**; three of D279's reds green; suite **198 binaries, 881 passed, 20 failed, 21 ignored** (from 880/22/21).

1. **First, and it is a stall the new model exposed:** `the_ten_minute_scenario_builds_trains_and_fights` fails at 500 s with
   the barracks at `construction=Some(progress 0 of 1000)` and its builder `Idle`. A man shut out of his own build site is a
   broken match, not a look - the question is whether he never reached it, or reached it and could not get a place (the same
   tile-sharing hole that leaves `a_mate_who_is_stopped_becomes_solid` at **4,207 raw**).
2. **Two bodies on one tile cannot be planned round** - the search starts on the tile they share - so the avoid list is empty
   where it matters most. The game's answer is a *radius* query (`FUN_00844310`) whose tile array holds every object, not a
   tile list; ours holds counts. That gap is now the one to read.
3. **The eight crowd fixtures** assert the withdrawn settle ring (`a_walker_never_ends_a_tick_inside_a_friend`,
   `villagers_never_come_closer_than_one_diameter`, `six_settled_men_hold_their_places`, `ten_men_work_one_bush_a_body_apart`,
   `three_gatherers_all_work_the_same_square_on_its_ring`, `a_settled_man_is_not_displaced_by_a_later_arrival`,
   `a_newcomer_does_not_move_the_working_gatherers`, and `a_mate_who_is_stopped_becomes_solid`'s second assertion): they are the
   crowd slice's to rewrite to the game's rule - bodies overlap as they pass, a stopped body blocks, the loser stands for the
   drawn pause.
4. **One guard was lost on the owner's ruling**: `a_spawn_in_the_same_tick_does_not_move_the_wrong_entities` was a unit test
   and is deleted. It protected "a unit spawned mid-tick shifts the entity list, so movements are applied by id, not index" -
   **nothing covers that now**. If a wrong-entity movement is ever suspected, this is the first place to look.

**The second thing, and it needs no fork:** the port is **1.5x** off `main` on the acceptance fixture (39.59 s against
26.6 s) and **the remaining cost has no single owner**. Measured with the phase probe (timers on `World::step`, deleted
afterwards - they cost 0.15 ms a tick), at tick 14,400 of 48,000: **`decide` 12.8 s, `move_bodies` 6.8 s, `turns` 4.9 s** of
26.0 s, and nothing in the first 9,600 ticks stands out. That is a spread, not a defect: `decide` is about **36 us a
unit-tick** across ~25 units in a debug build, the shape of a large per-entity function rather than of one wrong question.
So the next slice there is a **profile of `decide`**, not a guess - the two wins this week both came from a question asked in
the wrong order, and there may be a third, but it has to be found the same way.

**The three divergences the conformance set names and `#[ignore]`s** - each with what it would cost:

1. **The hierarchy** (`zone_spatial_pyramid_search_hierarchical.cpp`). A flat A\\* over every tile on the 64 x 64 acceptance
   map. It becomes worth its complexity on a 480 x 480 map with hundreds of units planning per tick, which is not this map.
2. **The radius in the planner** (the original plans for *"the boundaries of where this unit's radii will go"*). Our search
   asks tiles and the **step** asks the disc, so a body too wide for a one-tile lane is planned through it and refuses its
   steps there. The wide body that must thread a narrow gap is the case to watch.
3. **The expansion budget** (`expansions > h`, cap 20,000; the guard's target is a *push*, `0x801861`-`0x80189d`, so what a
   budget-exhausted search answers is still unread). No budget: a hopeless search settles every reachable tile.

**Known reds on this branch (20, measured after D281) - now classified against the game** (`docs/reviews/2026-11-19-fixtures-vs-the-decomp.md`):

- **the fixture contradicts the game** (3, fixture work - rewrite them): `villagers_never_come_closer_than_one_diameter` (a global no-overlap invariant the game does not have: a *past* collider is dropped and the position stands), `ten_men_work_one_bush_a_body_apart` (asserts `working.len() == touching_place_count(...)`, a cap of seven, under a name that says ten and no cap; and it compares the engine to the helper the engine itself calls), `a_man_who_cannot_fit_takes_the_next_source` (the same cap; the retarget half is unread in both sources).
- **not a movement rule** (3): `a_villager_drops_only_within_half_a_diameter_of_the_drop_site` (the drop order's range is unread), `a_man_who_cannot_get_his_square_takes_the_next_tree` (our ring; the rule is the owner's D204/D241), `a_walled_off_wood_is_tried_once_and_then_left_alone` (the runner's refusal book).
- **the engine is wrong, and the cause is named** (6): `a_blocked_unit_goes_round_a_standing_body_and_arrives` (**4,619** raw against 8,000 - the plan carries tiles, the game carries a rect grown by the mover's radius, `FUN_007f9d80`), `a_walk_past_a_house_keeps_its_bodys_clearance` (same hole), `a_mate_who_is_stopped_becomes_solid` (same hole, two bodies on one tile), `the_loser_stands_for_the_drawn_pause` (our pause is written `ticks - 1`, so a 250 ms draw becomes **0.20 s**, under the game's floor), and the two animals reds (arrival is a centre distance where the game uses the **box gap** against `rangeDistance`, `func_002690E0`).
- **in line, with a caveat** (2): `six_settled_men_hold_their_places`, `three_gatherers_all_work_the_same_square_on_its_ring` (their doc comments cite the withdrawn 1.156 ring), `a_settled_man_is_not_displaced_by_a_later_arrival` (its `between >= 0.39` half is stricter than the game in that scene), `a_newcomer_does_not_move_the_working_gatherers` (asserts our seat book, not positions).
- **not attributed** (3): the two reseed reds, and `a_walker_never_ends_a_tick_inside_a_friend` is **in line** - its name is stale, its assertions are not; its red is an engine defect.
- **the stall** (1): `the_ten_minute_scenario_builds_trains_and_fights` - **0 of 1,000** builder progress in 1,400 ticks: the builder never entered the adjacency band (`step.rs:1610`) or `site_is_clear` refused every stroke (`build.rs:188`).

**Next, in order:**

1. **The radius in the plan** (`FUN_007f9d80`: a stopped object enters the search as a rect **grown by the mover's radius**;
   `FUN_00840830` grows the query box by `+0x268` and collides on `<=`). Our `replan_around_bodies` avoids the collider's
   *tile*, so a man keeps 0.577 of the standoff (`a_blocked_unit...` 4,619 of 8,000) and two bodies on one tile cannot be got
   round at all. That one change is the answer to three reds at once, and it was built once before (D211: *"the eight vertices
   of the octagon that circumscribes their clearance"*) and dropped in the D281 rewrite.
2. **The pause off-by-one**: `motion.rs:152` writes `ticks.saturating_sub(1)`, so a 250 ms draw stands **0.20 s**. Write the
   drawn duration.
3. **The builder stall** (`the_ten_minute_scenario_builds_trains_and_fights`): **0 of 1,000** progress in 1,400 ticks - either
   the builder never entered the adjacency band (`step.rs:1610`) or `site_is_clear` refused every stroke (`build.rs:188`).
4. **Rewrite the three fixtures that contradict the game** (`villagers_never_come_closer_than_one_diameter`,
   `ten_men_work_one_bush_a_body_apart`, `a_man_who_cannot_fit_takes_the_next_source`), and re-point
   `a_newcomer_does_not_move_the_working_gatherers` at positions rather than our seat book.
4. **The owner's acceptance run** - D279 and D281 both landed without one by his instruction. What to look at: whether the
   match resolves, whether a crowd reaches a tree, and whether a group order to one tile looks like the game.
5. **The arrival measure** for animals: "at the node" becomes the game's **box gap** within the job's range (`func_002690E0`)
   instead of the ring distance. That is the last two animals reds.
6. **The probe on `decide`**, for the remaining 1.5x.
7. **The three conformance divergences**: the radius in the planner is item 2 above; the hierarchy and the expansion budget
   are map-scale items, priced in the section above.
8. **One guard was lost on the owner's ruling**: `a_spawn_in_the_same_tick_does_not_move_the_wrong_entities` was a unit test and
   is deleted. It protected "a spawn mid-tick shifts the entity list, so movements are applied by id, not index" - **nothing
   covers that now**. If a wrong-entity movement is ever suspected, look there first.
