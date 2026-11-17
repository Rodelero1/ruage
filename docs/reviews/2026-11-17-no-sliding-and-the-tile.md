# No sliding: the rule holds, and the tile is what stops it landing

**Status: measured, not landed.** The owner corrected the rule and the direction in one message — *"man stands or walks. No
acceleration or deceleration"* (not "one whole step") and *"follow the game. No sliding."* The slide was deleted and the rule's
own fixture went green; fourteen fixtures went red on a single, named refusal. The engine is unchanged and the next slice is
identified precisely.

## What the rule is, in the owner's words

Constant speed: a man moves at his speed, or he does not move; nothing ramps up or down. "One whole step a tick" is only the
observable form of that at a fixed tick rate, which is why `whole_steps.rs` measures it — but the rule is about speed, and the
fixture's doc comment now quotes him rather than paraphrasing.

## The attempt

1. **The fixture, first.** `a_corner_blocked_past_it_is_stood_on`: the same walk as the whole-step test, with a body parked half a
   tile past the first corner. **Red** on the shipped code — 82, 81, 80… raw units a tick, **567 of 1,398** walking ticks short.
2. **The slide, deleted** (the original applies its step whole and answers a collision with a repath or a pause,
   `docs/movement-in-the-binary.md`). The fixture goes **green**: every walking tick is one step or a stand, and the walk still
   costs the ticks its geometry owes.
3. **The suite** (200 binaries, `--no-fail-fast`): **192 green, 14 red across 8 binaries** — the animals and gathering family
   first, then three fight fixtures, a line upgrade and the ten-minute scenario.

## The one refusal behind all of it

A gatherer ordered onto a sheep stops at **(316980, 316980)**: `WaitGround` from tick 40, `stalled` climbing to **267**, its route
unchanged. The step is refused by the disc test for a **164 raw unit overlap — 0.008 of a tile**. Raising the ground re-plan
budget from 2 to 40 changed nothing, because the pathfinder returns the same route: the route is right, and the *step test* is the
one that says no.

## Why no threshold closes it

- Allow that 0.008-tile brush — which is what the game does, since its collision is against an **object**, and a body standing
  against a trunk or a wall face brushes the tile — and `a_villager_walking_beside_a_building_keeps_its_body_clear` fails: it
  demands `body_clearance >= 0` on **every** tick.
- Refuse it, and the sheep fixture fails.

Neither fixture is wrong. Ours marks a **whole tile** blocked when anything stands in it; the game collides against an object with
bounds. So the missing piece is the **model**: a blocked tile must know what fills it — a tree's trunk is not a tile, a building's
footprint is exactly its tiles — and then one measure serves both, the slide has nothing to hide, and "no acceleration or
deceleration" is true everywhere rather than on open ground.

## Performance

- The corner fixture's own number, on the shipped mover: **567 of 1,398** walking ticks at a fraction of a step, **82, 81, 80…**
  raw units where a step is 800 - i.e. 0.107 of a step at the worst tick, and **82 / 800 = 10.1%** of the speed the man should
  be walking at.
- With the slide deleted: **0** short ticks, and the walk takes the `ceil(route/step) + 1` ticks its geometry owes — no cost, one
  mechanism removed.
- Suite wall time with and without the slide: within the same noise band as the D253 A/B (**44.4 s** against **39.6 s** on the
  same setting, `d253-suite.log` / `d252-ab.log`), so no CPU claim is made either way. The behaviour claim is the one that
  matters: 14 fixtures notice.
- Wall time of the two fixtures that decide the question, on this box: the corner fixture **1.0 s**, the animals binary **9.6 s**
  with the slide removed and **10.4 s** with it in place - i.e. the mover is not slower without the slide; it simply refuses.

## Stability

- **No new crash, hang or panic** in any of the three runs; the failures are assertions, and each has a named cause.
- **Determinism** holds: the same fixture fails at the same tick and the same count on every run.
- **What is NOT safe to ship**: a tree with the slide removed and this tile model, because the gather/build approach stops 0.008 of
  a tile short and never re-plans its way out. That is a freeze for a working economy — measured, `stalled` 267 — which is exactly
  the class of defect this project refuses to land.
- My own errors, again: I quoted the corpus of failures from a running suite twice before it finished (seven, then fourteen), and I
  reported the D254 count as "seven fixtures, four binaries" when the finished run said fourteen across eight. Both are corrected
  in D254, D255 and `docs/next-step.md`.
