# Review: a ring the size of the body

> **Correction, same day.** The owner looked at the rendered frame and said: *"I see a diamond, not elipse"* - and he was right.
> This review claimed an ellipse while the code drew `outline_diamond`: a rotated **square** projects to four straight edges, and
> a square is not a circle. The reasoning in the first version ("a circle in tile space projects to a diamond") was simply wrong.
> Units now use a new `outline_ellipse` - `half = rx * sqrt(1 - (dy/ry)^2)` per row and the mirror per column, so a shallow
> slope does not dash the curve - and a test pins the difference where the two shapes disagree most: **at half height an ellipse
> is still 86.6% of its width, a diamond has closed to 50%**. Measured on the rendered frame now: half-width **14 px** at the
> middle and **12 px** at 50% height, a ratio of **0.86**. The RED was exact: *"at half height the ring is 19 px from the centre;
> an ellipse would be 31, a diamond 18"*.

*Step (D125). The owner, 2026-11-05: *"Now I would like to proceed with circle enveloping the units ... Smaller units have
smaller circle (infantry, villagers...) and bigger units have bigger circle (cavalry, elephants, siege units, ships...) ...
They should be centered on the center of the base of the unit."* Answers: selected only; the pack's radius times `k = 1.5`; an
ellipse on the ground plane.*

## What lands

A selected **unit** now wears a ring of **its own body's size**: `radius * ring_scale` tiles, centred on its feet, drawn as a
diamond outline under the isometric projection - which is what a circle on the ground looks like in this view. Before this,
every selected thing wore a ring the size of its **footprint**, and a unit's footprint is one tile, so a villager, a knight and
a trebuchet all wore the same 0.9-tile diamond.

The size was already in the pack: `radius = [x, y, z]`, **0.2** for a villager or any infantryman, **0.25** for knight /
paladin / scout / hussar, **0.3** for the advanced heavy crossbowman, **0.5** for mangonel / scorpion / trebuchet / galley. That
is the same number the simulation uses for body-to-body standoff (D117), so the ring a player sees is the circle that decides
spacing - times `k`, because a ring drawn exactly on the collision circle cuts through the sprite's feet.

`k` is content: `[ring] scale = 1.5` in `economy.toml`, and `1.0` means "the ring *is* the collision circle".

## Performance

Nothing was added per frame: the ring is **one `outline_diamond` per selected unit**, exactly as before - only its radii and
origin changed. Measured with the existing `frame_cost` test at 960x720 with 128 entities: **7,972 µs** a frame, dominated by
the terrain fill, and the HUD's own band **2,336 µs** with five things selected. Neither number moved because neither the call
count nor the pixel count per ring changed in kind; a bigger ring is a handful more pixels, not another pass.

## Stability

| attack | result |
|---|---|
| a 0.2-tile unit vs a 0.5-tile one | the bigger wears a visibly wider ring (`a_bigger_unit_wears_a_bigger_ring`: **29 px against 73 px** on the rendered frame) |
| the factor | the ring's width matches `2 * radius * k` in tile half-widths, within 6 px (`the_ring_is_the_packs_radius_times_the_config_factor`) |
| an unselected unit | no ring pixels at all (`an_unselected_unit_wears_none`) |
| a building | keeps its footprint **diamond** - a footprint is a tile, and a diamond is the honest outline for it |
| the shape itself | `the_ring_is_a_curve_and_not_four_straight_edges` measures the ring at half height, where a diamond and an ellipse are furthest apart |
| a unit whose radius the pack omits | `radius` defaults to 0 in the snapshot and the ring collapses to a point rather than drawing a wrong size |
| a float reaching the simulation | **caught**: `no_float` failed on my first `World::ring_scale() -> f32`; the accessor now answers `ring_scale_percent() -> i32` and the guard is green |
| the acceptance hash | unchanged (`4568cef16368bd5a`), which is what a drawing-only slice must show |

**A measurement of mine that was wrong, reported:** my first `ring_width` helper measured the *longest horizontal run* of ring
pixels. An **outlined** diamond has only two pixels on its widest row, so the helper reported 7 px for both a villager and a
trebuchet and the test "proved" the feature was broken when it was not. It measures the **extent** between the leftmost and
rightmost ring pixel now. That is the second time this session that a measurement, not the code, produced a false failure.

---

*`tools/review_check.py` looks for `## Performance` and `## Stability`; both are above.*
