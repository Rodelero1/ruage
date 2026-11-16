# The pathing conformance set

**What it is.** `crates/conformance` asserts the pathfinding and obstacle-avoidance rules of **the original** against this
engine's planners - one rule per test, the source of every rule named in the test's doc comment. It is a second set, run on
its own, and it is **not** part of the pre-commit gate:

```sh
make conformance                              # the set: 28 rules, all green
cargo test -p ruage-conformance -- --ignored   # the register: 3 deliberate differences + 2 findings, all red by design
```

**Why it is not the gate.** A conformance test states what the *game* does. Where our engine deliberately does something
else it must stay visible without turning every commit red, so the differences are `#[ignore]`d with their reason and their
decision - `divergences.rs` (the game's rule, not ours) and `findings.rs` (our own rule, not delivered). Running
`-- --ignored` prints the whole register, which is how a later slice shows it closed one.

## Where the rules come from

`samuelcaldas/age-of-empires-2-decomp` - a **matching decompilation** of AoC 1.0c, AoK 2.0a and the PS2 ELF. Read at its
`main` on 2026-11-16; the pathing lives in `src/zone/`:

| module | what it settles for us |
|---|---|
| `zone_spatial_pyramid_check_tile.cpp` (`func_001D29F0`) | passage is a **per-edge** property: a tile carries a four-bit neighbour mask (N `0x2`, S `0x4`, W `0x1`, E `0x8`), built from the neighbours' terrain, and a step is legal only where the bit is set |
| `zone_spatial_pyramid_find_path.cpp` (`func_001D66E0`) | the obstacle grid is **scratch**: cleared and refilled per search, never the shared map |
| `zone_spatial_pyramid_eval_cost.cpp` (`func_001D35C0`) | a node is `(x, y, level, g, h, parent, dir)`; `h` is Euclidean from the squared distance; the frontier is a linked list popped in insertion order, not a heap; `g` is a **float** |
| `zone_spatial_pyramid_check_edge.cpp` (`func_001D4440`), `zone_spatial_pyramid_search_hierarchical.cpp` | the search is **levelled**: a coarse node spans `<< level` tiles and an edge is refined to a tile on its boundary |

Two limits, both load-bearing:

- **Its addresses are AoC/PS2, not HD 5.7.** A number that HD carries differently is pinned to HD's own site instead
  (`docs/movement-in-the-binary.md`): 16 per tile step with the octile table `table[min] + 16 * (max - min)`
  (`FUN_007ffd10`), Euclidean × 2500/100 for the estimate, standers inserted per search (`FUN_007f9d80`), a trapped start
  re-seeded by `2 x radius` (`FUN_007fb220`).
- **The repo carries no licence** (all rights reserved). So it is read for **behaviour** and cited by module - never
  transcribed. Nothing in `crates/conformance` is a copy of its expressions; each test states a rule in our own words and
  measures our own API.

## The rules, and where we stand

| # | rule | verdict |
|---|---|---|
| 1 | a diagonal step needs both its corners free | same |
| 2 | two free tiles in a blocked diagonal are not a gap | same |
| 3 | the goal may be a blocked tile (a node stands on its own tile) | same |
| 4 | the start may be blocked (a worker walks off its tree) | same |
| 5 | the map edge is a wall; an off-map goal is refused, not clamped | same |
| 6 | a route is a chain of king moves, no repeats, ending on the goal | same |
| 7 | a tile's occupants are **counted**, not flagged | same |
| 8 | open ground costs the octile distance and no more | same |
| 9 | a diagonal costs 1.4 of a cardinal; the octile table's ratio is 1.4142 | same within 2% |
| 10 | no route beats the octile lower bound | same |
| 11 | a detour is paid for at the gap, at its analytic cost | same |
| 12 | an equal-cost choice is resolved identically every run | same |
| 13 | a search does not depend on what was searched before it | same |
| 14 | an avoided tile is an obstacle the grid does not know about | same |
| 15 | the avoid set is per search, never written into the map | same |
| 16 | an avoided corner cannot be cut diagonally | same |
| 17 | the goal may be in the avoid set (a target, not an obstacle) | same |
| 18 | a walled-in goal has no route | same |
| 19 | an unreachable click is answered with a **standing** tile and a route | same (not the nearest one - finding 1) |
| 20 | every segment keeps the body's radius clear of every blocked tile | same |
| 21 | a wider body keeps a wider clearance | same |
| 22 | a body too wide for a gap is refused, not squeezed | same |
| 23 | a body's disc is kept clear **and not padded** | same (the exact clearance, D211) |
| 24 | a start inside a body is re-seeded outward and still arrives | same (D212) |
| 25 | a window beyond 40 tiles is declined, and says so | ours, by decision |
| 26 | the original answers the **closest point**, ours answers `None` | **different** (row 32, D134's caller layer) |
| 27 | the original's step cost is 16 with an octile table; ours is 10/14 | **different** (route shape agrees) |
| 28 | the original has a levelled hierarchy for long walks; ours declines past 40 | **different** (not built) |

## The findings

`findings.rs` holds two tests that state **our own** rule and fail on our code - they are not game differences, they are
defects the set found by writing the rule down. Both are the same defect:

> `nearest_reachable` answers **one step from the start**, not the tile nearest the click.

Measured 2026-11-16: on empty ground, from (1,1) toward (10,10), the answer is **(2,2)** with a one-step route. The search
pops its frontier best-first by distance to the click, but every tile pushed is already free, so the first pop is a
neighbour of the start and the function returns before the wavefront has moved. Its own doc comment (`path.rs:303-312`)
and D134's *"nearest legal standing point"* both claim the opposite.

Why the world suite never saw it: `click_into_the_woods.rs` spawns the villager **one tile** from the wood, so "a legal tile
within one tile of the block" is satisfied by the bug's own answer. The contract held for the case the fixture happened to
build.

Not fixed here: a behaviour change owes its own slice, its own failing test and its own measurement. `docs/next-step.md`
carries it.

## What it does not cover

- **No world, no tick.** Every rule is checked against `Walk`, `find_path`, `find_path_avoiding`, `line_clear`,
  `short_range`, `short_range_clear` and `nearest_reachable`. The step layer's collision rules (the pause, the past filter,
  the mate exemption) are the movement register's business, not this set's.
- **No timing and no route shape beyond cost.** A levelled search returns the same *cost* as a flat one; whether the very
  polygon matches is not something a tile API can answer.
- **Nothing derived from the decomp's C bodies is trusted as byte behaviour.** The bodies reconstruct PS2 assembly, and at
  least one (`find_path`, `func_001D66E0`) loops `w` for both axes - the reconstruction is a guide to shape, so a claim
  enters the table only where the masks, the node layout or a first-party source agrees.
