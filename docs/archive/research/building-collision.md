# Building collision: how the game does it (research)

*2026-11-08. The owner asked, with a screenshot of a villager standing on a town centre's grey: *"How is the building collision
handled by the game? Can you research?"* This note is the answer, kept separate from the change it justified (D161).*

## 1. Collision is a **tile footprint** derived from the dat

A building's `radius[0]` and `radius[1]` in `units.toml` are its **half-extents in tiles**. Our pack, read straight from the dat:

| unit | radius | footprint |
|---|---|---|
| town centre (`RTWC`, 109) | `2.0, 2.0` | **4x4** |
| house (`HOUS`, 70) | `1.0, 1.0` | **2x2** |
| barracks (`BRKS`, 12) | `1.5, 1.5` | **3x3** |
| palisade wall (`WALL`, 72) | `0.5, 0.5` | **1x1** |
| stone mine (`STONM`, 102) | `0.5, 0.5` | **1x1** |

The path search blocks those tiles; nothing else about a building is collision. That is the whole rule, and it is what our
`footprint_of` and `solid_tiles` read.

## 2. The footprint is **not all solid**

The town centre is the case that matters. Ours keeps a **solid core with a walkable yard around it** (D115,
`the_town_centre_keeps_its_solid_core_and_its_yard`), and the same fact shows up independently in **aoe2mc**, a project that converts
AoE2 games and scenarios to Minecraft: its commit *"Let units walk under the town center"* adds *"the cells of its footprint that units
walk through ("open": the town center)"* to the object description.

So **a unit standing on a town centre's footprint is the game working as intended**: it is standing in the yard, not in the walls.
The owner's screenshot showed exactly that - and the bug was that our picture painted the yard the same grey as the walls.

## 3. The **sprite is bigger than the footprint**

In the original, a building's artwork overhangs its tiles substantially - a 4x4 town centre's sprite covers well beyond the 4x4 tiles
it blocks. That is why units are routinely drawn *behind* building art, and why the owner remembers *"the white diamond … a bit larger
than the grey diamond"*: the selection outline follows the **art**, not the collision box.

Consequences for a tile renderer like ours:

- art that overhangs the footprint is **correct** and is not a collision lie;
- an outline drawn *inside* the silhouette is **wrong**, because it draws a boundary that is not the boundary;
- a unit painted over a building's edge is normal when the unit is in front of it, and **wrong** when it is behind - which is what
  `depth_of` (front corner for buildings, `x + y` for bodies) decides.

## 4. What we do now

1. The engine blocks the footprint's **solid** tiles only, and the yard stays walkable (D115).
2. `ruage_view::solids_of` asks the **walk grid** per footprint tile, so the picture is the collision rather than a parallel claim.
3. Solid tiles are painted `BUILDING` grey, walkable ones `BUILDING_YARD` (D161).
4. The selection outline is drawn **two pixels outside** the footprint, so it reads as "around the building", as the game's does.
5. Depth sorting stays as it was: a building sorts by its **front** corner, so a unit behind it is covered and a unit in front of it
   covers.

## 5. What this research does not settle

- **How far the original's sprites overhang** (in tiles) is art-side; our renderer has no sprites yet, so there is nothing to match
  until there is an asset pack (D18).
- **Whether the yard's shape is a ring or a rect** on every big building: our core is a rect, taken from the engine's own rule; the
  original's per-building shapes are in its art, not in the dat.
