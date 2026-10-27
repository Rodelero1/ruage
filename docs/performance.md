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

## Third pass, after the resource-enum deletion (2026-10-26)

Same probes, same machine, re-run three times because the first attempt was wrong (below):

| entities | µs/tick | ns/entity/tick | previous |
|---|---|---|---|
| 126 | 2.26–2.36 | 18.0–18.8 | 2.35 / 18.6 |
| 376 | 6.10–6.34 | 16.0–16.9 | 6.16 / 16.4 |
| 626 | 10.78–10.88 | 17.2–17.4 | 11.19 / 17.9 |
| 1126 | 18.09–18.25 | 16.1–16.2 | 18.63 / 16.5 |
| 2126 | 32.99–33.44 | 15.5–15.7 | 32.62 / 15.3 |

A ten-minute match (12,000 ticks, 126 entities): **27.4–28.5 ms** wall. Flat ns/entity again, so
replacing the enum with a content table cost nothing measurable: the tick never looked a resource
up, and widening the stockpile row from 4 columns to 8 touches only the state hash, not the loop.

**The measurement that was wrong, and why it is worth writing down.** The first run of this pass
reported 21.05 µs at 1,126 entities and **48.63 µs at 2,126 — a rising ns/entity (22.9), which is
the exact signature of the O(n²) the second pass removed.** It was noise: a `--release` build of the
probe itself was still running in the background, and this container has **two cores**. Nothing in
the code had changed between the bad run and the good ones. Rules that follow: measure with nothing
else running, repeat three times, and treat a *rising* ns/entity as a hypothesis to test rather than
a verdict - it is also what a busy machine looks like once the small worlds stop fitting in cache.

Stability, same pass: the four-hour probe (288,000 ticks) ends with the entity count it started
with (126 - no leak), and the same seed reproduces `hash=508a746d89cc76e1` exactly.

## Eighth pass: no tick path touched (2026-10-27)

Starting techs are granted at seed or table-load time, so the interleaved A/B has nothing to find, and
it found nothing: parity at 126 entities (2.23-2.28 against base's 2.37-2.53) and 1,126 (19.71-19.99
against 19.38-20.00). At 2,126 the numbers read 36.7-37.8 against 34.8-36.1, which is the **same 2-3%
residual the seventh pass recorded as unexplained** - not a new cost, and the box's load average was
4.8 throughout. The stability and scripted hashes both stayed put (`936e71d5a641044f`,
`0f5eea624d75d87a`) because those probes load no research tables; with `--techs`/`--effects` the
scripted run hashes `db92f158160e7a5b`, which is the intended difference: the opening state now
contains the Dark Age tech.

## Seventh pass: a field on `Unit` costs more than a lookup table (2026-10-27)

The prerequisite gate needed the variant relation (an age-up replaces the town centre 109 with 71), and
the first version put `variant_of: Option<u32>` on `Unit`. The interleaved A/B - the method from the
sixth pass - said the tick got **4-6% slower at 1,126 and 2,126 entities** while nothing new ran per
tick, which is the sort of number that has to be chased rather than explained away.

Chased: the same build with a `units.toml` stripped of the new field measured **identically to base**
(19.07/19.66 at 1,126 against base's 18.63-19.18), so the cost was not the code but the **parsed table's
footprint** - 8 more bytes on each of 1,166 records, for a relation only 190 of them have.

So the relation moved out of `Unit` and into `Units.variants`, a 190-entry map. `Unit` is byte-for-byte
what it was, and the gap narrowed to what the method's own bias produces (base run against base is ~2%
slower in second position):

| entities | base | new | reading |
|---|---|---|---|
| 126 | 2.23-2.25 | 2.26-2.35 | parity; 2.35 is one noisy round |
| 1126 | 18.95-19.20 | 19.43-19.76 | ~2-3% higher, order checked both ways |
| 2126 | 36.03-36.82 | 36.70-37.74 | ~2% higher |

A residual **2-3% at 1,126+ entities remains unexplained**: every new lookup happens on an order or a
deposit, none per tick, so this is codegen or layout rather than work. It is recorded as open rather
than dismissed, and it is not visible at 126 entities - a real match - where the two builds are
indistinguishable within noise.

## Sixth pass: measured against the previous commit, not against a memory (2026-10-27)

The fifth pass could only say "3-8% slower at the larger sizes, but the box was busy", which is not a
result. This pass fixes the method instead of the code: a **worktree of `HEAD`** built beside the
working tree, run **interleaved** (base, new, base, new), so the host's load cancels out of the ratio.
It needs `config/gamedata` copied in - that directory is generated (D8), so it is not in git and a
worktree starts without it, and a perf probe with no unit table just skips.

