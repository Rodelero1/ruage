# OPEN - D208 needs sub-tile clearance in the search, and that is the next slice

**Where it stands, 2026-11-14, on `d208-time-not-geometry`.** Both halves of the original's collision rule are built:

- **no clamp**: a step is never refused because it would end inside `r1 + r2`. The binary's only radius-against-radius
  retraction has its result discarded at the movement site (`0x8a7fe7`), and its position setter clamps against the map
  border alone. `bound_to_reach` is no longer called from `collide`.
- **time, not geometry**: the loser of the id comparison is paused 0.25-1.50 s, and a collider that has **not** moved since
  the route was planned is *past* and dropped (`path->createdAt >= other->lastMoveTime`). `Route.created`,
  `Entity::last_moved`, one `put_route` choke point, and `World::is_past`.
- **standing bodies are search obstacles** (D208b): `stander_tiles` returns the tiles of units that did not move on the last
  tick - the original's rule is "insert into the per-search rectangle list iff not ignored and velocity is zero"
  (`FUN_007f9d80`) - and both `plan_path` and the re-plan pass that set to the search. Per search, never in the shared grid.

**Green:** `head_on.rs` 2/2 - two men meeting head-on pass each other, and the loser stands for the first stand of the drawn
0.25-1.50 s.

**Not green, and each for a different reason:**

| test | measured | why |
|---|---|---|
| `group_overlap::two_units_ordered_apart_stay_solid` | closest **151 raw** against an **8000** reach | **the test is now obsolete**: D208 says two men who meet *do* pass each other. It must be rewritten to assert the D208 outcome (both arrive, the loser pauses 0.25-1.50 s), not "never overlap". |
| `group_overlap::a_mate_who_is_stopped_becomes_solid` | closest **4207 raw** | a **real gap**: after a stop the man is solid again, and the walker should route round him - but he is **adjacent**, and our search works on **tile corners**, so no detour exists that does not begin inside his tile. The fallback straight line then ghosts through him. |
| `footprint::a_head_on_pair_in_a_one_tile_corridor_never_overlaps` | 7346 raw on tick 66 | the same gap, in its sharpest form: a one-tile corridor has no tile-level detour at all. |

**The next slice is therefore the search's resolution, not another movement rule.** The original's search works on continuous
rectangles (each body a box of `pos ± radius`, grown by the mover's radius), so it can thread a gap a tile-corner graph
cannot describe. Ours cannot, and no arrangement of step rules has ever fixed that - three slices tried.

Two ways forward, and they cost differently:

1. **sub-tile clearance in `short.rs`** - nodes offset from a body's disc rather than tile corners (what the earlier
   "free-space pather" note proposed). Faithful, and it is the piece that makes D208's *past* test sound everywhere.
2. **restate the two failing guards** to D208's outcome and accept that an adjacent body is walked through until (1) lands.
   Cheaper, honest, and measurable: the acceptance hash moves either way.

Measured on this branch (`--seed 7 --ticks 48000`, release): `entities=158 players=2 hash=c2ed74016ee533ea repaths=508
made_room=0 repaths_body=174 repaths_ground=334`, against `main`'s `entities=160 hash=c3b10f4b09e17a43 repaths=324
made_room=147 repaths_body=83 repaths_ground=241`.


## The 18 failures this branch carries (gate `d209e`, private target dir)

- a_body_on_the_line_is_counted_as_a_body
- a_broken_file_in_a_present_pack_fails_the_fixture
- a_content_pack_with_our_own_resources_loads_and_spends
- a_depleted_node_stops_yielding
- a_man_who_cannot_get_his_square_takes_the_next_tree
- a_mate_who_is_stopped_becomes_solid
- a_partial_load_comes_home_when_the_node_runs_out
- a_present_but_broken_marker_fails_rather_than_skipping
- a_repath_route_is_pulled_like_a_planned_one
- a_unit_rallied_to_its_own_building_stays_inside
- a_villager_rallied_onto_stone_gathers_stone_not_the_wood_around_it
- a_villager_walks_around_a_forest_wall_and_never_enters_it
- an_enemy_body_on_the_line_changes_the_decision
- an_enemy_is_only_visible_while_somebody_watches
- the_higher_id_pauses_for_a_quarter_to_one_and_a_half_seconds_and_keeps_its_route
- three_gatherers_on_one_bush_do_not_pile_up
- two_units_jammed_on_the_same_tile_neither_stands_for_ever
- two_villagers_on_one_node_cannot_take_more_than_it_holds


Two of these (`a_broken_file_in_a_present_pack_fails_the_fixture`,
`a_present_but_broken_marker_fails_rather_than_skipping`) are this worktree's **symlinked** `config/gamedata`, not the code.
The rest need triage on a checkout whose pack is real - `gather` is 21/21 in isolation, so at least part of this list is the
same environment effect leaking into fixtures that load the pack.
