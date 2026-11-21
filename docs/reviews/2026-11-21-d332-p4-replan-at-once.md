# Review: ground that refuses a step is re-planned at once, not a second later (P4, D332)

*Change: `replan_after_ground` loses its `REPLAN_TICKS` gate - the original re-plans on **every** terrain collision, and the throttle made a
man stand for up to a second while the search he needed waited out a timer. One early return deleted; the stall-fallback that shares the same
constant stays.*

## The defect, witnessed

`docs/parity-plan.md` P4: `if self.tick - route.created < REPLAN_TICKS { return; }` in `replan_after_ground`, so a route younger than 20 ticks
could not be re-planned even when the ground under it had gone stale. The original's epilogue is
*"We have %d non-past collisions and %d terrain collisions, repathing"* (`0x128b`) - a re-plan on the collision itself.

`immediate_replan::a_wall_in_the_lane_is_replanned_the_tick_it_is_seen`: the walker is ordered ten tiles east on open ground (route asserted to
be **one leg**, so the fixture is testing a stale route and not something else), and two ticks in a palisade segment is placed in the tile
**immediately ahead of him**. The fixture then walks him **one tick at a time** until his position stops changing - the refusal - and asks what
the plan did on that tick:

```
on tick 4 ground refused him and the route was not re-planned (0 -> 0 re-paths): a stale route is a man
standing still while the search he needs waits out a timer
```

Green with the gate gone. **Mutation-proved the other way**: putting the gate back reddens it at the same tick with the same message.

**Two fixture errors of mine on the way**, both caught by the test refusing to pass rather than by reasoning: the wall was first placed **four
tiles** ahead (out of reach for a hundred ticks, so nothing was ever refused), and the refusal was then checked in a **three-tick** window from
the moment of placement - the palisade is queued and lands on the next tick, and the walker needs ten ticks to reach it. The final version asks
the question in the only place it can be answered: the tick the position stops advancing.

## Performance

Two trees, release, seed 7, idle; parent `0ca81ba` against this change.

| | before | after |
|---|---|---|
| acceptance hash | `bef8b77b84aaf8d8` | `e248486298910e28` |
| re-paths, 48,000 ticks | 1,918 | **1,727** (-10%) |
| acceptance wall | 10.18 / 10.35 s | **10.26 / 10.30 s** |
| stockpile at 2,400 s | 1,949 / 960 / 130 | 1,961 / 960 / 130 |
| entities at 48,000 ticks | 127 | 127 |

**The cost the throttle was protecting against did not appear**: re-paths fell rather than rose, because re-planning at once produces a route
that is no longer refused - the throttle was buying repeat searches of a route that had already gone stale. The wall-clock difference is inside
the run-to-run spread.

**The worry the register recorded** - *"a refusal every tick is a search every tick"* - is answered by the count: 1,727 re-paths over 48,000
ticks is one re-plan per 28 ticks across the whole match, not one per tick.

## Stability

- Suite and conformance logs are staged with this change; the movement suites that own this rule (`evasion`, `replan`, `collision`,
  `crowd_piles_up`, `invariants`) are the ones that would notice.
- **What still shares the constant**: the D279 fallback - after `REPLAN_TICKS` consecutive refusals, an off-grid goal falls back to its tile
  centre - is a *stall* threshold and is untouched. It is a different question from "may I re-plan now", and the doc comment says so, because
  the two used to be one constant doing two jobs.
- **The risk it carries**: a man whose goal is buried behind new walls now searches every tick until the stall threshold fires. That is what
  the constant still gates, and it is why the fallback is checked in the same function rather than left to the caller.