| entities | base (HEAD) | new | reading |
|---|---|---|---|
| 126 | 2.18, 2.31 | 2.39, 2.43 | noise swamps it: 2.18-2.43 across four runs on a loaded box |
| 1126 | 19.26, 19.34 | 19.37, 19.30 | identical |
| 2126 | 36.90, 37.09 | 36.89, 36.51 | identical |

Load average 3.2-4.4 throughout, from outside this container (no process in it was using CPU), which is
why the interleaved ratio is the only number worth reading. **Nothing in this slice runs per tick**: the
`[[train.entry]]` roles are resolved when the pack loads, and the construction maximum is refreshed when
a research completes. Two hashes agree with that: the four-hour probe still prints `936e71d5a641044f`
and the 600-tick scripted run still prints `0f5eea624d75d87a`, both unchanged from the previous pass.

## Fifth pass, after health, speed, gather rate and carry (2026-10-26)

Nothing in this slice runs per tick: health and speed are read when a unit spawns, is upgraded or has
a tech land, and gather rate and carry capacity when a gather order lands. So the expectation was "no
change at 126 entities", and that is what the probe shows (two clean runs; a third run was discarded
because the load average was 3.0 on a two-CPU box and every number in it was inflated by half).

| entities | fourth pass | fifth pass | ns/entity |
|---|---|---|---|
| 126 | 2.27-2.40 | 2.36-2.38 | 18.7-18.9 |
| 626 | 11.13-11.43 | 11.60-11.84 | 18.5-18.9 |
| 1126 | 18.68-19.31 | 19.39-20.42 | 17.2-18.1 |
| 2126 | 34.51-35.96 | 36.74-39.58 | 17.3-18.6 |

The larger sizes read 3-8% above the fourth pass, and I am **not** claiming that as a regression: the
machine's load average was 3.0 against 2.3 for that pass, one repetition was obviously contaminated,
and a code path that only runs on spawn/order/damage cannot add per-tick cost. A ten-minute match is
28.0-28.3 ms against 27.9-28.8.

The stability probe is the better evidence for this slice: 288,000 ticks still end at 126 entities and
the hash is **`936e71d5a641044f`, exactly what the fourth pass printed** — the state did not change
shape, because no field was added. A scripted 600-tick run also hashes identically against `HEAD`'s
build (`0f5eea624d75d87a`), which is what "behaviour-preserving when nothing is researched" means
when it is checked rather than asserted.

## Fourth pass, after research and modifiers (2026-10-26)

Same probes, three runs each, and one methodological note that matters: the earlier passes' numbers
were taken on a *separately measured* machine state, so they are the baseline this pass is judged
against (2.26-2.36 us/tick at 126 entities, 32.99-33.44 at 2,126).

| entities | before (v0.55) | after (v0.56) | ns/entity before -> after |
|---|---|---|---|
| 126 | 2.26-2.36 | 2.27-2.40 | 18.0-18.8 -> 18.0-19.1 |
| 626 | 10.78-10.88 | 11.13-11.43 | 17.2-17.4 -> 17.8-18.3 |
| 1126 | 18.09-18.25 | 18.68-19.31 | 16.1-16.2 -> 16.6-17.2 |
| 2126 | 32.99-33.44 | 34.51-35.96 | 15.5-15.7 -> 16.2-16.9 |

A ten-minute match (126 entities, 12,000 ticks): 27.9-28.8 ms, unchanged in practice. **At 2,126
entities the tick is ~5% slower, and the cause is not the new code path.** The first attempt blamed
the obvious suspect - a second per-tick scan of the entity list, added for the research timer - so it
was merged into the production scan (one pass, two lists). Re-measured: no change at 2,126. What did
change is `Entity`: `Option<Research>` is another 16 bytes on a struct that is walked once per tick,
so fewer entities fit per cache line. That is the honest price of research state; boxing the job
would take 8 of the 16 bytes back if it ever matters, and a real match (126 entities) does not feel
it.

Stability, same pass: the four-hour probe still ends with 126 entities and no growth, and the same
seed still reproduces its hash exactly - `936e71d5a641044f` now, where the previous pass printed
`508a746d89cc76e1`. That change is expected and worth stating: the hash now covers `researching` per
entity and the per-player `researched` set, so any state-shape change moves it. Behaviour is
unchanged (entity count, damage arithmetic, and every non-research test still passes).

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
