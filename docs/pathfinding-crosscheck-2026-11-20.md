# Pathfinding and obstacle avoidance: where we match, where we differ (crosscheck, 2026-11-20)

*Requested by the owner: crosscheck our implementation against the original game, the decompilation and our own sources, and **flag the
differences**. Sources: the REA pseudocode corpus (`/opt/data/tools/rea/pseudo-aokhd`, HD 5.7), `docs/pathfinding-levels-evidence.md`,
`docs/movement-in-the-binary.md`, `docs/pathfinding-conformance.md` (which carries the AoC decompilation's statements),
`docs/press-depth-evidence.md`, and the code as it stands at `b262c1a`.*

## What matches, with the anchor

| behaviour | original | ours |
| --- | --- | --- |
| flat tile search | 8-way; h = octile ×16 with 23 diagonal (`FUN_007ffd10`, `FUN_007ffc80`) | `Ground::octile`, 23 + 16 - **same table** |
| budget | expansions ≤ `euclid × 2500 / 100`, hard cap **20,000** (`0x800d57`) | `ESTIMATE = 25`, `EXPANSION_CAP = 20_000` |
| exhausted search | returns the **best settled** node, first-found wins on ties | same, with the octile tie-break (D286) |
| unreachable goal | "come as close as you can" | same: the route ends at the nearest settled tile |
| levelled search exists and is the land default | `FUN_008000d0` → `FUN_007ee980` | tier 2 of `route_with` (D302) |
| levels and cells | levels 3..0, a cell `2^level` tiles, indexed `tile >> level` | `Levels`, same four levels and indexing (D296) |
| descent | coarsest usable cell, floor at level 0 | `Levels::descent` (D297) |
| coarse neighbours | **four**, never diagonal | four |
| coarse step cost | 10 open / 300 hugging an obstacle / 2000 a body | same three rows (D298) |
| coarse frontier | insertion-sorted list on `f`, newest first among equals (`FUN_007ed500`) | same order, a `Vec` with a binary insert |
| coarse route shape | a chain refined from the parent chain | a chain refined to cell centres (D299) |
| obstacle test | box grown by the mover's radius, collision on `<=` (`FUN_00840830`) | the box ruler (D286), same `<=` |
| collision response | no press floor; a pause of 0.25-1.50 s and a re-path (`FUN_008a4360`, D290) | no press floor; pause and re-path (D290) |
| straight-line shortcut | **unverified** (see below) | tier 1 of `route_with` |

## Where we differ, flagged

> **Rows #5, #6 and #7 were closed after this file was written (D334, 2026-11-21), each by a later read rather than by a
> change of mind, and the file is kept as the record of what was believed on 2026-11-20.** Read the table for #1-#4 and
> #8-#10 only:
>
> - **#5, refinement granularity** - fixed by **D322**: `Levels::refine` emits the crossing of each shared edge, not the cell
>   centre. (The binary scans the span outward from the interpolated point; ours scans from the low end - still a difference,
>   and a smaller one.)
> - **#6, bodies priced at 2000** - **refuted by D328**: `[0x4d]` is the eight-direction run-mask grid `FUN_007ef3f0` builds, not
>   an occupancy grid, and the only body-aware test in the path system (`FUN_00801a20`) is in the flat search. There is no
>   2000-per-body row to adopt, so this was not a difference from the game at all.
> - **#7, the radius in the planner** - fixed by **D306**: `Ground::fits`, `pyramid_for(reach)` and `leg_is_clear(..., reach, ...)`
>   carry the body's width into the plan. What remains of #7 is `P3`'s box inset and the plan/step agreement, not a point planner.
>
> The four rows below that are still live are #1 (the search window), #2 (the 100/600 terrain rows), #3 (the window-distance
> term) and #10 (the straight-line tier); #4 is #5; #8's nudge is gone (D279) and the forty-tick finish with it (D292); #9 is a where-not-when difference.

