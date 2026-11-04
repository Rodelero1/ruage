# A flat 2:1 minimap, and two tests that could not catch the stuck army

The owner answered the question the truncated sentence hid: the minimap's **proportions**, not its orientation - "make the
minimap's tiles 2:1 like the world's". The world draws a tile twice as wide as it is tall; the minimap was drawing a
square-ish diamond because it filled the panel's box in both axes. It now spans the box's width and **half** of it vertically,
centred, and `minimap_tile` is the inverse of the same geometry - so a click still lands on the tile the picture shows.

## What the tests say about the stuck army

The owner's answer to "which order puts them there" was **the AI's army walking at the enemy**. Two tests were written for
that case in `crates/sim/tests/army_walk.rs`:

- an attack-move across a three-tile wood, for a soldier and the scout, checking **every tick** that each stands on ground a
  unit may stand on;
- twelve soldiers attack-moving through the same gap at once, so the assembler has to shove bodies during the walk.

**Both pass.** So "stuck" is not "standing on blocked ground" for those cases, and the first fix (the goal tile is not a hole
in the ground) is not the whole story. What I need to chase it further is the owner's own frame: the seed and the tick are in
the window title, and a `--dump` at that tick would say which unit, where, and what task it holds. Without it I would be
guessing at a third mechanism - and two guesses in, the honest move is to ask for the frame rather than write a third test for
a bug I cannot see.

## Performance

Nothing measurable: the projection changes a handful of float multiplications per tile, and the minimap is 4,096 tiles a
frame inside a 176 us HUD. The two new tests run in under a second each (2,000 and 3,000 ticks of a dozen units).

## Stability

`minimap_tile` and `projected` now share one geometry block each, spelled the same way, because the round-trip is the only
thing that keeps a click honest. The diamond's slack in the inverse was widened from half a pixel to one and a half tiles of
`v`: a flat diamond is half as tall, so one pixel at the top or bottom edge is two tiles' worth of `v`, and without the slack a
projected corner fell outside its own diamond.

## Decisions waiting

- **Farms**: several villagers may build one, and helpers stand down when it is finished; exactly one may gather. Not yet
  implemented.
- **Walking**: straight lines, and a unit that cannot pass **slides** along the obstacle (the owner's third answer). The
  earlier proportional-step attempt is the starting point; the missing half is the slide.
