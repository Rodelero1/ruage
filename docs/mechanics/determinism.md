# Determinism and sync

This file is different from its siblings: it is not facts about the game, it is the
**constraints our engine must satisfy** (goal G1 in SPEC §2), plus what we know about how
the original achieves it.

## What the original does — **inferred**

Evidence is indirect but consistent:

- The binary is 32-bit x86 C++ built from the same tree as the DE (`phoenix`) codebase
  (build path leak, SPEC §3.4).
- Multiplayer in the Genie engine is lockstep with periodic checksums; the community's
  sync-error folklore ("out of sync") is the signature of a deterministic lockstep sim
  whose clients diverged.
- The dat stores floats throughout (speed 0.8, reload 1.8), so the original almost
  certainly simulates in **floating point** and relies on identical FP behaviour across
  machines.

**Unknown:** whether the original's sync check is a state hash, a memory checksum, or a
command-log digest; and at what tick interval.

**Direct testimony — the engine's own programmer.** Matt Pritchard (Ensemble Studios) is
quoted in the community infosheet describing exactly this class of problem:

> "The game simulation was not designed to deal with significant variance in the game turn
> time […] I am referring to things like the x86 Floating Point control word getting changed
> […] causing slight variances (that could result in out of syncs, etc), and precision
> problems with using floating point numbers (as distance from 0 goes up, you lose mantissa
> precision / numbers that can not be truly accurately expressed in floating point)."

That is the strongest available evidence for two entries in the constraint list below: the
original simulates in floats, and it desyncs when float behaviour varies. It also explains
why lockstep worked at all — everyone ran the same binary on the same architecture.

## What we should do — constraints

These are the rules that make a sim verifiable, independent of the original:

1. **No floats in simulation state.** Fixed-point (or integer) for position, velocity,
   timers, and resources. Floats may be used to *load* the dat and are converted once at
   load; the sim never sees them. Rationale: FP arithmetic is not associative and differs
   across compilers/CPUs/optimisation levels. The dat's floats are inputs, not dynamics.
2. **One seeded RNG, explicit stream.** All randomness (map `start_random`, unit
   dispersion, conversion, accuracy rolls) draws from a seeded generator whose algorithm
   and seed are recorded in the state. No `rand()`, no time, no thread ids.
3. **Stable iteration order.** Every collection the sim iterates is ordered by a stable key
   (entity id), never by hash-map order or insertion order of a parallel container.
   Rationale: hash iteration order changes between runs and builds; it is the single most
   common cause of desync in reimplementations.
4. **Command log, not state replication.** Inputs are recorded as (tick, player, command).
   A replay is re-running the log; the state hash at tick N must match. This makes desync
   reproducible and debuggable instead of a mystery.
5. **Hash every N ticks.** A cheap 64-bit hash over the canonical state; mismatch = bug,
   not bad luck. This is the acceptance test for step 4's exit criterion.
6. **Deterministic map generation.** The `.rms` interpreter must draw from the same seeded
   stream as the sim (see `world-rms.md`) — otherwise two clients get different maps from
   the same seed.
7. **No wall-clock, no threads in the sim loop.** Parallelism, if ever added, must be over
   independent partitions with deterministic merge order.

## What this costs

Floats in the dat mean every value needs a fixed-point representation and a decision about
precision: 1/256 of a tile is finer than unit radii (0.2 tiles), so `Q16.16` or
`int32 scaled by 256` are both adequate. Speed 0.8 tiles/s at 20 ticks/s = 0.04 tiles/tick,
which is representable exactly only at sufficient precision — pick the scale so that
`speed × tick_duration` is exact for all dat speeds, or accept and document rounding.

## Open

- Tick rate: original is widely believed to be ~20 ticks/s for unit movement. **unknown**,
  and it changes every timer in the game.
- Precision policy for dat floats (**design decision**, not yet made).
- Whether any tech effect or civ bonus uses a non-representable multiplier (1.1 and 1.25
  are fine; 1/3 would not be).

## Why this matters before step 4

If the sim is written with floats and fixed later, every test that passed will be
re-baselined. If it is written fixed-point from the start, the first state hash is already
the contract. That is the entire argument for deciding this now rather than after the
vertical slice.

## Where our tick shape differs from the original (2026-10-29)

Two divergences recorded from prior art, neither of them a bug to fix now:

- **The unit AI is per player, not per unit.** genie's `docs/ENGINE_SYSTEMS.md`: "Only one player's
  unit AIs run their full update in a tick: the player whose turn it is; the other players' only take
  their orders and notifications." Our aggro scan is staggered per *unit*
  (`combat.aggro_scan_ticks`), which spreads the same work but on a different rule. Changing it would
  shift every existing test's timing for something no player can see; record, do not adopt.
- **One pather, not six.** The original ships a tile pathfinder, a long pather, a short pather, an old
  hull pather, an obstruction mesh and a zone map (same table). We have one A*, which is a bounded-
  scope choice (U7), and openaok shows it is a playable one.
