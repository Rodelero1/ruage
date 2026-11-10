# The jump is a placement, and two walks at fixing it failed (D173 - OPEN)

*2026-11-08. The owner, after D169-D172, with three screenshots: *"villagers tend to jump towards the bush (I noticed with tree, too), pile up on each
other, forage from greater distance, appear and disappear in rapid succession... This is regression of previous step."*

**Status: this step fixes nothing in the engine and says so.** `main` is where it was (`004b704`); what is new is a reproducer, two guards, and the
record of two designs that were tried, measured and thrown away. I would rather leave that on the table than ship a third guess.

## What is reproduced, and what is not

| report | reproduced? | evidence |
|---|---|---|
| **jump towards the bush** | **yes** | `gather_motion::a_gatherer_never_moves_further_in_a_tick_than_its_speed` - the gatherer moved **15,600 raw units (0.6 of a tile) in one tick**, where its speed allows 1,000. It is `settle_workers` (D169), a placement. |
| pile up on each other | **no** | `three_gatherers_on_one_bush_keep_their_own_places`: three villagers on one bush end ≥ 0.5 tiles apart and ≤ 0.8 from it. Passes at `004b704`. Passes too with the placement removed, so this test cannot tell them apart. |
| forage from a distance | **no** | same test measures ≤ 0.8 of a tile for all three. The owner's picture is a **berry patch**, where most sides of a bush are blocked by other bushes - not covered by this fixture yet, which is the next thing to reproduce. |
| appear and disappear | **no** | `crates/view/tests/gather_visible.rs` renders 120 consecutive ticks of a working villager and finds its colour in every frame, and never missing from the snapshot. A single-worker case does not show it; the report is of three. |

## Two walks, both rejected, with numbers

1. **Make the walk itself end at the standoff** (`start_gather` plans to the touching point). This was D169's first attempt, which failed because
   nodes stood on tile **corners** (D172). With the corners fixed it works for one worker - and fails for three: a villager whose chosen side is
   taken by the time it arrives **stops 1.57 tiles out and never returns**, because at that distance "beside the node" no longer holds and the
   blocked tile refuses every step. Reproduced by
   `three_gatherers_on_one_bush_keep_their_own_places` with that change in place.
2. **Step toward the standoff every tick while working.** No jump, no placement - and it costs the acceptance run **225 repaths against 24** with the
   placement, because the standoff (and a reachability search) is recomputed per working gatherer per tick, and bodies in each other's way earn a
   repath each time. The repo's own rule is that a plan search per worker per tick is what the acceptance run's slow ticks are made of.

A third shape - a **one-waypoint approach route** earned on arrival, re-checked each tick - is half-built and was abandoned when it produced the same
1.57-tile stranding as (1). It is the most promising of the three, and it needs the "beside the node" rule widened to cover a worker pushed a tile and
a half out by its neighbours.

## What I got wrong in this step, said plainly

I built (2) and read its acceptance numbers as a success - 155 entities against 153 - without looking at **repaths 225 against 24** until the run after.
That is the same mistake as the D165 count error: I quoted the number that looked good and not the one that was bad. The counters are in the table above
so the next session starts from them.

## Guards added that do pass

- `gather_motion::a_settled_gatherer_does_not_jitter` - a settled worker's position does not change for 60 ticks. It passes **both** with and without
  the "nearest side wins" ordering, so it is a guard and not proof of that ordering; said here rather than implied.
- `gather_motion::three_gatherers_on_one_bush_keep_their_own_places` - three workers, one bush, no stacking, all within 0.8.
- `crates/view/tests/gather_visible.rs` - 120 ticks, the marker is drawn on every one.
- `gather_motion::a_gatherer_never_moves_further_in_a_tick_than_its_speed` - **ignored**, and labelled: it is the reproducer for the jump.

## Performance

No engine change, so the acceptance hash is unchanged (`6d23d672eb18ef40`) and nothing is slower. **Measured**: the new sim tests add **1.5 s** to the
suite (`gather_motion` 1.5 s for three tests) and the view test adds **3.2 s** (`gather_visible`, 120 ticks each rendered and scanned); the gate's wall
clock is recorded with this commit. The rejected design (b) is the one performance number worth remembering: **225 repaths against 24**.

## Stability

- **Nothing in the engine changed, so nothing can regress** - the risk this step carries is zero, and the cost is that the owner's jump is still there.
- **The reproducer is labelled, not deleted**: an `#[ignore]` with the reason and a pointer to the review, so the next session starts from a failing
  measurement rather than from my summary of one.
- **Three guards now run on every gate**: a settled worker does not jitter, three workers on one bush keep their own places, and a working villager is
  drawn on all 120 ticks of a render loop.
- Suite: **777 passed / 0 failed / 20 ignored** (the extra ignore is the reproducer), clippy `-D warnings` clean, fmt clean, `review_check` 0.

## Addendum: four designs tried for the walk, all measured, none shipped

*The same day, after the owner chose "teach it to walk" (option A). Each was built far enough to be measured, then reverted.*

1. **Make the gather order's route end at the standoff.** With nodes on tile centres this works for one worker. With three, a worker whose
   chosen side is taken by the time it arrives **stops 1.57 tiles out and never returns**: at that distance it is no longer "beside the node",
   and every step toward the node's blocked tile is refused.
2. **Step toward the standoff every tick while working.** No jump at all - and the acceptance run's **repaths go from 24 to 225**, because the
   standoff *and* a `can_reach` path search are recomputed per working gatherer per tick.
3. **A one-waypoint approach route, earned on arrival.** This is the design that looked right, and it deadlocked in a way worth recording: the
   gather order's **own route still ended on the node's blocked tile**, so `next_waypoint` returned a point no step could ever reach, the route
   never finished, and the approach was never earned. Measured on three villagers: `at_node=true routing=true ways=1 next=(10.50,6.50)
   walkable=false`, stationary for the rest of the run. Dropping a route whose last waypoint sits on the node's own blocked tile fixed that and
   got all three to **0.86** of the bush - and then two of them settled **0.42** apart.
4. **Rotation per worker (`id % 4`) plus claims on spots other workers are walking to.** The rotation removed the jam, and the claims took the
   crowding the wrong way: the third worker was pushed out to **1.16 tiles**, because its own side was spoken for and every other side was too.

**What that says**: the placement is load-bearing. With a node's tile **fully blocked**, the walk cannot end where the bodies touch, and every
workaround trades one of the owner's four complaints for another. The real fix is the one design that has not been tried: **a node as a body of
its content radius (0.5) rather than a blocked tile**, with the *planner* still avoiding its tile - so a walk can end at 0.7 because 0.7 is real
geometry, not an exception. That is a step of its own, not a patch.
