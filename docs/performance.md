# Performance

Measured, not assumed. Probes live in `crates/sim/tests/perf.rs` and are `#[ignore]`d — they print
timings rather than assert behaviour, so they are not tests:

```
cargo test --release -p ruage-sim --test perf -- --ignored --nocapture
```

Machine: 2 vCPU / 4 GiB container. Numbers are release-mode.

## After the first adversarial pass (2026-10-25)

| entities | µs/tick | ns/entity/tick |
|---|---|---|
| 126 | 2.35 | 18.6 |
| 376 | 6.16 | 16.4 |
| 626 | 11.19 | 17.9 |
| 1126 | 18.63 | 16.5 |
| 2126 | 32.62 | 15.3 |

A ten-minute match at 20 Hz (12,000 ticks, 126 entities): **28 ms** wall.

Flat ns/entity = linear in entities. At 2,000 entities a tick costs 33 µs, which is 0.65 ms per
simulated second — 0.06% of one core. There is no performance problem to solve here yet.

## What the first pass found, and why the numbers before it were bad

The tick used to collect this tick's movement as `(id, dx, dy)`, then resolve each id with a linear
scan to apply it — **O(n²) per tick**. Before the fix:

| entities | µs/tick (before) | ns/entity/tick (before) |
|---|---|---|
| 126 | 8.77 | 69.6 |
| 626 | 146.24 | 233.6 |
| 1126 | 468.41 | 416.0 |
| 2126 | 1727.42 | 812.5 |

Rising ns/entity is the signature: the per-entity cost *grew* with the world. Changing the collection
to carry the entity's index made the apply loop O(1) per entity: **53× faster at 2,126 entities**, and
the ten-minute match went from 104 ms to 28 ms.

### The bug that fix nearly introduced

Indices are only valid while the entity list does not move — and `insert` keeps the list **sorted by
id**, so a unit spawned mid-tick shifts every index after it. Applying by index alone would then move
the wrong entities, and the entire suite passed without noticing, because no test both moved and
spawned in one tick.

The tick therefore carries `(index, id, dx, dy)`: the index is the fast path, the id is the check. If
the entity at that index is not the one the decision was made for, the apply falls back to an
id lookup — slower, correct, and only on ticks where the world changed shape.
`a_spawn_in_the_same_tick_does_not_move_the_wrong_entities` covers it.

## The next risks, in order, none measured yet

1. **`find` and `find_mut` are linear scans** (27 call sites, several inside per-entity loops). The
   idle-world probe does not exercise them, because nothing fights or deposits. A battle at 2,000
   units resolves a target per attacker per tick — the same O(n²) shape that was just removed, one
   layer up. The fix is an id → index map kept in step with `insert`/`retain`, or a binary search
   that relies on (and asserts) the sorted-by-id invariant.
2. **`spawn_from_building`** scans every entity for each candidate tile: O(n) per spawn, O(n·tiles)
   in a production boom. Fine at 8 barracks, worth a spatial index if building counts grow.
3. **A\* per order, no path cache.** 64×64 is cheap; 4v4 with constant re-tasking is where it would
   show. Not measured because nothing re-paths aggressively yet.
4. **Six `Vec`s allocated per tick.** Measured as noise (2.35 µs/tick at 126 entities), so left alone
   — a pool would be premature complexity for a cost below the measurement floor.
5. **`retain` every tick** even when nothing died. Also noise; kept for readability.

The rule that matters: **do not optimise what has not been measured**, and when a number changes
shape with n, suspect a lookup inside a loop rather than a constant factor.

## Stability pass (2026-10-25)

Probes in `crates/sim/tests/stability.rs`, also `#[ignore]`d:

- **Four simulated hours** (288,000 ticks) on the authored map: no panic, entity count flat at 126
  from start to finish, tick counter exact. 0.63 s for the run, so a whole match is far cheaper than
  real time.
- **The same seed twice, four hours**: identical state hash (`6a619a6227c77db1`). Determinism was
  previously checked only over ten minutes; a divergence needing hours to surface - an accumulator,
  an unordered iteration - would have slipped past.

### The leak this found

`paths` (route per entity) was never pruned when an entity died. A unit killed mid-walk kept its
remaining waypoints **forever**, and `state_hash` walks `paths`, so a dead unit's route was part of
the hash as well. A four-versus-four match with hundreds of deaths would accumulate routes nobody
walks; the idle probe could not see it because nothing dies.

Fixed by collecting the buried ids in the same `retain` that removes them and dropping their routes.
`a_dead_unit_does_not_leave_its_route_behind` covers it and was watched failing first
(`left: Some(1)`).

### What is still not covered

Nothing kills entities over a long run, so the hours-long probe exercises the idle backbone only. A
long *fighting* run - scripted production on both sides for four hours - is the missing stability
test, and it needs a scenario that keeps making units.
