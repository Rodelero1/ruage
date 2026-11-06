# A re-plan that fails spends one of the route's two attempts (D135)

*2026-11-06. The 17x tick that D134's acceptance match ran into, found and fixed in the same evening.*

## What the slow state was

D134's match ran at **3.5 ms a tick against 0.3** from tick ~10,000, with the same 23 buildings, 31 units, 101 resource
nodes, the same task histogram, the same path lengths and the same plan count in both worlds. Splitting the headless loop
put it beyond doubt: **AI 2,294 ms, simulation 28,942 ms** against **2,749 / 1,724** - the simulation, by 17x.

The instrument that named it was the one the code already had almost in place: the re-plan path (`pass.repaths`). Counted,
then timed:

| | D134 branch, 12,000 ticks |
|---|---|
| re-plan attempts | **2,559** |
| of which failed | **2,533** |
| time inside re-planning | **8,089 ms** of about 20,000 ms |
| average per failed search | **3.2 ms** |

**98.9% of the re-planning was thrown away**, one search a tick, for as long as a unit stood against ground it could not
answer for.

## Why the cap did not stop it

D127 caps ground re-plans at **two per route**: `collide` asks `route.stalls < MAX_GROUND_REPLANS`, and the count is
incremented in the re-plan's **success** branch only. A re-plan that failed left the count where it was, so:

- the same request was made on the next tick, and the next, for the rest of the match;
- and for a unit with **no route entry at all** - a goal it cannot reach, nothing left to follow - `paths.get_mut(&id)`
  returned `None`, so there was no counter to fail to increment. That is the case the acceptance match was in.

The body-repath branch is safe by construction: a failed search inserts the body into the route's `avoiding` set, and
`collide` will not ask again for a body the route already avoids. Ground stalls had no such memory.

## The fix

A failed re-plan **spends one of the route's two attempts**, and creates the route entry when there is none. Nothing else
changes: a failed search still leaves the old route in place, the unit still waits or slides, and the refusal still goes
into state.

## Performance

| | before D135 | after |
|---|---|---|
| re-plan attempts, 36,000 ticks | **11,260** | **192** |
| failures | **11,070** | **4** |
| time re-planning | **35.4 s** | **13.3 ms** |
| whole match, release, three runs | **47,496 ms** | **11,828 ms** median (11,719 / 12,055 / 11,828) |
| state hash | `f328ebc8d4d34b78` | **`f328ebc8d4d34b78`** |
| entities / made_room | 141 / 187 | 141 / 187 |

**The hash does not move.** That is the point: every search removed by this change decided nothing. The match is the same
match, four times cheaper.

## The test

`crates/sim/tests/repath_budget.rs` builds the case rather than waiting for it: a walker on open ground, then a **wall
across the whole width** and a **wall on its goal**, so no route to the goal exists at all. It asserts **at most two failed
re-plans in 600 ticks**.

Watched failing first:

```
177 failed re-plans in 600 ticks against ground that cannot be crossed:
a search a tick is the cost that made the D134 branch's match run at 3.5 ms a tick instead of 0.3
```

`World::repath_failures()` is scaffolding added before the test, so the failure above is an assertion and not a compile
error. It is a counter like `repaths()` and `made_room()`: not state, not hashed, and the thing a test can hold an engine
to.

## Stability

| attack | result |
|---|---|
| ground that can never be crossed (wall across the world + walled goal) | two attempts, then the unit stops asking (the test) |
| a unit with **no route** and a goal it cannot reach | the case that looped; a failed re-plan now creates the entry, so the cap applies |
| a body in the way, not the ground | unchanged: the body joins `avoiding` and `collide` does not ask again for a body the route knows |
| a route that succeeds | `stalls` increments exactly as before - the success path is untouched |
| the two-attempt budget itself | still two: the test asserts the same number D127 documented, now against failures as well |
| determinism | `stalls` is state and stays in the hash; the hash before and after the fix is identical, so nothing about the match's behaviour moved |
| a unit re-ordered by the player | `start_walk` inserts a fresh route, so the budget resets with the order - the same as D127 |

## This step's own errors

- First I assumed the loop was **body** repaths and wrote the fix for those. The measurement said otherwise: `body.is_none()`
  was almost always true, and the hash did not move - which is how I knew the fix had done nothing. The `is_none()` route
  case was the real one.
- My first `repath_budget` fixture (walls on the shipped map) **passed before the fix**, because a wall five tiles wide can
  be walked around. A fixture that does not reproduce the bug is worse than no fixture: it says "fixed" for free.
- The 8.1 s figure for the D134 branch was measured at 12,000 ticks against a 20 s run; the 35.4 s figure for the same
  defect at 36,000 ticks. Both are true and they are not the same measurement - the report says which is which.

**711 passed / 0 failed / 20 ignored**; clippy and fmt clean. Merge with D134: `main` moves to the acceptance
baseline this pair fixes - `f328ebc8d4d34b78` / 141 / 188 repaths, walled-off refusals **0**.
