# 2026-11-22 - D348: the levelled search's window, and what our tier actually stands in for

A read, not a slice: no line of `crates/` changed. The deliverable is the window's value, the mover's ladder, and the
correction of what three landed decisions had assumed about the mechanism they measured.

## Performance

Two numbers, both already on the tree, now attributed correctly:

- **The tier's remaining gap is +2.8% ground** on the same 92 walks (`TierShadow`: 637 waypoints / 65,242,638 raw against the
  flat search's 515 / 63,486,927), and D348 says what it is: the coarse **guide's** shape, not a cost-model defect.
- **The levelled search's window is one cell**: half-extent `radius_sum + margin`, and both real callers pass `margin = 0`
  (order-level `FUN_008000d0` via `FUN_00528650:63`; the mover's `path+0x20`, which `FUN_007fd4b0` initialises to 0). A
  search clipped to one cell cannot answer a 56-tile walk, which is why the binary's long routes come from the waypoint path
  plus the straight-line/tile ladder.

No timing was taken for this note because nothing executable changed; the acceptance figures it rests on are D347's
(`7782dd38d7557514`, 134 entities, 12.12 s median of three) and the probe's (`tier_probe`, release).

## Stability

Nothing to attack - no code. What was attacked is the *argument*, and this is how:

- **The window's margin**, traced through three layers: `FUN_007f0be0` reads it as `param_3 + param_4`; `FUN_007ee980`
  forwards its own `param_6/param_7`; `FUN_008000d0` forwards its 9th argument. The two call sites that matter were read
  rather than assumed: `FUN_00528650:63` passes `0`, and `FUN_008a4360:808` passes `path+0x20`, whose allocator
  (`FUN_007fd4b0`) writes 0 at `+0x20` (`+0x28` count, `+0x2c` capacity, `+0x30` array).
- **The ladder**, from the mover's own log strings in order (`0xf56` mid-level, `0xfef` straight line, `0x1026` tile-based) -
  the strings are the evidence, not an inference from call order.
- **The counter-hypothesis**: that the levelled search *is* the long-distance planner (openage's note). It survives only if
  some caller passes a large margin; `FUN_008003a0` and `FUN_00801aa0` forward a caller margin (one adds `3.0`), so the API
  allows it - but neither of the two paths a villager's walk takes does. That is stated in the row as the residual
  uncertainty, not resolved away.

**My errors, in the same breath:**

1. **Three decisions (D344, D346, D347) were measured against a mechanism I had mis-sized.** I called `FUN_007ee980` "the
   levelled search for long walks" when its window makes it a one-cell refinement; the numbers those rows publish are correct
   and their *attribution* is what D348 corrects.
2. My first reading of the window (in D347's review) called the margin "unread padding" and left it; the allocator's
   initialiser was two functions away and would have settled it then.
3. The decomp's argument mapping is unreliable for `FUN_008000d0`'s 15-parameter call, so the "9th argument" attribution is
   the weakest link in the chain - it is named as such rather than presented as certain.

**Not attacked**: which routine builds the high-level path (the next read), and whether the high-level path's granularity is
8 tiles or something else.
