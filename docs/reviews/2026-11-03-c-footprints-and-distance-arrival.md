# Review: C - footprint collision (C1b) and distance arrival (C2)

*Change: a body now blocks a disc of its `radius[0]`, summed for the pair (one whole tile when the content gives neither),
**on every step** - and the four arrival predicates that decide whether a worker, a builder, a hauler, a shepherd or a
melee attacker can act have been rewritten in the same units, so "too close to stand" and "close enough to work" are one
number rather than two. `World::reach_between` is that number, public so a test can assert it.*

Files: `crates/sim/src/lib.rs` (`body_tiles`, `bodies_at`, `body_at`, `blocker_at`, `collide`, `wait_or_give_way`,
`reach_between`, `body_radius`, `Route::gave_way`, `made_room`), `crates/sim/src/bin/headless.rs` (the counters in the
final line), `crates/sim/tests/footprint.rs` (new), `evasion.rs`, `collision.rs`, `group_move.rs` (tile assertions
rewritten as distances), `collision_probe.rs` (a probe that measures what the rule still lets through).

## Performance

**The acceptance match is now won.** At 36,000 ticks the run ends with **`players=1`** - the enemy town centre is gone at
**1,667 s** and the rules that look for one start refusing *"no enemy matches \"enemy_town_center\""* - where the landed
baseline (`1a31830`, B in, C out) was **`players=2`** with hash `5767526d89d11c35`. The economy is **ahead of the
baseline and then spends it**: at 1,200 s the bank is **752 wood / 825 food / 370 gold**, against the baseline's **729 /
792 / 370 at 1,600 s** - eight game-minutes earlier - and by 1,600 s it has been turned into **49 buildings** (baseline:
22) and a won match, leaving 192 / 654 / 110.

**The cost, measured three runs each on this box, same script, same seed:**

| | baseline `1a31830` | C1b + C2 | change |
|---|---|---|---|
| wall, 36,000 ticks | 9.82 / 10.00 / 10.02 s | 19.88 / 19.91 / 20.01 s | **+99%** |
| per tick | 0.28 ms | 0.55 ms | +0.27 ms |
| of the 50 ms tick budget | 0.6% | 1.1% | - |
| entities at the end | 159 | 153 | - |
| outcome | `players=2` | **`players=1`** | the victory |
| repaths / nudges | 0 / 0 | **100 / 166** | - |

**Where the 0.27 ms goes, by ablation** (same binary, one line changed each time):

- **no collision query at all** (`blocker_at` returns `None`) - **14.36 s**. So ~4.4 s of the regression is *not* the
  collision pass: the match is busier (armies converge and fight, workers reach nodes, 49 buildings instead of 22). The
  clause still prints `players=1`, so the victory is not a collision artefact.
- **a constant one-tile reach instead of the pair's radii** - **18.38 s**, and *more* blocking than the real rule (721
  repaths, 351 nudges): a bigger disc binds more. That ablation also priced the per-pair radius resolution, which is now
  done **once per body per tick** (`body_radius`, carried in the sorted body slice) instead of once per pair: 20.7 s ->
  18.7 s on its own (mid-slice, on a quieter box: the final build measures 19.9 s).
- **the body index**: a `BTreeMap` of tile -> `Vec` allocated per occupied tile on every moving tick; replaced with one
  sorted `Vec` of `(tile, id, index, radius)` answered by `partition_point`. Worth ~3% (20.9 s -> 20.5 s), and it removes
  150 allocations a tick.

Known and accepted: the pass is ~0.27 ms/tick heavier for a match that wins instead of drawing. At 50 ms/tick that is
1% of the budget, and 4.4 s of the change is the match doing more work rather than the query costing more.

## Stability

- **The rule binds.** A probe over the acceptance match (`how_deeply_bodies_still_overlap`) measures every body pair every
  tick: **2,745,269 overlapping pair-ticks of 16,242,417** (16.9%), deepest **9,629 raw (0.481 tiles)** - and
  **99.95% of those are stacks the map seeds**: villagers placed on the town-centre tile and left idle, which no step of
  theirs creates. Counting only **new** overlaps - a step that took a pair from apart to inside - there are **1,246 in
  36,000 ticks** (one per 29 ticks), deepest **1,752 raw (0.088 tiles)**, i.e. at most 22% of a footprint. Those are the
  give-way steps below.
