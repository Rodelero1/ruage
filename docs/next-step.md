# OPEN - the 12x is closed; the crowd is the fork, and the reds are named (D280)

**Where things stand (2026-11-19, measured).** D279's port is in `main`'s tree on this branch (`movement-clean-slate`):
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

**The second thing, and it needs no fork:** the port is **1.6x** off `main` on the acceptance fixture (42.62 s against
26.6 s) and **the remaining cost has no single owner**. Measured after D280 with the phase probe (timers on `World::step`,
deleted afterwards - they cost 0.15 ms a tick), at tick 14,400 of 48,000: **`decide` 12.8 s, `move_bodies` 6.8 s, `turns`
4.9 s** of 26.0 s, and nothing in the first 9,600 ticks stands out (`decide` 2.7 s, `turns` 1.2 s, `move` 0.9 s of 5.3 s).
That is a spread, not a defect: `decide` is about **36 us a unit-tick** across ~25 units in a debug build, which is the
shape of a large per-entity function rather than of one wrong question. So the next slice here is a **profile of
`decide`**, not a guess - the two wins this week both came from a question asked in the wrong order, and there may be a
third, but it has to be found the same way.

**The three divergences the conformance set names and `#[ignore]`s** - each with what it would cost:

1. **The hierarchy** (`zone_spatial_pyramid_search_hierarchical.cpp`). A flat A\\* over every tile on the 64 x 64 acceptance
   map. It becomes worth its complexity on a 480 x 480 map with hundreds of units planning per tick, which is not this map.
2. **The radius in the planner** (the original plans for *"the boundaries of where this unit's radii will go"*). Our search
   asks tiles and the **step** asks the disc, so a body too wide for a one-tile lane is planned through it and refuses its
   steps there. The wide body that must thread a narrow gap is the case to watch.
3. **The expansion budget** (`expansions > h`, cap 20,000; the guard's target is a *push*, `0x801861`-`0x80189d`, so what a
   budget-exhausted search answers is still unread). No budget: a hopeless search settles every reachable tile.

**Known reds on this branch, by class** (measured before and after D280; D280 moves none of them):

- **the crowd and the seat** (8): `a_walker_never_ends_a_tick_inside_a_friend`, `villagers_never_come_closer_than_one_diameter`, `a_mate_who_is_stopped_becomes_solid`, `six_settled_men_hold_their_places`, `ten_men_work_one_bush_a_body_apart`, `three_gatherers_all_work_the_same_square_on_its_ring`, `a_settled_man_is_not_displaced_by_a_later_arrival`, `a_newcomer_does_not_move_the_working_gatherers` - each asserts a rule the removed machinery provided. Under D279 a winner walks through a paused body and the settle no longer nudges, so these state a rule the engine no longer has. **They are the fixture half of the crowd fork above.**
- **the re-plan and the place model** (2): `a_blocked_unit_goes_round_a_standing_body_and_arrives`, `a_repath_route_is_pulled_like_a_planned_one`: a body collision no longer re-plans, so there is no "route round a standing body" to assert.
- **the gather refusal book** (3): `a_man_who_cannot_fit_takes_the_next_source`, `a_man_who_cannot_get_his_square_takes_the_next_tree`, `a_walled_off_wood_is_tried_once_and_then_left_alone`: the refusal these count is now decided by `start_gather`'s reach check, and the picker/scenario path does not record it.
- **edges** (7): `a_villager_drops_only_within_half_a_diameter_of_the_drop_site`, `a_walk_past_a_house_keeps_its_bodys_clearance`, `the_loser_stands_for_the_drawn_pause`, `both_sides_field_an_army`, `the_skirmish_only_refuses_what_the_fight_has_taken`, `a_queued_reseed_pays_the_farms_price_and_replaces_the_spent_plot`, `auto_reseed_is_a_player_setting_and_pays_when_the_plot_runs_out`: each needs a look and none is understood yet.

**Known reds that were already red on `main`** (`e63ed0d`):

- `animals::a_live_animal_is_a_gather_source` and `animals::the_shepherd_kills_the_sheep_and_the_gathering_resumes_on_the_body`
  - the arrival measure (ring distance vs the game's box gap), a separate slice, unchanged by D279 and D280.

**Next, in order:**

1. **The crowd**, above - an owner fork, not a slice to guess at.
2. **The arrival measure** for animals: "at the node" becomes the game's **box gap** within the job's range (`func_002690E0`)
   instead of the ring distance. That is the last two animals reds.
3. **The probe again, on `move_bodies`** - the one phase nobody has looked inside since the port.
4. **The owner's acceptance run.** What to look at: whether the match resolves, whether a crowd reaches a tree, and
   whether a group order to one tile looks like the game.
