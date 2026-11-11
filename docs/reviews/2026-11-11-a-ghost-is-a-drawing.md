# A ghost is a drawing, and the first stroke is the wall (D183, built as D185)

*2026-11-11. D183's model implemented in the order it was written down: walkable ghost -> eviction by route -> empty-site
gate -> first stroke solid -> refund, with the owner's own reproducer as the first test. Supersedes D152 (a foundation
blocked from insertion).*

## What the change is

- **`seed.rs` `insert`** blocks a building's solid tiles only when there is nothing left to build
  (`construction.is_none_or(|c| c.progress_ticks > 0)`). A finished building - the opening, a scenario, a saved game - still
  owns its ground; a fresh foundation claims nothing. **`build.rs` `take_the_ground`** claims it on the first stroke, and
  `Walk`'s occupant counter keeps the pair balanced, because the death sweep frees every solid tile of a dead building.
- **`build.rs` `site_is_clear`** is the empty-site gate: no unit of any player, and no animal, may stand on the footprint
  when construction starts. Own and allied units and animals are **told** to leave; an enemy is not, so he stalls the site
  (D183 §4). The refusal `SimError::SiteNotClear` goes out **once per state change** (`stalled_sites`).
- **`sites.rs` `evict_units_off` -> `step_aside` -> `exit_tile`** replaces `push_units_off`'s teleport with a planned walk
  to the nearest tile outside the footprint **that a route exists to**. The interrupted order is kept in `resuming` and
  re-issued on arrival (`resume_idle` now resumes a `Gather` as well as a `Hunt`).
- **`approach_footprint`** puts the builder on the footprint's outer ring, and the `Task::Build` arm measures adjacency
  against the whole rectangle rather than the anchor tile; a builder caught on his own site steps out by the shortest edge
  (`edge_out`) and keeps his order.
- **`site_refusal`** refuses the **black** (`SimError::SiteUnscouted`), allows the fog, and refuses a **visible** enemy or
  neutral unit (`SiteOccupied`). An enemy in the fog does not refuse the order - a refusal would leak him - so the site gate
  is what answers that case.
- **`memory.rs` `visible_to`** answers *no* for another player's untouched foundation; `site_refusal` and `site_has_lane`
  ask `visible_to`, so an enemy ghost neither holds ground nor counts as a wall.
- **`view` paints the ghost light grey** (`palette::FOUNDATION`), every tile of the footprint, because the renderer's own rule
  made it invisible: `solids_of` marks a footprint tile solid only when `tile_walkable` says no, and an untouched site blocks
  nothing - so the owner would not have seen the foundation they had just placed. Two view tests, one of them mutation-proved.

## The defects found on the way - mine, in the order I made them

1. **`work_position` sent the builder *inside* the walkable ghost.** Its ring is the anchor's eight neighbours, and for a
   2x2 house four of those are the site itself. With the site solid (D152) they were skipped by accident; with a walkable
   ghost the builder stood on his own site and the gate waited for him. Measured as *"the house needs exactly 500
   builder-ticks"* reading **1**. Fixed by `approach_footprint`.
2. **The gate told the builder to step aside**, which replaced `Task::Build` with a `Move` and made the builder forget the
   site - a permanent stall with one message. The builder is now excluded from the step-aside; if he is on the footprint
   the Build arm walks him off (`edge_out`).
3. **`approach_footprint` first returned the tile's *origin*, not its centre.** A tile origin is a corner shared with three
   other tiles, and one of them was a tree: the D177 clearance rule refused the last step and the builder jittered at
   `x = 4.78` for six hundred ticks instead of starting a farm. The plan now aims at `tile_centre`.
4. **The `reach` term in the Build arm's adjacency was true from *inside* the footprint** (the gap to the nearest footprint
   tile is zero), so a builder standing on his site counted as working. Fixed: adjacency requires `!inside`.
5. **`research.rs`'s `free_tile` asked `site_fits` as player 1** about a Hunnic villager's neighbourhood; with the scouting
   gate, every tile on that side of the map was refused for the wrong player - *"nowhere to build"*. The helper now asks for
   the villager's own player. A latent fixture bug the new rule exposed.
