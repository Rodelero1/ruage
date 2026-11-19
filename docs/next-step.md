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

**Next, in order** (post-D282: the radius in the plan, the pause off-by-one and the builder stall are all closed):

1. **Two bodies on one tile** - the last structural hole. The clearance is skipped for a body the mover already overlaps (else
   every plan starts inside its own clearance), so `a_mate_who_is_stopped_becomes_solid` stays red at **4,478 raw** and a man
   can be pushed through another who shares his tile. The game's answer is the radius query (`FUN_00844310`) over the objects
   on a tile, with the mover's own reach grown in; reading that consumer is the slice.
2. **A building's clearance** (`a_walk_past_a_house_keeps_its_bodys_clearance`): the clearance list carries bodies, not
   footprints, so a route still clips a house's corner by a rounding. The same `Clear` shape applies to a footprint's box.
3. **The seat churn** (the audit ruled these **in line**, so the engine is the wrong side): `six_settled_men_hold_their_places`,
   `three_gatherers_all_work_the_same_square_on_its_ring`, `a_settled_man_is_not_displaced_by_a_later_arrival`,
   `a_newcomer_does_not_move_the_working_gatherers`. A working man must not be moved off his place: the settle pass re-lays men
   who are already working, and `a_newcomer...` asserts our own seat book rather than positions, so it needs re-pointing too.
4. **Rewrite the fixtures that contradict the game** (`villagers_never_come_closer_than_one_diameter`,
   `ten_men_work_one_bush_a_body_apart`, `a_man_who_cannot_fit_takes_the_next_source`, and
   `a_walker_never_ends_a_tick_inside_a_friend`, whose scene no longer meets now that a head-on pair plans round each other).
5. **The arrival measure** for animals: "at the node" becomes the game's **box gap** within the job's range (`func_002690E0`)
   instead of the ring distance. That is the last two animals reds.
6. **The `decide` profile** - 39.58 s against `main`'s 26.6 s, a spread with no single owner (`decide` 12.8 s, `move_bodies`
   6.8 s, `turns` 4.9 s at tick 14,400 of 48,000).
7. **The three conformance divergences**: the radius in the planner is item 1's neighbour; the hierarchy and the expansion
   budget are map-scale and priced above.
8. **The lost guard**: `a_spawn_in_the_same_tick_does_not_move_the_wrong_entities` was a unit test the owner had dropped, and it
   was the only cover for "a mid-tick spawn shifts the entity list, so movements are applied by id, not index".

**Parked by the owner (2026-11-19): *"The failures to train militia do not seem relevant to the pathfinding. Skip them for
now."*** `skirmish::both_sides_field_an_army` (*"player 1 never fielded infantry"*) and
`skirmish::the_skirmish_only_refuses_what_the_fight_has_taken` (*"player 1 has no unit militia"*) stay **red and not looked
at** on this branch. They are a script deadline, not a movement rule - the step is refused because the building it trains
from is not finished - and they flip on a tick of movement timing (red on D279, green after D281, red after D282's one extra
tick of pause). **Not ignored and not skipped**: they still fail the suite, and the next session should know that is expected
on this branch.
