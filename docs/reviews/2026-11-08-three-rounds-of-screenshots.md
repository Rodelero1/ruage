# Three rounds of screenshots, and what I could not reproduce (D163)

*2026-11-08. The owner, third time: *"Issue is still here"* - with a picture of a town centre whose structure is dark grey, its yard light
grey, and **three blue villagers on the dark part**.*

## What I did to the picture, and why

Two changes, both of which follow from the owner's own model:

1. **The yard is painted as ground.** The walkable part of a footprint was painted in the *entity* pass, which runs back to front by depth.
   A building's depth is its **front** corner, so a building is painted after every unit north of it - and a unit standing in the yard
   (legal, D115) was painted over by the building's own yard tiles. It is now painted with the terrain, **before any entity**, so a unit in
   the yard is drawn *on* the yard. Only the **solid** tiles remain in the entity pass, so a unit behind a wall is still hidden by it.
2. **A debug switch**: `RUAGE_DEBUG_TILES=1` prints, once a second, every unit's tile and whether the walk grid accepts it, plus every
   building's solid mask to stderr.

## What I could **not** do: reproduce the screenshot as a failing test

`crates/view/tests/yard_drawing_order.rs` asserts the property the owner is describing: a villager standing on a yard tile of the town centre
is not painted in `BUILDING` (structure) and not left as `BUILDING_YARD` (covered by the yard). It **passes**, and - the honest part - its
**mutation did not fail**: putting the yard tiles back in the entity pass left the test green. So this test is a **guard, not a proof**: it
does not reproduce the picture.

Three rounds of "the villager is inside the building" and I have now tested and could not reproduce:

- a route planned before a building appears walks through it (D158: goes round);
- the player's `WalkTo` cutting through (D158: no);
- the builder left inside its own house (D158: no);
- a miner on the node's own tile (D158: no);
- drawn footprint ≠ blocked footprint (D158/D159: equal);
- the diamond half a tile off (D160: was real, fixed);
- the yard painted as wall (D161: was real, fixed);
- a unit in the yard painted under the building (D163: plausible, fixed by drawing order, **not proved**);
- a unit's **centre** on a solid tile (D162: asserted impossible for 1,200 ticks).

At this point the pattern says the remaining case is in a world state I have not built - the live match, with the scenario's own scripted
orders - and pictures cannot carry it.

## What would settle it, and it is now one command

```
RUAGE_DEBUG_TILES=1 cargo run --release
```

Then, at the moment the villager looks inside the building, the stderr lines say exactly:

```
DBG tick=… unit=… tile=(x, y) walkable=true|false pos=(raw_x, raw_y)
DBG tick=… building=… unit=Some(id) tile=(x, y) mask=[…]
```

- `walkable=false` on a unit's own tile → a **collision** bug, and the mask line says which building's tile it is.
- `walkable=true` and the tile is inside the building's footprint → the **mask** is wrong for that building, and the mask line shows it.
- `walkable=true` and the tile is *outside* the footprint → the unit is legally beside the building and the remaining fault is **drawing**
  (its square overlapping the neighbour's diamond), which is a sprite-size question: our unit is a 0.4-tile square drawn at its centre, and
  the game's answer is a sprite that is smaller than its tile and drawn with a shadow at the feet.

## Performance

**No simulation change**; the acceptance hash is byte-identical. The yard pass moves work, it does not add it: those diamond fills happened
either way, once per yard tile. `RUAGE_DEBUG_TILES` is read once per frame (an env lookup only when set) and prints once a second.
**Measured**: the new suite runs in **0.76 s**; the full gate is recorded with this commit.

## Stability

- **The drawing order is now principled rather than incidental**: ground, then yard, then entities back to front, then highlights. A unit can
  only be covered by something that is actually in front of it.
- **The debug switch writes to stderr only when asked**, and never touches the simulation, so a replay is unaffected.
- **What this does not claim**: that the owner's screenshot is fixed. It claims two changes that follow his model, and one command that turns
  the next report into numbers.
- Suite: **767 passed / 0 failed / 19 ignored**, clippy `-D warnings` clean, fmt clean.
