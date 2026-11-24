# next-step.md before D292 (kept for the slices it describes)

**Where this branch stands (2026-11-19, measured, `blunt-port`).** D286's four edits are **landed**: an order names the object
a click landed on (`Command::Move`/`WalkTo`/`Task::Move` carry `target: Option<EntityId>`, filled by the window from
`pick_entity`); `ground::Clear` is a **box**; the plan keeps clear of the **blocked tiles'** boxes as well as bodies
(`solid_boxes`, `crosses_an_object`, `leg_is_clear`); and the step asks the same box question, **cutting a step at the target's
face** (`target_contact`). Acceptance fixture **39.58 s -> 33.74 s**; suite **199 binaries, 884 passed, 21 failed, 21 ignored** - four of the 23
baseline reds green, two joined (the owner's parked skirmish pair - see item 4). Full account, including the nine corrections the suite forced
onto the slice: `docs/reviews/2026-11-19-d286-one-box-ruler.md`, and D286 in `docs/decisions.md`.

**The red set now, by name (21).** `a_mate_who_is_stopped_becomes_solid`, `six_settled_men_hold_their_places`,
`three_gatherers_all_work_the_same_square_on_its_ring`, `a_settled_man_is_not_displaced_by_a_later_arrival`,
`ten_men_work_one_bush_a_body_apart`, `villagers_never_come_closer_than_one_diameter`, `a_walker_never_ends_a_tick_inside_a_friend`,
`the_loser_stands_for_the_drawn_pause`, `a_man_who_cannot_fit_takes_the_next_source`,
`a_man_who_cannot_get_his_square_takes_the_next_tree`, `a_villager_drops_only_within_half_a_diameter_of_the_drop_site`,
`a_depleted_node_sends_the_villager_to_the_next_one_of_its_kind`, `a_forager_whose_bush_is_finished_goes_to_a_farm`,
`a_gather_step_can_put_a_villager_on_a_farm`, `a_walled_off_wood_is_tried_once_and_then_left_alone`,
`a_tree_is_down_within_sixteen_seconds`, `a_tree_is_still_standing_ten_seconds_in`, the reseed pair
(`a_queued_reseed_pays_the_farms_price_and_replaces_the_spent_plot`, `auto_reseed_is_a_player_setting_and_pays_when_the_plot_runs_out`),
and the parked skirmish pair.

**Next, in order:**

1. **The trees (four reds, one owner).** `a_tree_is_down_within_sixteen_seconds` and `a_tree_is_still_standing_ten_seconds_in`
   say a tree's hit points do not fall at the pack's 1.5 hp/s (measured: 20 hit points left after 16 s); the two "next tree"
   reds say a woodcutter whose tree ran dry does not move on. Both are the *chop* path, not the walk, and both were red before
   this slice - read `grant_takes`'s chop carry (`fell_carry`) against the pack's `fell_milli_per_second` before touching
   anything else, because a tree that never falls also never empties, which would explain the second red for free.
2. **Two bodies on one tile** - the last structural hole, unchanged by this slice: the clearance is skipped for a body the mover
   already overlaps, so `a_mate_who_is_stopped_becomes_solid` sits at **4,478 raw**. D283's refutation stands (a per-collider
   veto freezes a crowd: **7 then 7**), so the next attempt is a **net** rule or the game's own radius query (`FUN_00844310`).
3. **The crowd fixtures** that assert the withdrawn settle ring (`six_settled_men_hold_their_places`,
   `three_gatherers_all_work_the_same_square_on_its_ring`, `ten_men_work_one_bush_a_body_apart`,
   `villagers_never_come_closer_than_one_diameter`, `a_settled_man_is_not_displaced_by_a_later_arrival`) - the crowd slice's
   to rewrite to the game's rule: bodies overlap as they pass, a stopped body blocks, the loser stands for the drawn pause.
   `a_newcomer_does_not_move_the_working_gatherers` was re-pointed by this slice (the staleness test is a box test now) and is
   **green**; the rest still assert our own seat book.
