# OPEN - P11 (the levelled search hierarchy), after the gather seat model was finished (D295, 2026-11-19)

**Where this branch stands, measured (`blunt-port`).** The gather seat model is finished end to end (D287, D289, D290, D291, D293,
D294, D295) and **the suite is 900 passed / 6 failed / 21 ignored** - from 893 / 14 / 21 at the start of the day. The six reds are at
the foot of this file.

## Where the parity work stands

- **P8 (the gather seat book) is finished, end to end.** D287 (no re-lay), D289 (every man is sent to the tile he was ordered to), D290 (there is
  no press floor), D291 (the engine re-targets within a kind only), D293 (a gatherer presses in) and D294 (the book is deleted).
  D294 deleted the book and D295 accepts a gather on the tile a man already stands on (it was refused as a walled-off node, and the
  refusal is remembered for the match). Evidence: `docs/press-depth-evidence.md`, `docs/gather-seating-evidence.md`, and the reviews.
- **The crowd spreads by the game's own rule**: D204's same-kind re-target, now triggered by the **refusal count** for a man whose
  route is spent (`world/step.rs`), because the old trigger (`Standoff::NoRoom`) no longer exists.

## The next slice, in order

1. **P11 - the levelled hierarchy**, crosschecked and shaped (`docs/pathfinding-levels-evidence.md`): four levels 3..0, cell =
   `2^level` tiles, a coarse A* over the cells, refinement to cell-centre waypoints, and a flat tile A* kept as the fallback.
   **11a landed** (`Levels::of`, the four maps by the stamping rule - D296) and **11b landed** (`Levels::descent`, the coarsest usable
   level - D297, built on a labelled interpretation of the descent's second byte).
   **Next: 11c, the coarse A* and its refinement**, whose witness is now correct and red:
   `crates/conformance/tests/divergences.rs::the_original_solves_a_long_walk_in_its_hierarchy` fails with *"a 58-tile walk came back as
   1 waypoint(s)"*. The pieces to write, all read from the binary: a node is `(x, y, level, g, h, f, parent)` (`FUN_007ef170`), `h` is
   Euclidean (`FUN_007ee760`), the frontier is an **insertion-sorted linked list on `f`, newest first among ties** - not a heap
   (`FUN_007ed500`) - and refinement pushes a cell-centre waypoint per coarse node (`FUN_007ee980`'s chain walk, `FUN_007edb80` +
   `FUN_007eeda0` + `FUN_007fd880`). The coarse step cost is the one number not yet read: `FUN_007ed8f0` answers `|dx| + |dy|` between
   cell centres, which is a *descent* cost, so the A* `g` is still unknown and must be read before it is chosen.
2. **P1 - the search budget**, against `FUN_007f9d80` / `FUN_008007f0`, once P11 lands.
3. **P4 - the re-plan throttle** and the off-grid goal fallback it leans on; then **P3** (the plan agreeing with the step about
   touching - a certified route currently asks for a landing that touches two blocked tiles and the body waits in `WaitGround`).
4. **The two `reseed` fixtures** (`auto_reseed_is_a_player_setting_and_pays_when_the_plot_runs_out`,
   `a_queued_reseed_pays_the_farms_price_and_replaces_the_spent_plot`) - economy, not movement.

## The six reds, verbatim

```
a_man_who_cannot_fit_takes_the_next_source
a_queued_reseed_pays_the_farms_price_and_replaces_the_spent_plot
a_settled_man_is_not_displaced_by_a_later_arrival
a_villager_drops_only_within_half_a_diameter_of_the_drop_site
a_walled_off_wood_is_tried_once_and_then_left_alone
auto_reseed_is_a_player_setting_and_pays_when_the_plot_runs_out
both_sides_field_an_army
six_settled_men_hold_their_places
ten_men_work_one_bush_a_body_apart
the_skirmish_only_refuses_what_the_fight_has_taken
three_gatherers_all_work_the_same_square_on_its_ring
```

*Their files: run `cargo test --workspace --offline --no-fail-fast` - the full list with test binaries is in
`/opt/data/cache/scratch/suite-book1.log`.*```
a_queued_reseed_pays_the_farms_price_and_replaces_the_spent_plot
a_villager_drops_only_within_half_a_diameter_of_the_drop_site
a_walled_off_wood_is_tried_once_and_then_left_alone
auto_reseed_is_a_player_setting_and_pays_when_the_plot_runs_out
both_sides_field_an_army
the_skirmish_only_refuses_what_the_fight_has_taken
```
a_man_who_cannot_fit_takes_the_next_source
a_queued_reseed_pays_the_farms_price_and_replaces_the_spent_plot
a_settled_man_is_not_displaced_by_a_later_arrival
a_villager_drops_only_within_half_a_diameter_of_the_drop_site
a_walled_off_wood_is_tried_once_and_then_left_alone
auto_reseed_is_a_player_setting_and_pays_when_the_plot_runs_out
both_sides_field_an_army
six_settled_men_hold_their_places
ten_men_work_one_bush_a_body_apart
the_skirmish_only_refuses_what_the_fight_has_taken
three_gatherers_all_work_the_same_square_on_its_ring
```

*Their files: run `cargo test --workspace --offline --no-fail-fast` - the full list with test binaries is in
`/opt/data/cache/scratch/suite-book1.log`.*
