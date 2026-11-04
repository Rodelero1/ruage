# The bar gets a shape, and the frame gets a layout

**Step:** U1 of the interface work (owner's answers, 2026-11-03): the game's own bottom bar - command grid left, selected
object middle, minimap right - anchors instead of magic numbers, and **it has to work on wide screens**.

**What was wrong, measured.** I rendered the interface at 1280x540 before touching it and looked at the picture: the
readout sat in the top-left corner, the command card floated above the bar with a gap beside it, the whole right half of the
bottom bar was empty, and a health bar sat flush against the frame's bottom edge. A pile of magic numbers (`8`, `60`, `76`,
`96x28`) spread through five functions, each of which laid out its own corner.

---

## 1. One layout, anchored

`ui::Layout::of(width, height)` - the single answer to "where is everything":

- the **top bar**, the **view**, the **panel** tile the frame exactly (tested at 4:3, 16:9, 21:9 and 800x600);
- the stockpile is measured from the **left** edge, the five bar buttons and the minimap from the **right**, and the
  middle takes what is left - which is the whole of "it works on a wide screen", and the same idea OpenAoK implements by
  cutting and repeating a fixed 4:3 bitmap. We have no art to match, so anchors say it directly;
- the panel is the game's three zones: `grid` (five by three at 40 px, the game's own button size), `selected`, `minimap`;
- a frame too small for the interface **clamps** rather than drawing off the edge.

Seven tests in `crates/view/tests/layout.rs`, written first and watched failing to compile: the frame tiles, the right-hand
things stay 8 px from the right edge at 800/960/1280/1920/2560, the bar buttons march inwards in order without overlapping,
the panel's zones are in the game's order and do not overlap, the grid is fifteen slots inside its zone, and a 320x200 frame
keeps everything inside.

## 2. Widgets with states

`ui::WidgetState { Normal, Hover, Pressed, Disabled }` and `ui::draw_widget`: a face per state, a **lit edge above and to
the left and a shadowed one below and to the right**, and a pressed widget whose bevel is the other way about. The speed
boxes, the card's buttons and (next) the menu's rows all draw through it, so "this can be pressed" is one rule instead of
three ad-hoc rectangles. The card's buttons are now **labelled placeholders** - the owner's choice - with the name, the
price and, when refused, the engine's words in red.

## 3. The bug the layout caught

`speed_boxes(width)` was laid out for a *nominal* 720-tall frame while `speed_selector` drew into the real one, so on a
1080-tall window the speed buttons were drawn in one place and hit-tested in another: **a click that missed**. The
signature now takes the frame's height and the window passes its real size. `hud` fills the panel from `Layout::panel`
rather than from a second `hud_panel_height()`, which had already drifted to 76 against the layout's 142 - the zones were
being drawn on ground that was never painted, and a probe caught it (the pixel was transparent, not the panel's colour).

## Performance

Measured with the project's ignored benchmark, `--release -- --ignored --nocapture --test-threads=1`, 960x720, 126
entities, five selected, three runs:

| | before U1 | after U1 |
|---|---|---|
| whole HUD | 151-195 us | **90-112 us** |

The panel got **taller** (76 -> 142 px) and the HUD still got cheaper, because two things changed underneath:

- `Frame::fill_rect` wrote **one pixel at a time**; the panel is a 960x142 rectangle, 136,000 writes for a background.
  It now fills a row at a time with slice copies (`as_chunks_mut::<4>`), and the whole HUD dropped by ~40 us.
- `Layout::of` was being derived **four times per frame** (once in `hud`, again in `speed_selector`, again in
  `minimap_rect`) and costs two small allocations each time. It is now built once and passed down: 316-355 us -> 122-141 us
  before the `fill_rect` change, and 90-112 after.

**The two regressions I introduced and then fixed, reported because they are the interesting part:** filling the three zones
cost 130,000 pixels a frame (316-355 us); they are now outlines, which is what makes the zones read anyway. And the
row-wise `fill_rect` was worth more than the taller panel cost.

## Stability

- **One source of truth for geometry.** The bug above existed because two functions each knew the panel's height; the fix
  is that `Layout` is the only place a rectangle is computed, and the drawing, the hit tests and the tests all read it.
- **Anchors instead of tables**: nothing is positioned by an absolute x any more except the 8 px margins, so a wide frame
  cannot put a control somewhere it was not drawn.
- **A tiny frame is clamped**, not an arithmetic overflow waiting to happen: every width uses `saturating_sub`.
- **Known gap, next step:** the bar's middle (where the age goes) and the selected zone's name/portrait are still empty -
  that is U3 and U2 respectively, and leaving them empty is honest rather than filling them with invented content.
- **Known limitation:** at 40x40 with a 1-pixel bevel the buttons read as "cells" more than "raised buttons" at 1280x540;
  the bevel's contrast is a one-line change when the look is next reviewed.
