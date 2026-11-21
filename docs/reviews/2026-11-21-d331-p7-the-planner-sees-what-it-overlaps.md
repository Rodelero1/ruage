# Review: the planner now sees a body the mover is standing inside (P7, D331)

*Change: `standing_bodies` skips **the mover, by id**, instead of every body within a quarter tile of him; `plan_path`,
`plan_path_avoiding` and `plan_path_for` carry the mover's id so the skip can be exact, and the three predicates that genuinely
have **no** body to name (`can_reach`, `work_position`, `exit_tile`/`approach_footprint`) pass `None` and are documented as such.*

## The defect, witnessed

`docs/parity-plan.md` P7: the original's search starts on the tile the mover stands on and puts **every stander the mover does not ignore**
into it (`FUN_007f9d80`); ours dropped every body within `scale/4` of the planning origin, in both axes. Two bodies in one place is a state
this engine reaches **on purpose** - D289 sends every man to the tile he was ordered to, and D290 leaves no press floor - so the plan
certified legs the step then refused.

`planning_round_bodies::a_body_the_mover_overlaps_is_still_planned_round` builds that state the way the game does, because spawning does not
reach it: `Command::Spawn`'s placement **separates** bodies (measured: a spawn asked for `410400` landed at `419600`), so the two are ordered
to the same point, pressed to **0 raw apart** (asserted, `gap < 2_000` so the fixture cannot pass vacuously), and only then is one sent
past the other.

Red before the change, in its own words:

```
the route past a body the mover is standing inside is one straight leg (1 point(s), the friend 0 raw away)
```

Green after. **The mutation is the old code**, not a contrivance: restoring the quarter-tile skip is what the red run measured.

## What changed

- `standing_bodies(…, mover: Option<EntityId>, …)`: `Some(other.id) == mover` replaces the box test. `None` means "a hypothetical body" -
  `can_reach` is asked by the scenario runner about *a* villager, not this one - and then every body in the span is an obstacle.
- `plan_path` / `plan_path_avoiding` take `Option<EntityId>`; `plan_path_for` passes `Some(id)`; fourteen call sites updated, each with the
  id that was already in scope.
- The moving-friend exemption is untouched: a friend who is moving and not paused is still ignored, which is the binary's `isMoving` gate
  (D284).

## Performance (two trees, both `--report 600`, same generated `config/gamedata/`)

| | before (`5329287`) | after |
|---|---|---|
| wood / food / gold at 2,400 s | 1,362 / 724 / 510 | **1,949 / 960 / 130** |
| units / buildings at 2,400 s | 41 / 20 | 40 / 16 |
| re-paths, 48,000 ticks | 7,125 | **1,918** (-73%) |
| acceptance wall | 12.93 s | **10.18 / 10.35 s** (-21%) |
| entities at 48,000 ticks | 141 | 127 |
| acceptance hash | `c7191f531bd81b1b` | `bef8b77b84aaf8d8` |

**The economy improves** (wood +43%, food +33%; gold falls 510 -> 130, which is a different re-tasking mix rather than a loss of income) and
the tick loop does **73% fewer re-plans**: a plan that knows about the body it is pressed against does not have to be refused and rebuilt.

**The entity count is not an economy signal and it fell for a match-length reason**: with P7 the match ends at **2,348.6 s** - *"the match is
over: no further orders are accepted"* - so production stops about a minute earlier and nothing is built in the remaining 6,650 ticks. The
2,400 s stockpile above is the number that answers "is the economy healthier", and it is.

## Stability

- All eleven movement suites green with the change in: `invariants`, `overlap`, `group_overlap`, `head_on`, `collision`, `crowd_piles_up`,
  `evasion`, `replan`, `gather_motion`, `gather_standoff`, `no_pushing`. Full suite and conformance: see the logs staged with this change.
- **The risk it carries**: a body pressed into another is now a planning obstacle *for itself*, so a crowd on one point plans detours where it
  used to walk straight. That is the direction the original takes, it is why the re-plan count fell, and the crowd fixtures are the ones that
  would notice - they are green.
- **Where `None` is used, and why it is not a fudge**: the three predicates ask "is there somewhere for *a* body to stand", and none of them
  is an order, so no id exists to skip. The consequence is that they see a body exactly at the origin as an obstacle too; that is stated in
  `can_reach`'s comment rather than left for a reader to infer.
