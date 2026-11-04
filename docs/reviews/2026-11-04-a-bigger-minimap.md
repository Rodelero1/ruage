# A bigger minimap: as wide as a 2:1 diamond allows

The owner asked for a larger minimap and showed the game's own bar, where the map fills a quarter of the width. The panel's
minimap zone was a fixed **160 px**, chosen when the minimap was a square grid of tiles; once the diamond became 2:1 (a flat
diamond twice as wide as tall) the box's *height* became the limit, not its width - a 160-pixel box was leaving the bottom
third of the bar's right side empty.

The zone is now `width / 4`, clamped to `2 x (panel height - 12)` px wide (beyond that the extra box is empty corners, since
the diamond is flat) and floored at 160 so a small window still gets the old size. At 1280x720 that is **268 px instead of
160**: the diamond itself goes from 158x79 to 266x133, which is 2.8 times the area.

## Performance

No change to the number of tiles drawn (4,096) - each is a bigger diamond, and `fill_diamond` costs one row loop either way.
The HUD is unchanged at 176 us a frame in the same probe. The layout arithmetic is four comparisons per frame.

## Stability

The zone is bounded on both sides (`max(160)`, `min(width / 3)`), so no width the window can take produces a minimap that runs
under the selected-object zone or off the frame; `the_interface_owns_its_pixels` and the rest of `layout.rs` pass at 960x720,
1280x540 and 800x600, which are the shapes the anchored layout is asked to survive.

Open: the speed selector still sits under the minimap and is now narrower than it - five 18-pixel boxes under a 268-pixel
zone, which reads as a row of buttons stuck to the left of a wide map rather than a control belonging to it.
