# The HUD that was written and never drawn

**Step:** A3 of the owner's queue (2026-11-03): the HUD that already existed gets drawn.

**What was wrong.** `ui::Hud::of` (stock with the pack's resource names, population, a row per selected thing, the match
outcome) and `ui::minimap` (the world at two pixels a tile, fog included) were both written, both tested - and the window
called neither. It drew the top bar and the speed selector and nothing else: a player could box six villagers and see no
sign of what was selected, and had no map. Tests of a function nobody calls prove the function, not the game.

---

## What is drawn now

- **The top bar**, unchanged, but fed from the `Hud` rather than straight from the snapshot, so the numbers a player reads
  and the numbers a test asserts come from one place (`Hud::of_selected`, added for the window, which holds a plain
  id list rather than a `Selection`; `Hud::of` delegates to it so there is one implementation).
- **The bottom panel** (`hud_panel_height` = 68 px): a row per selected thing, five at most - a health bar in
  `HP_GREEN`/`HP_RED` on a `HP_TRACK` bed, the load in hand, the production queue, and a construction bar in the order
  marker's colour. One pixel of `MARK` along the panel's top edge, because on a dark map a panel that is the void's own
  colour is a panel nobody can see.
- **The minimap** in the lower right, above the panel, a boxed 128x128 copy of `ui::minimap` at two pixels a tile. Boxed
  for the same reason: unexplored ground is nearly black.
- **An outcome banner**: a full-width strip across the middle, green for a win and red for anything else. Colour is the
  whole vocabulary available - the bundled font has digits and a slash, no words (D82) - so the banner speaks in colour and
  the tests name the two colours.
- **The diagnostic renderer** (`bin/frame`) uses the same `ui::hud`, so the PNG the project produces for review cannot
  show less than the window does. It had been showing less.

## Test-first

Three tests were written and watched failing to compile against a HUD that did not take a `Hud` at all, then against the
missing `selection_row_origin`, `minimap_rect`, `banner_point`, `palette::PANEL/WON/LOST`:

- `the_selection_panel_paints_a_health_bar` - a 40% selected thing paints `HP_RED` at the row's second pixel, and with
  nothing selected the same pixel is `PANEL`;
- `the_minimap_is_copied_into_the_corner` - three pixels of the panel's minimap are compared with `ui::minimap`'s own
  pixels, so "drawn somewhere else" fails;
- `a_decided_match_paints_a_banner` - green for a victory, red for a draw, nothing for an undecided match.

Two older tests had to be told the truth about the new layout: one asserted the HUD paints nothing below the top bar, which
was true when there was no panel and is not true now (it now walks the band between the top bar, the panel and the
minimap's box).

## Performance

Measured with the project's own ignored benchmark, `--release -- --ignored --nocapture --test-threads=1`, three runs,
960x720, 126 entities, five selected:

| | plain frame | frame + HUD | HUD cost |
|---|---|---|---|
| run 1 | 532 us | 689 us | **157 us** |
| run 2 | 528 us | 679 us | **151 us** |
| run 3 | 512 us | 707 us | **195 us** |

So the whole HUD costs **151-195 us a frame**, nearly all of it the minimap copy (16,384 `put`s at two pixels a tile). That
is under 1% of a 33 ms frame, and it is measured rather than assumed because the minimap is redrawn every tick. **The first
measurement I took was wrong and is reported as such:** running both benchmarks in one process without
`--test-threads=1` produced "764 us with the HUD, 986 without" - a HUD that made the frame faster, which is contention, not
a result.

## Stability

- **`hud` is a pure function of `(Hud, Snapshot, Speed)`**: no state, no clock, no I/O. Every pixel it paints is reachable
  from a test without a display, which is what let the three assertions above be written at all.
- **A window shorter than the panel draws the top bar and stops** (`top >= bar` guard), so a tiny window shrinks the HUD
  instead of panic-slicing outside the frame.
- **A frame smaller than the minimap's margin** (`mx < 0`) skips the minimap rather than wrapping coordinates.
- **The panel is a list, not a summary:** five rows maximum, so a 40-unit box does not turn the HUD into a wall of bars;
  the count above five is not shown, which is a limitation recorded here rather than hidden.
- **What a player still cannot do (A1, postponed by the owner):** the window plays player 1's side *for* the player - it
  loads `acceptance.toml`, whose personality issues orders for the same side. The HUD now shows that AI's work as if it
  were the player's.
