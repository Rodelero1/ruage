# Review: a rallied unit is born inside, and the doorway machinery goes

*Change: a unit rallied to its own building is garrisoned outright at completion - no walk, no distance check - because it was
born there; `Command::Rally` treats a point anywhere in the building's **footprint** as "inside"; `check_garrison` measures
distance to the **footprint** rather than the anchor tile; and the `entering` machinery that existed to walk a unit through a
door it was born behind is deleted. D220.*

## Performance

**Removes work rather than adding it, and the saving is in simulated seconds.** The old path spawned the villager, aimed it at
the building's unwalkable anchor tile and left it **orbiting for 100 ticks - 5 s of game time** (measured: (14, 16) → (13, 16)
→ (14, 15) → (15, 15) and round again); with a free door tile it instead **stalled for 35 ticks (1.75 s)** against a scout at
(15, 16) before the blocked-order rule finished the order. Both are gone: a rallied unit is garrisoned on the tick it is
trained, and the two rally fixtures that failed for 600 ticks now finish inside one. On the tick path the change is a **deletion** - `step_is_walkable` loses its `entering` parameter and the
`self.entering.contains_key` lookup per step, and `resume_idle` loses a `BTreeMap` removal per finished job. The one addition
is a pass over a building's footprint (at most a 4x4) inside `check_garrison`, which runs on a garrison **order**, not on a
tick.

## Stability

- **Three defects in one path, each found by measuring rather than reading.** (1) The goal was the building's **anchor** tile,
  which is inside the footprint and cannot be stood on, so the unit orbited it - the same anchor-versus-footprint confusion
  that `check_garrison` had. (2) Aiming at a free tile instead exposed a second one: the walk had **no planned route** (the
  path set `Task::Move` directly), so a single body on the corner refused every diagonal and the order stalled until the
  blocked-order rule ended it. (3) Ending it two tiles short made the arrival check refuse the unit as `TooFar`, because that
  check measured from the anchor too. Each fix alone left the fixture red; the rule that settled it was the simplest one -
  a unit born at a building is inside it.
- **The parked fixture was the specification and stayed unedited.** `rally::a_unit_rallied_to_its_own_building_stays_inside`
  was red since `e21b44d` with its measurement in the `#[ignore]` sentence; the fix un-parks it unchanged. Nothing was
  "restated" to make it pass.
- **A second fixture covers the half the first one could not see.** The parked test rallies to the anchor tile; the new
  `a_rally_onto_another_tile_of_the_building_is_the_building` rallies to (x+1, y+1) - inside the 4x4, outside the anchor - and
  fails on the old comparison. Two tiles, two defects.
- **The forest-pocket fixture had to move, and that is a finding in itself.** Its "unreachable pocket" at (4, 4) sits inside the
  4x4 footprint of the town centre anchored at (1, 1), so under the corrected rule that rally meant "stay inside" and the
  villager garrisoned instead of walking to the wood. The map now puts the pocket at (3, 6) - clear of the footprint - and the
  comment says why, so the next reader does not restore the old coordinates.

Known and accepted:

- **`World.entering` is gone.** Nothing else wrote it, and a side table with no writer is a lookup every mover pays for. Its
  removal is part of this slice, not a follow-up.
- **Garrison capacity and "can this building hold units" are still checked**, so a production building never keeps a unit it
  did not just make (D38), and a full building leaves the unit standing outside.
- **`RallyPoint::Inside` still exists as state** and is still what a rally to "inside" records; only the walk to reach it is
  gone.

---

*This file is what `tools/review_check.py` looks for: `## Performance` with a number, `## Stability` with content, staged
with the change. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
