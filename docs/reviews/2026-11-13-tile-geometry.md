# Tile geometry: corners, borders, and where a circle touches

The module the owner asked for, with its tests. **No standoff has moved yet** - this is the foundation, and moving the gather and
build standoffs onto it is the next slice.

## Performance

Not on the tick path yet: nothing calls the module, so no match changes and the acceptance run is not re-taken. The four tests run
in **0.01 s** in total. `World::step` is untouched.

## What it gives

`ruage_sim::tiles`, integer-only so `no_float` keeps holding:

- `tile_corners(tile, scale)` - four corners in raw map units, anticlockwise from `(x, y)`.
- `distance_to_tile(point, tile, scale)` - **zero inside**, otherwise the distance to the nearest border point. With it, "a circle may
  never overlap the tile" is one comparison: `distance >= radius`.
- `touching_places(tile, radius, spacing, scale)` - where a circle of `radius` touches the tile without overlapping, at least
  `spacing` apart, in a fixed order.

**Measured with it:** a villager (radius 0.2, one diameter apart) has **11 places** on one tile; a radius-0.4 unit has **6**; a
radius-0.05 unit has **41**. The owner's *"max 9 per each"* and the ten men his save shows on one bush both sit inside eleven, and no
constant was written down to get there.

## Stability

**The tests bite - proved by mutation, because they never went red on their own** (the module did not exist, so the first failure was a
compile error, which proves nothing about behaviour):

- `distance_to_tile` forced to return zero: **4 of 4 tests fail**;
- `touching_places` forced to a zero radius: **3 of 4 fail**;
- restored: **4 pass**.

That deviation - implementation and test in one commit, because the API did not exist to test against - is stated rather than glossed.

**Determinism is asserted**, not assumed: the same tile must give the same list in the same order, because the simulation is hashed and
the ring this replaces is about to be read inside a tick.

**What is deliberately not done.** The gather ring (1.156) and the builder's standoff still measure from the tile centre. Switching
them changes the band every gather test asserts, rewrites the places the owner's saves measure (0.70-0.99), and needs its own
before/after numbers; doing it in the same commit as the geometry would make both unmeasurable.

**Formatting.** `cargo fmt --check` was red on the first push (the module and its test needed rustfmt's line breaks); `cargo fmt --all`
applied and re-checked clean, in this commit rather than the last so the history says what happened.
