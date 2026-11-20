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
  **cell centres - `n * 2^level + 2^(level-1)` tiles, which is `+ 0.5` only at level 0**. So the route handed to the movement code is a **coarse polyline**, not a tile path.
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
2. Therefore **which level the descent really lands on** came down to what that second byte means. Read as *blocked*, the loop would
   search coarsely in cluttered ground and at tile level in open ground - backwards from any hierarchy. Read as **usable**, it stops at
   the coarsest level whose cell is usable, which is the behaviour a hierarchy is supposed to have.

**The evidence for "usable", read further on 2026-11-19:** the descent reads byte 1 while the stamper writes byte 0, so byte 1 cannot be
the search's own mark; and `FUN_007ed8f0` - the quadtree descent's child test - reads the **same byte** and requires `& 0xf == 0xf`
("clear in all four directions") plus a non-zero byte before a child counts. So byte 1 is the cell's persistent passability, and bit
`0x20` of it is the same "this cell is usable" flag at the top of the nibble range. **11b is built on that reading, and the reading is
labelled an interpretation, not a certainty, in `Levels::descent` and in D297.**

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

## Reading further: the machine is a quadtree, and the driver is still not pinned (2026-11-19, later)

Following the node pool out of `FUN_007ee760` turned up more of the machine and **not** the function that drives the coarse search. What is
now established, each with its call site:

| function | what it does | evidence |
| --- | --- | --- |
| `FUN_007ef170` | node allocator from a pool: `[0]=cell byte, [1]=x, [2]=y, [3]=level, [4],[5]=goal, [6]=f, [7]=g, [8]=next, [9]=parent` | 32 lines, called with a parent as its 6th argument by the searches below, with `0` by `FUN_007ee760` (the start has no parent) |
| `FUN_007ee760` | opens **one** node - the start of the route - and pushes it; returns it | 86 lines; its node's parent is `0`, so the chain it returns is length 1 |
| `FUN_007ed600` | the **masked quadtree neighbour walk**: recurses down `(level+1, x>>1, y>>1)` while a cell is not open, and searches up to four children gated by a direction mask (`param_6 & 3`, `& 5`, `& 10`, `& 0xc`), linking through `+0x24` | 121 lines |
| `FUN_007ee600` | the ray down the quadtree: from a cell toward the goal, picks the best of four children by `FUN_007ed8f0` and returns the first cell where the path stops being clear | 79 lines |
| `FUN_007ed8f0` | the child test: `-1` unless a cell's second byte has all four low bits set and byte 0 is non-zero, else `abs(dx) + abs(dy)` | 50 lines |
| `FUN_007ef950` | opens **8 neighbours at level 0** with `g = h = 0` and pushes them - a reachability flood, not a cost search | 202 lines, 8 calls to `FUN_007ef170(..., 0, 0, 0, 0, 0, 0, 0)` |

**So the levelled machine is a quadtree with masked neighbour walks and a node pool, not the heap/priority-queue A\* the decompilation's
`find_path` describes.** The driver that assembles a *coarse route* out of those pieces - the one that would set a node's parent and `g` -
is one of `FUN_007ef950`, `FUN_007efec0` (272 lines), `FUN_007f0650` (208), `FUN_007ed280` (116), `FUN_007ed600` (121) or `FUN_007edf00`
(143), and **none of them has been read far enough to say which**, because each mixes the search with its caller's bookkeeping.

**What this blocks.** 11c's coarse step cost (`g`). `FUN_007ed8f0`'s `|dx| + |dy|` is the *descent's* child score, and the flood in
`FUN_007ef950` carries `g = 0`, so neither is the A\* cost. Building 11c on a guessed `g` would be a deliberate difference, which the
owner's rule forbids - so 11c waits for that one read, and everything up to it (11a, 11b, the witness) is landed and measured.

**The next read, in order:** `FUN_007efec0` (the largest of the six), then `FUN_007f0650`; in each, the question is *what is passed as
`FUN_007ef170`'s 4th argument (`g`) and 6th (`parent`)*, because whatever that is, it is the coarse A\*'s cost.

## FOUND: the coarse A\*'s node opener and its cost table (`FUN_007ed280`)

Read 2026-11-19, last of the six candidates. `FUN_007ed280(level, x, y, direction, extra)` opens one neighbour of a coarse node:

