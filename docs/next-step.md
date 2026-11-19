# OPEN - the clean slate is in; what it costs, and the next three slices (D279)

**Where things stand (2026-11-18, measured).** D279's port landed on branch `movement-clean-slate`: `path.rs`, `short.rs` and
`world/movement.rs` are **gone** (1,663 lines), and `ground.rs` (grid + one A\* + the line test), `world/motion.rs` (one whole
step a tick, four refusals) and `world/placement.rs` replace them. The withdrawn mechanisms are listed by name in the decision
row. The suite's last run (`--no-fail-fast`, after the final fixes): **198 test binaries, 876 passed, 22 failed, 21 ignored**;
conformance: 13 passed, 3 ignored.

**The first thing to fix, and it is measured.** `ruage-sim --test acceptance_economy` took **26.6 s** earlier in the same
session and takes **322 s** on this branch (debug, 48,000 ticks). Two experiments refute the obvious suspects - pausing
disabled entirely: **455 s**; every refusal short-circuited: **316 s** - so it is neither the pause nor the body scan.
**The box has no `perf`**, so the next session's first job is to profile the tick (a `std::time::Instant` around each phase
of `World::step`, printed every 400 ticks, is enough) and find where the 12x goes. The working hypothesis is *more of the
match functioning*: gather orders succeed now that the closest-tile planner does not refuse them, so more villagers walk and
every step stamps vision - but it is unproven and must not be quoted as the answer.

**The second thing, and it is visible:** a **crowd walks slowly**. Every collision pauses the higher id for
0.25-1.50 s (the original's own `setPause`), and the old settle - which nudged each man along the ring a fraction of a tile a
tick - used to hide it. Measured on `a_newcomer_does_not_move_the_working_gatherers`: eight villagers 10-12 tiles from one tree
seat **one** man at work inside 1,200 ticks. This is the first thing to price, and the honest options are (a) shorten the
drawn pause, (b) let a man whose goal is a *node* walk over his own mates the way a selection does, or (c) leave it - a crowd
converging on one tile is not the ordinary case. **No decision yet: this is a fork for the owner.**

**The three divergences the conformance set names and `#[ignore]`s** - each with what it would cost:

1. **The hierarchy** (`zone_spatial_pyramid_search_hierarchical.cpp`). A flat A\* over every tile is `<T>` us per whole-map
   route in a debug build on the 64 x 64 acceptance map. It becomes worth its complexity on a 480 x 480 map with hundreds of
   units planning per tick, which is not this map.
2. **The radius in the planner** (the original plans for *"the boundaries of where this unit's radii will go"*). Our search
   asks tiles and the **step** asks the disc, so a body too wide for a one-tile lane is planned through it and refuses its
   steps there. The wide body that must thread a narrow gap is the case to watch.
3. **The expansion budget** (`expansions > h`, cap 20,000; the guard's target is a *push*, `0x801861`-`0x80189d`, so what a
   budget-exhausted search answers is still unread). No budget: a hopeless search settles every reachable tile. Microseconds
   here, unbounded on a full-size map.

**Known reds on this branch, by class** (the exact list is in the review note):

- **the crowd and the seat** - eight fixtures that assert what the *removed* machinery did: `a_walker_never_ends_a_tick_inside_a_friend`, `villagers_never_come_closer_than_one_diameter`, `a_mate_who_is_stopped_becomes_solid`, `six_settled_men_hold_their_places`, `ten_men_work_one_bush_a_body_apart`, `three_gatherers_all_work_the_same_square_on_its_ring`, `a_settled_man_is_not_displaced_by_a_later_arrival`, `a_newcomer_does_not_move_the_working_gatherers`. Under D279 a winner walks through a paused body and the settle no longer nudges, so these state a rule the engine no longer has. **They are the honest measure of what "blunt port" cost**, and they are the fixtures the crowd slice above has to rewrite.
- **the re-plan and the place model** - `a_blocked_unit_goes_round_a_standing_body_and_arrives`, `a_repath_route_is_pulled_like_a_planned_one`: a body collision no longer re-plans, so there is no "route round a standing body" to assert.
- **the gather refusal book** - `a_man_who_cannot_fit_takes_the_next_source`, `a_man_who_cannot_get_his_square_takes_the_next_tree`, `a_walled_off_wood_is_tried_once_and_then_left_alone`: the refusal these count is now decided by `start_gather`'s reach check, and the picker/scenario path does not record it.
- **edges** - `a_villager_drops_only_within_half_a_diameter_of_the_drop_site`, `a_walk_past_a_house_keeps_its_bodys_clearance`, `the_loser_stands_for_the_drawn_pause`, `both_sides_field_an_army`, `the_skirmish_only_refuses_what_the_fight_has_taken`, `a_queued_reseed_pays_the_farms_price_and_replaces_the_spent_plot`, `auto_reseed_is_a_player_setting_and_pays_when_the_plot_runs_out`: each needs a look and none is understood yet.

**Known reds that were already red on `main`** (the baseline was taken on `main` `e63ed0d`):

- `animals::a_live_animal_is_a_gather_source` and `animals::the_shepherd_kills_the_sheep_and_the_gathering_resumes_on_the_body`
  - **red on `main` too** (D276 recorded three animals reds; this branch has two). They are the arrival measure (ring
  distance vs the game's box gap), a separate slice, unchanged by D279.
- Everything else in the tree is green.

**Next, in order:**

1. **The crowd**, above - an owner fork, not a slice to guess at.
2. **The arrival measure** for animals: "at the node" becomes the game's **box gap** within the job's range (`func_002690E0`)
   instead of the ring distance. That is the last two animals reds.
3. **The owner's acceptance run.** D279 was ordered without one ("don't run verification runs, I'll check it myself"), so the
   first real reading of a match under this model is his. What to look at: whether the match resolves, whether a crowd reaches
   a tree, and whether a group order to one tile looks like the game.
4. Then the three divergences, cheapest first: the radius in the planner is a day and changes no route's legality; the budget
   is a read plus a number; the hierarchy is a rewrite of the search.
