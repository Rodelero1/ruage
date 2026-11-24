# Review: the coarse step cost table (P11c, D298)

*The last unread number of the levelled search. `FUN_007ed280` opened it: it is the coarse A\*'s neighbour opener, and its cost table is
nothing like a textbook grid A\*'s.*

## What the binary says

| cell being entered | cost |
| --- | --- |
| on the map border or the search window's edge | **10** |
| open, and all four orthogonal neighbours clear | **10**, plus `(coord - window_lo) + 10` in the direction of travel - **farther from the edge you left costs more** |
| open, but with a blocked orthogonal neighbour | **300** |
| blocked, but a body may cross it (`FUN_00773bf0`) | **100** (or **300** when `FUN_00773c80` disagrees) |
| a body may not cross it | **600** |
| **a body standing on it** | **2000** |

`g = step + parent's g`, `h = window_hi - coord` **along one axis only** (Euclidean h is used only for the start node), and the frontier
is `FUN_007ed500`'s insertion-sorted linked list on `f`, newest-first among equals. **This is a tuned 4-directional search, not an
admissible 8-way A\***; the tuning is the game's route shape.

## What landed

`Levels::step_cost(level, cx, cy, body) -> Option<i32>`, the rows our content can express:

- blocked → **`None`** (never entered);
- a body on the cell → **2000**;
- open with a blocked orthogonal neighbour → **300**;
- open and clear on all four sides → **10**.

**The 100 and 600 rows are named and not invented**: they need "crossable but slow" terrain, and this content model has walkable-or-not
only. When terrain costs arrive, those rows are where they go - said in the function's doc comment, not just here.

The distance-from-the-edge term is deliberately **not** in `step_cost`: the search adds it, because the search is what knows the window.

## Test-first

Written and watched failing: `error[E0599]: no method named 'step_cost' found for struct 'Levels'`. Then a **fixture error of mine, again
of the same kind as the last one**: I asked `cell_of(3, (16, 16))` about the level-3 cell holding a wall at `(16, 15)` - a different cell -
and got a correct `300` where I expected a refusal. Fixed to ask the wall's own cell, and the three fixtures are green:
**3 passed / 0.00 s**.

## Performance

- `cr::coarse_cost`: **3 passed / 0.00 s**; each answer is one blocked lookup plus at most four more, i.e. **O(1)**;
- full suite: **910 passed / 6 failed / 21 ignored**, from 907 / 6 / 21 - the three new fixtures and nothing else;
- the red set is **byte-identical** to before this commit (`diff` of the sorted names reports nothing);
- clippy clean at `-D warnings` after two `i32::try_from` calls that could not fail became `i32::from` (clippy was right: the map is a
  `u16` and a `u16` always fits an `i32`).

## Stability

**The risk in a cost table is silent asymmetry** - a cost that makes a route prefer one side of a wall cannot be seen in a single
fixture, only in a route. So the table is kept as its own function with its own fixtures (each row a `300`-scale difference), and the
search that consumes it will be judged by routes, not by costs.

**The two unimplemented rows are a known gap, not a silent one**: a search over this table treats every non-walkable tile as 600-and-refuse
rather than 100-and-cross, which makes our coarse routes *more* conservative than the original's on rough ground - the opposite direction
from routing through a wall, which is the direction a mistake should fail in.
