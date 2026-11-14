# OPEN - D208 is half built on `d208-time-not-geometry`: the head-on is solved, and solidity is not yet

**Measured, 2026-11-14.** `crates/sim/tests/head_on.rs` is **2/2 green** on this branch: two men meeting head-on now pass
each other, and the loser stands for the **first** stand of the drawn 0.25-1.50 s. That is the original's mechanism, read
out of the binary the same day: no step is clamped against a body (the radius retraction's result is discarded at
`0x8a7fe7`), the loser is paused, and the winner's next route - planned after the loser stopped - classifies him **past**
(`path->createdAt >= other->lastMoveTime`) and walks through.

**What is not solved, and why this branch is not on `main`.** The *past* test is only sound because the original's search
treats a **standing** body as an obstacle rectangle (`FUN_007f9d80`: insert as obstacle iff the object is not ignored and
its velocity is zero, or it is in the must-collision list). Our search does not: `Walk` is terrain only, and a re-plan avoids
just the bodies it collided with. So `created >= moved` also makes every **standing** man "past", and two of D207's guards
say so, measured:

```
a_mate_who_is_stopped_becomes_solid      MEASURED a stopped body was walked over: closest 4207 raw against an 8000 raw reach
two_units_ordered_apart_stay_solid       MEASURED the closest these two came was 151 raw, inside the 8000 raw reach
```

**The next slice is therefore D208b, and it is a search change:** a body that is **not moving this tick** goes into the
route's blocked-tile set (the per-search rectangle list the binary builds in `007f6ff0`, rebuilt per search, never written
into the shared grid). Then *past* is sound again: a standing man is planned around, a paused loser is the one exception,
and a man who moves into your path after planning is non-past and forces the repath.

**Cost of what is already here** (`--seed 7 --ticks 48000`, release): `entities=158 players=2
hash=c2ed74016ee533ea repaths=508 made_room=0 repaths_body=174 repaths_ground=334` against `main`'s `entities=160
players=2 hash=c3b10f4b09e17a43 repaths=324 made_room=147 repaths_body=83 repaths_ground=241`. **More repaths (+57%)** and
no made-room steps at all - the trade D208b has to be measured against.

**Also on this branch, uncommitted in spirit:** `Route.created`, `Entity::last_moved`, `put_route` (one choke point that
stamps a route with the tick it was planned), `World::is_past`, and the removal of `bound_to_reach` from the body path.
`bound_to_reach` itself is kept for now - the attack and building paths still use its idea - but it is no longer called from
`collide`.
