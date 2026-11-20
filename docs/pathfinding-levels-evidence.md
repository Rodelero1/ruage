# The levelled pathfinder: what the binary actually does (P11 crosscheck, 2026-11-19)

*The owner: **"Proceed in accordance with original game and decomp. Crosscheck each step before proceeding."** This is the crosscheck of
P11 before any code: is there a levelled search in this binary at all, and if so what are its levels, its cell size, its switch and its
budget? Every claim below is anchored by address.*

## The answer, in one line

**There are two pathfinders and the levelled one is the land-unit default**: `FUN_008000d0` picks between a **flat tile A\***
(`FUN_008007f0`) and a **four-level coarse-to-fine search** (`FUN_007ee980`) by asking a registry (`FUN_007ef140` → `FUN_007ef0f0` →
`FUN_007edab0`) whether the path object has a handler - and a path object exists for the terrain grid.

**This corrects my first reading**, which is worth recording: `FUN_008007f0` alone reads like *the* search (a 480×480 cost array, an
expansion budget), and on that evidence I was ready to write down that the original is flat and that P11 would be a deliberate
divergence. Only the wrapper `FUN_008000d0` shows the second branch. **One function is not the system.**

## The levelled search, `FUN_007ee980`

```c
iVar7 = 3;                                   // level 3: cells of 8x8 tiles
while (true) {
    iVar4 = (int)param_2 >> iVar7;           // x >> level
    iVar6 = (int)param_3 >> iVar7;           // y >> level
    if (iVar7 == 0 || (*(byte *)(*(int *)(*(int *)(*in_ECX + iVar7 * 4) + iVar4 * 4) + 1 + iVar6 * 2) & 0x20) != 0)
        break;
    iVar7 = iVar7 - 1;
}
local_24 = FUN_007ee760(iVar7, iVar4, iVar6, (int)param_2, (int)param_3);   // coarse A* over cells
```

- **levels are 3, 2, 1, 0** and a level's cell is **`2^level` tiles** per side: 8×8, 4×4, 2×2, 1×1. One pointer per level at
  `*in_ECX + level*4`, so the grid is four parallel maps.
- **the descent stops at the coarsest level whose cell carries flag `0x20`** (the same bit `FUN_007ee760` sets while opening a node),
  and at level 0 unconditionally.
- **`FUN_007ee760(level, cx, cy, tx, ty)`** returns null on an empty cell, else allocates a node with
  **`FUN_007ef170`** (`operator_new(0x2c)` from a pool at `in_ECX[0x24]`), whose fields are `[1]=x [2]=y [3]=level [6]=f [7]=g (param_4)
  [8]=next [4],[5]=goal` and `[0]` = the cell's byte in that level's bitmap. `f = g + h` with **h = euclid** (a `float10`
  sqrt), so the coarse search uses **straight-line h in cells**, not the octile h of the tile search.
- **refinement walks the chain outwards**: `FUN_007edb80(node, &x, &y)` gives a node's tile, and `FUN_007eeda0(node, &fx, &fy, nextX,
  nextY, ...)` steps from one cell toward the next, and each step that succeeds pushes a waypoint with `FUN_007fd880(&point)` -
  **cell centres `+ 0.5`, at `2^level` spacing**. So the route handed to the movement code is a **coarse polyline**, not a tile path.
- the search frees its nodes back to three pools (`in_ECX[0xe]`, `[2]`, `[0x19]`) at the end and un-marks each (`*b &= 0xf`).

## The flat tile search, `FUN_008007f0` (the other branch)

- a **480×480 per-tile cost array** (`in_ECX + 0x38414`, index `x*0x1e0 + y`, 0xE1000 bytes ending at `0x119414`), with a second
  57,600-int array at `in_ECX + 0x14` that is cleared to `-1` every thirty-second search (`0x2db424 += 8`, wrapping at `0xf0`);
- **h is octile, scaled by 16**: `FUN_007ffd10` returns `DAT_00afd2a0[diag] + 16 * straight`, and the neighbour cost is the same
  metric (`FUN_007ffc80`, without the `0x1df` clamp);
- the expansion is the classic delta form: `f' = g(nbr) + (h(cur) - h(nbr)) + g(cur)`, and a node is dropped when
  `seen < 8 && cost[node] <= f'`;
