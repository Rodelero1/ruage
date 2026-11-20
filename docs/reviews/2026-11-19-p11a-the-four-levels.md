# Review: the four level maps (P11a, D296)

*First code of P11. The crosscheck that decided its shape is `docs/pathfinding-levels-evidence.md`; this is the piece of it that did
not depend on the one unknown (the descent's second byte flag).*

## What landed

`crates/sim/src/levels.rs` - `Levels::of(&Ground)`, four blocked-cell maps, level 0 the tiles and each coarser level an OR-reduction of
2×2 blocks:

- `cells(level)` = `ceil(tiles / 2^level)` per side, via `div_ceil`, so an odd map keeps its last column and row;
- `cell_of(level, tile)` = `tile >> level`, the binary's own indexing (`iVar4 = (int)param_2 >> iVar7`);
- `blocked(level, cx, cy)`, with anything off the map answering **blocked** - nothing may be routed off the ground;
- rebuilt per call from the ground, because ground changes when a building is stamped or a tree is felled, which is why the binary
  rebuilds its own per search.

**The rule is the binary's stamping rule**, not a choice of mine: `FUN_007ede40` stamps an obstacle rectangle at level 3 and
`FUN_007f1100` recurses *down* while the cell is unmarked, so a coarse cell carries the mark of every obstacle that touches any of its
tiles. **A cell is blocked when any tile under it is blocked.**

## Test-first, as the rule requires

Written before the code and watched failing three times over, in the order the failures came:

1. `error[E0432]: unresolved import ruage_sim::Levels` - the type did not exist;
2. after the type: `Invalid { key: "levels.toml: map.town_center", detail: "both players need a town centre" }` - the fixture map was
   not a game;
3. after the fixture: all four green, **4 passed / 0.00 s**.

`crates/sim/tests/levels.rs`:

| fixture | claim |
| --- | --- |
| `an_open_map_is_open_at_every_level` | nothing is stamped on an empty map, at any level |
| `one_wall_tile_closes_the_eight_by_eight_cell_that_holds_it` | one tile closes the 8×8 cell that holds it, every other 8×8 cell stays open, and level 0 is the tiles |
| `a_four_by_four_hole_is_open_at_level_two_and_closed_at_level_three` | a level is fine enough to hold the route: a 4×4 hole is open at level 2, closed at level 3, and the level-2 cell beside it is closed |
| `each_level_is_a_quarter_of_the_one_below_it` | 16×16 → 16, 8, 4, 2 cells per side |

## Performance

- the four maps cost **one pass per level** over the ground: 16×16 here, and on the largest map the game allows (480×480) 230,400
  `bool`s at level 0 plus a quarter each level up - about 307 KB for the pyramid, allocated per search. The binary holds an equivalent
  (a byte per cell plus a per-search stamp) and rebuilds it, so this is the same order; it is **not** pooled yet, and pooling is a cost
  to measure when the search that uses it lands, not before;
- `crates/sim/tests/levels.rs`: **4 passed / 0.00 s**;
- the full suite: **904 passed / 6 failed / 21 ignored**, from 900 / 6 / 21 - the four new tests and nothing else;
- clippy clean at `-D warnings` across the workspace and all targets, after `div_ceil` replaced my hand-rolled ceiling.

## Stability

**The risk in a new grid is that it disagrees with the tiles it was built from**, so the red-set diff was the gate: sorting and diffing
the failed names before and after reports **nothing**, meaning no existing rule moved, and the four new fixtures pin the disagreement
cases directly - an empty map stamped nowhere, one wall tile closing exactly the one 8×8 cell that holds it, a 4×4 hole open at level 2
and closed at level 3, and the cell counts per level.

**A level that is too coarse would route a body through a wall**, which is why the "any tile blocks the cell" rule is the safe direction
and the fixtures assert both sides of it: the cell holding a wall is closed, and the cell that is a hole is open. A worst case that is
still wrong - a cell with one wall tile and fifteen open ones being unusable at that level - is *the game's* behaviour, and the descent
that avoids it is 11b, which is where it belongs.

## What is not here, deliberately

**The descent (11b)**: `FUN_007ee980` tests bit `0x20` of a level cell's **second** byte while the stamper writes the **first**, and the
second byte's producer has not been read. Read literally, the loop would search coarsely in cluttered ground and at tile level in open
ground, which is backwards from any hierarchy I can justify - so it is not guessed, and the coarse A\* (11c) will be written against the
level maps without it.
