# The jump was a gait (D180)

*2026-11-08. After D177 gave the ground test the body's radius, the owner asked: **"how about villagers jumping and stacking up at the same place or
foraging from greater than allowed distance?"** Three complaints from D173, measured again before touching anything.*

## Measured first

| | test | before |
|---|---|---|
| jump | `gather_motion::a_gatherer_never_moves_further_in_a_tick_than_its_speed` | **FAILS**: *"the gatherer jumped 15600 raw units in x on tick 38, and its speed allows 1000"* |
| stacking | `gather_spread::two_woodcutters_on_one_tree_take_different_sides` | passes - D171's four sides hold |
| distance | `gather_standoff::a_woodcutter_stands_where_the_bodies_touch` | passes at **0.70** of a tile |

So one of the three was still open, and it was the one that matters most: the body **teleported**.

## The defect: speed is per second, and the settle read it raw

`settle_workers` placed an arrived worker on its standoff in one tick. The first fix - "take a step toward the standoff, capped by speed" - still
jumped **15,600 raw units**, because `entity.speed` is stored **per second** (16,000 here) while a step is per tick (÷ 20 = 800). Reading it raw
allowed a 1.56-tile step. With the division, and `step_towards` - the movement pass's own arithmetic, which never rounds a step to a fraction of a
scale unit - the worker walks the last third of a tile in two ticks.

## The second defect the first fix exposed

Writing `entity.velocity` from the settle broke a **completely different** test: `a_worker_does_not_look_beyond_its_reach` (D149). A villager fifteen
tiles from a patch walked to it anyway. `velocity` is not a report of what happened - **the simulation reads it back**, and a per-second value left
there by a one-tick step made the rest of the tick disagree about where the body was going. The settle now changes the position and nothing else.
That is worth remembering the next time something looks like it "just" records movement.

## Why the earlier four designs failed and this one does not

D173's attempts all tried to make the *route* end at the standoff - one-waypoint approach, rotation with claims, eight spots with a deadline. Each one
was a second order layer, and the acceptance run showed it: **6,118 to 19,096 repaths** against 24. This one adds no route at all: the existing route
ends on the neighbouring tile exactly as before, and the last 0.3 to 0.7 of a tile is **walked by the movement rule that already exists**, at the
speed the body already has. Nothing is planned twice.

## Acceptance

| | D177 | D180 |
|---|---|---|
| entities | 138 | **154** |
| repaths | 529 | **297** |
| made-room | 125 | 110 |
| hash | `661e970e58bc0c57` | **`834efe7c5efd3c62`** |

**Repaths fell by 44%** as a side effect: a worker that teleported to its standoff invalidated every route that had been planned around where it
*was*, so the planner was re-planning for a body that had moved without walking. Walking it is cheaper than placing it.

## Performance

**Measured**: the gate ran in **482,025 ms** (`gate102.log`) - the machine was loaded, and the suite itself grew by the un-ignored jump test; the
acceptance run's wall time is unchanged. The settle costs one `step_towards` per arriving worker per tick, and it runs only for workers that are
beside their node and not yet touching it - one or two ticks each.

## Stability

- **The jump is now in the suite, not ignored**: the `#[ignore]` is gone and the test runs with everything else.
- **The 15,600 number is recorded twice** - as the defect and as the first fix's own failure - so the "per second versus per tick" mistake is written
  down where the next person will hit it.
- `velocity` is documented in place as state the sim reads, not a log line.
- Suite: **785 passed / 0 failed / 20 ignored**, clippy `-D warnings` clean, fmt clean, `review_check` 0.
