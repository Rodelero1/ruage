# The drag on screen: two clicks, a green ghost and a red one (D156)

*2026-11-08. The owner asked for the UI drag after D155, which shipped only the sim side. His rules, unchanged: **first left click
starts the run, second click ends it**, walls run *vertically, horizontally and at 45 degrees*, and *"segments overlapping
ineligible area getting red tone"* - a semi-transparent preview, red where the ground will not take it.*

## What shipped

**`ruage_view::placement::Placement`** - the whole rule, with **no window in it**, because a `winit` loop cannot be driven from a
test:

- `new(villager, unit)` → the run is *awaiting its first click*, and **nothing is drawn**;
- `click(&world, tile)`: the first press **starts** the run and returns `None`; the second **places** it and returns the command;
- `hover(tile)` lengthens the run - and does nothing before the start, because there is no run;
- `segments(&world)` → every tile in drag order with `valid: bool`;
- `tiles()` is `World::wall_line`, **not a second implementation**: the ghost and the command cannot disagree (D155).

**`ruage_view::draw_placement(frame, camera, segments)`** - a diamond per segment: **green** `[120,220,140]` where the ground will
take a wall, **red** `[230,80,70]` where it will not, filled by the new `Frame::blend_diamond(…, weight)` at 96/256 and outlined
in the same colour. The blend is integer-only (`>> 8`) so two machines draw the same picture.

**The window** (`crates/view/src/bin/window.rs`): `B` starts a placement for the first selected thing, the map's left click is
owned by the placement while one is up (it does not select anything), the pointer's tile feeds `hover` from `CursorMoved`,
**Escape cancels a placement before it opens the menu**, and the ghost is drawn over the ground, under the HUD.

## The tests

`crates/view/tests/placement.rs`, four tests:

```
the_first_click_starts_the_run_and_the_second_places_it   ok   (compile-RED first: the type did not exist)
a_taken_tile_is_red_and_the_rest_of_the_line_is_not        ok
a_run_with_no_valid_tile_places_nothing                   ok   (RED first: my test called `.expect` on a first click that
                                                                returns `None` *by design* - the bug was in the test)
the_ghost_is_drawn_in_two_colours                         ok   (proved by mutation, below)
```

The first is the contract: after the first click the run is **one tile**, after the hover it is **the five tiles under the
pointer**, and the command's `tiles` equal `shown` **exactly** - the ghost and the order are the same vector.

**`the_ghost_is_drawn_in_two_colours` passed on arrival**, so it was proved by **mutation**: making `draw_placement` ignore
`valid` and always use the green made it fail (`a refused segment is drawn red-dominant`), and the mutation was reverted. A
drawing test that cannot fail is decoration.

## Performance

**No change to the simulation**: the acceptance hash is byte-identical again, `45596da62c9e3668`, 135 entities, 125 repaths,
116 made-room. The view work is per-frame and only while a placement is up.

**Measured**: the placement suite (four tests, including two full world seeds and a 1280×720 render) finishes in **1.78 s**. The
ghost's cost is one `blend_diamond` + one `outline_diamond` per segment, over the same clipped scanline walk `fill_diamond` uses -
for a hundred-tile drag that is ~200 diamond fills a frame, and only between the two clicks. `segments()` asks
`World::is_free_ground` **twice per tile** per frame (once in `tiles()`'s caller, once directly) - a map lookup, not a search; if
it ever matters the fix is one `Vec` reused, not a cache.

## Stability

- **The all-red case is a state, not an error**: a second click on a line with nothing valid in it returns `None` **and leaves the
  placement up**, so a botched drag costs a click rather than the placement. Tested.
- **A placement does not select, and a selection does not place**: while `placement` is `Some`, the map click never reaches
  `pick_at`; the HUD keeps its own clicks because the placement is asked *after* `ui::over_hud`.
- **Escape order matters**: cancel first, menu second - a menu drawn over a live ghost is two ways to say "never mind".
- **Provisional, and labelled as such**: `B` is the only way in until the villager's build card exists; the notice says what it
  does rather than leaving a key nobody knows. Every other building still has no placement path, which is the honest gap.
- Suite: **753 passed / 0 failed / 19 ignored**, clippy `-D warnings` clean, fmt clean.

## Not done, deliberately

- **The villager build card**: placement is reachable by key only. That is the next slice and the one a player will notice.
- **Ghost of a *building*** (not a wall): the state machine is wall-shaped; a single-building placement is the same two clicks with
  a one-tile line, so it is cheap once the card exists.
- **Sound, cursor change, and the cost readout while placing** - interface polish, not rules.
- **Rotating a diagonal gate / wall variant**: content work, still only `palisade_wall` (D155).
