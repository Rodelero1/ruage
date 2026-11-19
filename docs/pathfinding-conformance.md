# The pathing conformance set

> **REWRITTEN BY D279 (2026-11-18).** The set now states the ported model's rules - the grid, one A\* at the original's
> costs, the line test, the closest-tile answer - across five files (`cost_model`, `walkability`, `obstacles`, `findings`,
> `divergences`): **13 tests green and 3 `#[ignore]`d**, because D279 supersedes D249's "nothing is ignored": the three
> ignored ones are the layers this port deliberately does not carry (hierarchy, radius-aware search, expansion budget), each
> with what it would cost. Everything below this note describes the *previous* set and the rules it asserted.

**What it is.** `crates/conformance` asserts the pathfinding and obstacle-avoidance rules of **the original** against this
engine's planners - one rule per test, the source of every rule named in the test's doc comment. It is a second set, run on
its own:

```sh
make conformance
```

**Nothing in it is `#[ignore]`d** (D249, the owner: *"None of the conformance tests is to be ignored. If it collides with HD
functionality, please tell me and let me decide with provided findings from the game."*).

**State of the set at 2026-11-18, with `--no-fail-fast`: 36 tests, 35 green, 1 red.** (The first run of the day reported
"15 tests, 14 green, 1 red" and that number was wrong twice: `make conformance` stops at the first failing binary, so four of
the seven test files had not run at all. The second red —
`walkability::open_ground_costs_the_octile_distance_and_no_more` — only appeared once the whole set ran, and it was a fixture
that predated the port: it asserted an **optimal** route, which a weighted search does not give. It now asserts the game's own
guarantee, `octile <= cost <= octile x 25/16`, and passes.) The table below is the *original* 29-rule survey and its verdicts are the ones read on 2026-11-16; several
have moved since, so read it as the register of what was read rather than as the current score:

- **green now, red in the table**: row 27 (the cost model - `the_originals_step_cost_is_sixteen` passes; `path.rs` carries
  **16 / 23 / weight 25**, landed in `16853ac`), and rows 19/26/31's closest-point answer (the search settles on the tile
  nearest the goal rather than answering `None`).
- **the one red**: row 28, `divergences::the_original_solves_a_long_walk_in_its_hierarchy` - `short_range` declines a 58-tile
  walk because its window is bounded at 40 tiles, and the original has a levelled search for exactly that.
- **not in the set at all**: the expansion budget (`expansions > h`, cap 20,000) and the octile table's exact values; both are
  read but not ported, and neither has a fixture.


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

## Rule set 1 read to the end: the edge mask is **derived**, so passage needs no new representation

Rule set 1 (`crates/conformance/tests/walkability.rs`, 8 tests, all green) rests on
`zone_spatial_pyramid_check_tile.cpp`'s four-bit neighbour mask. The question a port has to answer is whether that mask is
**authored** (a wall stored between two tiles, which our per-tile `bool` could not express) or **derived** (a cache of the
four neighbours' passability, which it can). Read at 2026-11-18, both halves of the file:

- **The reconstructed C** (`func_001D29F0`, file bottom): the mask is built by four neighbour tests, each of which only asks
  `thresholds[neighbour_terrain] > 0.0f` - a per-terrain passability threshold - and sets `0x2` N, `0x1` W, `0x4` S, `0x8` E.
  A tile whose *own* ground fails the threshold gets `*out_flag = 0` and returns.
- **The MIPS listing at the top of the same file is stricter than the C, and the C is lossy**: at `001D2A38`-`001D2A6C` the
  *own* tile's descriptor is tested for bit `0x4000` and, when it is set, every edge bit is cleared; at `001D2AC8`-`001D2AD0`
  the **neighbour's** `0x4000` suppresses that neighbour's bit. `0x4000` is the **obstacle stamp** -
  `zone_spatial_grid_stamp_obstacle_region.cpp:15` builds it as `((type << 8) & 0x3F00) | 0x4000 | (id << 16)`, i.e. "a thing
  stands on this tile".

So the game's rule is **`bit set <=> that neighbour is passable ground with nothing standing on it`** - exactly
`!walk.blocked(neighbour)`, which is what this engine tests, and exactly why the mask is a **one-byte cache** rather than a
second representation. **Nothing needs to be ported to "have rule set 1"**: the per-tile predicate keeps the same set, and the
diagonal rule (both corners) is the AND of two bits, which is the both-corners test already in `path.rs`.

**What a future reader should not redo:** do not add an edge-mask type to `Walk` to "match the original" - it would be a second
copy of `blocked()` that can disagree with the first, and the original keeps it only because a byte is cheaper than three
lookups. **What is worth keeping** is the C-versus-MIPS lesson: the reconstructed C body dropped the `0x4000` test, so a rule
read only from the decomp's C would have said objects do not affect passage.

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
