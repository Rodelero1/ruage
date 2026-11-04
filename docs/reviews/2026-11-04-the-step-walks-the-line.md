# Review: the step walks the line

*Change: `step_towards` no longer clamps each axis to its own remaining gap. It moves `delta` **along the segment** and
lands exactly on the point when the remainder is within one step, so `position == waypoint` stays the spend test and every
`position == target` arrival rule needs no new case. Alongside it: a repath's route is now pulled the way a planned route
already was (`pull_route_avoiding`, obeying the search's own avoid set), because most long walks in a busy base are
repaths and an unpulled route is walked tile centre to tile centre. D110; the design the next slice follows is
`docs/pathfinding-from-the-talk.md`.*

Two details are the whole of the correctness work, and both were found by a test rather than reasoned out in advance:

- **No axis is ever abandoned.** A component that rounds to zero leaves a purely axis-aligned step, and an axis-aligned
  step into a wall has no slide to fall back on: a villager on its way round a house sat at `y = 27.9995` tiles with a
  horizontal step whose landing tile was the blocked `(4,27)` and stayed there for the rest of the match.
- **A zero step moves nothing.** The one-raw-unit floor is what makes the first rule safe, and it also made a unit the
  pack gives no speed creep a raw unit a tick for ever.

## Performance

Interleaved against a `HEAD` worktree (`3cfd783`) with the same `config/gamedata/`, three runs each, 36,000 ticks, seed 7,
`config/acceptance.toml`:

| | wall clock | per tick | entities | repaths | made_room | outcome | hash |
|---|---|---|---|---|---|---|---|
| `HEAD` | 8.75 / 8.92 / 8.75 s | 0.25 ms | 131 | 311 | 185 | `players=1` | `92ad4956a7093175` |
| this tree | 8.73 / 8.91 / 8.73 s | 0.24 ms | 126 | 194 | 145 | `players=1` | `098fd53e146f0911` |

**Neutral, and the tick is not where the change is.** `step_towards` is a handful of integer operations per moving entity;
what moved is the *match*: **117 fewer repaths** (311 -> 194) and 38 fewer nudges, because a unit that walks the line it was
given stops being shoved off it, and five fewer entities because a diagonal is no longer sqrt(2) faster than a straight
one (see below). The hash move is expected and is the point: what a unit walks to is what it does next.

**The one behaviour change that is not cosmetic: diagonals are no longer 41% faster.** The old step moved `delta` on *each*
axis, so a 45-degree walk covered sqrt(2) x `delta` of ground per tick. The dat's speed is tiles per second, so the old
figure was a bug - and every schedule written under it is now short. `config/scenario.toml`'s builder step went 380 s ->
390 s: at 380 the builder was re-tasked with the barracks at **921 of 1,000 ticks**, and it stood half-built for the rest
of the match, which the scenario suite caught and no unit test would have.

## Stability

Four attacks, three caught and one survivor that was then deleted rather than trusted:

| mutation | result |
|---|---|
| restore the per-axis clamp | `a_walk_at_fifty_nine_degrees_is_one_straight_line` fails: the walk leaves its line by **2,057 thousandths of a tile** (the 45-degree leg then horizontal) |
| drop the one-raw-unit floor | `a_villager_walks_round_a_house_rather_than_through_it` fails: pinned at `x = 3.97` of a walk to `x = 10` |
| drop the `delta <= 0` guard | `a_unit_with_no_speed_stays_where_it_is` fails: 50 raw units in 50 ticks |
| let the pull's intermediate jumps exempt the end tile | **survives the suite** - and rightly: the search only ever returns free ground for an intermediate node, so the rule was dead. Removed with `line_clear_with`, which existed only to carry it. |

Defects this pass found in its own work, in the order they were found:

1. **The first fix was wrong and the test said so.** The proportional step alone broke `a_villager_walks_round_a_house`:
   the walker stopped dead a third of a tile short of the house's west face. The cause was not the step but the *aim*: the
   route jumped `(3,28) -> (5,28)`, and `line_clear`'s endpoint exemption allowed a line whose last tile was a wall.
2. **My own number was wrong twice.** The failing test's diagnostic divided by the squared length, so it printed "176
   thousandths of a tile" for a walk that was 2.06 tiles off, then "41,159,673 thousandths" after the square root was
   added but the scale was not. The measurement was right each time; the sentence was not. Fixed, and the assertion itself
   never needed a square root (it compares `cross^2` against `bound^2 x length^2`).
3. **A tolerance I nearly shipped.** Rounding the landing tile instead of flooring it made the house test go green and then
   let the same villager walk *through* the house - tiles `(4,27)` and `(5,27)` held it for 40 ticks. Reverted; the
   one-raw-unit floor is the honest version of the same idea and does not move the model.
4. **The scenario's half-built barracks** (`921 / 1,000` ticks), described above.

Sample-state coverage, not just endpoints: the straight-line test samples **every tick** of a 466-tick walk and asserts
each position against the order's own line, and the house test samples every tick for standing on the footprint; the
waypoint-spend and node-arrival rules are equality tests, so `a_walk_lands_exactly_on_its_point` pins that a landed step is
*exactly* the point and not a rounding unit short of it.

Known and accepted:

- **Corners are still tile centres** (M2). The walk is now straight between waypoints, but a waypoint is the middle of a
  tile, so a unit still turns there and can graze a wall by up to half a tile. That is `short.rs` and the next slice.
- **Collision is unchanged**: bodies slide, one nudge per route, `push_units_off` untouched. The original's
  "no pushing allowed" is still an open question for the owner (D110 §4), not a decision taken here.
- **`Task::Hunt`'s arrival** is still `dx == 0 && dy == 0 && position == to` - not the `reaches` radius the other tasks
  use. It passes, and it is the next thing that will bite when a patrol's end is unreachable; named rather than changed,
  because it is a different bug with its own test to write.
- The acceptance match still wins at 36,000 ticks in both trees, so no scenario-based *outcome* test can tell the two
  apart - the diff is in the ledger (entities, repaths) and the hash.

---

*This file is what `tools/review_check.py` looks for: `## Performance` with a number, `## Stability` with content, staged
with the change. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
