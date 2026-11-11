# The picker's cap — adversarial review of D186 addendum 2

*`claimed_by_other` answered "is this source taken?" with "is anybody gathering it". That is D182's one-worker-a-square surviving
in the picker after D186 replaced it in the gather arm, and it left villagers idle beside a patch with places free.*

## What it did, measured

- A woodcutter whose patch ran dry, **four tiles** from a large patch holding **two men of nine**, ended the run with
  `Task::Idle`. Nothing within his reach was "free", because two men had made that patch count as full.
- The **eleventh** man ordered onto a square already holding **nine** - the case the cap exists for - found no other source and
  stood still, where the neighbouring square held one man of nine.

## What the fix is

`on_node >= GATHER_RING_SLOTS`, counting `Gather` and `Return` (a man carrying a load home still holds his place - the same
count `start_gather`'s cap uses). One comparison, no new state.

**Pinned by `crates/sim/tests/gather_room.rs`** (two tests) and **mutation-checked**: restoring `>= 1` turns both red with
`Task::Idle` in the message. The test file also carries the first passing guard for the "next source" path, which had none.

## Performance

**The acceptance match** (`config/acceptance.toml`, seed 7, 36,000 ticks = 1,800 s at 20 Hz), rebuilt binary in both cases:

| | before (D186 addendum 1) | after |
|---|---|---|
| hash | `caad24320e6900f0` | **`08a111f13b7c43ec`** |
| entities at 1,800 s | 136 | **150** |
| repaths | 528 | **352** |
| made_room | 158 | **226** |

The match moved, so these are two different matches rather than a like-for-like tick - but the direction is the point:
**repaths fell 33%** and fourteen more entities were alive at the end, which is what stops happening when a villager joins a
patch instead of standing beside it and being re-planned over and over.

**The tick** (`step_cost`, 128 entities): `World::step` **6.8 us**, `Snapshot::of` **91.8 us** - the tick is unchanged from the
6.8 us of the D186 baseline, and the snapshot is inside the 91.8-101.6 us spread this example has shown on this box.

**My own error, again, and the same one as earlier in this work.** The first acceptance run of *this* change reported the exact
hash of the *previous* build, and I nearly wrote "inert on this scenario". The binary under `target/release/ruage-headless` had
not been rebuilt - `cargo run --example step_cost` compiles the library and the example, not the headless bin. Every number above
comes from a binary built after the change, and the rule that would have prevented it is now in the skill: **rebuild the thing
you are about to measure, by name.**

## Stability

- **Determinism untouched**: one integer comparison; no floating point, no new state, no new ordering.
- **Two `#[ignore]`s removed, none added.** D173's ignored test (a gatherer never moves further in a tick than its speed) passes
  and is un-ignored; an ignore that no longer describes anything is a lie in the suite.
- **The picker now calls `can_reach` for candidates it used to skip**, bounded as before by `truncate(8)`, and only on the
  "next source" path - not per tick.
- **Not covered**: a ring buried in a forest has about eight usable places (the neighbouring tiles), not nine, so the cap can
  over-subscribe by one there; the tenth man finds no slot and stands. Rare, and it fails to *idle*, not to crash.
