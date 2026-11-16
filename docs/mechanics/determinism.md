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

**Corroborated 2026-10-30, for DE and not yet for 5.7.2.** `librematch/genie` now writes its own
determinism rules down (`CODING_STYLE.md` §12) from a port that matches DE build 158041 bit for bit,
and they are the rules *of a float simulation*: keep the game's `f32`/`f64` field by field, round
each sum and product on its own (no fused multiply-add, no `a * b / c` as `a * (b / c)`), convert
float to integer through `cvttss2si` rather than Rust's saturating `as`, use the game's own float
functions. Two of the rules are about data structures rather than arithmetic, and they are facts
about the original too: a list removal **swaps the last element into the free place** (so list order
carries a history), and a sort is **MSVC's introsort, which is not stable** (so ties fall as the
game's do). Nothing here contradicts the inference above; it names the price of reproducing it, which
is the argument for constraint 1 rather than against it.

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

## Where our tick shape differs from the original (2026-10-30): ours is simultaneous, theirs is ordered

The third divergence, and the largest, found in `librematch/genie`'s `world/step.rs` and the commit that
emptied its `Context` (`94f902f`): **the original updates its objects one at a time, and a call on another
object takes effect at once.** Gaia's objects, then players 1 to 8, each player's objects in list (slot)
order; a later object in a tick sees what an earlier one did in the same tick - a hit that landed, a unit
that died - because the game's objects read the world's and the players' fields live rather than from a frame
taken when the tick started. A dead object's actions are torn down in its own life step, so a unit killed
earlier in the tick does not swing later in it.

Ours is the opposite shape, deliberately: `Sim::step` decides every entity's work in one pass over an
immutably borrowed list, against the state the tick opened with, and applies what it decided afterwards
(`strikes`, `shots`, `landings`, `deltas`, `deposits`). Nothing an entity does can change what another
entity decided in the same tick.

**What it costs us.** Two units can trade killing blows in the same tick and both die here; there, the one
whose update ran first wins with whatever it has left. Which of the two runs first there is player-and-slot
order, i.e. a consequence of history nobody can see; ours is id order, i.e. spawn order, also a consequence
of history, and then it does not matter because both blows land.

**Why we are not changing it now.** The simultaneous tick is order-independent by construction, which is a
determinism *advantage* and the reason our replay hashes are stable across refactors of the pass; the
ordered tick would move every fight's outcome by up to one tick and re-baseline the suite. **What is
missing is the test, not the decision:** nothing in `crates/sim/tests/` pins what happens when two units
kill each other on the same tick, so the behaviour is real but unexamined. Recorded here and as **U18**.

## The shape of the whole tick, from DE's side (2026-11-04)

`librematch/genie` now publishes the original's per-tick order as three ordered arrays
(`crates/engine/genie-core/src/world/schedule.rs`, lines 20-120), each step's doc naming the exe routine it ports:

- **before the players**, one step: `Commands` (the update's count and time, the streams' frame seed, the players'
  recorded commands);
- **for each player, Gaia first**, ten steps in order: `Clock`, `OpenGates`, `StartWork`, `Groups`, `WorldPlayer`,
  **`Objects`**, `NewTiles`, `ScoutMap`, `CloseGates`, `ClearTiles`;
- **after the players**, nine: `Draw`, `Ghosts`, `Watched`, `Reseed`, `Turn`, `Victory`, `FrameSeed`, `LastRandoms`,
  `Vision`.

That is **DE build 158041's** order, read from x64 `0x14…` addresses, so it is evidence about the engine family and not a
reading of our binary: HD 5.7's order has not been read, and the corpus is where it would be (`BaseWorld::update`, the
routine that calls the victory check and the players' vision pass). What it changes here is the question, not the answer:
U18 is no longer "what is the order?" in the abstract but "does HD put `Objects` in the same place?" — one step among ten,
after `Groups` and before `NewTiles`, with vision once at the very end. Two consequences if it holds there: a group's
motion is computed from the state the `Groups` step saw, before any object of that player moves; and the victory check
sees the whole tick's damage, both of which our single ordered-by-id pass gets for free by being simultaneous.

## HD 5.7's own tick, read (2026-11-15)

The paragraph above said HD's order "has not been read, and the corpus is where it would be (`BaseWorld::update`...)". It
has now been read, and the DE reading above turns out to be the same shape in the one thing it was asked about.

`FUN_00738720` at `0x738720` is `BaseWorld::update`: the profiler string `"baseworld update"`, the `aoeworld-at-%08d.txt`
status log it writes, and its assertions' path `...\aoc\source\game\baseworld.cpp` (lines `0xb6b`, `0xc7e`, `0xcb7`). In
order:

1. **Players in index order.** The outer loop walks the array at `+0x184 .. +0x188` (8-byte stride) from index 0 upward -
   Gaia first, then players 1-8 - and skips a player whose list is empty.