- **the budget is an expansion count from the straight-line distance**:
  `iVar20 = (int)FUN_007ffcd0(start, goal) * DAT_00aad420 / 100`, and `(int)dist * 0x19` (25) when `FUN_0074e020() == 1`;
  the loop breaks on `iVar20 < expanded` or on **`20000 < expanded`**;
- **on exhaustion it returns the best node it reached** (`local_490`, `local_458`), which is exactly the "best settled node" fallback
  this engine already has.

## The independent source, and it agrees

`docs/pathfinding-conformance.md` already carries the AoC decompilation's statement of the same machine, read before today from a
different binary:

- *"the search is **levelled**: a coarse node spans `<< level` tiles and an edge is refined to a tile on its boundary"*
  (`zone_spatial_pyramid_search_hierarchical.cpp`) - the `>> level` descent and `FUN_007eeda0`'s refinement, independently;
- *"a node is `(x, y, level, g, h, parent, dir)`; `h` is Euclidean from the squared distance"* (`zone_spatial_pyramid_eval_cost.cpp`)
  - exactly the field layout `FUN_007ef170` writes (`[1]x [2]y [3]level [6]f [7]g [8]next`) and the `float10` sqrt `FUN_007ee760` feeds
  it;
- *"the obstacle grid is scratch: cleared and refilled per search, never the shared map"* - the `-1` fill at `in_ECX + 0x14`, which this
  binary re-keys with a wrapping counter instead of clearing every time.

Two sources, two binaries, same structure. That is the crosscheck this step needed.

## What this means for us

| | original | this engine today |
| --- | --- | --- |
| land routes | **four-level coarse A\* → refine to cell-centre waypoints** | one flat A\* over tiles |
| tile search | octile h (×16), expansion budget `25 × euclid`, cap 20,000, best-settled on exhaustion | euclid h (×25), budget, best-settled |
| switch | a registry: levelled when the object has a handler, flat otherwise | none |

So **P11 is real and is the default**, and P1 (the budget) is not a separate problem: the budget belongs to the **flat** search, which in
the original is the *fallback*, so a land unit rarely meets it. That is why "the binary can only afford `h = 25 x tiles` because a coarse
node covers `<< level` tiles" was the right diagnosis in `docs/parity-plan.md` - but the mechanism is a **second search**, not a
different h.

## What is still unknown, and is not being guessed

Two details of the descent are **not** established from the code read so far, and will not be invented:

1. **The cell entry is two bytes**: the stamper `FUN_007f1100` writes bit `0x20` of the **first** byte, while the descent test in
   `FUN_007ee980` reads bit `0x20` of the **second** byte (`+1 + cell*2`). Two different flags. The first is per-search scratch (stamped
   on, cleared off); the second persists and its producer has not been found yet.
2. Therefore **which level the descent really lands on** is unknown. Read literally, the loop stops at the first level whose second-byte
   0x20 is set, coming down from 3 - which would search *coarsely* in cluttered ground and *at tile level* in open ground, and that is
   backwards from every hierarchy I can justify. Until the second byte's producer is read, 11b stays unbuilt.

`FUN_007ede40` stamps a rectangle at **level 3 only** (`>> 3`), then `FUN_007f1100` recurses down the pyramid while the cell it is at is
unmarked, and clips every stamp to the search's own window (`in_ECX[0x42..0x45] >> level`) - so the obstacle grid is per-search and
windowed, which is the AoC note's "scratch, cleared and refilled per search" in this binary's own words.

## The step, and its witness

The witness already exists and is red: `crates/conformance/tests/budget.rs` row 28 (`#[ignore]`d) - **"a 58-tile walk answers from the
hierarchy"**. It is un-ignored first and watched failing, then the hierarchy is built to satisfy it:

1. **11a - the level maps**: for each of the four levels, which cells are open, derived from the tile grid and the same `0x20` mark the
   binary keeps. Its own test: a wall one tile thick closes the level-3 cell and not the level-0 one; a 3-tile gap leaves level 2 open.
2. **11b - level descent**: the coarsest level whose cell containing the start is marked. Its own test: an open field answers level 3, a
   corridor answers level 1 or 0.
3. **11c - the coarse A\*** over cells with `h = euclid`, and refinement to cell-centre waypoints. Its own test: the 58-tile walk.
4. **11d - the switch**: land routes use the levelled search, and the flat search stays for what the registry refuses. Measured against
   the same fixtures that pin the flat search today.

Nothing above is implemented yet; this document is the crosscheck that decides the shape.
