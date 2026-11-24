# Review: the coarse A\* and its refinement (P11c, D299)

*The search itself. Built from `FUN_007ed280` (the opener and its cost table, D298) and `FUN_007ee980` (the descent and the chain walk,
D297); the conformance row that had been `#[ignore]`d for a year of sessions is now **green**.*

## What landed

`Levels::coarse_route(from, to) -> Option<CoarseRoute>` and `Levels::refine(&route, scale)`:

- the level is `descent(from).min(descent(to))` - the start's coarsest usable level, never finer than the goal needs;
- **four directions**, never diagonal;
- `g = step + parent's g` with `Levels::step_cost` (10 / 300 / 2000);
- `h` = Manhattan between cells;
- the frontier is **insertion-sorted on `f`, new before equal `f`** - `FUN_007ed500`'s order, with a `Vec` and a binary insert instead of
  a linked list;
- closed cells never reopen; a cell already on the frontier is skipped unless the new path is strictly shorter;
- the parent chain is walked back and reversed, then `refine` emits **one point per cell at its centre**.

`CoarseRoute { level, cells }` is public so a fixture can talk about the chain rather than about waypoints.

## Two deviations, both recorded, both safe

1. **No search window.** The binary clips every expansion to `in_ECX[0x42..0x45]`, a box whose padding is set somewhere not yet read.
   Without it this search can find routes the clipped one refuses - a superset, never an illegal route.
2. **`h` is Manhattan**, because `window_hi - coord` needs the window that was dropped.

Both are in the function's doc comment and in D299.

## Test-first, and three errors of mine on the way

`E0599` first (no `coarse_route`, no `Ground::scale`), then:

1. the fixture map declared `height = 96` with **49 rows** written - the loader refused it, correctly;
2. player 2's town centre sat at `(90, 90)` on a 49-row map - out of bounds, refused;
3. the conformance witness's lattice check used the **tile** convention (`n * 8 + 0.5`) for an **8-tile cell centre** (`n * 8 + 4`) and
   flagged a correct `4.00` as wrong. The evidence file carried the same mistake and is corrected.

## Performance

- `crates/sim/tests/coarse_route.rs`: **4 passed / 0.01 s** on a 96×49 map - a 58-tile walk searched at level 3 is 8 cells instead of 58
  tiles, which is the whole point of the hierarchy;
- the frontier is a `Vec` with `position`-based insert: O(n) per push on a frontier of tens of nodes at level 3, fine at this size and
  measured below; it is **not** a heap, deliberately, because the binary's order is an insertion-sorted list;
- full workspace suite: **914 passed / 6 failed / 21 ignored**, from 910 / 6 / 21 - the four new fixtures and nothing else; the
  red set is **byte-identical** to the previous commit (`diff` of the sorted names reports nothing);
- clippy clean at `-D warnings`, workspace and all targets;
- the conformance row `the_original_solves_a_long_walk_in_its_hierarchy` is **no longer `#[ignore]`d** and passes: `1 passed / 1
  ignored`.

## Stability

**A* over a cost table that treats "hugging a wall" as 30× open ground is stable in the direction that matters**: it will never prefer a
cell that touches an obstacle unless it is far cheaper overall, so the routes it returns are less likely to be the ones that jam a body
in a doorway.

**The failure mode this cannot see is a frontier that keeps growing** - the O(n) insert is quadratic in the worst case, and the
`a_walled_off_goal_has_no_coarse_route` fixture pins the *outcome* (None) but not the cost of getting there. That cost is the number to
watch when 11d wires this into `Ground::route` and a hundred units plan on one tick; it is not measurable at 96×49 with no units.

**Two known-unfaithful pieces remain**, both named: the window, and the 100/600 terrain rows from D298.
