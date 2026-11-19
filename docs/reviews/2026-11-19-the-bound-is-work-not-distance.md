# Review: the short-range solver's bound is on the work, not on the distance

*The conformance set's last red was `divergences::the_original_solves_a_long_walk_in_its_hierarchy`: "the short-range solver
declined a 58-tile walk; the original has a hierarchy for exactly this". It turned out not to need a hierarchy on this map -
it needed the 40-tile side window deleted, because that window is **ours**.*

## What was wrong

`short.rs` refused any window wider than **40 tiles a side** (`MAX_SIDE`). The original has no such bound: the register's
row is *"termination: stop after `h` expansions, never more than 20,000 - and answer the best so far"* (`FUN_008007f0`), a
budget **on work**. The difference is not cosmetic on a 64 x 64 map: a 58-tile walk across open ground builds **two**
nodes (the two endpoints) and costs nothing, so declining it for its length was a geometric bound the game does not have -
and it was the *whole* of the divergence this map can show, since a flat search reaches any open goal here.

## What changed

- **`MAX_SIDE` is deleted.** The window is still the endpoints' box plus a three-tile margin, clipped to the map; it is no
  longer refused for being wide.
- **The bound that remains is the work**: `MAX_RECTS` (96) and `MAX_NODES` (400), unchanged, now documented as the honest
  analogue of the original's expansion budget - a count of what the solver builds, not how far apart the two points are.
- The conformance fixture that pinned the window (`clearance::a_window_beyond_the_bound_is_declined`) is **restated** to
  the pair that straddles the rectangle budget: a dense field of spaced single tiles (121 rectangles) is declined, the same
  field at a quarter density (36) is solved, and a 58-tile walk across open ground is solved. That keeps the fixture's real
  claim - *the bound is stated, not silent* - while removing the rule that was ours.

## Performance

**Not yet measured, and that is deliberate rather than forgotten.** Removing the window means long walks now pay for a
visibility graph instead of keeping tile corners, so the acceptance fixture (`the_acceptance_match_trains_more_villagers`,
**26.6 s** on this tree at `a4331de`) is the number that decides whether this is free. A `cargo test --workspace` was
already holding the build lock when the change was made, so the timing run is queued behind it and **no figure is quoted
from a run that has not exited** (the rule that has burned this tree three times).

## Stability

- The conformance set is **green for the first time**: 9 test binaries, 0 failures, with `--no-fail-fast` (without it the
  set stops at the first failing file and under-reports).
- The workspace suite in flight at the time of this commit read **three** failures, and **none of them is attributable to
  this change**: two are the fixture loader's own guards (`a_broken_file_in_a_present_pack_fails_the_fixture`,
  `a_present_but_broken_marker_fails_rather_than_skipping`), which fire on the **linked worktree's symlinked
  `config/gamedata`**, and the third (`a_crowd_walking_through_a_gap_stays_on_free_ground`) ran from a build that predates
  this edit. A clean run of all three is owed before this slice is called landed.
