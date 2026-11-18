# D273 - the click was half a tile out, and it was the view's half tile: review

*The owner, twice: **"Selecting a unit and right clicking somewhere will not send the unit to the place I chose but a bit above it"**,
and after the crowd fix: **"No, the unit never goes to the exact spot. The issue is exactly as it was. Center of the unit circle is not
aligned with the spot (not even close)."***

## The defect

The view has **two** conversions between a screen pixel and the world, and they disagreed by exactly half a tile:

- **drawing**: `project(drawn_at(position))` - `drawn_at` subtracts `0.5` because a world position and a projected tile differ by half
  a tile (the projection centres each tile's diamond on the tile's *point*, while a simulation tile covers `x .. x + 1`).
- **ordering**: `pick_point(pixel)` returned the projection's space **unconverted**, and the right-click handed that straight to
  `Command::WalkTo`.

So clicking the very pixel a body was painted on ordered a spot **half a tile up and to the left** of it - not a rounding error but
the half diamond: **24 pixels** at `tile_pixels = 96`, measured.

## The fix

`Camera::world_point(pixel)` - `pick_point` plus the half tile, **named for what it is**, with `pick_point` left as the projection's
own inverse (the camera fixtures depend on that, and they are right). Both order paths use it: `ui::Selection::order` (the HUD's
right-click) and `window.rs`'s `cursor_point`. Nothing else changed: every drawing site already used `drawn_at`.

## Performance

Nothing runs per tick: both changes are a `+ 0.5` on a value computed **once per click** - nanoseconds, not measurable above noise.
The number that matters is positional: the click round trip went from **24 pixels** of error to **0 pixels** across a thirteen-by-nine
grid at `tile_pixels = 96` (24 px is exactly half a diamond, i.e. 0.5 tiles; the tolerance in the fixture is 1 px). The full suite
runs in **202 ok / 1 failed**, the same as before the change, so the click fix costs the test run nothing: **0 ms** of measured
difference (`--no-fail-fast` wall time is dominated by the sim fixtures and moves with the machine's 24% spread).

## Stability

- New fixture (`crates/view/tests/click_round_trip.rs`, **unignored**, written first and watched fail at **24 pixels**): click a
  grid of pixels, take `world_point`, draw it back with `project(drawn_at(..))`, and require the same pixel.
- `picking::a_right_click_orders_the_point_under_the_cursor` **amended**, and it is worth naming why: it asserted that a click on the
  diamond of tile-space point `A` orders `A` - which **is** the bug, since a body at `A` is drawn half a tile away from that diamond.
  It now asserts the invariant (the order equals `world_point` of the clicked pixel) rather than a constant that encoded the offset.
- All **41 view test binaries green**, including `camera`, `unit_centring`, `feet_on_the_point`, `ring_at_the_feet`, `picking`.