- **A jam cannot outlive the tick it forms in.** Three deadlocks were found by measurement and each has a rule:
  - two bodies side by side in one tile waited for each other from tick 15 to tick 2,000 (the one-wood-tree fixture);
  - a gatherer froze from tick 40,000 to 400,000 beside an **idle** friend - a body at rest never moves aside, so
    waiting on it is waiting for ever;
  - a repath around a body standing on the mover's **own tile** produced a nine-waypoint route that walked backwards,
    which crossed the other body head-on and re-jammed.
  The rules: a body you are **already inside** is not an obstruction (D52's "not made worse by moving"); a body on your
  own tile is waited for **only while it is itself moving**; a body **at rest** is stepped past (the plan routes round it
  from the next tick); and a **mutual** refusal gives way in entity order. Each is a last resort after clear -> slide ->
  make room, and each was watched failing before it was written.
- **Make room is capped, and the cap is why the match survives.** One nudge per route (`Route::gave_way`, carried across
  repaths): 166 nudges in 36,000 ticks. Uncapped, the earlier build turned the economy into a mosh pit - **89 wood / 20
  food / 50 gold at 1,600 s** against 729 / 792 / 370 - because every crowded unit stepped aside for every other and
  nothing arrived.
- **The tests that encoded the tile rule were rewritten, not deleted.** `two_villagers_crossing_never_share_a_tile...`,
  `a_crowd_sent_to_one_place_does_not_stack`, `a_crowd_ordered_to_one_tile_all_arrive_and_spread` now assert
  `world.reach_between(a, b)` between every pair. The old assertion was the old rule speaking; the distance is the
  stronger claim and it caught the regression below.
- **RED, watched, both halves of the slice:**
  - C1b: *"they came 7000 raw units apart on tick 112, inside the 8000 their footprints need"* - two villagers crossing,
    before the footprint rule;
  - C2: *"villager 128 never took anything from the body: a body on walkable ground can only be worked from its exact
    centre"* - the arrival clause reverted with the collision in place. A worker stopped 0.4 tiles short by its friend's
    footprint took nothing for the rest of the match, which is exactly the 89-wood run.
- **Mutations, each caught** (`footprint.rs` unless noted): tile instead of footprint -> *"they came 7000 raw units apart
  on tick 112, inside the 8000 their footprints need"*; make-room removed -> the same two tests, because a pair with no
  nudge falls through to the give-way rule and the crossing pair **overlaps** at 7,000 raw - which is the point of that
  assertion, and shows which rule does which job (make room prevents the overlap; give way is the escape hatch that
  allows one); the "already inside" rule
  removed -> `a_spawn_in_the_same_tick_does_not_move_the_wrong_entities` freezes the follower; the "at rest" rule removed
  -> `research::the_spear_line_upgrades_the_units_the_player_already_has` cannot bank (a gatherer frozen from tick
  40,000); the mutual give-way removed -> `gather::two_villagers_on_one_node_cannot_take_more_than_it_holds` leaves the
  tree full.
- **Determinism**: same hash `912386556784c88f` on every run, three times; the sorted body slice is ordered by
  `(tile, id)`, the tie-breaks are `(distance, id)`, and no map iteration reaches a decision.
- **Fixtures that had to move with the rule, all named:** the sheep tests (`a_sheep_stops_at_the_drop_site`,
  `the_shepherd_kills_the_sheep...`) needed the herding arrival in the same units - a sheep stopped by its shepherd never
  reaches the drop site's own tile; the rally pocket and stone tests needed the collision to let go of a unit that was
  frozen behind an idle body.

Known and accepted: two villagers ordered at one walkable node now stand **side by side** instead of stacking on its
centre, which is the point; a crowd at a bush queues at footprint distance and the ones beyond the ring wait, which is
what the game does and what the 49-building expansion shows is not starving the economy. The nearest-body search is nine
tiles - provable here because no unit's `radius[0]` exceeds 0.5 tiles (the largest sum is 0.4, and the content-less
fallback is one tile).

---

*Staged with the change for `tools/review_check.py`. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
