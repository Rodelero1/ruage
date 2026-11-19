# OPEN - the rules follow the decomp; the fixtures that do not are named (D282)

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

**Where this leaves the red set (19, measured after D282).** Three turned green this slice -
`a_blocked_unit_goes_round_a_standing_body_and_arrives` (4,619 raw against 8,000 -> keeps the clearance), the loser's drawn
pause (written `ticks - 1`, so a 0.25 s draw stood 0.20 s), and `the_ten_minute_scenario_builds_trains_and_fights` (the
builder stall: he plans round the man now). Two came back - `skirmish::both_sides_field_an_army` and
`skirmish::the_skirmish_only_refuses_what_the_fight_has_taken` - and they are a **timing knife-edge in the scripts**, the
same shape as the stall: one extra tick of pause and up to one detour is enough to miss a deadline that already recorded 921 of
1,000 at 62 s of a 70 s window.

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
4. **Rewrite the fixtures that contradict the game** (`villagers_never_come_closer_than_one_diameter`,
   `ten_men_work_one_bush_a_body_apart`, `a_man_who_cannot_fit_takes_the_next_source`, `a_walker_never_ends_a_tick_inside_a_friend`
   whose scene no longer meets, and `a_newcomer_does_not_move_the_working_gatherers`'s seat book) and re-point them at
   positions and at the rule the game actually states.
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
