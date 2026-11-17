# Whole steps: the rule is pinned, the fix is parked

**Status: the fixture landed; the fix is parked as a patch** (`docs/reviews/2026-11-17-fold-the-step-parked.patch`, 148 lines,
two files, D252). The owner asked for a fixture that villagers never slow down, verified first and then ruled: fixture and fix in
one slice. The fixture is landed; the fix is not landable on to-night's evidence, and the reason is measured below.

## What the rule is, and how it is measured

One step is `speed / tick_rate` raw units (800 for a villager at 20 Hz). On every tick a villager is walking, the distance he
covers **along his route** is one whole step, or zero - a refused step, a pause, a body in the way. The only short tick allowed is
the last, which lands on the goal and may not overshoot it. `crates/sim/tests/whole_steps.rs`, 2 tests, the walk one `#[ignore]`d
with the measurement in its message.

**Along the route, not across it.** The straight line between a tick's start and end is a chord, and a corner walked *through*
inside one tick makes that chord shorter than the path walked - so two earlier versions of this fixture could not tell a fold from
a lost tick. The measure that works is how much closer to the goal *along the polyline* the body got: the legs it spent, plus the
leg from the last spent point to where it stands.

## The defect

The mover aimed at the route's next point, and `step_towards` lands exactly on a point that is within one step. Right for the goal,
wrong for a corner: the tick spent the gap to the corner - **0.011 tiles, 0.107 of a step** - and the rest of the step was lost.

`step_speed_probe`, 5 villagers, 4,000 ticks (20,000 body-ticks): **89.00% stand, 10.93% walk a whole step, 0.07% - 15 ticks -
moved between 0.11 and 0.95 of a step**, every one decided `Take` (no slide, no pause, no refusal), and fourteen of them on one
long `Move` with `path_length` decrementing on each.

## The fix that is parked, and what it cost

`World::fold_the_step`: a body that **landed on a route point** walks the rest of that tick's step along the next leg - one point
at a time, spending each corner as the fold begins, refused by ground (`clearance_at < 0`) or by a body (`radius + other radius`),
bounded to two legs, never past the **last** point, and re-stamping sight (`stamp_entity`) after it moves.

It makes the fixture green - **0 short ticks, and the walk takes the `ceil(route/step) + 1` ticks its geometry owes** - and the
acceptance match is **bit-identical** with it in place (fold off 39.5 / 40.3 s against fold on 40.0 / 39.7 s,
`entities=138 players=1 hash=01a0fb771ee2e75b repaths=761`). Both of those are true and neither is enough.

**It freezes a gatherer.** `research::a_trebuchet_cannot_be_trained_before_the_imperial_age` banks **200 of 400** food, then
stalls for the rest of its **400,000** ticks with **1 repath**; a state dump shows one body holding a four-point route with
`Gather { progress 0 }` and a position that never changes from **tick 100,000 to 400,000**. The same binary passes that test in
**7.7 s** with the fold disabled - so the fold is the cause, and *how* it produces a body that can neither move nor re-plan is
**unread**. That is the line the tree draws (D252, and the movement reference's "when the red set moves, the premise is wrong"):
a movement slice whose consequences cannot be explained is parked on a patch with its numbers, not landed.

**The first attempt is in the same file's history**: spending corners *before* the step makes the aim the point beyond them, which
**cuts** the corner - a walker wedged against a forest edge for **68 ticks**, `Slide` at **1 raw unit** a tick.

## Performance

Measured, so the next reader does not have to take it again:

| | wall | entities | players | repaths | hash |
|---|---|---|---|---|---|
| fold off | **39.5 s**, **40.3 s** | 138 | 1 | 761 (body 445, ground 316) | `01a0fb771ee2e75b` |
| fold on | **40.0 s**, **39.7 s** | 138 | 1 | 761 (body 445, ground 316) | `01a0fb771ee2e75b` |

Two interleaved runs each, one binary, an env return inside the fold. **Bit-identical**, so the fold never fires in that script;
the medians differ by 0.1%, which is noise at this sample size and has no behaviour behind it. The cost that matters is not on
this table: it is the **400,000 ticks** of a fixture that never reaches its bank.

## Stability

- **Ground.** The fold checks its landing with the mover's own rule (`clearance_at(target, radius) >= 0`), so it cannot put a
  disc inside a wall; a refused fold leaves the tick short, which the old behaviour allows.
- **Bodies.** It checks every body within reach of the landing (`radius + other radius`) and does not move through one; it does
  not pause or repath, because a fold is the tail of a step the mover has already won.
- **Sight.** `stamp_entity` runs on the pre-fold position, so the fold re-stamps; the bitset is an OR, so a second stamp is free.
  `vision` (7 tests) green.
- **Arrival** is untouched: the last point is never folded past, so `position == goal` still lands exactly
  (`a_short_walk_lands_on_its_goal`, green in the landed tree).
- **What is NOT explained, and is the reason for parking**: one frozen gatherer in a 400,000-tick fixture. The dump is in the
  patch's review message; the next probe is a per-tick trace of that body's `last_step`, its aim and its clearance from the first
  tick it stops moving.
- **A test that fails in the tree and is nobody's slice here**: `army_walk::a_crowd_walking_through_a_gap_stays_on_free_ground`
  (`NoPath { from: (6, 20), to: (19, 19) }` at submit) fails **with this work reverted**, and is the open finding named in
  `docs/reviews/2026-11-16-follow-the-game-parked.md`.
- My own errors: a first full-suite run was invalidated by swapping source under it, and the first trace of the fold used a
  tick-window filter (`tick % 1000 < 40`) that printed nothing and nearly had me conclude the fold never fired. Both are why the
  numbers above are from single-binary A/Bs.
