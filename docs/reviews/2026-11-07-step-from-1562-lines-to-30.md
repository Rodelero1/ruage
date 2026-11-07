# `step` from 1,562 lines to 30 (D146)

*2026-11-07. The owner: "Proceed" — on a note saying stage 2 was the next job. `step` was one function of **1,562 lines**,
declaring nineteen vectors, walking the entity list once, then applying what it found. It is now **30 lines of named
phases**, and the tick reads in one screen.*

## What the split is

```
step()
  drain_orders()                                  queues → apply, refusals not failures
  let mut work = decide()?                        ONE pass, changes nothing, returns TickWork
  resume_fights(engaging, turning, homing)         hunts start, patrols turn, defensive units go home
  grant_takes(chops, takes)                       tree damage, then the load
  send_hunters(hunts)                             a hunter strikes its animal
  retask_workers(retasks)                         the next node of the same kind
  apply_switches(switches)                        a task change becomes a route
  shepherd(herds)                                 a shepherd strikes where the animal stands (D116)
  deposit_loads(deposits)                         whole resources in, fraction stays in hand
  finish_construction(builders)                   one builder-tick per tick
  land_blows(strikes, …)                          blows in order (D88)
  step_production()?                              training and research timers
  resume_idle(finished)                           what a unit does when its job ends
  move_bodies(deltas)?                            collide, move, repath, spend waypoints
  fly_shots(shots, landings, landed_arrows, …)    shots up, land, leave the list
  step_world_timers()                             leashes, conversion, shares, spent nodes, husks
  eject_garrisons(ejected)                        a building throws its garrison out
  sweep_dead(&blows)                              corpses, rot, the dead, the match decision
  tick += 1; observe()
```

`TickWork` is the state the decide pass fills: nineteen vectors, `#[derive(Default)]`, created and dropped inside `step`.
It exists so the phases take what they need instead of sharing a 1,500-line scope, and it is **not hashed and not carried
between ticks** — it dies with the call, which is why it cannot become a second source of truth.

## Performance — measured, and it moved the right way

Same release profile, same pack, 36,000 ticks, seed 7, three runs before and five after on an idle box:

| | runs (ms) | range |
|---|---|---|
| before (one 1,562-line `step`) | 11,579 / 11,492 / 11,578 | 11.49–11.58 s |
| after (nineteen functions) | 10,689 / 10,882 / 10,910 / 10,779 / 11,034 | 10.69–11.03 s |

**~0.6–0.9 s faster, 5–7%, and the ranges do not overlap.** The likely cause is that the old function kept nineteen vectors
alive across 1,500 lines and one enormous stack frame; the phases now have short lifetimes and small frames, so LLVM's
inlining and register allocation have less to fight. **That is an inference, not a measurement** — what is measured is the
wall clock, and the only other change was the shape of the code.

## Stability

- **The acceptance hash did not move**: `f328ebc8d4d34b78`, 141 entities, 188 repaths, 187 made-room, before and after, and
  it is the whole proof that the restructure is behaviour-neutral. Twelve extraction steps, hash checked as the last of them
  landed, then again after the `TickWork` rewrite.
- **Two things the compiler caught, both fixed before any commit**: `step_production` and `move_bodies` used `?`, so they
  needed `Result` returns rather than `()`; and `decide` inherited five `?` from the old body, so it returns
  `Result<TickWork, SimError>` and `step` calls `decide()?`.
- **One borrow the compiler caught**: `&mut work.blows` needs `let mut work`.
- **The gate caught one more**: with `sweep_dead` taking `blows: &[(EntityId, EntityId)]`, the old call site's
  `&blows` became a double reference — clippy `needless_borrow`, `-D warnings`, red gate. Fixed to `blows`, then the whole
  suite and clippy re-run on the final tree (green: 728/0/19, clippy 0, fmt 0).
- **The suite is unchanged in size and green**: 728 passed / 0 failed / 19 ignored, clippy `-D warnings` clean, fmt clean.
  No test was added or moved — this is a restructure, and the hash is its test.
- **Nothing was re-ordered.** The call sequence is the old statement sequence, which is what makes the hash proof meaningful:
  a change of order would still hash the same in an idle world and differ in a busy one.

## What is left, honestly

- **`decide` is 432 lines**, the largest function in the crate now. It is one `for` over the entities with a `match` on
  `Task` (Attack, Build, Return, Gather, Move, Garrisoned, Fly, Idle) — a shape that is long because the game has that many
  verbs, not because it is tangled. Cutting it per task (`decide_attack`, `decide_gather`, …) is possible and would need each
  arm to return its delta and push into `work`, which is a real change in shape for a modest gain.
- `move_bodies` 198, `sweep_dead` 118, `land_blows` 115, `step_production` 94, `shepherd` 90, `deposit_loads` 81,
  `resume_fights` 74 — all readable at those sizes.
- `world/step.rs` is 1,672 lines in **one file**: nineteen functions that share `TickWork`. A `step/` directory would be file
  shuffling; the thing that was wrong was the function, and it is fixed.
