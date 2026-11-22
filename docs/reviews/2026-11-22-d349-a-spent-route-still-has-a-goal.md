# 2026-11-22 - D349: a body with a spent route still has a goal

The owner's report - *"villagers stack on each other a lot (like 5 in the same spot)"* - measured, traced, and cut at one
mechanism. What landed is small: **a re-plan answers a body whose route is spent**, plus the corrected reading of
`FUN_007f9d80`'s ignore clause. What the measurement says about the pile itself is in `## What the pile is` below, and it is
not what the slice fixes.

## What changed

- `crates/sim/src/world/motion.rs`, `World::replan_goal`: the point a body-collision re-plan aims at is the route's last
  waypoint, **or the task's own `to` when the route is spent**. `move_bodies` walks a `Task::Move` with no waypoints along the
  task layer's direction, one whole step a tick, so a spent route still drives a man at his point - and
  `replan_around_bodies` bailed on the empty route, leaving him to walk straight through friends with nothing to steer round.
  Measured on `contact_needs_one_collision`: two men sent at one target from the same side ended **0 raw** apart, and every
  tick of the approach recorded the friend as a blocker with **0 re-plans**.
- `crates/sim/src/lib.rs`, `standing_bodies`: **comment only**, correcting the reading. The binary's clause is
  `(local_10 <= iVar4) && (isMoving() == 0)` - *not on the mover's own list* **and** *not moving* - so **movement alone**
  drops a body's box, whoever owns it, and the route's own list is a second clause that only ever adds. The engine already did
  this; the comment said the list mattered, which is what made the wrong slice look right (below).

## What the pile is, measured

`collision_probe` on the acceptance scene (36,000 ticks, seed 7): deepest overlapping group **11 bodies**, **317,041** pairs
closer than a fifth of a tile, on 35,336 of 36,000 ticks, deepest overlap the full footprint. New `pile_probe`, which
attributes each *new* overlap to the exemption the mover actually held at that tick:

- **mate 0, past 500, neither 164** (664 landings a mover can be named for, out of 877 new overlaps);
- every offender is a `Gather` into `Gather` or `Return` pair, and the printed ones are men arriving at **the same raw point**
  one after another (dx = 0, dy = 7,200 raw = 0.36 of a tile, against an 8,000-raw reach) - the last three quarters of a tile
  of a shared route.

So the pile is **the arrival, not the step**: a body collision never refuses a step in this engine (D279's port: the step is
taken, the higher id is paused, the mover re-plans), which is the binary's own epilogue. Nothing separates two men who stop
inside each other, and the game has no pushing either. The 500 "past" landings are the game's own exemption - a collider that
has not moved since the route was created - so the honest conclusion is that **the plan is where the game keeps men out of one
place, and our plan is doing what the binary's does**: a *moving* body is not an obstacle (`FUN_007f9d80:45`).

**A wrong slice, written, measured and reverted.** The first attempt made every body an obstacle in the plan (reading
`(not in my list) && !isMoving` as "the list is the only way out", and D237's *villagers are never mates* as licence to drop
the moving-body exemption). The suite caught it: `build::a_house_is_paid_for_and_finishes_in_its_create_time` and
`a_spanish_builder_needs_385_ticks_where_a_british_one_needs_500` refused their orders with
`NoPath { from: (15,14), to: (15,14) }`, and `overlap::a_newcomer_does_not_move_the_working_gatherers` with
`NoPath { from: (21,14), to: (20,14) }` - a crowd's own boxes make the tiles round a man unplannable, which is exactly the
defect the moving-body exemption exists to prevent. Reverted in the same session; the corrected comment is what remains.

## Performance

`cargo build --release --workspace` + `ruage-headless`, the acceptance scene, seed 7, 48,000 ticks, three runs on each side:

- **before** (`ed064fb`): hash `7782dd38d7557514`, **134 entities**, 37,204 re-paths, **12.12 s** median of three;
- **after**: hash `7782dd38d7557514`, **134 entities**, 37,204 re-paths, **12.17 / 12.12 / 12.18 s** (median 12.17).

**Bit-identical**: the hash, the entity count, the re-path count and the wall time are the same, so **the acceptance scene never
reaches the branch** - it is a fallback for a body whose route is spent, and no body in that scene collides with a friend on the
one tick its last waypoint is spent. That is the honest reading of a green hash: the witness for this change is the fixture, not
the scene, and the cost is **unmeasured on a scene that fires it** (the fixture's 800 ticks do fire it, and it ends more than a
reach apart instead of 0 raw).

The pile census is likewise **unchanged**: 349,824 overlapping pair-ticks (2.0173% of 17,340,819 inspected), 317,041 deeper than
half a footprint, deepest 0.400 tiles, **877** new overlaps, deepest new 1,603 raw, and `pile_probe`'s split the same
(mate 0, past 500, neither 164). This slice does not move the pile and does not claim to.

The reverted slice is in this note because its numbers are the argument against it: with every body an obstacle in the plan, the
acceptance suite's three order-refusing reds above, and no measurement of the pile taken because the suite stopped the slice
first.

## Stability

- **The route-goal fallback cannot loop.** A re-plan with a spent route now aims at `Task::Move::to`; the goal does not move
  with the body, so a failure leaves the old route and repeats at exactly the rate it did before (once per colliding step).
  Re-paths on the acceptance scene are identical, which is the measurement of that claim.
- **A body with no task that names a point** (`Gather`, `Return`, `Build`) still has no fallback: `replan_goal` returns `None`
  and the old behaviour holds. No scene in the suite changes because of it.
- **`contact_needs_one_collision`** attacks the state the owner described: two bodies under one order, one contact point, and
  asserts both that they end **at least one reach apart** and that both are still **within a tile of the target** - so a fix
  that keeps them apart by refusing to walk is red too.
- **The contact-cut question is open and filed.** The binary's arrival arm requires the collision list to be *exactly* the
  target (`if ((DAT_00de1998 != 1) || (local_dc != 1))` re-paths, 0x127d); this engine cuts before the body test. Wired and
  measured, it changed the census **not at all** - the cut only fires for a landing strictly inside the target's grown box,
  which no acceptance-scene walk does - so it went back out rather than land without a witness. It is the next `next-step.md`
  item with the fixture it needs (a fast walker whose step lands inside the target's box while a friend stands on the contact
  point).
- **`last_step` is only written when step tracing is on** (`record_step` returns early otherwise), which cost an hour of this
  session: a mover that had plainly walked read as `Idle`.

## One more thing in this commit: the conformance set did not compile

`make conformance` was failing to build - `crates/conformance/tests/divergences.rs:32` called `Levels::refine` with the two
arguments it had before **D343** gave it the walk's own endpoints, so the whole set was uncompilable and the "21 / 0 / 0" in
the handover was a memory rather than a run. Repaired by passing the two points; the set now builds and answers **21 tests, 21
passed, 0 failed** (4 + 2 + 3 + 2 + 2 + 3 + 5), which is D319's own trap ("a target the gate never compiles is a target that can
be dead") found in the wild. No engine behaviour changes.

## The fork this leaves with the owner

The next lever on the pile is the one the game also lacks a rule for, so it is his call rather than mine: with no pushing and
no place model (D293), the only way two men stop inside each other on purpose is that they share one point. The candidates,
each with its measured price, are in `docs/next-step.md`; the two that are readable from the binary are the contact-cut arm
above and `FUN_007f9d80`'s tile **rectangle** (a body blocks every tile its grown box touches, not only the tile whose centre
is inside it - our `inside_a_body` asks the centre).
