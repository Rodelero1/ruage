# Whole steps: a corner is folded into the tick, and only a walk folds

**Status: landed** (D252). The owner asked for a fixture that villagers never slow down, verified first (D251), chose *fixture
and fix in one slice*, and then ruled the fixture must never be skipped: **no `#[ignore]`, ever, without his word.** So the fix
had to land, and the reason it first did not is the useful part of this note.

## What the rule is, and how it is measured

One step is `speed / tick_rate` raw units (800 for a villager at 20 Hz). On every tick a villager is walking, the distance he
covers **along his route** is one whole step, or zero - a refused step, a pause, a body in the way. The only short tick allowed is
the last, which lands on the goal and may not overshoot it. `crates/sim/tests/whole_steps.rs`, 2 tests, **neither ignored**.

**Along the route, not across it.** The straight line between a tick's start and end is a chord, and a corner walked *through*
inside one tick makes that chord shorter than the path walked - so two earlier versions of this fixture could not tell a fold from
a lost tick. The measure that works is how much closer to the goal *along the polyline* the body got: the legs it spent, plus the
leg from the last spent point to where it stands.

## The defect

The mover aimed at the route's next point, and `step_towards` lands exactly on a point that is within one step. Right for the goal,
wrong for a corner: the tick spent the gap to the corner - **0.011 tiles, 0.107 of a step** - and the rest of the step was lost.

`step_speed_probe`, 5 villagers, 4,000 ticks (20,000 body-ticks): **89.00% stand, 10.93% walk a whole step, 0.07% - 15 ticks -
moved between 0.11 and 0.95 of a step**, every one decided `Take` (no slide, no pause, no refusal), fourteen on one long `Move`.

## The fix

`World::fold_the_step`, called from `move_bodies` after the step and before the waypoint spend: a body that **landed on a route
point** still owes the rest of the tick's step, so the remainder is walked along the next leg - one point at a time, spending each
corner as the fold begins, refused by ground (`clearance_at < 0`) or by a body (`radius + other radius`), bounded to two legs, never
past the **last** point, and re-stamping sight after it moves.

**Only a walk folds** (`Task::Move`, `Task::Hunt`). That narrowing is not taste; it is the measurement below.

## The freeze, and what it was

The first landing attempt made the fixture green and left the acceptance match **bit-identical** (fold off 39.5 / 40.3 s, fold on
40.0 / 39.7 s, `entities=138 players=1 hash=01a0fb771ee2e75b repaths=761`) - and then starved
`research::a_trebuchet_cannot_be_trained_before_the_imperial_age`: it banked **200 of 400** food and stalled for the rest of its
400,000 ticks with **1 repath**.

Traced, not guessed: one villager (`Gather { progress 0 }`, a four-point route) alternated every tick between
**(394394, 313527)** and **(395101, 313901)** - exactly one step apart - from tick 38,000 to 400,000. On odd ticks the mover walked
him 800 toward his route; on even ticks **`settle_workers` walked him 800 back** onto his gather place's standoff. Two passes, two
aims, and the fold's extra 800 was what kept them out of each other's phase. The route never advanced, `progress` stayed 0, and the
economy produced nothing more.

So the fold is confined to the tasks whose whole contract is "walk this route" - `Move` and `Hunt` - and left out of the tasks
whose approach a second pass also steers (`Gather`, `Build`, a deposit), and out of `Attack`, whose stop distance is a range
rather than a route point. With that, `whole_steps` is green and the trebuchet fixture banks its 400 in **8.9 s**.

**The attempt before that** is also worth keeping: spending corners *before* the step makes the aim the point beyond them, which
**cuts** the corner - a walker wedged against a forest edge for **68 ticks**, `Slide` at **1 raw unit** a tick.

## Performance

Acceptance match, release, seed 7, 48,000 ticks, two interleaved runs of each setting from one binary:

| | wall | entities | players | repaths | hash |
|---|---|---|---|---|---|
| fold off | **39.5 s**, **40.3 s** | 138 | 1 | 761 (body 445, ground 316) | `01a0fb771ee2e75b` |
| fold on | **40.0 s**, **39.7 s** | 138 | 1 | 761 (body 445, ground 316) | `01a0fb771ee2e75b` |

**Bit-identical**, so the fold never fires in that script - its walks are single-leg, so no body lands on a corner inside a tick.
The 0.1% between medians is noise at this sample size and has no behaviour behind it. Where walks do have corners
(`whole_steps.rs`) the fix turns **14 lost ticks into 0**, and the walk costs the `ceil(route/step) + 1` ticks its geometry owes.

## Stability

- **Ground.** The fold's landing is checked with the mover's own rule (`clearance_at(target, radius) >= 0`), so it cannot put a
  disc inside a wall; a refused fold leaves the tick short, which the fixture allows.
- **Bodies.** It checks every body within reach of the landing (`radius + other radius`) and does not move through one; it does
  not pause or repath, because a fold is the tail of a step the mover has already won.
- **Sight.** `stamp_entity` runs on the pre-fold position, so the fold re-stamps; the bitset is an OR, so a second stamp is free.
  `vision` (7 tests) green.
- **Arrival.** The last point is never folded past, so `position == goal` still lands exactly (`a_short_walk_lands_on_its_goal`).
- **The freeze is pinned by the fixture that found it**, not by a note: `a_trebuchet_cannot_be_trained_before_the_imperial_age`
  is a live test, and it is red with the fold applied to `Gather` (measured above: 200 of 400, 400,000 ticks).
- **A test that fails in the tree and is nobody's slice here**: `army_walk::a_crowd_walking_through_a_gap_stays_on_free_ground`
  (`NoPath { from: (6, 20), to: (19, 19) }` at submit) fails **with this work reverted**, and is the open finding named in
  `docs/reviews/2026-11-16-follow-the-game-parked.md`.
- My own errors, in order: a full-suite run invalidated by swapping source under it; a fold trace whose tick-window filter
  (`tick % 1000 < 40`) printed nothing and nearly had me conclude the fold never fired; and a first fix reported as
  "bit-identical, therefore safe" when bit-identity on one script says nothing about a 400,000-tick economy fixture.
