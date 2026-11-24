# Review: a click names its target, and one box ruler decides every distance (D286)

*The owner: **"follow the game/decomp"**, after reporting two clicks on one house that stopped at different distances - "1/2
radius" when he clicked the middle, "~0 radius" when he clicked just beside the border. This is the slice D286 wrote down and
this session executed: the four edits land together, because three of them alone made the third worse.*

## The rule, ported

1. **An order names its object.** `Command::Move` / `Command::WalkTo` and `Task::Move` gained `target: Option<EntityId>`, and
   the window fills it from the entity its click picked (`crates/view/src/bin/window.rs`, `order_at`). The collision block's
   own words are the rule: *"That collision is with our target, so we're done"* (`0x127d`), and the path-ignore list is built
   from `param_5[2]` (self) and the order's target (`local_45c + 8`).
2. **The clearance is a box.** `ground::Clear` is now `{ at, half, reach }` - the obstacle's own half-extents and the mover's
   radius, grown to `half + reach` per axis - instead of a disc of the pair's radii. The segment test is a segment against an
   axis-aligned rectangle by separating axis, in `i128`, so it is exact and identical on every machine.
3. **The plan keeps clear of blocked tiles' boxes.** `Ground::object_depth` / `crosses_an_object` read the box of every tile
   an **object** holds (a tree, a wall, a building) and grow it by the mover's radius - `FUN_007f9d80` (the search treats a
   stopped object as a rectangle grown by the radius) and `FUN_00840830` (the query box is grown by `+0x268`). Impassable
   **terrain** keeps the game's point test (`FUN_008aa480`), because that is what the original does.
4. **The step asks the same box question**, and a step that would end inside the order's own target is **cut at its face**
   (`World::target_contact`) instead of being refused - *"Setting position to exact waypoint"*. A walk ends on contact; a
   gatherer ends up with a box gap of exactly **0**, which is what `at_node` reads.

## Performance

- `acceptance_economy::the_acceptance_match_trains_more_villagers`: **passes in 33.74 s** (debug, 48,000 ticks). Before this
  slice: **39.58 s**; `main`: **26.6 s**. Faster, and it is not a rounding: the plan no longer hands the step routes the step
  refuses, so a blocked walker spends fewer ticks re-planning.
- The step's box test **alone** had measured **21.81 s** (D286's note); with the plan half in place the 6 s it saved came back
  as real work (routes that go round the thing rather than into it), which is why the four edits were never split.
- Costs: the step now reads a 3x3 tile neighbourhood per step per body instead of one tile (a `u16` compare each), and the line
  test reads the same neighbourhood per tile the leg crosses. Both are grid reads; no search was added and no allocation
  grew. The plan's clearance list is unchanged in size - objects are read from the grid, not from a list.

## Stability

**What was attacked, and what it cost.**

- **Same-count, different-set, measured both ways.** The suite as it stands: **199 binaries, 884 passed, 21 failed, 21
  ignored** (the final run, to exit). The baseline had to be measured, not quoted: the docs carried "20 failed", and a run of
  the same binaries at `5dd2c7e` on this machine found **23** by name. Against that set, **four** reds turn green -
  `corner_gap::a_walk_past_a_house_keeps_its_bodys_clearance`, `overlap::a_villager_who_is_gathering_does_not_move`,
  `footprint::no_unit_crosses_an_undepleted_resource_tile` and
  `animals::the_shepherd_kills_the_sheep_and_the_gathering_resumes_on_the_body` - and **two** join: `skirmish::both_sides_field_an_army` and `skirmish::the_skirmish_only_refuses_what_the_fight_has_taken`. Those two are
  the owner's **parked** pair, they flip on a single tick of movement timing (red on D279, green after D281, red after D282),
  and they flip again here for the same reason: one extra tick of pause and up to one detour is enough to miss a script
  deadline that already records 921 of 1,000. They are expected on this branch, not ignored. Nineteen of the 23 baseline reds
  are still red, unchanged.