6. **A cancelled site was resurrected by its builder** (pre-existing, not introduced here): `work_one_tick` writes
   `hp = max_hp * progress / total` every builder-tick, so a site deleted by its owner (`hp = 0`) came back to life and kept
   its ground. Found by `a_cancelled_site_gives_its_ground_back`; fixed with a guard - a dead site is not worked.

## Honesty: which tests were watched failing, and which were not

Nine of the eleven tests in `foundation_model.rs` were written before the code and failed for their own reason (the walkable
ghost, the gate and its one message, resume, the black/fog placement, the invisible ghost, the cancelled site - which failed
as "the cancelled site is still there" and found D185.1, and the enemy in the fog). **Three were written after their code and
are proved by mutation instead**, because the behaviour landed in the same cycle as the gate:

| test | mutation that reddens it |
|---|---|
| `the_first_stroke_turns_the_ground_solid` | `take_the_ground` commented out -> *"(8, 6) is walkable on tick 48"* |
| `a_unit_on_a_new_foundation_walks_off_it_instead_of_jumping` | `step_aside` sets the position instead of a route -> *"moved 30000 raw units in one tick, and its speed allows 1200"* |
| `a_farm_starts_with_a_worker_standing_on_it` | the gate exemption removed -> *"the farm never started"*; the eviction exemption removed -> *"the farmhand was moved off the plot"* |

`a_foundation_may_not_be_placed_on_a_visible_enemy_unit` had only a **compile-RED** (`SiteUnscouted` did not exist yet) and
went green with the rule in the same edit; what proves it is the pair - the same placement is *accepted* while the enemy is in
the fog (`a_foundation_may_be_placed_over_an_enemy_in_the_fog`), so the refusal is about seeing him, not about the tile.

## Performance

**The acceptance match** (`seed 7`, 36,000 ticks = 1,800 s at 20 Hz, `config/acceptance.toml`), three runs each side,
interleaved by build:

| | before `bc03bdf` | after |
|---|---|---|
| hash | `dc9cf9905f65b61c` | **`3aaf6037ff27d283`** |
| entities | 137 | **143** |
| repaths | 618 | **517** |
| made_room | 307 | **273** |
| wall time (median of 3) | **10.14 s** (9.89 / 10.14 / 10.50) | **10.73 s** (10.69 / 10.73 / 11.07) |
| players alive at 1,800 s | 1 | 1 |

The match moved, so the wall clock is **not** a like-for-like tick measurement: it is a different village (six more
entities, a hundred fewer re-plans) because foundations no longer block and builders stand in a different place. +5.8% on a
different match, reported as that, not as a slowdown of the tick.

**The tick itself, measured** (`cargo run --release -p ruage-view --example step_cost`, 128 entities, three runs each):

| | before | after |
|---|---|---|
| `World::step` | 6.8 / 6.8 / 6.7 us -> **6.8 us** | 6.6 / 6.9 / 6.8 us -> **6.8 us** |
| `Snapshot::of` | 93.4 / 94.1 / 93.9 us -> **93.9 us** | 98.1 / 101.1 / 102.1 us -> **101.1 us** |

The tick is unchanged within noise. The snapshot is **+6.5% (93.9 -> 101.1 us)**, and it is this change: the view asks
`visible_to` once per entity, and that now carries the "is this somebody else's untouched foundation?" branch. One branch
and one `Option` read per entity per frame, and the frame's HUD budget is 170 us - worth stating, not worth avoiding.

The new per-tick work is bounded by construction: `site_is_clear` runs only on a site's **first** tick, `step_aside` is
skipped once a unit is in `resuming` (one order and one route per unit, not a path search a tick), and `footprint_rect` is
arithmetic on a table lookup. `site_refusal` now scans the entity list once per call for the visible-unit rule; it is the
AI's placement search's hot path, and the acceptance run - which places every building the AI raises - shows no measurable
change in the tick.

**The drawing**: the ghost branch replaces the per-tile solids walk with a fill per footprint tile, so a site is *cheaper* to
paint than a building of the same size. No frame was measured for it: the picture is drawn from a snapshot this slice does not
change, and the branch runs once per building per frame with no allocation.

