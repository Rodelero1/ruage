# Review: the level descent (P11b, D297)

*Second piece of P11. 11a built the maps; this decides **which map the search starts on**, which was the step blocked on an unread byte.*

## The reading that unblocked it

`FUN_007ee980` descends from level 3 while a cell's **second** byte has bit `0x20` clear. The stamper `FUN_007f1100` writes the
**first** byte, so the second is not the search's own mark. `FUN_007ed8f0` - the quadtree descent's child test - reads the *same*
second byte and requires `& 0xf == 0xf` (clear in all four directions) before a child counts as usable. So byte 1 is the cell's
persistent **passability**, and `0x20` is its "usable" flag.

That makes the loop mean: **descend while the cell is not usable, stop at the coarsest level whose cell is** - fast coarse cells in the
open, fine cells in clutter, tiles as the floor. The other reading (bit set = blocked) would coarsely search the clutter and finely
search the open ground, which is backwards from any hierarchy; it is recorded in `docs/pathfinding-levels-evidence.md` as the reading
that was rejected, with its reason.

**This is an interpretation, and it is labelled as one** in `Levels::descent`'s doc comment and in D297. The byte's producer has still
not been read; what has been read is every other reader of that byte, and they all treat it as passability.

## What landed

`Levels::descent(tile) -> u8`: from level 3 down, the first level whose cell holding `tile` is unblocked, flooring at 0.

## Test-first

Written and watched failing: `error[E0599]: no method named 'descent' found for struct 'Levels'`, then a **fixture error of my own** -
the first version put the wall at `(9, 2)`, which is inside the very 4×4 cell the start sits in, so level 2 was correctly blocked and
my assertion was wrong, not the code. The wall moved to `(14, 6)` - same 8×8 cell, different 4×4 cell - and the claim became true.

`crates/sim/tests/descent.rs`, on a 32×32 open map:

| fixture | claim |
| --- | --- |
| `an_open_field_starts_at_level_three` | four starts across an open map all answer level 3 |
| `a_wall_in_the_coarse_cell_drops_the_start_one_level` | a wall in the start's 8×8 cell but not its 4×4 cell drops the start to level 2, while a start far away stays at 3 |
| `a_one_tile_corridor_starts_at_level_zero` | a 1-tile corridor blocks every level above 0, so the search starts on the tiles |

## Performance

- `crates/sim/tests/descent.rs`: **3 passed / 0.00 s**;
- the descent is at most three map lookups per search (levels 3, 2, 1, then level 0 given), i.e. **O(1)** against the search it starts -
  the cost that matters is the pyramid build, which 11a measured at one pass per level;
- full suite: **907 passed / 6 failed / 21 ignored**, from 904 / 6 / 21 - the three new fixtures and nothing else;
- clippy clean at `-D warnings`, workspace and all targets.

## Stability

**A wrong descent is a wrong search, not a wrong number**: if it starts too coarse, the coarse A\* can be asked to route through cells it
cannot see inside; if it starts too fine, the budget runs out sooner. The fixtures pin both directions of the failure (a start that must
drop, and starts that must not), and the third pins the floor, because a descent with no floor would loop or index level `-1`.

The red set is unchanged by this commit, so no existing route moved.
