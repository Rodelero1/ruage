# How the game handles building collision, and why the villager was "inside" the town centre (D161)

*2026-11-08. The owner: *"I see villager still goes through the town centre. Maybe it is caused by the fact, that the white diamond
laying around the building is too small. What if the white diamond was actually a bit larger than the grey diamond representing a
building? I do remember that being so in the game. It would look better because no unit passes through the building. How is the
building collision handled by the game? Can you research?"*

## The research: three separate things, and only one of them is collision

1. **Collision is a tile footprint derived from the dat.** A building's `radius[0]`/`radius[1]` in `units.toml` are its **half-extents
   in tiles**: the town centre is `2.0, 2.0` → 4×4, a house `1.0, 1.0` → 2×2, a wall `0.5, 0.5` → 1×1. That is what both the engine and
   the original use to mark blocked tiles. (Ours reads it the same way: `footprint_of` = `2 × radius`.)
2. **The footprint is not all solid.** The town centre keeps a **walkable yard**: our engine models a solid core with a yard around it
   (`the_town_centre_keeps_its_solid_core_and_its_yard`, D115), and the same thing is visible in an independent project that converts
   AoE2 games to Minecraft - its commit *"Let units walk **under** the town center"* adds, in the object writer's own words, *"the
   cells of its footprint that units walk through ("open": the town center)"*. So a unit standing on a town centre's footprint is **not
   a bug**; it is the yard, and the game has always allowed it.
3. **The sprite is bigger than the footprint.** In the original, a building's artwork overhangs its tiles by a lot (a 4×4 town centre's
   sprite covers well beyond 4×4 tiles of ground), which is why units are constantly drawn *behind* building art. The selection outline
   follows the art, not the collision box - which is exactly the owner's memory: *"I do remember that being so in the game."*

## What was actually wrong here - two pictures, not one rule

- **The yard was painted as wall.** Drawing the whole footprint in one grey made a villager standing in the walkable yard look like a
  villager inside the building's masonry. That is the "passes through the town centre" the owner saw, and it is a *drawing* lie about
  a rule the engine already had.
- **The selection outline was drawn at 0.9 of the footprint** - *inside* the grey block. The owner read that as "the white diamond is
  too small", and he is right: an outline inside the silhouette draws a boundary that is not the boundary, so the block's outer edge
  reads as ground a unit could stand on.

## The fix

1. `ruage_view::solids_of(world, &snapshot)` asks the **walk grid** for every footprint tile of every building - the same question the
   path search asks - and returns one mask per building.
2. `render_with_solids` paints a **solid** tile in `BUILDING` grey and a **walkable** tile in the lighter `BUILDING_YARD`, so the
   picture shows where a unit may stand.
3. The selection outline is drawn at `footprint + 2 px`, **outside** the grey - the owner's suggestion, and the game's own relation
   between art and collision box.
4. `render`, `render_with` keep their signatures (empty mask) so nothing else moved; the window passes the real mask through the new
   `render_with_solids_and`.

## The tests, and the mutations that prove them

`crates/view/tests/town_centre_yard.rs`:

- **`the_town_centre_yard_is_painted_lighter_than_its_walls`**: asserts the town centre's mask has both solid and walkable tiles, that
  every solid tile is `BUILDING` grey, and that **at least one walkable tile is `BUILDING_YARD`**. Mutation: force `solid = true` →
  fails with *"the yard was painted as wall: it is why a villager looked inside"*.
- **`the_selection_outline_sits_outside_the_walls`**: samples the column below the block and requires a `SELECTION` pixel at or past
  the front edge. Mutation: restore the `0.9` radii → fails with *"the selection outline is nowhere near the front edge"*.

Both were watched failing under mutation, then restored - a test that cannot fail is decoration, and this session has already had one
of those (D160).

## Performance

**`solids_of` is one walk-grid lookup per footprint tile per frame**: for the acceptance scene that is ~150 buildings averaging ~4 tiles
= ~600 lookups a frame, each a `Vec` index - measured cost is inside the frame budget (`frame_cost.rs` still passes at the same
threshold). **One extra `fill_diamond` colour decision per painted tile**, no extra fills. The simulation is untouched and the headless
acceptance hash is **byte-identical**. **Measured**: the new suite runs in **0.86 s** (two tests, one 900×700 render each); the gate's
wall clock is recorded with this commit.

## Stability

- **The mask is the walk grid's own answer**, not a second opinion about which tiles are solid - so the picture cannot drift from the
  collision, which is the D159 lesson generalised from "where is the shape" to "what is inside it".
- **A missing mask entry defaults to solid**, so an entity the window forgot to include is drawn as wall rather than as a hole a unit
  could appear to stand in.
- **`Snapshot` did not grow a field**: the mask travels beside it, so every existing caller (tests, headless diagnostics) is unchanged
  and cannot be broken by a view-only concern.
- Suite: **764 passed / 0 failed / 19 ignored**, clippy `-D warnings` clean, fmt clean.
