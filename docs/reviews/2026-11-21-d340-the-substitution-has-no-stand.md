# D340 - step 5 (waypoint substitution): read to the end, not landed, and why

**No engine line changed.** The step was opened as "a mover whose waypoint cannot be reached re-plans for ever, a stuck
unit with a live order", and the mechanism was read out of the game (`FUN_008a4360`, counter `+0x148`, threshold `> 2`,
*"faking it"*). What the reading and two probes then showed is that **our mover has almost no stand for the rule to
answer**, and that the pathology the step was written for is a different one. This note is the evidence; the decision
is D340.

## What was read

- `FUN_008a4360` (`move_obj.cpp`, HD 5.7): three branches on the reach test - reached (`0x11e7`), will reach next
  update (`0x11f7`), and "neither, and `2 < count`" (`0x1203`), which jumps to the **same** advance label as the
  second. So substituting *is* the deferred waypoint advance, on the third consecutive failure.
- `FUN_007ff650(bit, set)` - the flag writer that arm calls - sets a bit in a **path** object's word at `+0x30`, and
  walks to the parent path when the bit is 2. No position is written; the "fake" is an index advance, nothing more.
- `FUN_007f4330` (`motion.cpp:0x4d6`/`0x4e2`), the same loop in the older file, has **no** substitute arm.
- Independent kind of source: Rich Geldreich (Forgotten Empires), *On Age DE's pathing/movement* (2018-02-28) -
  *"Waypoints along a path can be skipped if a unit can safely move from its current position to the next waypoint"*;
  Age1's units *"would sometimes cause units to oscillate around their current waypoint until it figured out it was ok
  to go to the next one"*.

## Why it does not land

**A body never refuses our mover's step.** `refusal()` returns `WaitTarget` or `WaitGround`; for a body it adds a
`Clear` box, pauses the higher id and takes the step (D279 rule 4, D281). `StepDecision::WaitBlocked` is constructed
nowhere in `crates/` - `grep -rn WaitBlocked crates/sim/src` finds only the `is_a_wait` list. So the counter that would
drive substitution (`stalled`, `motion.rs:150-160`) only ever counts **ground** refusals, and the tree's single
substitution is `replan_after_ground`'s off-grid fallback at `REPLAN_TICKS` (20) - our constant, not the game's.

**Measured, both scenes in `crates/sim/tests/substitute_probe.rs`:**

| scene | re-plans | longest stand |
|---|---|---|
| a walker catching a slower villager beside the seeded base (200 ticks) | **196** | **0** ticks |
| the same catch-up in a clear lane (260 ticks) | **0** | 0 ticks |

The first is the pathology the step named - a re-plan **every tick** - and the walker is *walking* throughout it, so the
game's predicate ("won't reach it this update or the next") is false and a waypoint substitution could not have fired.
Two further scenes were staged and thrown away: a move order at a building ends on **contact** in ~50 ticks, and a
builder beside its site is *adjacent* and working, which is a legitimate zero delta (the earlier reading of that as a
frozen builder was wrong, and is corrected here).

**The threshold was drafted and dropped.** Moving the off-grid fallback from 20 to the game's 3 is the only part of the
rule with material, and no fixture in hand produces a walk that stands on an **off-grid** aim for three consecutive
ticks: the sampled stalls are 1-19 ticks. A constant changed without a witness is exactly what SPEC §11 forbids, so it
was not changed.

## Performance

Nothing to measure on an unchanged engine, so the numbers are the ones the next step must move: the acceptance match
runs **40,127 re-paths over 48,000 ticks** (D339, `36ceb342bd94acc9`, 134 entities, wall 15.93 s median of three), and
the base-side probe above attributes 196 of them to **200 ticks of one walker's** contact with one body - a re-plan per
tick of contact, each a whole-map search. `substitute_probe` runs in 0.78 s at debug, so re-running it costs nothing.

## Stability

- Suite on this tree: **217 result lines, 936 passed, 0 failed, 21 ignored** (`cargo test --workspace --no-fail-fast`,
  exit line read; baseline log `suite-step5-baseline.log`).
- No production code touched, no fixture re-pointed, no `#[ignore]` added: the only new file is an ignored probe that
  prints. `cargo fmt`/`clippy` unaffected by a test-only file, and both were run before the commit.
- Attacked: the rule itself (three arms, two files, the flag writer), the assumption that a body refuses a step (it
  does not - the enum variant is dead), the assumption that a re-plan means a stand (the probe separates them), and the
  first two fixture designs (both measured scenes the engine does not produce).
