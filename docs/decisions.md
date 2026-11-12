
### D189 - your own units pass through each other on the way

**The owner chose it**, between the two ways out of the crowd problem: *"1"* - bodies may overlap while walking, collision at rest
only. His words for the need were *"I think that capping is pointless"* and the rule behind it, *"either villager fits in or
don't"*.

**The defect it fixes, measured.** With bodies solid, ten men sent at one bush cannot be delivered: the men behind cannot pass the
men in front, and the settle - which moves a body **directly**, not through the collision pass - either stepped inside a neighbour
(closest pair **0.144**, then **0.345**) or left a man stranded outside the band (**1.543**, **1.962** tiles from the bush, never
working). Whichever step direction was tried, one of the two failed.

**The rule.** A unit with **more than one waypoint left** ignores other **friendly** bodies: it takes the step it planned, and two
of the player's own villagers may overlap in transit. **The last step still respects a body**, because the final approach is where
order matters: without that bound a builder stacked on an idle villager and never reached its footprint (a house at **hp 1** after
2,000 ticks), and arrivals became a race rather than a queue. **Enemy bodies are unchanged**: `bound_to_reach` still holds two
players' units apart, which is the pass D131 was about.

At rest, the gather ring keeps men apart by reservation rather than by collision: a place is a point on the ring one body (0.40)
clear of every other place, and the settle walks a man exactly onto his place (`off_place` is any deviation, not a tolerance - a
0.125 tolerance let two men finish **0.376** apart, and 0.025 let them finish 0.345).

**Measured on landing:** ten men work one bush (nine was the ceiling), closest pair of places **0.408**; eighteen work one tree at a
body each, which is `ring_capacity()`; the acceptance hash and counters are in the review.

### D191 - tile geometry: the border is what a villager stands against

**The owner:** *"Do we know coordinates of the tiles and their corners? If not, it should be added. Because all work of villagers
depends on that (maybe except taking meat from animals) ... Villagers building or gathering resources (except meat from animal)
must be aligned with borders of the tiles, not center of it."* And on size: *"center stays at least half the diameter of the circle
(however big that circle is)."*

**Why the centre was wrong.** Every standoff so far measured from the tile's **centre** - 0.70 (D169), then 1.156 (D186/D188). His
saves show men at **0.70** from a resource tile's centre, which is `0.5 + 0.2`: a villager's circle **touching the tile's border**.
Two edges meeting put the same circle at `0.707 + 0.2 = 0.91`, and his saves carry men there too (0.76-0.99). The border is the
thing the circle touches; the centre was a proxy that happened to be right only for one particular row.

**The new module** (`crates/sim/src/tiles.rs`, public):

- `tile_corners(tile, scale)` - the four corners in raw map units;
- `distance_to_tile(point, tile, scale)` - zero inside, the distance to the nearest border point outside. "The circle may never
  overlap the tile" is then one comparison: `distance >= radius`;
- `touching_places(tile, radius, spacing, scale)` - the points where a circle of `radius` touches the tile without overlapping, at
  least `spacing` apart, in a **fixed order** (the simulation is hashed; a geometry that reorders itself is a desync waiting).

**The capacity falls out of it, for the first time without a number in a table.** Measured with the module: a villager (radius 0.2,
spacing 0.4) gets **eleven places** on one tile - the outline is `4 + 2 * pi * 0.2 = 5.26` tiles and a body takes 0.4 - a unit with
radius 0.4 gets **six**, and one with 0.05 gets forty-one. The owner's *"max 9 per each"* and the ten men his save shows on one bush
both sit inside eleven.

**Not yet switched.** The gather and build standoffs still measure from the ring and the centre; this commit adds the geometry and
its tests only, because moving the standoff changes every gather test's band and is its own slice (`docs/next-step.md`).

### D192 (parked) - the standoff is the tile's border, not a ring round its centre

**What it says.** A worker stands with **his circle touching the node's tile**: `distance_to_tile <= radius`, places `spacing` apart
along the outline, chosen nearest-first. His saves measure it - men at **0.70** from a resource tile's centre (`0.5 + 0.2`) and
**0.99** where two edges meet - against the 1.156 ring D186 read from the row *behind* them. The capacity follows from the geometry
(**eleven** places for a villager) and the outer band disappears: no place on the tile, then another resource within four tiles.

**Why it is parked rather than merged.** Every gather test asserts it and the acceptance run improves (`made_room` **96 -> 0**), but a
build test fails: a villager told to step off a ghost oscillates on its edge for 2,000 ticks while the builder walks at the same tile.
That is a movement deadlock the standoff exposed, and it must be understood before the rule ships. See
`docs/reviews/2026-11-13-touching-standoff.md`.
