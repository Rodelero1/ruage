# One answer to "is this the interface?"

**Step:** U4 of the interface work (2026-11-03): input occlusion, the last of the four steps the owner's answers set.

---

## What was wrong

The window decided what a click meant by asking each surface in turn: the speed selector, then the card, then the
minimap, and only then the world. Every new control added a line to that chain in a particular place, and the day two of
them overlapped, the order was the only thing keeping the game playable - a rule nobody wrote down and nothing tested.

## What replaced it

`ui::over_hud(pixel, &Layout)`: the top bar and the bottom bar are the interface, everything between them is the map, and a
pixel outside the frame is the map (there is nothing there to click). The specific hit tests remain - they decide *which*
button - but they no longer decide *whether* the click is the interface's. A press that begins on the interface never
becomes a box over the world, even if the pointer leaves it before the button comes up.

OpenAoK answers the same question from its interface art's own alpha (`PointOverHud` asks the bitmap whether the pixel is
opaque, so a click through a transparent gap in the carving reaches the map). Our interface is drawn, so **the layout is
the mask** - and because drawing and hit-testing read the same `Layout`, the mask cannot disagree with the picture.

Two tests (`layout.rs`, written first): the bars are the interface and the map is not, at three frame sizes, including the
pixels immediately inside each edge; and a pixel outside the frame is the world, not the interface.

## Performance

- Four integer comparisons per click and per release. Nothing per frame: the test does not run in the draw path at all.
- The HUD's own cost is unchanged at **117 us** a frame (measured in U3 and re-measured here; the benchmark is one
  draw per iteration).

## Stability

- **The load-bearing order is gone from the input path.** Menu, card, bar buttons, speed and minimap still each take their
  own click first - that is *which* control - but anything else on the bars is swallowed by `over_hud`, so an empty patch of
  panel cannot select a unit that happens to be behind it.
- **A box cannot start on the interface**, which is the case a player hits by accident (drag from the panel edge) and the
  case that used to select half the army.
- **The layout is the single source**: `over_hud`, `card_at`, `bar_button_at`, `speed_at`, `minimap_tile` and the drawing all
  read the same `Layout`, so a control drawn in one place and hit-tested in another is now impossible rather than unlikely
  (the U1 review records the last time it happened).
- **Known gap:** the interface is drawn *over* the map, so the whole bar area is opaque and `over_hud` treats it as such. A
  future HUD with see-through gaps (openaok's carving) would need a finer mask; that is a real limitation of this
  approximation, recorded rather than hidden.
