# D264 - the legal standoff: review

*A change to where the short-range planner is pointed. Measured before and after from **one worktree each**, same configs, same
seed, three runs apiece.*

## What changed

`World::legal_standoff(at, radius)` and one call site: the short-range planner (`short_range_clear`) is given a goal the body can
stand on instead of an aim that sits inside an obstruction. Plus `push_out_of_buildings`, which the standoff asks.

Nothing else in the mover moved: the slide is still in place (see Stability), the step test is unchanged, and no constant was tuned.

## Performance

**The acceptance match, 48,000 ticks, seed 7** (before = `HEAD` in a worktree at `/opt/data/cache/scratch/age2hd-before`; after =
the working tree; both `--release`):

| run | before | after |
|---|---|---|
| 1 | 43,691 ms | 18,014 ms |
| 2 | 41,267 ms | 18,395 ms |
| 3 | 39,920 ms | 17,016 ms |

A second before-set, from the background job that built the same worktree (it printed after this note was first written):
**49,560 / 40,102 / 43,427 ms** - same hash, same counts. So the before-range is **39.9-49.6 s**, a **24% spread on an unchanged
binary**, and the after-set's 17.0-18.4 s sits below every one of the six. The spread is the machine (a 4 GiB / 2 CPU cgroup), not
the change, and it is the reason the mechanism - ground re-plans 316 -> 122 - is the number to trust, not the wall clock.

**Repaths**: 761 -> 431 in total (body 445 -> 309, **ground 316 -> 122**). The ground re-plans are the tile search's, and they fall
by 61% because the short-range planner now answers the approach instead of returning `None` and handing it to the search.

**This is not a like-for-like speed-up and must not be quoted as one.** The match itself plays out differently (see Stability),
which is where most of the time went. The honest claim is the mechanism: **the good planner is now on the gather path**, and the
tile search's re-plans fall accordingly.

**Cost of the new code**: the standoff is a bounded ring search (eight directions, eight rings, at most 64 `clearance_at` and 64
`push_out_of_buildings` calls) and runs **once per plan**, only when the aim is blocked. `animals` runs 5.45 s before and 5.67 s
after (the same machine, ten runs' typical spread is larger than that difference).

## Stability

- **Full suite, `--no-fail-fast`**: **200 binaries, 199 ok, 1 failed** - the pre-existing
  `army_walk::a_crowd_walking_through_a_gap_stays_on_free_ground`. Identical to the baseline at `3c147b0`. No new red.
- **Determinism**: the two acceptance runs printed the **same hash** (`336c60c17233edf3`) and the same repath counts. The hash has
  *changed* from the baseline (`01a0fb771ee2e75b`), which is expected and is reported below.
- **The acceptance match plays out differently, and this is not yet explained**: before, 138 entities and **1** player were left at
  tick 48,000; after, 133 entities and **2** players. One player is no longer eliminated. The most likely cause is the economy
  running properly (a gatherer who could not reach a node now can), but I have **not attributed it**, and it should be before D264
  is treated as settled. Reported rather than smoothed over.
- **The slide is still load-bearing**: deleted with this change in place, the animals family goes **3 red of 11** -
  `a_live_animal_is_a_gather_source`, `the_villager_follows_the_sheep`,
  `the_shepherd_kills_the_sheep_and_the_gathering_resumes_on_the_body`. A gatherer's place is kept to within a fifth of a tile by
  `settle_workers`, those nudges are far under a step, and a whole-step-only rule leaves him a fraction short. The reason is now
  recorded at the slide in `movement.rs` and in D264; the next piece is a whole-step approach in `settle_workers`.
- **One error of my own, kept**: the first cut of `legal_standoff` was added to `lib.rs` against an anchor that did not exist and
  the patch failed; the second attempt was placed above `body_clearance`. No trace in the tree, but the ordering of the two doc
  comments (standoff before clearance) is the result.
