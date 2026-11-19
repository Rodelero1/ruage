# OPEN - the 12x is closed; the crowd is the fork, and the reds are named (D280)

**Where things stand (2026-11-19, measured).** D279's port is in `main`'s tree on this branch (`movement-clean-slate`):
`ground.rs` (grid + one A\\* + the line test), `world/motion.rs` (one whole step a tick, four refusals) and
`world/placement.rs` replace 1,663 deleted lines. D280 closed the **12x regression** it left: `ruage-sim --test
acceptance_economy the_acceptance_match_trains_more_villagers` was **330.97 s** and is **42.62 s** (debug, 48,000 ticks;
`main` is 26.6 s). The picker now asks four cheap questions and a distance of every node, then one place and one route of
**the nearest eight** - and it no longer answers a node with nothing left in it, which had four villagers bouncing between
two spent bushes every tick (7,369 re-tasks by tick 14,400).

**The first thing left, and it is visible:** a **crowd walks slowly**. Every collision pauses the higher id for 0.25-1.50 s
(the original's own `setPause`), and the old settle - which nudged each man along the ring a fraction of a tile a tick -
used to hide it. Measured on `a_newcomer_does_not_move_the_working_gatherers`: eight villagers 10-12 tiles from one tree
seat **one** man at work inside 1,200 ticks. The honest options are (a) shorten the drawn pause, (b) let a man whose goal is
a *node* walk over his own mates the way a selection does, or (c) leave it - a crowd converging on one tile is not the
ordinary case. **No decision yet: this is a fork for the owner.**

**The second thing, and it needs no fork:** the port is **1.6x** off `main` on the acceptance fixture (42.62 s against
26.6 s) and the next hotspot is not yet named. The probe that found the last two is in `docs/reviews/2026-11-19-d280-the-picker.md`
(phase timers on `World::step` and on `apply_turn`, deleted afterwards because they cost 0.15 ms a tick): the split at
tick 14,400 after both halves should be taken again before anything is guessed - `move_bodies` was **3.0 s by tick 9,600**
and 6.2 s by 14,400, which is the largest of the phases that never had a defect found in it.

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
