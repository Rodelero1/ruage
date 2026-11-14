# D207 - a group walks over itself; everyone else is solid

**Owner, 2026-11-14:** *"OK. 1 now and 2 later"* - orders submitted in the **same tick** form the group while the engine has
no selection; an explicit group id on the command replaces the heuristic later.

## Why this, and why it replaces the last three attempts

The binary read of 2026-11-14 (dispatched after the owner asked what the original actually does) settled a question three
engine slices had been guessing at:

- The **path search never reads the route's ignore list**. `FUN_008007f0` takes the path as *output*; its only body-aware
  input is a set built from the mover's own fields - its own id, its target's id, and its **selection mates** (the other
  units ordered with it), kept only while they are moving (`+0x150`/`+0x2a0`, verified at `FUN_008007f0:139-140`). It is
  consulted as **exact tile membership**, no radius, no cost penalty (`FUN_00801a20`).
- Every other body is **invisible to the plan**. The collision is classified at run time by `FUN_007fe010`: if the body is
  on the route's exemption list the collider is *dropped entirely* - no pause, no repath, no bound; if not, the loser of the
  id comparison pauses 0.25-1.50 s and repaths.
- The exemption list is filled **after** the search, from the bodies the search saw within a 3x3 of each waypoint
  (`FUN_007ffd70` -> `+0x2db42c`, drained by `FUN_007fffe0`).

Forgotten Empires says the same thing in public, on their own bug forum (*Investigations into Pathfinding*, 2026): *"units
give up their formation (and therefore block each other) when: stop button is pressed / click on enemy unit / attack move.
So this actually looks like intended behaviour right now."* And, on units apparently ghosting through a line: *"Military
units moving in groups can go over each other."*

So the rule the engine was missing is not "route round a body" (D200's re-plan attempt), not "push it aside" (the nudge),
and not "let everybody overlap" (D189): it is **formation membership**. The last three slices each tried to answer a collision
with a *geometry* rule, and each ended as an exemption list of its own - measured in the branches `d205-pause` (33 red, then
18, then 31 as the mechanism moved) and the registered attempt before it.

## What was built

- `Route.mates: BTreeSet<EntityId>` - the men this unit was ordered with, hashed (two runs that disagree about a group
  diverge the moment a collision happens).
- `World.ordered_now: BTreeSet<EntityId>` - scratch, emptied at the end of every `drain_orders`; every `Move`, `WalkTo`,
  `AttackMove` and `Patrol` that lands in the tick joins it, and `stamp_group` then writes the tick's set into each member's
  route. A lone order leaves its unit with **no** mates.
- `World::is_mate(a, b)` - **mutual**: each must have the other on its route. Mutuality is the fidelity: a man whose order
  was cancelled has no route, so his old mates stop walking over him the same tick - which is exactly the developer's list
  ("stop button is pressed").
- `collide` rule 1b - if the blocker is a live mate, the step is **taken** (`TakeOverlapping`): no reach, no wait, no
  repath. That is the original's "drop the collider entirely", not an approximation of it.
- A re-plan **preserves** `mates`: the order did not change, only its route.

## The red run, verbatim, before any production change

```
test a_mate_who_is_stopped_becomes_solid ... ok
test two_units_ordered_apart_stay_solid ... ok
test a_group_ordered_together_walks_over_each_other ... FAILED
MEASURED the closest the group came was 8000 raw (0.4 tiles), and a solo walker is held to 8000 raw: a group that never
gets closer than the reach never walks over itself
test result: FAILED. 2 passed; 1 failed
```

The two guards were green on `main` before the change and stayed green after it - they are the half of the rule that stops
the exemption widening into D189's walk-through-everybody. Only the group case was red, which is the only case this slice
changes.

After: `3 passed; 0 failed` in 1.97 s.

## Acceptance: byte-identical, and that is the finding

`--seed 7 --ticks 48000 --report 400`, release:

```
tick=48000 entities=160 players=2 hash=c3b10f4b09e17a43 repaths=324 made_room=147 repaths_body=83 repaths_ground=241
  400.0s wood  80 food  45 ...   800.0s wood  96 food  49 ...  1200.0s wood 135 food 268 ...
 1600.0s wood  75 food 120 ...  2000.0s wood 395 food 660 ...  2400.0s wood 185 food 450 ...  units 20/31/35/44/46/48
```

`main` (D204): the **same hash and the same every number**. The acceptance script issues one order per tick - one villager
at a time - so it never forms a group, and D207 is inert on it by construction. Said plainly: **the acceptance match cannot
be the evidence for this rule**, and the three tests are. The hash being identical to the byte is the proof that the change
is inert where it should be, which is worth more than a moved timeline here.

## Performance, measured, and one number of mine that was wrong

Interleaved A/B, both binaries in `build/`, alternating under the same box load:

| run | `main` | D207 |
|---|---|---|
| round 1 | 30,479 ms | 31,266 ms |
| round 2 | 30,366 ms | 30,750 ms |

**+1.5%** end to end - 48,000 ticks in 30.4 s -> 31.0 s, i.e. **6.3 -> 6.5 us/tick**. The cost is two `BTreeMap` lookups
per *collision*, not per body per tick.

**My own error, reported in the same breath:** the first measurement read **47.8 s against main's 30.7 s** and I nearly wrote
it down as a 55% regression. It was the box: `loadavg` was 3.9 from another session working in this same checkout, and the
two numbers had been taken twenty minutes apart. The interleaved pair above is the honest one, and the lesson is the one this
project keeps relearning - **a wall-clock number taken once, on a shared machine, is not a measurement**.

## Stability

- **No new per-tick allocation** on the hot path: `stamp_group` builds two small sets only on ticks that carry orders.
- **Scratch is not state:** `ordered_now` is emptied inside `drain_orders`, so a replay that queues the same commands in the
  same ticks cannot diverge on it; it is deliberately **not** hashed.
- **`mates` is state and is hashed**, written only when non-empty, so a world with no group orders hashes exactly as it did
  before this existed.
- **Mutual by construction**, so there is no one-sided ghosting: A can only walk over B if B's route says the same.
- **Bounded:** the exemption cannot re-plan, cannot pause, and cannot grow - it is a set written once per order.
- **Known risk, by design:** the same-tick heuristic means a caller that issues eight separate moves in one tick gets
  group behaviour it did not ask for. That is the owner's accepted interim, and the explicit group id (option 2) replaces
  it; the register row stays open in `docs/next-step.md`.

## The gate, and the one test the rule had to be reconciled with

`cargo test --workspace --no-fail-fast`: **187 binaries, 845 passed, 1 failed, 22 ignored.** The failure was
`footprint::a_head_on_pair_in_a_one_tile_corridor_never_overlaps`, and it failed for the right reason: its fixture ordered
both men **in one tick**, which under D207 is a group, and a group is allowed to overlap - so the pair squeezed past each
other in the corridor and the invariant broke.

The fixture is now **ordered a tick apart**, which keeps the test doing its original job (a *non-mate* pair stays solid), and
its doc comment carries two corrections rather than a silent edit:

1. it had been justified by *"no pushing allowed"* (Klingers, page 9) - a quote about **pushing**, which the binary read then
   showed is not about overlap at all: the collision block never clamps a step against another body;
2. it is expected to become an **arrival** test when D210 lands the pause and the temporal "past" rule, because the game's
   answer to this geometry is not a permanent standoff.

That is a restatement with a reason, not a weakened assertion: the pair still must meet and still must never come inside the
reach, and the group case it used to cover by accident is now covered deliberately in `group_overlap.rs`.