- **Three fixtures went red on the first cut, and all three were the engine's fault, not theirs:**
  - `footprint::a_head_on_pair_in_a_one_tile_corridor_never_overlaps` - *"the pair never came closer than 140000 raw"*. Both
    bodies are spawned on **tile corners**, so each already overlapped the tree row beside it by 4000 raw; a plain yes/no box
    test then refused **every** step. Fixed by asking the game's question of the **change** (`object_depth`: a step may not take
    a body **deeper** into an object), which is identical wherever a body starts outside one and lets a body that starts inside
    walk out. The original never has this case - its placement phase (`func_002699A0`) keeps bodies out of objects - so this is
    our recovery rule, and it is named as such in the code.
  - `footprint::both_workers_on_one_body_take_from_it` - *"villager 130 never took anything"*. A corpse's radius in the dat is
    **0**, so its box is a point; the target arm then stopped a worker within its own radius of it, and `at_node` (box gap
    `== 0`) could never fire. Fixed twice over: a body the dat gives **no radius has no footprint and is walked over**, and the
    target cut lands the body exactly on the face rather than a step short.
  - `passage::two_trees_touching_at_a_corner_leave_no_gap` and `building_passage::a_big_unit_walks_through_a_one_tile_gap` -
    the first cut certified the **line between two tile centres** and let the body walk a different line. A trebuchet (radius
    **0.5**, the widest body in the dat) spawned on a tile corner was certified a straight walk down a one-tile gap and then
    refused every step into it. Fixed by certifying the leg the body **actually walks** (`leg_is_clear`, raw points, first leg
    from the body's own position).
- **The full suite found three more classes, all of them the same mistake in different clothes** (the first suite run after
  the movement fixtures were green: **47 failed against 20 before**). Each was traced to a centre-distance test standing in
  for the game's box measure, or a box standing in for what is solid:
  - **A box for the whole footprint is not a box for what stands.** A builder walked *onto* a farm plot and an untouched
    foundation (both walkable, D183) and the target arm stopped him at the footprint's edge: farms never seeded, houses never
    finished, a barracks never completed, so every later fixture that spends a bank failed for want of a bank. The target is
    now its **solid** tiles (`solid_boxes`), so a farm, a fresh foundation and a town centre's yard are walked on and only the
    wall stops anyone.
  - **In reach is box to box, in every arm.** Melee contact and the shepherd's blow were centre distances against the
    radii sum. On a diagonal, a mover stopped by the target's **box** is farther away by its centres than the radii sum - so a
    villager stood with its box against a sheep and never struck (`animals`, `evasion::an_arms_route_ends_where_it_can_strike`).
    Both now use `gap_squared_to` / `box_gap_sq`, which is what `func_002690E0` is.
  - **A place is stale only when a body's box covers it.** The seat book's staleness test was the pair's reach plus a
    twentieth of a tile - a *centre* distance - and with bodies now resting at box contact a diagonal neighbour sits nearer by
    the centres than that: a man at work lost his place to a newcomer (`overlap::a_newcomer_does_not_move_the_working_gatherers`,
    measured, place `(396000,314375)` became `(396000,303750)`). The test is `box_gap_sq(...) == 0` now, which is the rule the
    owner stated (*"villager is never to be nudged to leave place he is working at"*) read with the game's own measure.
  - **Arrival cannot be an equality on a step grid.** `at_node` asked for a box gap of exactly **0**. The step is cut at the
    face, but the settle can seat a man a fraction short - measured, a miner stood with `progress 0` and the bank stuck at
    **100 for 400,000 ticks**. It is now "within one step" (`gap <= step^2`), the same "a radius, not an equality" shape C2
    settled elsewhere, and the honest quantized form of the game's continuous contact.
- **What the fixtures now pin** (`crates/sim/tests/click_on_a_building.rs`, written first and watched failing): the two clicks
  stop at the **same** distance within one step, the body's box never enters the footprint on any tick, and a walk that names a
  house **ends when it touches it** instead of stalling for forty ticks against it.
- **A deliberate difference, named.** The original collides on `<=` (touching is solid). Ours collides on overlap
  (`<`), so a body that comes to rest exactly at its radius can still walk along a wall; with `<=` and integer rounding a body
  could be refused every step while standing exactly on the face, which is the wedge D279 spent a session removing. The
  distance a walk stops at is unaffected (contact is the same point), and `target_contact` puts it exactly there.
- **Not measured by this slice**: a full-suite run of the previous commit was not taken on this machine, so the red-set diff
  is against the movement fixtures plus the documented 20 (below); and the 400,000-tick fixtures were not re-run.

```
cargo test --workspace --offline --no-fail-fast
```


## The reviewer's findings (`make ai-review`, HEAD `6d8ef1c`)

The run was **budget-limited**: 64 files, 6 comments, 967,044 tokens, 4m54s, and its own log names the files it **did not
read** - `ground.rs`, `world/motion.rs`, `world/step.rs`, `world/sites.rs`, `world/orders.rs` among them, i.e. most of the
engine and all of the movement pass. So this note cannot claim the diff was read end to end; it can only account for the six
findings that came back, all of which are in `lib.rs` and the new fixture:

- **Accepted - `entry_parameter` carried a negative denominator.** A step with a negative component returned
  `(far - from, delta)`, both negative: the "which axis enters later" cross-multiplication therefore compared inverted
  fractions, and the guard that follows read the fraction as *already inside* - so a body walking **west** or **north** into its
  target was cut to **zero movement**. Fixed by normalising both terms (`(from - far, -delta)`), and **pinned by a unit test
  watched failing first** (`the_target_cut_is_the_same_in_every_direction`), plus a fixture for the owner's click from the
  east. Reported honestly: the fix landed before its test, because the review found it rather than a fixture - the test was
  written afterwards and the bug put back to watch it fail.
- **Accepted - `push_out_of_target` could land inside a neighbour tile's box**, because each solid tile was pushed out of
  independently. A click in the middle of a two-tile wall could be pushed to a face still inside its other half, and the plan
  then had a goal no legal step could reach. The pass now repeats until the point stops moving.
- **Accepted - `state_hash` ignored `Task::Move`'s new `target`.** That field changes where a step is cut and when the order
  ends, so two runs differing only in it would have had an identical fingerprint. It is hashed now (a presence byte plus the
  id), in the same shape the other task arms use.
- **Accepted, fixture-side - the new fixture's `open_patch` did not check the tiles it spawns the men on**, its deadline was a
  bare `180` whose comment mis-stated the step length, and the loop ran all 2,400 ticks for a static outcome. The patch now
  covers the whole band the men stand in, the deadline is derived from the body's own step and speed, and the comment says
  what the loop is for.
- **Refused - nothing.** All six were right; two of the three engine findings were defects that no fixture in the tree
  exercises (a westward cut, a multi-tile push-out), which is the strongest argument in this note for reading the reviewer's
  findings against the code rather than against the suite.
