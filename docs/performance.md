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

## Twenty-fourth pass: an arrow costs a scan when it is fired, and nothing per tick (2026-10-27)

A shot is an entity, so it is walked by every per-tick pass like any other - but it does no work of its own
beyond one `step_towards` and a comparison, and it exists for the eight-ish ticks of its flight. The cost
that is not obvious is **id allocation**: `spawn_projectile` takes one past the highest id, which is a walk
of the entity list per shot. Trained units already do exactly this, so it is consistent rather than new, and
at 126 entities it is a rounding error - but in a fight with thirty archers firing every two seconds it is
thirty walks of the list per volley. If that ever shows in a profile, the fix is a stored `next_id` on
`World`, and then the whole engine gets it rather than projectiles alone.

Neither anchor moved: the stability probe is an idle world and the scripted run's first arrow lands before
its 2,400-tick window in the same shape as before (`b30d49d89f8bcf4f`, `05c91e3bff3f9eee`).

## Twenty-third pass: the pause costs one scan per producing player per tick (2026-10-27)

The ceiling pause is the first *unconditional* per-tick addition in a while: for every player with a
building in production, the tick counts that player's units to compare against their ceiling. It is one
walk of the entity list per such player - two in a two-player match with both sides producing - and it
short-circuits the whole thing when nobody is training, which is the common case in the probes. At 126
entities that is a couple of hundred comparisons, which is why both anchors stay put
(`b30d49d89f8bcf4f`, `05c91e3bff3f9eee`); it is recorded here because it is the shape that would matter at
scale, and the fix when it does is to keep a per-player unit count rather than recounting.

## Twenty-second pass: the tick loop did not change shape (2026-10-27)

The queue could have been done by turning `Entity.training` into a `Vec`, which would have widened the hot
array by 24 bytes for every entity to serve a handful of buildings - the pass-seven mistake. Instead the
head is still `Option<Training>` on the entity, the tick loop still walks one job per building, and only the
handover (once per finished unit) touches the side table. `Entity` is unchanged in size, and the tick costs
exactly what it cost before: one `saturating_sub` per building with a job.

The anchors are unchanged (`b30d49d89f8bcf4f`, `05c91e3bff3f9eee`), and this time that is meaningful rather
than incidental: the scripted run trains villagers, so it does enter the training code - it simply enters
the same code as before, because the head of the queue is where it always was.

## Twenty-first pass: one comparison swapped for another, in the tick (2026-10-27)

Ranged combat is the first change in six passes to touch the tick path: the attack branch used to compute
two `abs_diff`s on tiles and now computes two subtractions and two multiplications on raw positions when
the attacker has reach. Melee attackers keep the tile comparison, so a world of spearmen runs the code it
ran before. This is not something the idle probe can resolve - it has no attacking entities at all, which
is why both anchors are unchanged (`b30d49d89f8bcf4f` four-hour, `05c91e3bff3f9eee` scripted at 2,400
ticks). Worth stating plainly: the anchors are unchanged because they never enter the code that changed,
not because the change is free. What can be said is the shape - a handful of integer operations per
attacking entity per tick, no division, no square root, no allocation - and that it is bounded by the number
of attackers rather than by the number of entities.

## Twentieth pass: two scans per order, none per tick (2026-10-27)

Housing adds two walks over the entity list - population, and the finished houses and town centres - to
`check_train`, which runs when an order is placed. Nothing is stored: both numbers are derived, so `World`
did not grow and no tick does any new work. The idle probe therefore has nothing to see and nothing was
claimed from it; the four-hour probe is unchanged at `b30d49d89f8bcf4f` and the scripted run is unchanged
at `05c91e3bff3f9eee`, because the scenario's villager count never reaches the town centre's five.

This is the first step in a while where *no* hash moved, which is worth a sentence: the feature changes what
a player may do, not what the world is.

## Nineteenth pass: counting happens on a research, not in a tick (2026-10-27)

The prerequisite check now walks a tech's requirement list and counts, in both places it exists: an order
and the availability resolver. Both run when a tech is researched, never inside a tick, and nothing was
added to `World` - `min_required` lives on the `Tech` table that was already loaded. So the idle probe has
nothing to see and nothing was claimed from it. The four-hour probe is unchanged at `b30d49d89f8bcf4f`
(that world loads no tech tables). The scripted run moves again, to `05c91e3bff3f9eee`, because more techs
are granted at load now that the OR-sets can be satisfied.

