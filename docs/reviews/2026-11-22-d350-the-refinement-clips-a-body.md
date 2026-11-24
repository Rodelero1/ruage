# 2026-11-22 - D350: the pile's next slice is three ports, and the last of them is where the clip is made

A read plus two ports that were measured and **reverted**, so this note carries numbers rather than a landed change.

## What was done

The fixture `body_blocks_its_tiles` was written first and watched failing: a stander on the lattice corner `(16,16)`, a walker
from the centre of `(10,10)` to the centre of `(20,20)` - a line that passes exactly through that corner - and an assertion that
the walker's own centre never enters the pair's grown box at any tick.

- **Before any change**: inside the clearance at tick 180, gaps **7,677 / 7,612 raw** against **8,000**.
- **After the tile rectangle** (`inside_a_body` asks the tile's square, not its centre - `FUN_007f9d80` → `FUN_007f6ff0`):
  **still inside**, 7,650 / 2,355 raw, deepest 350 raw; the route had already bent round the corner, so the tile test was not
  what clipped it.
- **After the rectangle and the smoothing's bodies** (`FUN_008acf20`'s test is `FUN_00840c60`, which walks the object system's
  buckets, so units are in it; `smooth` now certifies with `leg_is_clear`): the route went from **one straight point** to three -
  `(15.5,16) -> (16,16.5) -> (20.5,20.5)` - and the walker **still** clipped the corner by **350 raw** on one axis.

So the clip is made in the **refinement**: `FUN_007eeda0` scans the cell's tile span along the direction of travel and takes
the first tile where the direction is passable, and "passable" there is the tile test that consults the non-ignore objects on
the tile (`FUN_00801a20`); `Levels::refine` interpolates by geometry alone. The smoothing can only delete waypoints, so a
crossing placed inside a body survives it.

## Performance

No engine change landed, so the engine's own numbers are unchanged and are quoted as the baseline the next slice must improve
on, not as a result of this work:

- acceptance `7782dd38d7557514`, 134 entities, 37,204 re-paths, **12.17 s** median of three (D349's commit);
- pile census: **349,824** overlapping pair-ticks (2.0173% of 17,340,819), **317,041** deeper than half a footprint, deepest
  group **11**, new overlaps **877**, `pile_probe` split mate 0 / past 500 / neither 164.

**Cost of each reverted port, measured in route shape rather than time**: the tile rectangle blocks up to four tiles per corner
body instead of none; the body-aware smoothing deleted **2 of 3** waypoints on the failing walk instead of 2 of 2 - both change
route shapes on the acceptance scene, and neither was priced on the wall because neither was kept. The next slice prices them
together, on the census and on the wall, and that is the number that decides whether they stay.

## Stability

- Both ports were reverted **because their own fixture still failed**: a half-fix that leaves the rule broken reads as a fix in
  the log and is not one.
- The fixture is preserved outside the tree at
  `/opt/data/cache/scratch/age2hd-pending-fixtures/body_blocks_its_tiles.rs`. It asserts both directions - the walker never
  enters the clearance, and he still arrives - so a fix that keeps men apart by refusing to route is red too.
- The tree is clean at the end of the slice: `git status --short` empty, suite untouched from D349's green run.
- **What this does not prove**: that the three ports together move the pile. The failing walk is a synthetic scene; the census
  is the witness, and it has not been run with any of the three in place.
