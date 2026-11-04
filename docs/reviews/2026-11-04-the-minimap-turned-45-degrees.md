# The minimap turned 45 degrees, with a box you can grab

Owner's note, 2026-11-04:

> hud should be also 45 degrees turned. Just like map. It should also show rectangle (empty fill, white margin) in it based
> on where on the map I am, also it should change the size based on the proportion of the map I see, and it should be
> possible to grab the rectangle (not just by the margin but by the inside, too) and move it on the map (and that should move
> the view, too), also by left clicking on the hud the minimap rectangle should move its center to the point of click on hud
> and view should move too.

Five things asked for, and all five are in:

1. **The minimap is a diamond.** The same projection the world is drawn on: `u = tx - ty` across, `v = tx + ty` down, with
   tile (0, 0) at the diamond's top point and (w-1, 0), (w-1, h-1), (0, h-1) at its right, bottom and left. One function,
   `projected`, and its inverse `minimap_tile`.
2. **The view box is the camera's own rectangle**, projected: the four corners of the screen are picked back to tiles
   (`Camera::visible_corners`) and drawn as the slanted parallelogram they are. Not a bounding box - a box would be a lie
   about an isometric screenshot.
3. **It shrinks as the player zooms in**, because it *is* the camera: the test asserts a 2x zoom makes the quad's widest
   span smaller, and a camera at the map's origin draws its box at the diamond's top rather than in the middle.
4. **It can be grabbed by the inside as well as the edge.** A press anywhere on the minimap starts a drag; the camera follows
   the pointer, clamped to the map's edges so a fast mouse does not drop the view half way. Releasing anywhere ends it.
5. **A plain left click still moves the view to the clicked tile** - that is the same press, one frame before the drag
   continues.

The box is drawn with an empty inside and a white margin: a 1-pixel white line with a grey line one pixel out, because a
single white line over pale ground is invisible.

## Tests, written first

`crates/view/tests/minimap.rs` (three, before the geometry existed):

- `the_projection_and_its_inverse_agree` - six tiles round-trip, the map's corners land on the diamond's four points, and the
  diamond stays inside the box. Two real bugs came out of this test and nothing else: truncation instead of rounding read tile
  (32, 32) back as (31, 31), and a span that used the box's width instead of its last addressable pixel put the right corner one
  pixel outside the box.
- `the_corners_of_the_box_are_not_the_map` - a click on a corner of the rectangle is a click on nothing. The first version of
  the diamond test compared `u` and `v` against the box separately, which admits exactly the corners; the diamond is
  `|u|/across + |v - down|/down <= 1`.
- `the_view_box_is_the_camera_and_shrinks_with_zoom` - zoom 2x narrows the quad, and a camera at the origin draws near the top.

`crates/view/tests/hud.rs` had `the_minimap_is_copied_into_the_corner`, which asserted the old square minimap pixel for pixel.
Rewritten as `the_minimap_is_a_diamond_in_the_corner`: the middle of the box is map, the four corners are panel, and the map's
first tile is at the top of the box.

`crates/view/tests/ui.rs` had two tests that built the old square minimap frame directly; they now ask the fog and the HUD's
own pixel instead, so they test the picture rather than a printer that no longer exists.

## Performance

- The HUD costs **176 us** at 960x720 with the diamond minimap and the view box, against **172 us** without the box: the box
  itself is 4 us of four Bresenham lines. The diamond costs about 60 us over the old square minimap (117 us before this step,
  measured on a quieter machine), for 4,096 `fill_diamond` calls a frame - 0.7 % of a 25 ms tick.
- A rendered frame at 1280x720, 9,000 ticks in: `tick=9000 entities=40 visible=199`. Looked at: the minimap is a diamond in
  its frame with a white parallelogram on it, the box sits over the visible ground, and the corners of the frame show panel.

## Stability

`minimap_tile` returns `None` for anything outside the diamond, so a click in the dead corner of the box cannot move the
camera to a tile that is not on the map - and a drag that leaves the box is clamped rather than refused, so a fast mouse does
not lose the view. `minimap_view` clamps its corners to the map for the same reason.

The old `ui::minimap` (a square map in a `Frame`) is deleted rather than left beside the diamond: two printers of one picture
is how the panel height drifted to two numbers earlier in this project.

Open: the view box does not yet respond to a *click on its edge* differently from a click inside, because both start the same
drag - which is what the owner asked for. A resize handle at the corner (drag to zoom) is the natural next thing, and it is
not in this change.