## Eighteenth pass: a load-time list, granted only on a research (2026-10-27)

Shadow techs add roughly two hundred ids to `availability_techs`, and that list is walked by
`grant_availability`, which runs when a tech is researched - never inside a tick. Nothing was added to
`World` beyond entries in a set that already existed (`researched`), so the idle probe has nothing to see
and nothing was claimed from it. The four-hour probe is **unchanged** at `b30d49d89f8bcf4f`, because that
world loads no tech tables at all and the list is empty there. The scripted run with every table moves to
`431d5b8fb2076854`: the shadow techs are in `researched`, which is hashed, and a replay that lost them
would refuse the Feudal buildings.

## Seventeenth pass: farm food is priced at two moments, not every tick (2026-10-27)

The farm food line adds nothing to the tick: the total is computed when a plot is placed and, for existing
plots, once when a tech finishes (`grow_plots`, a single pass over the entity list). No state was added to
`World`, so there is nothing for the idle probe to see and nothing was claimed from it. The four-hour probe
is unchanged at `b30d49d89f8bcf4f`; the scripted run now reaches the build order (900 ticks was 45 s and the
build is at 60 s), so the determinism anchor is the longer one: 2,400 ticks, `046fd1d8d8d0301c`.

## Sixteenth pass: a change the idle probe cannot see, and a wrong guess about why (2026-10-27)

Farms add per-tick work, but only for workers: two tile conversions and two comparisons in the gather and
build arms. The idle probe is all-idle, so it should be blind to this - and it was not: **base 2.16-2.18
us/tick against 2.32-2.73** over two interleaved rounds, load average 2.8-3.0.

The first hypothesis was the pass-seven one, that the change had grown `Entity`. It was checked rather than
asserted, and it was **wrong**: a throwaway `size_of` test gives `Task 32, Entity 184` on *both* builds -
the extra field fitted in existing padding. (It has since been removed again for other reasons, at no cost
in bytes.) So the difference is either the floor from pass thirteen or an effect of code placement; either
way it is not attributable, and the numbers above are recorded as unresolved rather than as a regression.
The four-hour probe is a better signal and it is unchanged: 126 entities, `b30d49d89f8bcf4f`.

One caveat worth keeping: the scripted 900-tick hash did not move either (`936b127946fd2ef3`), because 900
ticks is 45 s and the scenario's first build order is at 60 s. A hash that "did not change" only means
something if the replay reached the code.

## Fifteenth pass: teams are seed-time and per-attack (2026-10-27)

`team_of` runs inside `check_attack`, and the team array is resolved once at seeding. No tick path, so by
the thirteenth pass' finding this is not something the idle probe can resolve, and it was not re-measured
through it. What is measurable: the four-hour probe is **unchanged** at `b30d49d89f8bcf4f`, because the
teams are hashed only when content names them and no probe does. A world that names teams hashes
differently by design - the team decides who may attack whom.

## Fourteenth pass: judged by inspection, because the probe cannot see it (2026-10-27)

Per-player civs add one array to `World` (`[Option<u32>; 8]`, 32 bytes) and change two lookups from
world-level to player-level. Both lookups run on an order, never per tick, so by the thirteenth pass'
finding this is not a change the idle probe can resolve - and a fresh A/B would only re-measure the ±8%
floor. What is measurable is reported instead: the four-hour probe is **unchanged** at
`b30d49d89f8bcf4f` (it loads no civs, so every entry is `None` and the hash loop writes nothing), while
the scripted run with all tables moves to `936b127946fd2ef3`, because both players' civ ids are now hashed
per player - which is the point: a replay cannot lose *whose* civ was which.

## Thirteenth pass: the idle probe cannot resolve a change with no tick path (2026-10-27)

This pass added a civ-level unit bar - the Huns cannot build houses. Nothing in it runs per tick: the bar
is a scan of one effect's command list, and only when a build or train order is placed. The interleaved
A/B nevertheless reported **+6% at 126 entities**, reproducibly, in three rounds and both orderings
(base 2.27-2.35 µs/tick, new 2.39-2.51). So it was chased rather than explained away:

1. The first suspicion was the `BTreeSet` the change had put on `World`. **Removed** it (the bar is now
   derived on demand) and re-measured: 2.42-2.44, unchanged. Not the field.
2. The next was the data: every A/B since the seventh pass had run the base worktree with a
   `units.toml` **stripped of `variant_of`**, because back then the base could not parse it. That strip
   makes the base's unit table smaller than the new build's. Fed the current binary the stripped file and
   it measured 2.23-2.27 - matching base - while both builds with the full file split 2.25-2.27 (base)
   against 2.44-2.51 (new). So the strip does bias the comparison, **but it is not this gap**.
