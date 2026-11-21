# Review: the skirmish was one-sided because its clock was stale (D315)

*Change: one number in `config/skirmish.toml` - player 2's attack moves from **230 s to 600 s**. No engine code, no assertion
touched. The suite goes green: **927 passed / 0 failed / 21 ignored**.*

## What was wrong

Both reds' messages named the same thing - `player 1 has no unit militia` - and the step list said where the militia should have
come from:

```
500.0s  Refused("player 1 has no finished barracks")
525.0s  Refused("player 1 has no unit militia")
```

So the talk was about the *barracks*, not the militia. Instrumented (a probe printing each side's possessions, the foundation's
progress and villager 2's health every ten seconds):

```
t=320s sites=[]                                    villager 2 hp 40
t=340s sites=["Some(12) progress Some((119, 1000))"] villager 2 GONE (killed)
...
t=680s sites=["Some(12) progress Some((119, 1000))"]  (still 119 of 1000)
```

**The barracks froze at 119 of its 1000 build ticks** because its builder was dead: player 2's two militia were attacking from
**230 s**, and villager 2's health read 40 → 37 → 31 → 25 between ticks 260 s and 310 s before it vanished. Player 1 never fielded
anything, so the 525 s attack step could only refuse.

**The scenario's own clock was stale, and it says so itself**: its comments record timings measured under earlier movement and
economy rules ("a script that only works at the exact tick is a script that fails on the next rate change"). Measured on this tree
with player 2's attack held back: player 1's barracks site at **320 s**, **422 of 1000** at 350 s, finished before **400 s**, and
militia in hand at **525 s**. Player 2's barracks stands at 70 s and its militia exist by 230 s - a five-minute head start in a
mirrored opening that was never evenly timed.

## What landed

`config/skirmish.toml`: player 2's attack at **600 s**, with the measurement in a comment beside it. Everything else - both
barracks, both training orders, both gather schedules - is untouched, because player 1's timeline is sound on its own.

**Not touched, and worth naming:** player 2's opening is still much faster than player 1's (barracks 70 s against 320 s). It no
longer decides the test, but "mirrored" is the file's own word for that section and the mirror is not even. Making it even is a
content decision with a balance question behind it, not a fix.

## Test-first

The witnesses were already red and were **watched failing** with the messages above; they are green now with no assertion
changed. The property they check is the content's ("both sides field an army"), so the content is what was wrong.

## Performance

- **Acceptance match, release, seed 7, 48,000 ticks**: **9.54 s / 9.21 s**, **bit-identical** state `89ff37d844cfd11f`
  (129 entities, 10,026 repaths) against `e50b1bd`. Expected: `config/skirmish.toml` is not in the acceptance pack, and nothing
  else changed - so this is the check that the change is inert outside its scenario, not a measurement of it.
- Full workspace suite: **927 passed / 0 failed / 21 ignored** - the **first fully green run** of this session, from
  **925 / 2 / 21**. Nothing joined and both skirmish reds left.
- clippy clean at `-D warnings`, `fmt --check` clean.

## Stability

- The change is a **constraint on a scenario**, not on the engine: no state, no determinism, no per-tick cost. `the_skirmish_is
  _deterministic` still passes (two runs, one hash).
- **The fixture can still fail**: it failed for six weeks for exactly the reason it names, and the probe above is what proved the
  cause rather than the symptom. A future economy change that slows player 1 past ~600 s will redden it again, which is the
  fixture doing its job.
- Known and accepted: the fight is now 300 s long (600 → 900) rather than 670 s, so `the_two_sides_fight` has less time to see
  damage - it passes, and the margin is the army's walk plus the first blows (both armies are on the map at 600 s).
