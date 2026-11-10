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

## Addendum 2: the owner's rule, measured, and where the tile-blocked model stops

*The owner then specified the model exactly: a tile holding gold, wood, stone or berries **fills the tile**, a villager **approaches it by
touching it with its circle**, and scattered round trees come later. That is the model we already have - and measured against his words it shows
where the wall is.*

**The geometry, written down**: a worker's circle (radius 0.2) touches a **tile** (half-extent 0.5) at `0.5 + 0.2 = 0.70` on an axis, and at
`0.5 + 0.2 / sqrt(2) = 0.641` in each coordinate on a **diagonal**, which is `0.907` of a tile from the node's centre. Those are **eight places**
round a blocked tile - four axes, four corners - and every one of them is *outside* the blocked tile, so every one is ground a worker may stand on.
Four sides was a shortage (three villagers on one bush fought over four spots). A diagonal spot is *not* at `0.5 + reach`: using the sum of the
radii there puts the worker **1.41 tiles** out, a full tile away, which is the complaint this step is about.

**What the eight spots fixed**: three villagers on one bush, all three touching the tile (0.70 axis, 0.91 diagonal), no stranding, and the jump
gone - the walk ends at the spot because the spot is real ground. `gather_motion` proves it: the per-tick movement test passes, so there is no
placement left to see.

**What it broke, and why that is the real answer**: a **walkable** node - a carcass, a felled stump - is a node a worker stands *on*, and with the
placement gone nothing keeps two workers from standing on the same point: measured, `both_workers_on_one_body_take_from_it` ends with the two
**0 raw units apart**. The placement was doing that job too, and the collision will not do it for a body at rest.

So the honest conclusion stands, and it is now specific: **with a node's tile fully blocked, the spot that touches it is not a tile centre, and
nothing in the movement rules may end a walk there.** Every version of this either places the worker (the jump) or leaves it a tile out. The step
that ends it is not another patch on the gather arm: it is **walks that may end at a contact point**, plus a rule for a walkable node that keeps a
second worker off the first's spot. Both are movement work, and both deserve their own tests. `main` is left as it was - `cdb8ee4`, the placement,
the jump, and a gather that otherwise does what the owner asked.

## Addendum 3: the walk-to-the-spot design, measured - and it is not shippable

*After the owner said "proceed", the design the previous addendum called promising was built: eight spots round the tile, a claim per worker,
a one-waypoint route to the spot, a two-second deadline, a fallback that works from where the worker stands. It is correct by the tests and
**unusable by the counters**.*

**What it got right**: the jump is gone (`a_gatherer_never_moves_further_in_a_tick_than_its_speed` passes), three villagers on one bush stand at
0.70 / 0.91 - touching, as the owner defines it - and nobody is stranded. Every targeted test green, 780 passing.

**What it did to the acceptance run**, in three measured steps:

| variant | repaths | entities |
|---|---|---|
| shipped (`cdb8ee4`) | **24** | 153 |
| spot chosen in the arm, order walks to the tile centre | **6,118** | 155 |
| spot claimed at order time, order plans to the spot | **10,115** | 146 |
| order back to the tile centre, claim at order time | **19,096** | 134 |

Roughly **three orders of magnitude** of re-planning for a third of a tile. The cause is structural rather than a bug: a worker that must reach a
point it can only sometimes touch **enters and leaves "touching" as its neighbours press on it**, and every cycle is a fresh route, a fresh claim
and a fresh set of collision re-plans. The placement never had that problem because it was atomic - one write, no route.

**So the conclusion is now firmer than a guess**: the last third of a tile must **not** be bought with extra routes. It has to be a property of
the movement and arrival layers - a walk whose *destination may be a contact point*, and a body that understands it is standing at its work
without being re-tasked. Anything that answers it with another order will keep costing thousands of re-plans.

`main` stays at `cdb8ee4`: the placement, the jump, and a gather that is otherwise what the owner asked for.
