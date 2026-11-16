# The pathing conformance set

**What it is.** `crates/conformance` asserts the pathfinding and obstacle-avoidance rules of **the original** against this
engine's planners - one rule per test, the source of every rule named in the test's doc comment. It is a second set, run on
its own:

```sh
make conformance
```

**Nothing in it is `#[ignore]`d** (D249, the owner: *"None of the conformance tests is to be ignored. If it collides with HD
functionality, please tell me and let me decide with provided findings from the game."*). One test per rule, and the state of
the set is the honest answer: **28 green, 5 red**. The red five are three rules of the game this engine does not keep (U25 a,
b, c) and one defect of ours with its fix measured and held back (U25 d). A green run would be the claim *we keep all 33*.

**Why the gate never sees it.** The crate is its own workspace (`crates/conformance/Cargo.toml` carries `[workspace]`, the root
lists it in `exclude`), so a red conformance test cannot turn the repository's gate red - `cargo test --workspace` does not
build it. That is what lets it be red without holding the tree hostage, and it is why the set must stay honest rather than be
weakened into passing.

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
| 19 | an unreachable click is answered with a **standing** tile and a route | same - **but not the nearest one**: U25 (d) |
| 20 | every segment keeps the body's radius clear of every blocked tile | same |
| 21 | a wider body keeps a wider clearance | same |
| 22 | a body too wide for a gap is refused, not squeezed | same |
| 23 | a body's disc is kept clear **and not padded** | same (the exact clearance, D211) |
| 24 | a start inside a body is re-seeded outward and still arrives | same (D212) |
| 25 | a window beyond 40 tiles is declined, and says so | ours, by decision |
| 26 | the original answers the **closest point**, ours answers `None` | **red** - U25 (a) |
| 27 | the original's step cost is 16 with an octile table; ours is 10/14 | **red** - U25 (b) |
| 28 | the original has a levelled hierarchy for long walks; ours declines past 40 | **red** - U25 (c) |
| 29 | the tile answered for an unreachable click is the one nearest the click | **red** - U25 (d) |

## The defect the set found, measured twice

`findings.rs` holds two tests that state **our own** rule, not one of the game's. Both found the same defect:

> `nearest_reachable` answers **one step from the start**, not the tile nearest the click.

Measured: on empty ground, from (1,1) toward (10,10), the answer is **(2,2)** with a one-step route. The function pops its
frontier best-first by distance to the click, but every tile it pushes is already free, so the first pop is a neighbour of the
start and it returns before the wavefront has moved. Its own doc comment and D134's *"nearest legal standing point"* both
claim the opposite.

Why the world suite never saw it: `click_into_the_woods.rs` spawns the villager **one tile** from the wood, so "a legal tile
within one tile of the block" is satisfied by the bug's own answer. The contract held for the case the fixture happened to
build.

**The fix was written, measured and reverted** (D249). Sweeping the reachable region answers the right tile - and costs the
acceptance match: **155 entities and a win at `45b5bd2`, against 125 and no win**, same worktree, same data, seed 7, one file
different. The cause is measured too: the script issues exactly two unreachable clicks, both `(26,13) -> (25,17)`, and the
sweep walks the villager six tiles to `(25,19)` where the bug walked him one - a diverted villager at 1,303 s is enough to
lose the win. The call itself is cheap (**143 us** on a 64x64 map, 1.1-3.2 ms on 200x200, fallback path only); the match is
what refused it. Adopting the rule therefore means **retuning `config/acceptance.toml`**, which is ours to retune - the game
has no one-step answer. That is the owner's call (U25 d).

## What it does not cover

- **No world, no tick.** Every rule is checked against `Walk`, `find_path`, `find_path_avoiding`, `line_clear`,
  `short_range`, `short_range_clear` and `nearest_reachable`. The step layer's collision rules (the pause, the past filter,
  the mate exemption) are the movement register's business, not this set's.
- **No timing and no route shape beyond cost.** A levelled search returns the same *cost* as a flat one; whether the very
  polygon matches is not something a tile API can answer.
- **Nothing derived from the decomp's C bodies is trusted as byte behaviour.** The bodies reconstruct PS2 assembly, and at
  least one (`find_path`, `func_001D66E0`) loops `w` for both axes - the reconstruction is a guide to shape, so a claim
  enters the table only where the masks, the node layout or a first-party source agrees.