| # | area | original | ours | why it is there | direction of the error |
| --- | --- | --- | --- | --- | --- |
| **1** | **search window** | every expansion clipped to `in_ECX[0x42..0x45]`, a box whose padding is set in code not yet read | no window - the whole ground is searchable | unread padding; refusing to guess it | **superset**: we find routes the game would refuse, never an illegal one |
| **2** | **coarse cost rows 100 / 600** | "blocked but a body may cross" = **100**; "not crossable, and the search still opens it" = **600** | absent: a blocked cell is refused | our tiles carry no cost between walkable and not | **more conservative** on rough ground |
| **3** | **window-distance term** | open cells cost `10 + (coord - window_lo) + 10` (directions 1/2) or `10 + (window_hi - coord) + 10` (4/8) - travelling away from the edge you left costs more | not implemented (needs the window of #1) | side effect of #1 | **different route shape** near map edges; not yet measurable in a fixture |
| **4** | **coarse h** | one-dimensional: `window_hi - coord` along the axis of travel | Manhattan between cells | #1 removes `window_hi` | minor: both are lower bounds on a 4-way grid |
| **5** | **refinement granularity** | an edge is refined **to a tile on the shared boundary** (`FUN_007eeda0`, `zone_spatial_pyramid_check_edge.cpp`), so the game's own routes are finer than cell centres | one waypoint **per cell centre** | that is what `FUN_007ee980`'s chain walk emits for a node, as read | **coarser routes than the game's** - visible at 8-tile spacing |
| **6** | **bodies in planning** | a body's cell costs **2000** and the route passes through it (`FUN_007ed280`, occupancy grid `in_ECX[0x4d]`) | a body's tile is **blocked** (`avoid`, D281) for the flat search, and the levelled tier ignores bodies entirely | D281 chose the re-plan; matching 2000 is not implemented | **behavioural**: we route *around* friends where the game routes through and pays - a crowd's routes differ |
| **7** | **radius in the planner** | obstacles are expanded by the mover's radius **inside the search**, so a gap too narrow for the body is never planned (`zone_spatial_pyramid_check_cell_step.cpp`, the "shrink the unit to a point" talk) | the radius lives in the **step** (`World::clearance_at`), so a route may be planned through a lane the body cannot fit and then refuse its steps | D279 parked it; the conformance row is `#[ignore]`d with that reason | **real and visible**: every route in this engine is a route a *point* could walk |
| **8** | **pause mechanics** | `setPause(0.25-1.50 s)` at `0x8a8141`, `getMaxVelFwd` returns `0.0` during it, and the re-path at `0x12b2` carries an ignore list (four entities, cap 255) | a 40-tick (`STALLED_TICKS`) refusal counter that nudges, plus the re-plan; **no 255-entity ignore list** | D200/D281 shape | **different**: our nudge has no counterpart in the binary; its ignore list has none in ours |
| **9** | **cost-array lifetime** | a persistent per-search stamp array with a wrapping generation counter (`0x2db424 += 8`, wraps at `0xf0`) | a cached pyramid invalidated on every object change (D301) | D301 | equivalent in behaviour; a difference in *when* the work is done |
| **10** | **straight-line tier** | **unverified** - no reader of the binary's move code has established whether a clear line short-circuits the search | tier 1: a clear leg answers in 477 ns with one waypoint | 477 ns is cheaper than any search and every fixture passes | **possibly ours alone**: if the binary always searches, our tier is a deliberate speed difference |

## Unverified, and therefore not claimed either way

1. **Whether the binary short-circuits a straight line** (difference #10). Not read yet; the move code's callers
   (`FUN_008a4360`, `FUN_008a39a0`, `FUN_008a3a20`) have not been traced for a pre-search line test.
2. **The search window's padding** (difference #1).
3. **The 100/600 rows' terrain semantics** (difference #2) - what the game considers crossable-but-slow.
4. **Whether every land unit enters the levelled search**: the switch is a registry lookup (`FUN_007ef140` → `FUN_007ef0f0` →
   `FUN_007edab0`) and the composition of that registry has not been read.

## The four differences that change what a player sees

> **As of D334 this list is down to one and a half**: #7 is closed (D306), #6 was never a difference (D328), #5 is closed
> (D322). What is left is #3 (routes near a map edge do not bend away from it) and the unverified #10 (our straight-line tier).

1. ~~**#7, radius in the planner**~~ - closed by D306; the plan is no longer a point's plan.
2. ~~**#6, bodies priced at 2000**~~ - refuted by D328; there is no such row in the binary.
3. ~~**#5, refinement granularity**~~ - closed by D322; waypoints are on the cell boundary.
4. **#3, the window-distance term** - routes near a map edge should bend away from the edge; ours do not (`D299`).

Numbers 1, 2 and 4 are all consequences of one unread thing (the window) or one unmodelled thing (terrain cost). Number 7 is a design
decision this project made under D279 and has never revisited with a measurement.
