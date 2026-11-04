# Readable text, an honest bar, and the stripe that was a banner

The owner's flaw list (2026-11-04) has fourteen items. This is the first batch: the ones that are the *interface lying or
shouting*, plus the two shape bugs behind them. The behavioural items (straight-line movement, smarter scouting, enemy hit
points on click, attacks on units, farm ownership) are separate steps with their own tests and reviews.

## What changed

- **The font is 5x7.** It was 3x5 - two rows of counting - and the owner asked for "more readable font". The new table carries
  digits, capitals, and the punctuation a HUD needs: `:` `.` `/` `-` `+` `_` `(` `)` `%`. Nothing else had to change: the
  drawing code asks `font::WIDTH`/`HEIGHT`, so the shapes improved and the layout code kept working. Capitals still fold, and
  a character with no glyph still takes no room.
- **The top bar has two rows and is laid out from its own numbers.** The old bar wrote the column name under the number and
  placed the *population at the last stock column's x* - which is why the owner saw stone and population on top of each
  other, unreadable. `stock_columns` walks the columns, measuring each name and number with `font::text_width`, and the
  population starts past the last one. A five-digit stockpile now moves its neighbours instead of overprinting them.
- **The "strange green stripe" was the victory banner.** It painted an 8-pixel band across the entire screen and no text at
  all (`let _ = bx;` - the x was computed and unused, because the old font had no letters). It is now a box with a word in it:
  VICTORY or DEFEAT, at twice the text scale, centred where `banner_point` says.
- **A building's name survives an age-up.** `role_name` matched the exact unit id, so the moment the Feudal Age replaced the
  town centre (109) with its variant (71) the panel called it "unit 71" - the owner's "buildings get reamed upon move to the
  next age". The lookup now goes through the family root.
- **A scout is a soldier.** `[metrics] military_roles` did not list `scout_cavalry`, so it was drawn as a worker's square; the
  view draws a triangle for anything the pack calls military.
- **Units are always painted over buildings.** Depth alone hid a villager standing in the back half of a house or a farm; the
  owner asked for the opposite in as many words. `layer_of` now has three passes - ground cover, buildings, units - and depth
  orders within each.
- **The minimap's view box keeps its shape at the edge.** Its corners were clamped to the map, which bent the parallelogram
  into something that was not the camera's view. They are now projected as they are and the *drawing* is clipped to the box.

## Tests

Each change came with the test that would have caught it:

- `every_glyph_fits_the_box` in `text.rs` now checks seven rows of five bits (it caught nothing but its own assumption);
- `the_hud_shows_one_swatch_per_column_and_the_population` and `each_stock_column_shows_its_name` now ask `stock_columns` and
  `stock_rows` rather than fixed offsets - the same functions the drawing uses, so a test cannot drift from the layout;
- `a_decided_match_paints_a_banner` now counts the banner's colour in a box around `banner_point` instead of sampling one
  pixel, and checks a draw is not a win;
- `a_unit_behind_a_building_is_still_painted` replaces `…is_covered_by_it`, and
  `things_are_painted_from_the_back_of_the_diamond_to_the_front` gained the new layer order (`[10, 20, 9, 21, 22]`: the
  resource, then the building, then the units by depth). Both are rewrites of tests that encoded the old rule, recorded here
  rather than quietly deleted.

## Performance

The font is 5x7 where it was 3x5: `draw_text` writes 7 rows instead of 5, so a line of text costs 40 % more pixels. Measured
where it matters - the whole HUD, at 960x720 with five selected objects - the cost is unchanged at the round: **176 us** before
this batch and **176 us** after (the earlier 176-us figure was measured on the same probe this session; the bar's second row
and the larger glyphs trade against the removal of the fixed-offset arithmetic). A frame is 991 us, most of it the world.

## Stability

`stock_columns` cannot index past its own vector (`get(column).copied().unwrap_or(8)`), and an empty stockpile draws nothing -
the test that pins it had to be fixed when the layout change made it index `columns[0]` on an empty list.

The layer order changes what a player sees, not what the simulation does: no test in `ruage-sim` moved, and the acceptance run
is untouched.
