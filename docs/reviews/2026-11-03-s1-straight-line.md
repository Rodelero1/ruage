# Review: S1 - the straight-line shortcut

*Change: the original's first attempt at any movement (`doIntelligentMovementUpdate`, move_obj.cpp 4,079-4,245) -
`path::line_clear` walks the Bresenham line between two tiles and, if every tile on it is walkable **except the goal's
own**, the route is the goal alone instead of an A\* path. `World::plan_path` asks it first; a blocked line costs one
line walk and then the search it would have paid anyway. Three tests in `crates/sim/tests/straight_line.rs`.*

## Performance

**The number that matters is that it did not make the match faster.** The acceptance run at 36,000 ticks took **12.66 s**
wall (~0.35 ms/tick) where E1a took 10.8 s: fewer searches, but straighter routes walk villagers into each other more
often, and each collision can buy a repath. The outcome is unchanged too - **`players=2`** again, hash
`c4e63f8ff29c8b0a`, and the economy is ahead of E1a late (398 food at 1,600 s against 337) while behind early (5 wood /
10 food at 400 s against 95 / 0). So the shortcut is right by the original's shape and **is not the lever that recovers
the victory**; that has to be said rather than implied by a green suite.

What the shortcut does buy, exactly and measurably: an open six-tile walk stores **one** waypoint where the search stored
six (`an_open_line_is_a_single_waypoint`), and a blocked one stores more than one and goes round
(`an_obstacle_across_the_line_forces_the_search`). The line walk is O(tiles on the line) against an A\* that expands the
map; the wall-clock above is the honest end-to-end figure and it moved the wrong way.

## Stability

- **The goal's own tile is exempt**, which is the original's *non-target* qualifier: a node's tile holds the thing being
  worked and a body you were ordered onto is a target, not an obstacle - `the_goal_tile_does_not_block_its_own_line`
  pins it.
- **The corner rule matches the search's**: a diagonal step needs both of its corners free, so a line that would clip the
  corner of a wall is not a line a unit may walk. A line that clips what the search would refuse cannot route a unit
  through terrain the grid blocks.
- **A blocked line is never worse than before**: the fallback is the same `find_path` call, with the line walk spent on
  top of it.
- **Two fixture traps found while writing the tests, both in the tests:** `next_id` called twice before a step returns
  the same id twice, so the second spawn is refused as a duplicate (the same trap the evasion tests hit - the ids are now
  taken up front); and **a spawned building is a foundation, which blocks nobody until a villager first works it** (D61),
  so an obstacle the line must respect has to be a node carrying hit points - a tree - not a house.
- **Hash moved again** (`c4e63f8ff29c8b0a` from `7c0b9132ed2074f8`): routes are different geometry now, which is what the
  slice is for.

Known and accepted: the shortcut is tried for **every** `plan_path`, including approach routes to nodes and targets, so
a unit ordered onto a body walks straight at it and stops beside it - the arrival rules handle the stop, which is why
that case needed no special code.

---

*Staged with the change for `tools/review_check.py`. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
