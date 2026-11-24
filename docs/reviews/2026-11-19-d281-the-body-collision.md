# Review: the body collision is the original's (D281)

*The owner, asked which of three ways a crowd should behave: **"Follow what does the original game and decomp do."** Read out
of `FUN_00844360`'s epilogue and the path layer, implemented and landed; and on the one fixture that had made the first
attempt a half-fix: **"Is it unit test or conformance test? If unit test, drop it."** It is a unit test, and it is dropped.*

## What the original does (read, not inferred)

- **A collision against the order's target ends the order** - *"That collision is with our target, so we're done"*.
- **Any other non-past collision is counted, and the mover re-plans** - *"We have %d non-past collisions and %d terrain
  collisions, repathing"* - and the **higher id of each surviving pair** is told `setPause(rand*1.25/32767 + 0.25)`
  (`0x8a80ff`-`0x8a8141`).
- **The epilogue never writes a position**, and the retraction the radius query computes is discarded (`0x8a7fe7`): bodies
  *do* overlap for the ticks they pass.
- **A collider is *past* when `path->createdAt >= other->lastMoveTime`** (`move_obj.cpp:2063`).
- **The path-ignore list is the mover's moving group-mates and nothing else** (`isMoving`), so a stopped unit is not passed
  through: it blocks, and the mover's re-plan is what gets round it.

## Performance

Measured on this tree, debug, 48,000 ticks:

- `the_acceptance_match_trains_more_villagers`: **42.62 s -> 39.59 s**. A mover that bends round a body stops buying refused
  steps against it, so the re-plan pays for itself.
- **One search per collision**, with the colliders' tiles in an avoid slice (`Ground::route_avoiding`), and **no budget** -
  the original has none. What bounds it is the loser's pause and the *past* test.

## Stability

**Three of D279's reds turned green**: `a_repath_route_is_pulled_like_a_planned_one`, `both_sides_field_an_army`,
`the_skirmish_only_refuses_what_the_fight_has_taken`. **Suite: 198 binaries, 881 passed, 20 failed, 21 ignored** (from
880 / 22 / 21).

**One new red, and it is the interesting one.** `the_ten_minute_scenario_builds_trains_and_fights` fails at 500 s: the
barracks is still `construction=Some(progress 0 of 1000)` and its builder is `Idle`. The game's model made another villager
solid and the builder never reached his site - a **stall**, not a rule being broken. It is the first item of the next slice,
and it is named here rather than smoothed over.

**The test that was dropped, and what goes with it.** `a_spawn_in_the_same_tick_does_not_move_the_wrong_entities` was a
**unit test** (`crates/sim/src/lib.rs`, `mod tests`). Its scene put two bodies on one spot walking the same direction at the
same speed, which the game's rule cannot separate: the follower is one step inside the reach every time his pause expires,
so he is paused again, for ever. **The owner's correction stands and my first explanation was wrong**: a paused man is told
*once per expiry*, he stands, and he resumes - the repetition happens only when the pair never separates, which is exactly
that scene. The guard the test carried - a mid-tick spawn shifting the entity list, so movements must be applied by id and
not by index - is **now unguarded**, and is recorded as a gap rather than replaced by a weaker test.

**Two defects found inside the attempt, both fixed before it landed:**

1. **The smoothing pass undid the plan.** The search bent the route round the man and `pull` straightened it back through
   him, because `line_clear` knew only terrain: the re-plan answered a **one-point route straight at the body**, every tick.
2. **Re-stamping the route's clock made the avoided man *past*.** `put_route` wrote `created = tick` on every route,
   including a re-plan, and the next tick's past test then walked through the man it had just avoided - **4,207 raw inside
   an 8,000 raw reach**.

**What this does not fix.** Two bodies that share a tile cannot be planned round at all - the search starts on the tile they
share - so `a_mate_who_is_stopped_becomes_solid` still ends **4,207 raw** inside a stopped man's reach, and its second
assertion ("nothing inside the reach after the meeting") is our old retraction, not the game's rule. The eight crowd
fixtures keep asserting the withdrawn settle ring. Both stay on the known-red list; the builder's stall is the first thing
to fix there.
