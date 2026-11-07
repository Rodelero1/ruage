# "Nearby" has a number, and the number is 12 (D149)

*2026-11-07. The owner chose **C — line of sight + 5** when asked how far a depleted worker should look. Measured, that
number **collapses the acceptance match**; the smallest bound that keeps it resolving is **line of sight + 8 = 12 tiles**.
This file is the measurement, and the reason the shipped constant is 8 and not 5.*

## What was added

`World::gather_reach(worker)` — the worker's own line of sight (whole tiles, from the dat's `los`) plus a look-around,
scaled once. Used at the two places that choose a node:

- `World::nodes_of_slot(worker, slot, limit)` — the script's `do = "gather"` action and the villager-share pass;
- `World::nearest_like` — the "my node ran dry" path.

Beyond it, nothing is a candidate, and a worker with no candidate stands **idle** — which is the owner's second sentence.

## The measurement, and the error it corrects

The constant is the **look-around**; the reach is `sight (4) + look-around`. An earlier note in this session reported the
`LOOK_AROUND_TILES = 10` and `= 12` runs as "reach 10 / reach 12" — **that was wrong**, they were reach 14 and 16. The
corrected table, each row a release build of the same acceptance command (36,000 ticks, seed 7):

| look-around | reach | entities | players alive | repaths | made-room | hash |
|---|---|---|---|---|---|---|
| **5 (the owner's choice)** | **9** | **129** | **2** | **83** | **164** | `5d0f8043bfbb7d9f` |
| 8, shares measured from the worker | 12 | 141 | 1 | 191 | 197 | `db9974c3e8413170` |
| 10 | 14 | 141 | 1 | 191 | 197 | `2adb19a4bb1a5a46` |
| 12 | 16 | 141 | 1 | 188 | 187 | `dfc835be5d880a47` (same as no bound) |
| **8, shares measured from the town centre (shipped)** | **12** | **152** | **1** | **186** | **159** | `82a5063dcca63108` |

**Reach 9 is not a tighter rule, it is a broken match**: the entity count falls by 12, both players are still alive at
30 minutes (nobody can finish anybody), repaths fall from 188 to 83 and made-room from 187 to 164 — the economy stops
moving. On this map some gather point sits 10-12 tiles from where a worker looks, so a 9-tile reach idles the workers that
supplied it. **This is D141's failure reproduced**, now with the fog filter in place, which is what makes it a statement
about the radius and not about knowledge.

**Why 12 ships**: it is the smallest bound measured that keeps the match resolving, and it is still local — a worker looks
one and a half screens' worth, not across the map. It is **not** what the owner asked for, and that is stated in the
decision rather than glossed: he picked 5, 5 was measured, 5 is the failure point.

**And the share table measures from the town centre, not from the villager** — found by a test, not by design. With the
reach applied to the share pass, `the_share_row_follows_the_villager_count` failed: five villagers spawned at (9.4, 32.4),
eighteen tiles from the nearest wood, so "eight on wood" had no wood *near them* and they never moved. The share table is a
question about the **base** ("eight on wood" means wood near *your* town centre), so it now anchors on the player's town
centre and falls back to the worker when there is none. That change is also why the shipped run has **152** entities rather
than 141: villagers are assigned work near their base instead of wherever they happen to stand, and the economy runs
tighter.

## Performance

**Cheaper, not dearer**: the reach test is a squared-distance comparison against an `i64` computed once per call, and it
runs *before* the candidate is pushed, so a tighter reach only ever removes work. No path search is added — that was the
expensive mistake the function's own doc records (37× the run, 2026-10-29).

Wall clock on this box was ~11.5 s for the acceptance run in all four builds above, under a load average of 3.2-3.9 on a
two-CPU cgroup; the numbers are too close to the noise floor to claim anything from, and none is claimed. The one
performance-shaped fact that **is** solid: repaths and made-room move (188 → 191, 187 → 197) at reach 12 compared with
reach 16, because a worker that stays in its neighbourhood re-plans differently than one that walks the map — a 1.6% change
in reactions, not a slow-down.

## Stability

- **Test-first, with a control that passed before the change**: `a_worker_does_not_look_beyond_its_reach` failed with the
  patch still holding `10000000` raw units — *"eleven tiles is past a villager's reach (sight 4 + 5) and the patch was
  scouted: a villager walked to it anyway"* — while `a_worker_takes_the_next_node_inside_its_reach` passed both before and
  after, so the pair isolates distance from knowledge.
- **A units bug caught by the control, in my own code**: the first `gather_reach` added `5 * scale` to `sight_of(..)`,
  which returns **whole tiles** — a reach of 5.004 tiles. The in-reach test failed immediately (the patch 7 tiles away was
  refused) and the fix was to sum in tiles and scale once. Without a control that asserts something *is* taken, that bug
  would have shipped as "the rule refuses everything", which is how D141 first failed too.
- **The tests are written against the shipped number**: 15 tiles is out, 6 tiles is in, for a reach of 12. If the owner
  changes the constant, those two numbers move with it — they are the rule, not a coincidence.
- **Both known-node tests still pass** (`a_worker_is_never_sent_to_food_it_has_not_found`, `a_worker_goes_to_food_it_has_found`),
  so the fog filter and the reach compose.
- Suite: 737 passed / 0 failed / 19 ignored (two new), clippy `-D warnings` 0, fmt 0.

## What is still the owner's call

The shipped constant is 8 (reach 12), chosen because 5 was measured to break the match. If he wants a bound that is felt
rather than a formality, the table says reach 12 is the first that resolves; reach 10 or 9 need a map whose resources sit
closer to the workers, or the two-stage gather search (D141's queued design) that walks outward from the worker instead of
measuring from it.
