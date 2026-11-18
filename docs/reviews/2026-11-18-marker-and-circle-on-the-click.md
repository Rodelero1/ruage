# D274 - the marker and the circle both land on the click: review

*The owner, after D273: **"Wherever I click, the yellow cross appears lower then were I click. And then the unit doesn't move its center
of the circle to the place I clicked. It seems to me that the whole problem was solved by shifting by certain number of pixels lower. In
reality, I want the center of the circle be the place where unit stands on and where the center of the unit base goes to."***

## What was still wrong

D273 fixed the **order** and missed two other consumers of the same frame:

1. **The yellow cross** was drawn `camera.project(at)` where `at` is now a **world** point. `project` speaks the projection's tile
   space, so the cross landed **half a tile down and to the right** of the click - exactly the half diamond, which is the "lower than
   where I click" he saw.
2. **The selection circle** was raised by its own half-height (D179: *"align the center of the ellipse with the bottom side of the
   square/triangle"*), so its **centre** sat above the ground point. The owner's latest word supersedes that: *"the center of the circle
   be the place where unit stands on."*

## The fix

- The cross is drawn `camera.project(drawn_at(at))` - the **same** conversion every body, ring and health bar already uses.
- The unit's ring is drawn **centred** on the ground point, with no raise.
- `ring_at_the_feet` is renamed `ring_on_the_ground` and now asserts the new rule (the ring is symmetric about the feet: as much above as
  below), with both of the owner's quotes in its header so the reversal is visible rather than silent.

## Performance

Nothing per tick: the marker is one `-0.5` on a value computed **once per order** (nanoseconds), and the ring's centre is one subtraction
removed from a value computed once per selected unit per frame. Measured on the fixtures that matter: the end-to-end click test runs in
**0.84 s** for the whole binary (setup + 600 simulated ticks + a render), which is within noise of the **0.83 s** it took before the
change; the full suite is unchanged at **202 ok / 1 failed**.

## Stability

- New end-to-end fixture (`click_round_trip::the_circle_ends_on_the_pixel_that_was_clicked`): click a pixel **13 px right and 9 px up**
  from a diamond's centre, submit the order the window would submit, run 600 ticks, then take the circle's centre exactly as the
  renderer computes it and require it within **1 px** of the click. Green.
- `selection::an_order_marker_appears_where_the_order_went` **amended**: it asserted the marker at `project(tile-space point)`, which is
  the same defect in test form; it now clicks a pixel, orders its world point, and requires the marker on **that pixel**.
- `ring_at_the_feet` -> `ring_on_the_ground`, asserting the centre rather than the lowest row.
- All view test binaries green; full suite unchanged.
- **Honest note on the reversal**: D179's rule and this one cannot both hold with a base-anchored sprite - the owner has asked for the
  ring at the feet before and now for its centre on the spot. The later instruction wins, it is recorded in both the code and the
  fixture, and the one word "feet" puts it back.
