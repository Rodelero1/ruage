# Review: the pather can be told to route around a tile that is not terrain

*Change: `find_path_avoiding(walk, from, to, avoid)` - the A\* search with a per-call set of tiles to treat as
impassable - and `find_path` becomes its wrapper with an empty set. This is step 1 of D98a (the owner's ruling: adopt
the original's evasion, `doIntelligentMovementUpdate`'s "repathing" on a non-past collision, move_obj.cpp
4,600-4,760), and it is deliberately the only step in this commit: **nothing calls it yet**, so behaviour is unchanged
and the wiring - collision detection at the new position, the per-route avoid set in state, the repath - is the next
slice, planned in `docs/next-step.md` under "Evasion, the original's shape".*

## Performance

Measured on the project's own 64×64 map, corner to corner (16,16) → (47,47), release, 20 repetitions each, with the
avoided tile taken from the plain path's own midpoint so the comparison is a real detour rather than a failure:

- **plain `find_path`: 28 µs per search**
- **`find_path_avoiding` with one tile: 125 µs per search** (4.5×)

The difference is node expansion, not the set lookup: one obstacle on the diagonal forces A\* to explore a second lane
instead of walking the octile line, which is the expected shape for a 10/14 grid. 125 µs is 0.25% of a 50 ms tick at
20 Hz, and a repath is only paid when a collision is *new*, so the per-tick budget is untouched in a world where nobody
is blocked. The probe asserts the detour search **succeeds** before it times it - a timing over a failing search would
have measured something else entirely, which is the first version of this probe (120 µs, no assert) and worth stating.

## Stability

- **A unit is not terrain, and the search keeps that boundary.** The obstacle set lives for one call, so two units
  crossing do not poison each other's plans, and `Walk` still knows only terrain, walls and buildings.
- **The corner rule follows the obstacle**: a diagonal step needs both of its corners open, and an avoided tile closes
  the corner the same way a wall does. Without it a unit would clip through the corner of the body it is routing
  around - and the test in this commit is exactly what catches the plain-tile version: **mutation-proved**, deleting
  `avoid.contains` from the step test fails `a_search_can_be_told_to_avoid_a_tile` with the route printed straight
  through the forbidden tile (`"the forbidden tile is not on the route: [(1, 0), (2, 0), ...]"`).
- **The goal may be in the avoided set**, for the same reason a blocked goal is allowed: a node's own tile is occupied
  by the thing being worked, and a repath toward a target must not refuse just because the target is a body.
- **No state, nothing hashed**: the set is an argument. The slice that keeps it (per route) is the one that adds a
  hashed table, and `docs/next-step.md` says so.
- **The test derives its own obstacle** from whatever the pather chose, so it does not encode map coordinates that a
  content change would invalidate - the same reason the fixture's tiles are searched rather than hand-picked.

Known and accepted: the capability has **no caller in shipped code yet** (only the test and the probe). That is the
dead-knob shape this project has been bitten by, and it is accepted here for one commit only because the next slice's
first line calls it; if that slice does not land, this function goes with it rather than lingering.

---

*This file is what `tools/review_check.py` looks for: `## Performance` with a number, `## Stability` with content, staged
with the change. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