2. **That player's objects, one at a time.** For each player it walks an object list (head `+0x48`, node `next` `+0x5c`,
   node owner `+0x60`) and skips nodes owned by anybody else. Per object: **one virtual call decides removal** (`+0xa0`
   answering 0 unlinks the node and calls vtable slot 0, the class's destructor) and **otherwise one virtual call runs the
   object's own update** (`+0x84`). The methods cannot be named - both are virtual and the binary has no symbols - but the
   shape is *sweep, else step*.
3. **Death is applied in the victim's own turn**, not in a pass after the tick. That is the mechanism behind U18's answer:
   a unit killed by an earlier-ordered object is destroyed before any later object runs, so it does not swing, step,
   gather or finish anything for the rest of that tick.
4. **Per-player bracket**: `+0x130` before the object walk, `+0x68` after it, `+0x134` for each player already visited;
   then three more per-player loops gated on `FUN_00727810()`.
5. **The turn is real, and it is `pturn`.** The index at `+0x100` is advanced by one at the end of the update and wrapped
   at the player count - and `+0x100` is `pturn`, the field `docs/save-format.md` reads out of the save header. This is the
   **`Turn`** step the DE schedule above lists *after* the players, found independently in 5.7. It is T16's axis.

What is still not read: the order **inside** one object's update. The DE port's `world/after_step.rs` names one (the step,
faith regeneration, charge refill, passive abilities, the powerup clamp, a building's production and the changed tiles),
and 5.7's own sequence inside `+0x84` has not been read at that granularity.

What it changes for us: `crates/sim` still decides every entity against the tick's opening state, so the *order between
objects* is what this settles. T12's first slice takes the part the two-pass shape can honestly carry - the ordered blow
pass now runs `(player, id)` instead of by entity id, and a building dead at the end of that pass finishes no production
or research - and the per-object loop is what is left. `docs/next-step.md`, T12; D229.

## The turn, read (2026-11-16)

T16 asked what 5.7 does with `pturn`. It is read, in three places, and the three agree.

**In our own binary.** `BaseWorld::update` (`FUN_00738720`, `0x738720`) accumulates the update's elapsed milliseconds,
**per player**, in the word at `+0x104 + 4 x player` - the loop's body does
`this[player + 0x41] += this[0x1b]` for every player it walks. At the end of the update it **zeroes the word belonging to
the player whose turn it is** (`this[pturn + 0x41] = 0`), advances `pturn` (`+0x100`) by one and wraps it at the player
count. Two neighbours name the fields for us: `FUN_00735960` (`0x735960`, `baseworld.cpp:0x24b`) is the reset that clears
`+0x100` **and exactly nine words `+0x104 .. +0x124`** - Gaia plus eight players - and `FUN_0072d0b0` (`0x72d0b0`) is a
one-line getter, `*(u32 *)(this + 0x104 + 4 x player)`.

**In the DE port, which reproduces 158041 bit-for-bit.** `WorldStep::Turn` (`world/schedule.rs:274`) is the same two lines -
`*clock_mut(player_turn) = ZERO; player_turn = (player_turn + 1) % players` - and `PlayerStep::Objects`
(`schedule.rs:323`) is where the turn is **used**:

```rust
let turn = if player_id == self.player_turn { Some(*self.clock_mut(player_id)?) } else { None };
self.update_objects(player_id, (run.dt, turn))?;
```

So the turn player's objects are updated with **the time accumulated since their own last turn**, and every other player's
objects with `None`. `unit_ai` says what the two cases are (`unit_ai/unit.rs`, the `ai_run` doc): on the owner's turn the
**full update**, otherwise **"its check between turns"**. `pturn` and `player_time_delta[player_id]` are also fields of the
save's world header (`world/load/world/mod.rs:77`), so the turn is **persisted state**, not a runtime detail.

**What that means for us, and why the turn is not built.** The turn is not an order of updates - our tick already has one,
and D230 built the player rotation for it. It is a **clock**: one player's objects per tick get the full unit-AI update,
carrying the elapsed time since that player's last turn, and the rest get a cheap between-turns check. Two things our engine
does not have, and one thing it must not lose:

1. **No per-player clock.** Our simulation time is a fixed tick; there is no per-player accumulator to zero and hand out.
2. **No two-tier unit AI.** `decide` runs every entity fully, every tick, and there is no "between turns" half to run in
   the other eight ticks - inventing one is a game-behaviour decision (how fast the computer player reacts), not a reading.
3. **It is saved state.** The save loader (`P1`, `World::from_state`) will need `pturn` and the nine-word array, so the
   fields are owed by P1 whichever slice builds the AI.

**Basis.** `FUN_00738720`, `FUN_00735960`, `FUN_0072d0b0` in `AoK HD.exe`; `librematch/genie` at `62831fa`
(`world/schedule.rs`, `unit_ai/unit.rs`, `world/load/world/mod.rs`). `docs/next-step.md`, T16; D232.

## The other way out of the simultaneous volley (2026-10-31)

`Zaydme/openaok` proves a second answer to the same problem, and it is worth naming before anyone reopens the
tick. It keeps an **ordered** tick (60 Hz, 32.32 fixed point) and removes the simultaneity *by hand*, as a
recorded deviation from the original: soldiers set on the same enemy begin their first blow up to six tenths of
a second apart (`docs/IMPROVEMENTS.md`, "Volleys spread out"), pinned by a scenario that makes ten archers loose
at one target together and checks the first arrows land on a spread of ticks. It also pins the swing to its
victim at the moment the blow begins, so a scout standing between two wolves wounds the one it swings at.

So there are three positions and each is internally consistent: the original resolves blows in object order; we
resolve them simultaneously and let two units trade killing blows; openaok resolves them in order and then
deliberately desynchronises them. If U18 is ever decided by "fights look wrong" rather than by fidelity, the
spread is the cheap lever - it changes the look without changing the tick's shape or the apply pass.
