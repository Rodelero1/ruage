# OPEN - step 5 is read and refuted (D340); next is step 6, the search's body visibility

**Where this stands:** suite **217 result lines, 936 passed, 0 failed, 21 ignored**; conformance **21 / 0 / 0**; acceptance
`36ceb342bd94acc9`, 134 entities, **both players alive at 48,000**, 40,127 re-paths, **15.93 s** median of three. The
*order* to 100% conformance is **`docs/conformance-sequence.md`**; this file is the immediate step and the outstanding
forks.

**Step 5 (waypoint substitution) is finished as a read, not as code** (D340, `docs/reviews/2026-11-21-d340-the-substitution-has-no-stand.md`).
The rule is read to the end - `FUN_008a4360`'s three arms, the counter at the mover's `+0x148`, the threshold `> 2`, and
`FUN_007ff650` decoded as a bit in the *path's* `+0x30` flags word - and it does not land, because **a body never
refuses our mover's step** (`refusal()` answers `WaitTarget`/`WaitGround` only; `StepDecision::WaitBlocked` is dead
code), so the game's "cannot reach the waypoint" count only ever sees ground. Measured: beside the base, contact costs
**196 re-plans in 200 ticks with the walker walking throughout**; in a clear lane the same catch-up costs **0 in 260**.
The off-grid fallback's threshold (20, ours) was drafted down to the game's 3 and dropped - no fixture produces a
three-tick stand on an off-grid aim. Probe kept: `crates/sim/tests/substitute_probe.rs` (ignored, prints).

## The next step: 6 - a body that is not moving is an obstacle, even with a route

**Prior implementation: `standing_bodies` (`crates/sim/src/lib.rs:1991-2030`).** A body is skipped by every plan when it
*has a route and its pause is over*:

```rust
let moving = self.paths.get(&other.id).is_some_and(|route| !route.points.is_empty());
if moving && other.pause_remaining == 0 { continue; }
```

So a villager working beside a site, or one standing where ground refused him, is **invisible to the search** while a
route is on his books - the plan certifies legs through him, and the step then walks through him (D279 rule 4 takes the
step). Pinned by `planning_round_bodies`, `stall`, `body_boxes`, `collision`, `crowd_piles_up`; none of them asserts a
*standing* body with a route being planned round.

**The rule, read** (`FUN_007f9d80:27-56`, HD 5.5.7 `pathfinding.cpp`): for each object in the route's list the search
includes it as a grown box **unless** its id is in the mover's ignore list **and** it is not moving:

```c
if (((local_10 <= iVar4) && (iVar4 = (**(code **)(*piVar3 + 0x2a0))(), iVar4 == 0)) || iVar6 < param_6) { ... insert ... }
```

`local_10 <= iVar4` is "not found in the ignore list"; `[vtbl+0x2a0]` is the same `isMoving` `FUN_008a4360` logs as
`isMoving = %d`; the box is the object's own `+0x34`/`+0x38` radii grown by `param_2` (the mover's radius) and inserted
through `FUN_007f6ff0(&box, 1)`. **A body with a route but zero velocity is an obstruction.** openage's note agrees from
the other side: the high-level search *"ignores moving objects, e.g. units or wildlife"* - the exemption is *moving*, not
*ordered*.

**Our equivalent of velocity** is the tick's own movement: `entity.last_moved` is written at the end of `move_bodies` for
every body that took a step. The test to write is `self.tick.saturating_sub(1) <= entity.last_moved` - moved on the
previous tick - and `velocity` is **not** it (it is a fixture-only field for idle drift, `lib.rs:2313`).

**Witness owed:** a builder standing at work beside his site (a live `Build` task, a live route, zero motion - the state
measured at `(15,15)` with `route=[(14.5,15.5)]` for 400 ticks) and a third man ordered past him: today the third man's
route runs through the builder's box, with the cut it bends round. Assert on the route's points against the builder's
grown box, and then on the walk (he passes *beside*, not through).

**Cost to watch:** the acceptance match has many working men, so this makes plans stricter everywhere - re-paths,
entity count, hash and wall all move. One variable, full A/B.

## Also ready, independent of that

- **A waiting walker is the same rule** (this step covers it): the binary's test is velocity zero, so a walker paused at a
  crossing is already an obstacle here; a walker *jammed* without a pause is not.
- **The 300 row** (step 7) and **the refinement's scan direction** (step 8): both reads done, each a small slice. Step 8 is
  also what would let the 16-tile floor fall (D338's re-test).
- **The level floor** (D303): the clause that keeps the tier off in a wooded map. Route-wide; needs its own A/B.
- **The substitution threshold** (D340): the game's `> 2` against our `REPLAN_TICKS` (20) in `replan_after_ground`, waiting
  for a fixture that stands on an off-grid aim for three ticks.
- **The mate rule (step 11) is already the game's**: `stamp_group` (`world/step.rs:110-144`) gives mates only to **military**
  units; villagers never join one. What is left is a *formation model* for soldiers - a gap of its own.

## Needs the owner

- **An evenly mirrored skirmish**: player 2's barracks order is at 70 s where player 1's is at 320 s.

## Parked on what this box does not have

- **P3's inset** (`+0x268`), the **100/600 terrain bits**, the **octile table's values** (`DAT_00afd2a0` is runtime storage),
  the **search window's padding** (D299), and the **straight-line tier** (the caller of `FUN_008000d0`). The **equal-`f` sift
  order** is *not* parked - `FUN_00801e90` is in the corpus and is one read. And the **writer of the collision ignore list** is
  one read: it is what decides when a military formation fills it.
- **The acceptance wall's attribution** (D337): 15.7 s against 9.7 s for the key alone, never profiled per pass.
