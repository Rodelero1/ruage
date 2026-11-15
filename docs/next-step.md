# OPEN - two arrival failures are left, and the per-plan search cost still has no number

**Where it stands, 2026-11-15.** The collision story is closed and the field it depended on is now honest: D210 (a collision
is settled by time and by the clock), D210b (a standing body is an obstacle to the search that meets it), D211 (the search
bends round a body inside a tile - discs with an octagon of shoulders), D212 (a search that starts inside a body is re-seeded
at `2 x clearance`, only *away*), and **D213** (`last_moved` is the tick a body last *moved*, or the tick it came into
being). D213 is what turned the first three on: measured, the acceptance match runs at `main`'s wall-clock with
`repaths 765 -> 380` and `repaths_body 446 -> 121`, and it un-parked
`no_pushing::a_man_who_cannot_get_his_square_takes_the_next_tree` on its own. Reviews:
`docs/reviews/2026-11-14-d211-shoulders.md`, `docs/reviews/2026-11-15-d213-last-moved.md`.

## The open items, in the order I would take them

1. **A rallied villager never garrisons.** `rally::a_unit_rallied_to_its_own_building_stays_inside` is `#[ignore]`d in
   `crates/sim/tests/rally.rs` with the sentence. A villager trained at a town centre whose rally point is the town centre's
   **own tile** never reaches `Task::Garrisoned` (measured `inside = 0` after 600 ticks). It passes on `e0312f9` and has failed
   since `e21b44d`, with or without D211/D213 - so it is in the *arrival* family, not the collision one. Probe: print the
   trained unit's `task`, `position` and `route_points` each tick; the question is whether it reaches the goal tile, whether
   `reaches` accepts that distance for a building, or whether rule 2's goal-tile branch (`WaitGoal`) is holding it.
2. **A builder with a live route is asked for no step at all.** `build::a_content_pack_with_our_own_resources_loads_and_spends`
   is `#[ignore]`d in `crates/sim/tests/build.rs`. Measured: a worker ordered to build at `(3, 4)` on a **6x6** map holds one
   waypoint at tile `(5, 4)`, stands **0.47 tiles** from it and its step is **`Idle`** for 1,900 ticks, `repaths = 0`. This one
   **fails on `main` too**, so it predates the collision work; the suspicion is the standoff/arrival predicate for a 2x2
   footprint on a cramped map, not the search.
3. **The search's own cost, per plan, has never been isolated.** `crates/sim/tests/perf.rs` (ignored) at a fixed entity count,
   with and without a stander on the route: eight nodes per body and a disc test per edge are real costs, and the only
   measurement so far is the whole match's wall-clock. D211's own decision says so in its words.
4. **An explicit group id on the command** (register row from D207/D208, unchanged): "same tick" is not "same selection"
   until `mates` comes from the command instead of `ordered_now`.

## Hazards a fresh session must not re-learn the hard way

- **A missing pack passes every world test silently.** `config/gamedata/` is gitignored, so a fresh worktree has none, and
  every fixture does `let Some(mut world) = common::fresh() else { return; }`. A whole file reports `ok` without running an
  assertion. **Copy** the four `.toml` files in; never symlink them (the pack-loader tests write into that directory).
- **One `CARGO_TARGET_DIR` per checkout** - two worktrees sharing one swap test binaries under a running suite.
- **A doc that promises a mechanism is not the mechanism.** "Eight shoulder points round every body" was in a review and a
  comment for a day and did not exist. When a review claims a mechanism, `grep` for it.
- **A test that plans *around* the body it means to collide with measures nothing.** With D210b/D213 live, a fixture that
  places a stander **before** the order gets a route round him: four fixtures had to move the body *after* the route exists
  (`collision`, `evasion`, `step_decision`, `repath_reasons`). The failure reads "the scene never met the body" - if you see
  that, the fixture is stale, not the rule.
- **A `#[must_use]` left over a deleted function** lands on whatever is inserted next to it: `put_route` inherited one from
  an orphaned standoff doc block and `-D warnings` refused the tree. Read the diff around a structural edit.
- **`make plan-check` resolves `ruage-tools/...` against the checkout's parent**, so a worktree outside `/opt/data/workspace`
  reports a good citation as missing. Check it in the workspace checkout.