4. **The parked skirmish pair** (owner: *"The failures to train militia do not seem relevant to the pathfinding. Skip them for
   now."*) - a script deadline, not a movement rule; they flip on a single tick and are expected red here.
5. **The drop-site red** (`a_villager_drops_only_within_half_a_diameter_of_the_drop_site`) - the return order's range is unread.
6. **The decide profile** - the remaining gap to `main`'s 26.6 s.
7. **The lost guard**: `a_spawn_in_the_same_tick_does_not_move_the_wrong_entities` was deleted by the owner's ruling and was the
   only cover for "a mid-tick spawn shifts the entity list, so movements are applied by id, not index".

## Where the parity push stands (2026-11-19, `9ef8e97`)

P1/P2 landed (`c11e49d`): the search reports its expansions, answers the octile-nearest settled cell, and is **not** budget-
enforced - at `h = 25 x tiles` a flat search truncates a legitimate detour (measured: a one-tile walk answered fifteen tiles
east), so the budget lands **with** the levelled search (P11). P3, P4 and P5 were each attempted and each is blocked on the seat
book: **P8 is the keystone** and the next slice. The full order and every measured reason is in `docs/parity-plan.md` and
`docs/reviews/2026-11-19-p3-p4-p5-attempted-the-blockers.md`.

## Prior implementation of the landed slice (D203) - kept because the next slice reads it

| subject | where it lives now | what pins it | what it does today |
|---|---|---|---|
| a move order | `lib.rs:486` `Task::Move { to }`, `lib.rs:581` `Command::Move { to }`, `lib.rs:4091` `WalkTo`/`Move` group stamp | `building_passage`, `click_spot`, `click_into_the_woods` | carries a **place**, never an object: a click on a building is "walk to this spot" |
| an order's target | `world/motion.rs:479` `order_target` | `collision.rs`, `gather_*` | `Attack`/`Gather`/`Return`/`Build` only - `Move` has none, so the target arm cannot fire for a click |
| the clearance | `ground.rs:32` `Clear { at, reach }`, `still_inside`, `crossed_by` | `body_clearance.rs` | a **disc**: reach is the pair's radii sum, tested by squared distance |
| the plan's obstacles | `lib.rs:2023` `standing_bodies`, `ground.rs:322` `inside_a_body` | `a_blocked_unit_goes_round_a_standing_body_and_arrives` | **bodies only**; a building's footprint is the grid's whole-tile test, so a leg may clip its corner by a rounding |
| the step's ground test | `world/motion.rs:401` `tile_walkable` on the landing tile | `building_passage` (3 fixtures), `corner_gap` | the **tile**, not the body - a centre may reach a footprint's border (`0.07` from it, the owner's "~0") |
| the click | `crates/view/src/bin/window.rs:498` `pick_entity` -> `WalkTo` | - | the picked entity is used for `Gather`/`Attack` and **dropped** for a walk |

**Measured before this slice** (`blunt-port` at `5dd2c7e`): acceptance fixture **39.58 s**, suite 198 binaries / 881 passed
/ 20 failed / 21 ignored. The step's box test **alone** was tried and reverted: it passes acceptance in **21.81 s** and reds
`a_villager_walks_round_a_house_rather_than_through_it`, `a_villager_walks_round_a_half_built_house` and
`a_big_unit_walks_through_a_one_tile_gap`, because the plan handed it routes it now refuses - which is why the four land
together.

---

**Where things stand (2026-11-19, measured).** D279's port is on this branch (`blunt-port`, HEAD `5dd2c7e`):
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

**Next, in order** (post-D286: the one box ruler, four edits, one slice - see the decision row):

0. **D286's slice**: `Command::Move`/`Task::Move` gain an optional target; `ground::Clear` becomes a box; the plan's
   clearances include blocked tiles' boxes; the step asks the same box question and the route ends on the exact waypoint.
   Measured alone, the step half passes acceptance in **21.81 s** and reds three green fixtures, so the four land together.

1. **Two bodies on one tile** - the last structural hole, and now with a measured refutation behind it. The clearance is
   skipped for a body the mover already overlaps (else every plan starts inside its own clearance), so
   `a_mate_who_is_stopped_becomes_solid` sits at **4,478 raw** and a man can be pushed through another who shares his tile.
   **Tried and reverted (D283):** "never step deeper into a body you are already inside" bounds the overlap exactly as the game
   does - the pair went **4,478 -> 7,728 raw against an 8,000 reach**, i.e. the contact step and nothing more - and it **froze
   the crowd**: the acceptance crew stopped growing (**7 then 7** between 400 s and 800 s), because a man inside three
   clearances has no legal step at all. The next attempt needs a rule that bounds the *pair* without refusing every step a body
   makes while it is inside somebody - a net-motion test (deeper into one, further from another) or the game's own radius query
   (`FUN_00844310`) over the objects on a tile, with the mover's reach grown in. That consumer is still the thing to read.
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