```c
// the search window first - a cell outside in_ECX[0x42..0x45] is refused outright
if (param_1 < in_ECX[0x42] || in_ECX[0x44] < param_1 || param_2 < in_ECX[0x43] || in_ECX[0x45] < param_2) return;
iVar2 = *(int *)(*(int *)*in_ECX + param_1 * 4);          // the level's grid
if ((*(byte *)(iVar2 + param_2 * 2) & 0x10) != 0) return;  // closed (0x10) - already done

// ... iVar6 = the STEP COST, by direction param_4 (1, 2, 4, 8 = the four directions) ...

if (*(char *)(param_2 + *(int *)(in_ECX[0x4d] + param_1 * 4)) != '\0') iVar6 = 2000;  // a body stands here
uVar5 = FUN_007ef170(param_1, param_2, 0,                    // level pinned to 0 here: a tile node
                    (float)(iVar6 + (int)*(float *)(param_3 + 0x1c)),   // g = step + parent's g
                    (float)iVar4,                            // h = distance to the goal ALONG ONE AXIS
                    param_3, 0, 0, 0);                       // parent = the node it came from
FUN_007ed500(uVar5, 0);                                      // sorted insert into the frontier list
*pcVar1 = *pcVar1 + '\x10';                                 // close the cell
```

**The step cost table**, exactly as the code branches:

| situation | cost |
| --- | --- |
| on the map border or the window's edge | **10** |
| an open cell whose four orthogonal neighbours are all clear | **10**, plus `(coord - window_lo) + 10` (directions 1/2) or `(window_hi - coord) + 10` (4/8) - **farther from the edge you are leaving costs more** |
| an open cell with a blocked orthogonal neighbour | **300** |
| a cell that is blocked but `FUN_00773bf0` says a body may cross | **100**, or **300** if `FUN_00773c80` says otherwise |
| a cell where a body may not cross | **600** (+10 if `param_5`) |
| **a body standing on the cell** | **2000** |
| the neighbour is the window's far edge in the direction of travel | `-100` (a sentinel: it skips the cost, `LAB_007ed468`) |

**h is one-dimensional**: `iVar4 = (window_hi - coord)` along whichever axis the direction moves on - not Euclidean. Euclidean h is used
only for the *start* node in `FUN_007ee760`. And the frontier is `FUN_007ed500`'s **insertion-sorted linked list** on `f`, with
`f = g + h` stored as a float at `0x18`.

So the levelled search is a **4-directional A\* over coarse cells with a heavy non-uniform cost table** (a cell body: 2000; hugging an
obstacle: 300; open ground: 10 plus a distance-from-the-edge term), a **1-D heuristic**, and a sorted linked-list frontier. It is not an
8-directional grid A\* and it is not admissible in the textbook sense - it is tuned, and the tuning is the game's behaviour.

**11c is therefore buildable exactly**, and the order is: the cost table (a fixture per row, since each row is a `300`-scale difference),
then the coarse A\* with its window and its 4 directions, then the refinement to cell centres.

## The pyramid cannot be cached until bodies and blockers are told apart (2026-11-19, D300 step 1)

D300's first step was "cache the pyramid on `Ground`, invalidated by the dirtied flag". Reading `Ground` to do it showed why that is not a
cache problem:

- `Ground::blocked(x, y)` is `!walkable(x, y)`, and `walkable` is terrain **and** `occupants == 0`;
- `block`/`unblock` are called by **every unit** as it steps on and off a tile, so `occupants` changes on almost every tick;
- therefore **any** pyramid built from `blocked()` is stale after every unit move, and a dirty flag on it fires constantly. The cache would
  almost always miss.

**The binary does not have this problem because it separates the two kinds of occupancy**: static obstacles are stamped into the level maps
(`FUN_007ede40` / `FUN_007f1100`, per search and clipped to the window), while **bodies** are a *cost row* read live during expansion
(`FUN_007ed280` scores a cell with a body on it **2000** and lets the search through it). Two structures, two lifetimes.

Our `Ground` has one: `occupants` counts trees, buildings and units together. So D300's step 1 is really a **modelling** step:

1. split `Ground`'s occupancy into **static blockers** (terrain, trees, building footprints) and **bodies** (units), which is the
   distinction the original keeps and the one the `step_cost` 2000 row already assumes;
2. build the pyramid from the static side, invalidated only when a static block changes (a building placed or destroyed, a tree felled);
3. pass the body set to the coarse search as the **cost row**, not as blocked cells - which is `FUN_007ed280`'s shape exactly.

**That also removes two of the engine's current approximations**: the flat search treats a unit's tile as blocked (so a crowd plans round
its own friends), where the original prices it at 2000 and lets a route exist. It is a behavioural change as much as a performance one, and
it needs its own witness before it lands.

**Nothing is implemented for this yet.** It is written down because it is the actual next step, and because a "cached pyramid" bolted onto
`Ground` as it stands would have been a cache that never hits.