3. What is left is code layout: nothing in the change is reachable from an idle tick, and the same code
   measured twice in the same process reproduces to 0.01 µs, so the difference is between *builds*, not
   runs.

The honest conclusion, and the reason this is written down rather than rounded off: an idle-tick probe at
126 entities has a between-build sensitivity of roughly ±8%, which is the same order as the changes we
have been reporting since the seventh pass. The "~2% residual" recorded there and carried forward in four
later passes is almost certainly this sensitivity, not a regression. What the probe can still be trusted
for is a change that *does* run per tick (the entity-count scaling curve), and what should be used to
judge a change that does not is the inspection plus the unchanged hashes - the four-hour probe still ends
at 126 entities with `b30d49d89f8bcf4f`.

## Twelfth pass: upgrade gating is load-time plus per-order (2026-10-27)

The unit -> upgrade-tech map is built once at table load, and the two gates (available? upgraded?) run on
`check_train`, i.e. when a player orders a unit. Nothing runs per tick. The interleaved A/B: 126 entities
2.42 against 2.37-2.38 µs/tick (both rounds), 1,126 19.73-20.30 against 19.53-19.54, 2,126 36.80-37.48
against 36.91-38.60. That is the same ~2% band recorded since the seventh pass - the one already
attributed to code layout rather than work - and it is reported rather than treated as free.

The four-hour probe is unchanged again at `b30d49d89f8bcf4f`; the scripted run with all tables hashes
`470c148c117eae84` at 900 ticks, the difference from the previous pass being the scenario's own change
(a Dark Age barracks now trains militia, because a spearman needs the Feudal Age).

## Eleventh pass: availability is load-time and per-order (2026-10-27)

The two availability sets (109 gated units, 81 bookkeeping techs) are derived once when the research
tables load, and the per-order check walks the player's researched set. Nothing runs per tick. The
interleaved A/B: 126 entities 2.24-2.26 against 2.27-2.28 µs/tick, 2,126 37.19-37.42 against 37.08-37.19
- parity; 1,126 reads 19.80-19.83 against 19.39-19.45, the same 2% residual recorded since the seventh
pass, on a box at load 4.2.

Hashes: the four-hour probe is unchanged at `b30d49d89f8bcf4f` (it loads no research, so nothing is
gated and nothing auto-grants), while the scripted run with `--techs`/`--effects`/`--civs` moves to
`66179905f8bcffa8` - the difference being three techs the opening position now applies for itself
(220 barracks, 523 palisade gate, 332 outpost) and one it no longer has to.

## Tenth pass: a civ's bar list is derived per order (2026-10-27)

No tick path changed: the restriction list is resolved inside `check_research`, i.e. when an order is
placed, and never per tick. The interleaved A/B ran twice; the first pair is parity (1,126 entities 19.73
against 19.80 µs/tick, 2,126 37.11 against 37.44) and the second was contaminated — base measured 24.79
and 48.23 on a box whose load average was 4.7, which is the same signature as the false regression in the
fourth pass. One clean pair plus an a-priori argument that nothing runs per tick is the honest report;
the contaminated pair is stated rather than quietly dropped.

Hash: the four-hour probe still prints `b30d49d89f8bcf4f` (that world loads no civ, and the civ id is
hashed only when set, so it hashes exactly as before), while the scripted run **with** `--civs` prints
`692abe64904f0474` — the intended difference, a world that knows which civilisation it is.

## Ninth pass: the opening stockpile is seed-time only (2026-10-27)

Interleaved A/B against `HEAD`: 126 entities 2.28-2.35 against base's 2.34-2.38, 1,126 19.05-19.69
against 19.20-19.30, 2,126 36.35-37.57 against 36.47-36.99 - parity in both orderings, which is what a
seed-time assignment into a fixed array should look like. Load average 3.0-4.0 on the host throughout.

Two hashes moved, both **because the feature is state**, and both are the intended difference rather
than a drift: the four-hour probe now ends at `b30d49d89f8bcf4f` (was `936e71d5a641044f`) and the
600-tick scripted run at `d5fe4af2413a0791` (was `0f5eea624d75d87a` without research tables), because
every player opens with 200/200/100/200 and the script's own first step now researches Loom. The probe
still ends at 126 entities with no growth.

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
