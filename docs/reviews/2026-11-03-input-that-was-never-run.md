# Three input rules that were written and never run, and a minimap nobody could click

**Step:** P2 of the owner's queue (2026-11-03): band select, double-click, and the minimap as an input.

**What was wrong.** `Selection::click`, `Selection::band` and `Selection::double_click` were written for the window and
**nothing called them** - the window hand-rolled its own "select what is under the pointer" and had no box and no
double-click at all. The minimap was drawn from the previous step and had no pick: a picture, not a control.

---

## 1. The three rules, tested

`crates/view/tests/picking.rs` (written first, watched failing):

1. **A band takes the player's units inside it and no building.** The box is a drag, and a box that swept up a town centre
   would make every attempt to gather three villagers a disaster; the game does not sweep buildings, so neither does the
   engine's box.
2. **A double-click takes every unit of that kind that is on the screen** - and this test found a real bug: the first
   implementation took *every* entity of that kind in the snapshot, and a snapshot is the whole world, so a double-click
   picked up villagers thirty tiles away that the player cannot see. Fixed in `double_click` by projecting each candidate
   and testing the camera's viewport.
3. **A click on the middle of a building selects it**, not only its anchor tile - the rule the window's own picking already
   had, now pinned where it lives.

Two of my own mistakes, reported because they are the reason two of these are worth having:

- the first band test boxed `(6,6)` to `(18,18)` in tiles, and **two tiles on one isometric diagonal project to the same
  x** - the "box" was a vertical line, the test failed, and it looked like a bug in `band`. The box is now built from the
  projection of two tiles that are not on one diagonal, and the test asserts it is a rectangle before using it;
- the first minimap test expected pixel 66 to be tile 32; at two pixels a tile, pixel 64 is tile 32 and 66 is 33. The test
  was wrong, the code was right.

## 2. The minimap is a control

`ui::minimap_tile(pixel, rect, map_width, map_height)` is the inverse of the layout: a pixel inside the box is a tile, a
pixel outside is `None`. It exists as a function rather than arithmetic in the window because the day the box moves, a
second copy of the layout would be wrong.

The window now: clicks the speed selector, then the minimap (which **centres the camera** on the clicked tile), and
otherwise starts either a click or a box - decided on release by whether the pointer travelled more than four pixels, with a
second click in the same place within 400 ms being a double-click.

## Performance

- The pick per release builds a `Snapshot` (one pass over the entity list, measured at ~500 us for a 126-entity world in
  the frame benchmark, of which this is a fraction) - **once per mouse release**, not per frame.
- No new per-frame cost: the minimap pick runs only on press, and `minimap_rect`/`minimap_tile` are integer arithmetic.

## Stability

- **The window no longer keeps its own idea of what is selected.** It holds a `ui::Selection`, the type that owns the three
  rules, so the picture and the input cannot disagree about what a box means.
- **A double-click cannot reach off-screen units**, which was a real defect found by the test rather than by playing.
- **A press that starts on the minimap or the speed selector never becomes a box**: both return before `press_at` is set, so
  the release has nothing to complete.
- **A release with no press** (`press_at` is `None`) does nothing, which is what a release after a menu click has to do.
- **Restart clears the press and the last-click** as well as the selection: a stale `last_click` from the previous match
  would turn the first click of the new one into a double-click.
- **Known limitation, recorded:** the window's own wiring is compile-checked only - there is no display on this machine, so
  "the release handler picks the right rule" is argued from the code, not measured. The three rules it calls *are* tested;
  the ten lines that choose between them are not.
