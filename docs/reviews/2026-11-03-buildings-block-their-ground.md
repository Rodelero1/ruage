# Review: buildings block their footprints (and the AI that cannot live with it)

*Change: a building occupies and blocks its **whole footprint** from the tick its foundation exists - not only the
palisade wall, which was the only role content named in `[build] blocking_roles`. A farm stays a floor (its mask is all
`.`), a town centre keeps its solid core and walkable yard (its mask), and placement refuses any overlap between
footprints, with an expired farm the single exception. Two supporting rules came with it: a raw-space diagonal step may no
longer cut the corner of a blocked tile, and the placement search asks whether a **whole footprint** fits, with a lane
around it.*

Owner's observation, 2026-11-03: *"Villagers pass through the buildings. only farm and partially Town Center can be walked
over when done, and only farm can be passed over during building it. All other buildings block passage since start of
construction. Building may not overlap in any case (only expired farm can be)."*

## Performance

**The rule works and the match does not.** The acceptance run at 36,000 ticks, against the landed baseline (`c979ba7`:
`players=1`, hash `912386556784c88f`, 19.9 s):

| | baseline | with buildings blocking |
|---|---|---|
| outcome | **`players=1`** (won at 1,667 s) | `players=2` - not won |
| hash | `912386556784c88f` | `c311cfb2158c03c6` |
| bank at 1,600 s | 192 wood / 654 food / 110 gold (after spending on the win) | 39 / 258 / 370 |
| entities / buildings | 153 / 49 | 164 / 50 |
| repaths / nudges | 100 / 166 | 101 / 200 |
| wall clock | 19.9 s | 47.4 s |

The 2.4x wall clock is the same effect as the bank: the base is bigger and busier per tick (50 buildings, more pathing
around them), not a slower movement pass.

**Four iterations, each measured, because each one moved the number:**

1. **Just the rule** (block the footprint, refuse overlap by footprint): the housing rule asked for the town centre's own
   ground **450 times** and the match froze at 8 units / 2 buildings, 200 wood. `is_free_ground` asks about *one tile*,
   so the search kept choosing a tile whose neighbour the town centre already owned.
2. **Footprint-aware search** (`site_fits`): 9 buildings, 20 units, and *"no walkable route ... the target is walled
   off"* - the search now packed every house against the town centre, and the village was a walled enclosure.
3. **A lane around each blocking building** (`site_has_lane`): 29 buildings, food starved at 238 - because the lane was
   demanded of **farms** too, and *"no free tile for a farm"* fired 85 times.
4. **A floor needs no lane** (a farm blocks nothing) **and a wider farm search** (`free_tile_near_node` scanned 4 tiles,
   too few for a 3x3 plot): 50 buildings, 164 entities, food 258, wood 39 - the economy spends everything on the base and
   still does not win.

**Where it stands:** the rule is implemented and tested, and the acceptance match - the project's standing measurement -
**regresses from a won match to a drawn one with a drained bank**. The cause is not the rule: it is that the AI's base
layout, its farm spacing and its food economy were designed when a building was a picture. That is the next slice, and it
is bigger than this one.

## Stability

- **Seven tests, all watched failing first** (`crates/sim/tests/building_passage.rs`): a finished house blocks all four
  tiles of its footprint; a villager ordered past it never stands on it and still arrives; a **foundation** blocks from
  the tick it exists; a farm is a floor while it is built and after, and a villager stands on it; the town centre's core
  is solid and its yard is walkable (four of sixteen); a house on the town centre's yard is refused `SiteOccupied`; a
  live farm's floor is not buildable-over while a drained one is.
- **The corner clip was a real defect the rule exposed.** With houses blocking, a villager walked *through* a house:
  the route is tile-legal, but the mover moves in **raw space**, so a diagonal step toward the next waypoint entered a
  blocked tile's corner the Bresenham line never touches. `step_is_walkable` now refuses that step and slides along the
  axis that is clear - the same both-corners rule the search uses - with the **goal's own tile exempt**, so a gatherer can
  still step onto the tree it works.
- **Ownership is not blocking.** `footprint_area` (what a building owns, for overlap) and `solid_tiles` (what blocks
  movement) are separate questions now: a farm owns its floor and blocks none of it.
- **The sweep stopped asking every building for its tiles every tick** - only a building with `hp <= 0` needs its ground
  freed, and with whole footprints that lookup would have been sixteen tiles a building a tick for nothing.
- **The suite is red, and that is why this is parked on a branch.** Two classes of failure, both honest consequences:
  - **~25 build/foundation fixtures place a building so that its footprint overlaps the town centre's** (the anchor tile
    was clear, which is all the old rule asked). Refusing them is the new rule working; each fixture has to move to
    ground it does not share, and moving a fixture changes what that test measures.
  - **One real defect of mine, fixed here:** `step_is_walkable` refused every step in a world with **no walk layer**, which
    is what a bare `World::new` test world has - two mover tests went red until it was taught that a world with no map
    knows no blocked ground.
  - One ordering change: `an_unreachable_site_is_refused_before_the_wood_is_spent` now gets `SiteBlocked` where it wanted
    `NoPath`, because the site check runs before the reachability search.

---

*Staged with the change for `tools/review_check.py`. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
