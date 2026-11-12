# The decision is a value: `StepDecision`, and the hostile body it found

The architecture review's item 4, and a defect it turned up within the hour: **enemy bodies were being overlapped** at rest.

## Performance - measured, one run at a time

| | before this slice | after |
|---|---|---|
| acceptance hash | `91572e72c33cee93` | `91572e72c33cee93` (unchanged) |
| entities / repaths / made_room | 142 / 409 / 0 | 142 / 409 (body 0, ground 409) / 0 |
| `World::step`, trace **off** | 6.6-6.7 us | **7.0-7.2 us** |
| `World::step`, trace **on** | - | 7.1-7.3 us |
| `Snapshot::of` | 89.3-103.4 us | 87.1-90.8 us |

**The tick pays 0.4 us (about 6%) for the decision being a value**, and that is the honest price of the review's item: the enum is
returned by `collide` on every body's step even when nobody records it. Two cheaper attempts were measured and rejected:

- a `BTreeMap<EntityId, StepDecision>` for the per-body trace cost **1.8 us a tick on its own** (6.7 -> 8.5, a third of the tick);
- `StepDecision::index()` implemented as a scan of `ALL` cost **0.5 us** (13 comparisons per body per tick), so it is a `match`.

The trace is **off by default** (`World::trace_steps()` turns it on) so a match does not pay for a diagnostic; the counters and the
per-body field are the 0.4 us that remain, and they are what a bug report needs.

## What was built

- `StepDecision`, thirteen named outcomes: `Idle`, `Take`, `TakeOverlapping`, `TakeAtRest`, `Slide`, `MakeRoom`, `GiveWay`,
  `GiveWayOwnTile`, `WaitGround`, `WaitTarget`, `WaitGoal`, `WaitOwnTile`, `WaitBlocked`. Every `return` in `collide` now names its
  rule, so the nine-rule function says *which* rule it used instead of returning a pair of deltas.
- `Entity::last_step` (not in the state hash) and `World::last_step(id)`; flat `[u64; 13]` counters with `World::step_count(step)`.
- Two tests, written first: `step_decision::a_friend_on_the_goal_tile_is_a_take` (the D192 deadlock as a value) and
  `step_decision::an_enemy_body_on_the_line_changes_the_decision`.

## The defect the second test found

The enemy test measured the closest approach between the walker and an **idle** enemy:

```
MEASURED the walker came 4743 raw units from the enemy; two bodies of radius 0.2 need 8000
```

**0.237 tiles against the 0.4 two bodies need**: enemy units were being walked through. Rule 8's exemption - *"a body at rest cannot
move aside, so waiting on it is waiting for ever"* - was written for **resource nodes**, which carry no radius (so `reach` falls back to
a whole tile, and a villager must be able to stand beside a tree to chop it). Applied to units, it let a mover pass straight through an
idle enemy.

The exemption now applies to **nodes, buildings and friends**; a hostile **unit** at rest is bounded by the reach. The full suite went
from 122 to **125 sim binaries green** with that change, so nothing the exemption was protecting had to move.

## Stability

- **Workspace gate: 181 test binaries, `EXIT 0`, `clippy -D warnings` 0, `fmt` 0** after the whole batch.
- **The trace cannot desync a match**: `Entity::last_step` is not in `state_hash` (the hash is an explicit FNV walk), the counters are
  not state, and the acceptance hash is unchanged at `91572e72c33cee93`.
- **Test-first, with the failures recorded**: `no method named last_step`, then `Some(Idle)` twice - the second time because two
  `Spawn`s in one tick took the same id (the fixture trap `evasion.rs` documents), which the test now names explicitly.
- **Decisions D193-D197** recorded (tests assert relations; errors say which kind; the next step is one step; the movement decision is a
  value; repaths by reason), each with its measured evidence, and `tools/decisions.py --check` green at **198 decisions**.
