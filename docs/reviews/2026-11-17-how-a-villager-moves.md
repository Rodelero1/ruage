# How a villager moves: the probe behind the owner's "they walk, or they stand"

**Status: a probe and three reads, no rule changed.** The owner asked for a fixture that villagers never slow down, and asked for
the game and the decompilation to be checked **before** anything is written. This is that work; the fixture is waiting on his
decision (`docs/next-step.md`, "Third question").

## What was read

- **`getMaxVelFwd`, `008a2a30`** (move_obj.cpp 3,069-3,145), read whole: it returns the unit's own dat speed
  (`unitdef+0xe4`), multiplied only by a motion-object factor (`[move+0x14]+0x6c`) when that is `> 0.0`, and **exactly 0.0 on a
  pause**. No terrain factor, no ramp, no partial-speed arm. The factor's only writer is the thiscall setter `0x7f4130`, which
  the scan shows is the sole store to that offset and has **no direct caller** - a virtual slot, so its callers are unread.
  Recorded in `docs/movement-in-the-binary.md`.
- **The decompilation** (`samuelcaldas/age-of-empires-2-decomp`, cloned and searched): **no speed, velocity, acceleration or
  max-velocity code at all**; its unit-movement and formation files are 16-47-line stubs and its only "acceleration" is camera
  drag. It cannot answer this question, and that is now written down so nobody searches again (`docs/prior-art.md`).

## What was measured

`crates/sim/tests/step_speed_probe.rs`, `--ignored --nocapture`: five villagers (four gathering, one on a long `Move`), 4,000
ticks, every body-tick bucketed by its displacement against one step (`speed / tick_rate` = 800 raw):

| bucket | body-ticks | share |
|---|---|---|
| stand | 17,800 | **89.00%** |
| walk one whole step | 2,185 | **10.93%** |
| partial (0.11-0.95 of a step) | 15 | **0.07%** |
| faster than a step | 0 | 0 |

**Cause, from the same run**: all fifteen partials are decided **`Take`** - not a slide, not a pause, not a refusal - and
fourteen of them sit on the one long `Move`, where **`path_length` decrements on every one**. A sub-step tick is a
**waypoint's landing**: the mover takes "the whole gap" when the remainder is within a step, so a waypoint costs it part of a
step. The fifteenth is an idle settle at the goal. Nothing was slower than a step for any other reason, and nothing was ever
faster.

## Performance

The probe's own cost: **4,000 ticks in 2.4 s** in a debug build, printing fifteen lines - it is an `#[ignore]`d instrument and
never runs in the gate. The number it exists for is the **0.07%** above: fifteen body-ticks in twenty thousand, each losing
between 0.05 and 0.89 of a step.

## Stability

Nothing in the engine changed, so nothing can regress; what was attacked is the *claim*. Three ways the 15 could have been
something else were checked and excluded: the decision name (`Take`, so no slide and no pause), the task (`Move`/`Idle`, so no
gather or build standoff), and the position (not a tile centre, so not the route's own corners). What is **not** established is
which line clamps the step: the doc's landing rule is the shape that fits, but the aim point is private to the world and the
probe cannot print it - the next instrument for that is an env-gated print in the mover, not a wider test accessor.

## What it does not decide

Whether the fixture lands now, and whether the landing clamp is removed in the same slice or its own. That is the owner's call,
written up as three options with costs in `docs/next-step.md`.
