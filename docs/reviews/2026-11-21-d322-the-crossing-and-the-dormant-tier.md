# Review: the chain refines to the crossing, and tier 2 turns out to be dormant (D322)

*Change: `Levels::refine` emits **the crossing of each shared edge** - the first tile of the cell's span the step can pass, with the point
on the boundary and the other coordinate at that tile's middle - instead of the cells' centres (`crates/sim/src/levels.rs`). The shape
fixtures that pinned centres are re-specified to crossings. The 16-tile gate was **tried and kept**, with the measurement that keeps it.*

## What the original does

`FUN_007ee980` walks the parent chain a node at a time; `FUN_007eeda0` turns each step into a point on the **boundary between the two
cells**: it expands the cell to the tile span it covers (`x << level .. ((x + 1) << level) - 1`), **scans that span tile by tile** along
the direction of travel, and takes the first tile where the direction is passable, interpolating the other coordinate. So the waypoints
sit where the passage is, on the boundary - not at the middles of the cells. Ours emitted centres, one per cell.

## Test-first

`coarse_route::the_chain_refines_to_the_crossing_of_each_shared_edge` replaces `the_chain_refines_to_cell_centres`: the first point is
still the start cell's centre (the caller discards it - the walk begins where the body stands), then **one point per step, on the `n * 8`
boundary line, at a tile's middle along it**, and no trailing centre because the goal is the caller's own point. **Watched red** on the
parent commit (`points.len() == cells.len() + 1` fails: centres give one point per cell), green with the change.

`coarse_route::a_blocked_walk_is_answered_by_the_hierarchy` was pinning centres too - `waypoint (160000, 10000) is not a level-3 cell
centre` - so its assertion became "every interior point lies on a level-3 boundary and at a tile's middle", which is the same claim about
the same route under the shape the binary gives it.

## The gate: tried, measured, kept

The plan was to remove the 16-tile gate now that the refinement matches. Measured on a 40x40 open map (single tree two tiles off the line,
walk from (8,6) to (8+gap,6); and a wall at x=12 with one gap at y=10, walk (8,6) to (16,6)):

| walk | gate on | gate off |
|---|---|---|
| past one tree, 6 tiles | **2** points | 2 |
| past one tree, 8 tiles | **2** | **7** |
| past one tree, 10 tiles | **2** | **7** |
| past one tree, 12 tiles | **2** | **8** |
| past one tree, 16 tiles | 9 | 9 |
| past one tree, 24 tiles | 11 | 11 |
| round the wall, 8 tiles | 3 | 3 |

Removing the gate **makes short blocked walks worse**, and not because of the refinement: the chain for a walk 8 tiles long, round one
tree, comes back at **level 2 with seven cells** - `(2,1) (2,2) (2,3) (3,3) (4,3) (4,2) (4,1)` - i.e. it dives six tiles down and back up
because a single tree closes a whole 4-tile cell. Seven crossings where the flat search wants two. The gate stays, and its comment now
records *this* reason rather than "until the refinement matches".

## Performance

- **The acceptance match is bit-identical**: `c7191f531bd81b1b`, 141 entities, 7,125 repaths, 12.6 / 12.7 s (parent: 141 / 7,125 / 12.9 s).
  Not "no regression" - **no effect at all**, and the reason was measured rather than assumed.
- **Tier 2 never answers in the match.** Instrumented over the full 48,000 ticks: **11,042** route calls, **1,677** of them at least 16
  tiles, **0** answered by the hierarchy. Two independent reasons, both counted:
  - every one of the 1,677 long calls carries a **non-empty `avoid`** (1,276 with one body, 213 with two, and a tail up to 16), and tier 2
    is skipped whenever bodies are being avoided - a coarse cell knows where objects stand, not where bodies are;
  - the coarse search returns **level 0** for every one of them. On this map the descent floors out at tile granularity, so `level >= 2`
    would have rejected them anyway.
- Full suite: see the log; the change touches `levels.rs` only, and `coarse_route`, `route_shape`, `straight_line`, `walk_line` are green.

## Stability

- `refine` is pure: it maps a cell chain and the ground's blocked bits to points. It reads `blocked(0, ..)`, has no state, and can only
  return points the caller's own leg test then certifies - the tier is abandoned if any leg is not clear, so a bad crossing degrades to the
  flat search instead of walking into a wall.
- The span index is bounded by `1 << level` with `level` capped at 15, and both tile conversions are `u16::try_from` with `continue`, so an
  out-of-map span cannot panic or wrap.
- **What this cannot fix, and now says so**: tier 2's dormancy. The hierarchy is dead in a full match while it is green in every fixture
  that exercises it - the fixtures were never the problem, the call sites are.

Known and accepted: the crossing choice is "first passable tile of the span", which is what the binary's scan does; the binary then
interpolates the other coordinate, and ours takes the tile's middle. That difference is invisible in the acceptance match (the tier never
fires) and is recorded here rather than papered over.
