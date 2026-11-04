# The minimap was fitted twice

The owner asked for a bigger minimap, twice: first "increase the size ... by widening the area the minimap is in", then
"bar is the same height, minimap fills it vertically and proportionally grows horizontally". Widening the zone was not enough,
and the reason is worth writing down: **the picture was fitted to its zone twice.**

`fit_minimap` shrank the map to "two pixels a tile" and centred the result - correct when the minimap was a square grid of
tiles, where a 64-tile map drew 128x128 inside a 160-pixel box. Once the minimap became an isometric diamond the projection
already preserved the map's own proportions, so the second fit only made it small: a 260x130 zone had a 128x128 picture in it,
and the 2:1 diamond inside *that* was 128x64 - a third of the height it could have used, centred in a wide empty box.

Now the zone **is** the picture: `fit_minimap` hands the whole rectangle to the diamond, which fills it (259x129 in a 260x130
box at 1280x720) and keeps 2:1 because that is the world's own tile shape. The zone itself is `2 x (panel height - 12)`,
clamped to a third of the width and floored at 160, so the bar's height is what decides how big the minimap can be - which is
exactly the owner's rule: "bar is the same height, minimap fills it vertically and proportionally grows horizontally".

## Performance

Unchanged: the same 4,096 tiles are drawn, each as a bigger diamond, and `fill_diamond` costs one row loop per tile whatever
its size. The HUD measurement stands at 176 us a frame.

## Stability

`fit_minimap` now has no arithmetic to get wrong, which removes a class of bug rather than one instance: there is one place
that decides where the minimap is (the layout) and one that draws it. A zone narrower than 160 (a very small window) is still
floored, and the diamond is clipped by `fill_diamond`'s own bounds rather than spilling into the selected zone.

Open: the speed selector is five 18-pixel boxes under a 260-pixel minimap, which now reads as stray buttons - it wants laying
out across the zone's width, or moving into the top bar.
