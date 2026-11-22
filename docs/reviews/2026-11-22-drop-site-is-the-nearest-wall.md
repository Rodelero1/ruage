# The drop site is the nearest **wall**, not the nearest anchor

**Date:** 2026-11-22 · **Branch:** `blunt-port` · **Owner's report:**

> *"I have noticed, that villagers prioritise dropping wood to lumber camp even town center is closer. The whole area of
> town center could be dropped at (impassable center doesn't need to be reached)"*

## What was wrong

`World::nearest_drop_site` ranked candidate drop sites by the distance from the carrier to the building's **`position`**,
which for a building is its **anchor tile** - the north-west corner of its footprint. A town centre is 4x4, so its anchor
sits two tiles in from its own wall on every side; a lumber camp is 2x2 and its anchor sits one tile in. A carrier standing
beside a town centre's east wall was therefore measured against a point four tiles away, and a lumber camp whose *corner*
was nearer won even though its *wall* was farther.

The second half of the report - that the whole area counts and the impassable centre need not be reached - was already
true: `at_the_wall` measures `footprint_gap` against the carrier's own radius plus one step (D246/D313), so any side, any
corner, banks. Only the *choice* was wrong.

## The change

`sites.rs`: the ranking key is now `(wall_gap, anchor distance², id)`.

- `wall_gap` is `footprint_gap`'s measurement against an entity already in hand. `footprint_gap` is now that function plus
  the lookup, so there is one measure of "how far is this building" (D246's own), not two.
- The tie-breaks keep the choice a pure function of the world: anchor distance, then id.

## Test first (D7, SPEC §11)

`crates/sim/tests/gather.rs::a_nearer_wall_wins_over_a_nearer_anchor`, written before the change and **watched failing**:

```
right: Some(EntityId(9100))      <- the town centre, whose wall is 1 tile from the tree
                                  <- the carrier chose the camp, whose anchor is 2.2 tiles off
```

The fixture asserts its own scene first (the town centre's wall *is* nearer; the camp's anchor *is* nearer), so a failure
says which measure was used rather than only that the wrong id came back. After the change: green.

One ordering detail the fixture documents: `submit_checked` **queues**, so both spawns need one `step()` before they can be
measured - the first version asked for `footprint_gap` on a building that did not exist yet and panicked on `tc gap`.

## Performance

Suite: **227 result lines, 945 passed, 0 failed, 25 ignored, EXIT=0**. Acceptance scene, 48,000 ticks, seed 7, three runs
each, `--report 1` on both sides:

| | wall (3 runs) | median | entities | re-paths | hash |
|---|---|---|---|---|---|
| before | 12.13 / 12.37 / 12.19 s | **12.19 s** | 134 | 37,204 | `7782dd38d7557514` |
| after | 10.10 / 10.05 / 10.35 s | **10.10 s** | 124 | 12,847 | `b5f981d1e20eb282` |

**-17.1% wall time and -65.5% re-paths.** The cost is where it was - one pass over the player's buildings when a worker
fills up - but carriers now bank at the near wall instead of walking across the map to a camp corner, so fewer trips end in
a re-plan.

Ranking is **O(B)** on the buildings already in the iterator. The first version called `footprint_gap` per candidate, which
is `find` (**a linear scan of every entity**) plus the measure: O(B·E), about 3,500 comparisons per call on this scene. It
was replaced with the inline `wall_gap` before landing; the shape is in the diff, not in the measurements above.

## Stability

- **The match outcome changed: a win became a draw.** `players=1` before (player 2 eliminated), `players=2` after. Final
  stock: wood 1,826 -> 1,990, food 512 -> 1,305, **gold 410 -> 10**.
- **Cause, measured**: the after-run ends with `rule at 2399.2s refused: player 1 has no gold node to gather` and
  `not enough gold: 20 needed, 10 in the bank` - **220 firings**. Faster banking mined the reachable gold out sooner, and the
  scene's AI has no move to farther gold, so the army stalls at 10 gold and nobody wins. That is a scene/AI consequence of a
  faster economy, not a hang and not a crash: every refusal is counted and the match plays on to 48,000 ticks.
- **Reported, not hidden**: this is the same shape as the warning in `apply_switches` ("176 entities and a won match became
  127 and a draw"), so it is written into `docs/next-step.md` as the next item rather than treated as noise.
- No panics, no `SimError` escapes: `EXIT=0` on the suite twice, and the acceptance scene's `last engine refusal` is a
  `NoPath` on an unrelated walled-off target, as before.
