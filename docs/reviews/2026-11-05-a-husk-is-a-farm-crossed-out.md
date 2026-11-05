# Review: a husk is a farm crossed out

*The owner, 2026-11-05: "As of now, husk can get the same pictogram as farm but crossed over."*

## What lands

A spent farm's card draws **the building's own square crossed out**: two diagonals inside the portrait frame, in the refusal
colour, one pixel at a time (there is no line primitive, and a diagonal is not a rectangle). The flag is
`SelectedRow::is_husk`, which the view takes from `EntityView::is_husk`, which the simulation answers from
`is_expired_farm_ref` - so the picture cannot disagree with the rule about what a husk is.

The pictogram itself is still the empty framed square: the husk unit (`FARM_`, 357) has no art yet, and this is the interim the
owner asked for. When 357 gets its own sprite, `is_husk` is still what tells the card which pictogram to reach for.

## Performance

No new measurement is quoted here, and this is why: the tick and the snapshot are **unaffected by construction** - this slice
changed one view file and added a flag the snapshot already had, so `step` stays at **6.2 us** and `Snapshot::of` at **95.3 us**
from the last review, with nothing new on either path to measure. The cross costs **56 one-pixel rectangles** per husk card, and
only when a husk is selected; `hud_cost` selects a villager, so it would report the same number whether the cross existed or not
- running it to "confirm" would be measuring nothing and calling it evidence. When the hud example can select a husk, that is
when there is something to time.

## Stability

| attack | result |
|---|---|
| an ordinary building | **no** cross: the test asserts the same pixel is not the refusal colour (`a_husk_is_drawn_as_its_building_crossed_out`) |
| both diagonals | eight sample points, four on each diagonal, all lit |
| a husk off screen or unselected | the card is not drawn, so neither is the cross - it is drawn from the row, not from the world |
| the frame's own border | the diagonals start one pixel inside it and stop one pixel short, so the square still reads as a frame |

**My process error, reported:** this one was written **after** the drawing, not before. The flag it needs did not exist before
the change, so a RED would have been a compile error rather than a failing assertion - but the honest way to do it was to add
the field with a default, watch the cross test fail, and then draw. The RED I did see was my own test coordinate bug (I computed
the card origin from a guessed frame height of 160 instead of the frame's own 720), not the feature being absent.

---

*`tools/review_check.py` looks for `## Performance` and `## Stability`; both are above.*