**The suite**: **802 passed / 0 failed / 21 ignored**, `make check` clean (fmt, clippy `-D warnings`, `cargo test --workspace`,
**174** tool tests). *Correction, mine:* the first draft of this paragraph said 806 and 173 - numbers I had not measured. The
gate said 802 and 174, and the difference between the two counts (789 before this slice, 802 after) is the 11 foundation tests
and the 2 view tests.

## Stability

Attacked, in the order the model can break:

1. **Block/unblock balance.** The first stroke blocks once; the death sweep unblocks once. `a_cancelled_site_gives_its_ground_back`
   starts a house, watches it go solid, deletes it, and requires all four tiles walkable again - so a cancelled building
   cannot leave an invisible wall standing. The same test found D185.1 (the builder resurrecting a cancelled site).
2. **Nobody inside a solid footprint.** `the_first_stroke_turns_the_ground_solid` asserts, on the tick construction starts,
   that every living unit in the world stands on walkable ground. This is the invariant the whole model rests on, and it is
   pinned rather than argued.
3. **A builder on his own site.** Attacked by the existing build tests (a villager ordered to build where it stands). The
   Build arm walks him off by the nearest edge, the gate then passes, and the order survives - `a_house_is_paid_for_and_finishes_in_its_create_time`
   is the test that caught the stall and now passes with the full 500 builder-ticks.
4. **A unit that cannot leave.** If `exit_tile` finds no reachable tile the step-aside is remembered anyway (in `resuming`,
   as `Idle` for anything but a gather or a hunt), so the ring search is **not** repeated every tick - the shape D135 exists
   for. The site stalls on one message. Accepted: D183 §4 says an enemy may stall a site for ever, and an ally that cannot
   move is the same answer.
5. **Two foundations on one tile.** An enemy ghost is invisible, so the placement is accepted; the first stroke decides, and
   `an_enemy_foundation_in_plain_sight_does_not_block_me` requires exactly one survivor - four square feet, one building.
6. **The farm.** `a_farm_starts_with_a_worker_standing_on_it` is mutation-proved twice: removing the gate exemption stops the
   farm starting, and removing the eviction exemption moves the farmhand off the plot. A farm is a floor and traps nobody.
7. **The message.** A stalled site reports once; `construction_waits_while_a_unit_stands_on_the_site` steps two hundred
   blocked ticks and requires the refusal's tick stamp not to move. Then the squatter dies, and construction starts on the
   next tick it can.
8. **Resume.** `a_worker_steps_aside_and_goes_back_to_its_gathering` drops a house on a working forager: it walks off, and
   is a forager again at its tree, taking food. A gather whose node died meanwhile is refused by `start_gather_internal` and
   the worker stands down - the same answer a gather to a dead node gives.

**Accepted and named, not fixed** (D185.2): a step-aside does not resume a `Return`, `Build` or `Attack`; a unit already
interrupted by a hunt is not told to step aside (the site waits for the fight, since `resuming` is also the "already
leaving" marker); a unit with nowhere to go stalls its site for ever, on one message.

## What the rule cost the existing suite

D183 changes what a foundation *is*, so the tests that encoded D152 had to be rewritten with the reason, not deleted:

- `building_passage.rs::a_foundation_blocks_while_it_is_still_being_built` asserted the opposite; it now asserts a fresh
  foundation is walkable and the same site blocks from the first stroke.
- `foundations.rs::a_foundation_i_can_see_blocks_me` asserted that a visible enemy *ghost* blocks; under D183 §2 it is
  invisible, so the test is `an_enemy_foundation_in_plain_sight_does_not_block_me` and requires one survivor of two
  overlapping sites.
- `foundations.rs`'s six blind-placement tests, and the fixtures in `building_passage`, `build_queue`, `farm_husk`, `gather`,
  `pass_through`, `wall_lines`, `garrison` and `research`, placed foundations on ground nobody had looked at. The black is
  now refused, so they look first: `common::look_away` spawns one villager, steps, deletes it, and leaves the tiles
  **Explored** rather than Visible - which is what a player's scout does, and what "in fog but never in black" means.
