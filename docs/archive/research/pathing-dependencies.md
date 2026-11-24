# What each open pathfinding difference depends on (2026-11-20)

*Follow-up to `docs/archive/research/pathfinding-crosscheck-2026-11-20.md`, which listed ten differences and four unknowns. This is the dependency answer:
for each one, what has to be read, modelled or decided before it can be implemented - and which of those are ours to do versus waiting on
evidence.*

## The dependency graph, in the order it can be worked

| # | difference | depends on | kind | can it start today? |
| --- | --- | --- | --- | --- |
| **1** | the **search window** | **read**: the window is not a hidden constant - `FUN_007ede40` writes `in_ECX[0x42..0x45]` from **its rectangle arguments**, and it is called once per entry of a rectangle list the path request passes in. So the window is the caller's rectangle list, and **what that list is** comes from whoever calls the request. | read (`FUN_008000d0`'s callers for the rectangle source) | yes - one more read, no code blocks it |
| **3** | the **window-distance term** | #1 (it needs `window_lo` / `window_hi`) | follows #1 | after #1 |
| **4** | **coarse h** as `window_hi - coord` | #1 | follows #1 | after #1 |
| **5** | **refinement to a tile on the boundary** | **read** `FUN_007eeda0` (208 lines) in full: it steps from one cell toward the next and returns a boundary tile, and the exact rule (how it picks the tile, and when it gives up) is not yet stated | read | yes |
| **6** | **bodies priced at 2000** | (a) per-cell body counts, which the engine can compute from entity positions per tick; (b) the flat search's `avoid` list changed from *blocked* to *cost 2000*; (c) a **decision**: D281 chose the re-plan, and pricing bodies instead changes every crowd route | model + decision | needs your call, not evidence |
| **7** | **radius inside the planner** | (a) the search must test the **grown box** per node, or hold one pyramid per radius class; (b) the levelled pyramid is built for a radius, so it either multiplies by radius or takes the largest body's; (c) a **decision**: D279 parked this, and it changes which lanes are legal | model + decision | needs your call |
| **2** | coarse rows **100 / 600** | the game's notion of "crossable but slow" terrain: our content model has walkable-or-not and no cost class, so this needs a **data** decision (add a terrain cost to `config/`, or read the dat's terrain table for what the game uses) | data | needs a data decision |
| **8** | **pause mechanics** | the 255-entry **ignore list** on the re-path: we have `plan_path_avoiding` already (D281), so the storage exists; what is unread is the four-entity cap and how entries are chosen | read (small) | yes |
| **9** | **cost-array lifetime** | done - D301's cache | - | closed |
| **10** | **straight-line tier** | trace `FUN_008a4360` / `FUN_008a39a0` / `FUN_008a3a20` for a pre-search line test | read | yes |

## The four unknowns, restated as tasks

1. **The window's source.** Not unread any more in the sense of a constant: `FUN_007ede40` sets it from its arguments. What is unread is
   who builds the rectangle list and in what order - the caller of the path request.
2. **`FUN_007eeda0`'s refinement rule** (difference #5's real content).
3. **The 100/600 rows' terrain semantics** - and, behind them, whether the dat carries a terrain cost table we already parse.
4. **The registry's composition** (`FUN_007edab0`): whether every land unit enters the levelled search or only some.

## What is *not* a dependency, and is worth saying

- **The levelled search itself is done** (P11: D296-D302). It is wired, cached, measured, and it cut acceptance by 32%. Nothing on this
  list blocks its use.
- **The tile search's parity is done**: octile table, budget, cap, tie-break, best-settled fallback all match, and the conformance rows
  that were `#[ignore]`d for the budget are the ones P1 can now un-ignore.
- **Bodies and radius are decisions, not evidence gaps.** Both are blocked on "do we want this?", not on "what does the game do?" - the
  game's behaviour is known (2000; grown boxes in the search) and written down.
- **Two differences cannot be closed without new content data**: the 100/600 rows need a terrain cost class, which is a `config/` change
  and a map-format change, not an engine one.

## The shortest path to closing the two visible ones

1. **Radius in the planner (#7)**: per-radius pyramids are the cheap route - four pyramids (0.20, 0.25, 0.40, 0.50 from the dat) instead
   of one, ~21 µs each per ground change at 64×65 - and the search then never plans a lane the body cannot walk. It needs a decision on
   the pyramid's lifetime, not a new read.
2. **Bodies at 2000 (#6)**: a per-cell body count (already derivable from `entities`), the flat search's `avoid` turned into a cost, and
   the coarse `step_cost`'s `body` argument fed from it. Also a decision: it changes how crowds route, which the owner has already ruled on
   once (*"let the crowds jam"*, D289) in the step.
