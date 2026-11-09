# The units were drawn above the walls on purpose, and that purpose expired (D165)

*2026-11-08. The owner, with the first real data: *"Units are able to walk over the buildings at the top side but cannot get close to the
buildings from the bottom side. Therefore it seems that there is a shift between graphics representation and path of units."* Plus a log that
shows only `building=` lines.*

## The "shift" is not in the simulation - measured twice

- **Constructed houses**: `built_house_alignment` builds a real house with a villager through `Command::Build`, waits for it, then asserts that
  **every drawn tile is blocked** and **every ring tile is free**. It passes, so the picture and the collision agree for a building a villager
  raised, not only for one placed by a command (D159's case).
- **Approach from both sides**: `approach_symmetry` puts two villagers three tiles north and south of a house, sends both to a tile one step
  from it, and checks where they end. Both arrive **exactly** on their goals: `north (30,29)` for goal `(30,29)`, `south (30,33)` for goal
  `(30,33)`. There is no north/south asymmetry in pathing.

## What *is* asymmetric: the paint

`layer_of` put **every unit above every building** - a deliberate rule from 2026-11-04, in the owner's words then: *"villagers and units should
be always visible over the buildings/farms"*. The reason was sound: farms were drawn as **floors**, and a villager behind a farm vanished
into it. The cost is the report here: a villager **north** of a house is painted **over its wall**, so it reads as walking through the
building - and a villager **south** of it, being in front, is painted correctly but stops one tile out, so the pair reads as a shift.

So the same rule produced both halves of the observation, and neither half is a collision bug.

## The fix

`layer_of` now gives buildings, units and projectiles **one layer**, so depth decides: a wall in front covers a unit behind it, and a unit in
front covers the wall. Resource nodes stay below everything (a bush is a short thing; the worker behind it must still be seen).

The reason the old rule existed is gone: **a farm's floor is painted as ground** since D163, because it is walkable, so no floor can hide
anybody.

## Tests, including two that had to be reversed

- `the_drawn_footprint_is_the_blocked_footprint` (D159) and `built_house_alignment` (new): the picture is where the collision is.
- `approach_symmetry` (new): north and south arrivals are identical.
- `a_unit_behind_a_wall_is_covered_by_it`: **reversed** from `a_unit_behind_a_building_is_still_painted`. The old test asserted the 2026-11-04
  rule; it now asserts the opposite, and says why in its comment. A test that encodes an owner decision has to be rewritten when the owner
  changes the decision - not quietly deleted.
- `things_are_painted_from_the_back_of_the_diamond_to_the_front`: its expected order is now **computed from the depths** rather than typed in,
  because the typed-in vector was the old layer rule written as a literal.

## About the log

The `unit=` lines were there - they are printed **before** the `building=` lines in the same block, so `tail -f` showed only the buildings.
`grep ' unit=' tiles.log | tail -20` is the command.

## Performance

**Simulation untouched**: the acceptance hash is byte-identical, and every change is in `ruage_view`. **Measured**: the depth suite runs in
**0.74 s**, `built_house_alignment` **1.11 s**, `approach_symmetry` **0.85 s**; the gate's wall clock is recorded with this commit.

## Stability

- **One ordering rule instead of two**: ground (with yards), then everything by depth, then highlights. There is no "layer" left that can
  contradict depth.
- **What this trades away, explicitly**: a unit behind a tall building is now hidden where it overlaps it. That is what "no unit passes through
  the building" means in a flat-shape renderer; the game gets the same effect from sprite height.
- Suite: **769 passed / 0 failed / 19 ignored**, clippy `-D warnings` clean, fmt clean.
