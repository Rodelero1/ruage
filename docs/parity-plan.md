# Parity plan: no deliberate difference from the original (owner, 2026-11-19)

> *"I want no deliberate differences to original game. If there is any, I want that adjusted to meet the original or cut off.
> Please plan the sequence of implementation steps to reach parity with original game and decomp."*

This file is the register of every place this engine's pathing and obstacle avoidance is **ours** rather than the game's, what the
original does there (with its address or module), and which of the two answers the owner's rule takes: **adopt the original**, or
**cut ours**. Each row is a step with a proof - a conformance row or a fixture watched failing - and a measurement.

Order is by dependency: the search's own rules first (they bound everything after), then the collision query, then our
inventions on top of it. The two rows marked **FORK** cannot be settled by me: they are places where the owner's own ruling
ordered behaviour the game does not have, so "meet the original" and "the rule I gave you" disagree.

## The register

| # | ours today | the original | answer | source | proof |
|---|---|---|---|---|---|
| P1 | ~~a search with **no budget**~~ **DONE 2026-11-19**: a hopeless goal expands every reachable tile | stops at **`h` expansions** (25 x Euclidean tile distance) and never past **20,000**, then answers the **best node it settled** - it pushes that node with cost 0 and the caller walks the parents from it | **adopt** | `FUN_008007f0_008007f0.c` (REA corpus), guard `0x800d4f`, tail `0x801855`-`0x80189d`, read 2026-11-19 | conformance: a sealed room's expansion count ≤ `h`, and the answer is the closest tile |
| P2 | ~~the nearest-cell answer picks by **(weighted h, g)**, later node wins ties~~ **DONE 2026-11-19** | picks by **octile distance to the goal**, `strict <` so the **first** node found wins | **adopt** | `FUN_008007f0`, `if (iVar21 < local_468)` where `iVar21 = FUN_007ffc80(node, goal)` | conformance: two equally-near cells, the one the search settles first answers |
| P3 | ~~a step is refused only on **strict overlap** (`<`)~~ **BLOCKED, measured**: the plan certifies a leg the step then refuses, so P3 lands with a plan/step agreement about touching - and the owner has ruled for the game (`<=`, D117 withdrawn) | the box test collides on **touching** (`<=`) | **adopt**: collide on `<=`, and let the target cut land one raw unit short of the face so a body can never be wedged by exactness | `FUN_00840830` (`<=`, `+0x268`) | fixture: a body pressed against a wall can still walk along it; the two clicks still end at the radius |
| P4 | a **terrain** refusal buys a new route once a second (`REPLAN_TICKS = 20`) - **BLOCKED on P8**: the off-grid gather/attack goal points need the fallback that sat beside the throttle | re-plans on **every** terrain collision, every update | **cut** ours: re-plan on every refusal | `0x122f`, `0x128b` (*"We have %d non-past collisions and %d terrain collisions, repathing"*) | measurement: acceptance fixture and the wall-press fixture (a man against a wall is the worst case) |
| P5 | an order blocked for **40 ticks** is finished (`finish_blocked_orders`) | no such rule: a walk that cannot advance **stands**; the pause is the only brake | **cut** ours | `0x12b2` (*"Don't know what to do about my collision(s)"*), and nothing in the epilogue ends an order | the fixtures that relied on it re-pointed, then the acceptance fixture |
| P6 | the ground holds a **count** per tile and treats a blocked tile as one **whole tile wide** | an object carries its own **radius pair** from the dat, and the query is a **radius over the objects on a tile** (`obj + 0x20 + index*4`) | **adopt**: per-object boxes in the ground's tile index, built from the pack's radii | `FUN_00844310` (the query), `FUN_00840830` (the box) | conformance: a tree smaller than a tile leaves the rest of the tile walkable; a body radius above half a tile is planned round |
| P7 | the plan's avoid list **skips a body the mover already overlaps**, so two bodies on one tile cannot be planned round | the search starts on the shared tile and **every stander the mover does not ignore is in it** | **adopt**: no skip; the shared tile's occupants are in the avoid list | `FUN_007f9d80`, D282's note | `a_mate_who_is_stopped_becomes_solid` green at its 8,000 raw standoff |
| P8 | a **seat book**: ring places, `respread`, `settle`, `standoff_point`, `gather_place` | no seat book at all: gatherers spread by **collision and pause** only | **FORK** - the book implements the owner's own ruling (*"villager is never to be nudged to leave place he is working at"*), which the game has no equivalent of | `FUN_008a4360`'s epilogue (no placement of a working body) vs the owner 2026-11-19 | - |
| P9 | arrival **"beside a held goal"** within a quarter tile (`D200`), and a crowd **ring point** for a group move (`D271`) | the game sends a group to **formation slots** around the clicked point; there is no "beside a body holding the goal" rule | **FORK** - D271 is the owner's ruling (*"the first comes to the spot and others stand around him"*), and formations are a feature we do not have | `zone_spatial_entity_adjust_heading_step` neighbours; formation slots are unread in HD | - |
| P10 | the pause duration is drawn from **our** seeded stream | the game's own `rand()` wrapper, which its own comment calls *"not considered sync safe"* | **keep** - same interval (0.25-1.50 s), reproducible | `0x8a80ff`-`0x8a8141`, `setPause` | already pinned by `the_loser_stands_for_the_drawn_pause` |
| P11 | one flat A* over every tile | **three** pathfinders: a levelled/mip-mapped route, a tile short-range one, and a polygonal one for combat | **adopt, priced**: the levelled search is a speed layer whose HD addresses are not read - the AoC decomp's `zone_spatial_pyramid_*` is the only source | `docs/pathfinding-conformance.md` row 28 (red, `#[ignore]`d) | conformance: a 58-tile walk answers from the hierarchy |
| P12 | no per-edge passage: a tile is walkable or it is not | the AoC decomp gives a tile a **four-bit N/S/W/E neighbour mask** - a step is legal per **edge** | **unread for HD**: no HD address pins the mask. Reading it is the step before adopting or cutting | `zone_spatial_pyramid_check_tile.cpp` (`func_001D29F0`) | - |

## Rulings and blockers, as measured tonight

- **P3's fork is ruled**: *"the game wins (a): adopt `<=`, a half-tile body stops at a one-tile gap, and D117 is withdrawn."*
  D117 no longer stands; `building_passage::a_big_unit_walks_through_a_one_tile_gap` must be re-pointed to the game's answer
  **when the rule lands**, not before (the engine cannot assert a rule it does not follow).
- **P1 is blocked on P11**: at `h = 25 x tiles` a flat search truncates a legitimate detour - measured, a one-tile walk answered
  with a node fifteen tiles east. The budget lands with the levelled search, or not at all.
- **P3 is blocked on the plan/step agreement about touching**: a route certified by `leg_is_clear` still asks for a landing
  that touches two blocked tiles, and the body stands in `WaitGround` for 3,000 ticks.
- **P4 is blocked on P8**: the goal fallback that made off-grid gather/attack points survivable is part of the book's furniture.
- **P5 is NOT free after all - it is the third step blocked on P8.** Cut alone (witness written first and watched failing:
  `crates/sim/tests/order_never_given_up.rs`, "the task is Idle after 120 ticks"), the order-finish turns out to be what breaks
  every crowd jam in the engine today: four fixtures went red - `a_mate_who_is_stopped_becomes_solid`,
  `the_loser_stands_for_the_drawn_pause`, `a_man_who_cannot_get_his_square_takes_the_next_tree`, and
  `collision::a_walker_never_ends_a_tick_inside_a_friend` (which presses one step deeper once nobody gives up). None of them is
  wrong: they describe a crowd whose only escape from a jam is that some men abandon their orders. With the order-finish gone,
  the collision pause has to do that work - which is exactly what the seat book's removal (P8) is supposed to install. So **P8 is
  the keystone**: P4, P5 and (after the plan/step agreement) P3 queue behind it, and P8 lands first.

## The order the next session should follow

1. **P8** - cut the seat book (ring places, `reserve_places`, `respread`, `settle_workers`, `standoff_point`), let collisions and
   pauses spread the crowd, re-point `overlap`, `gather_ring`, `gather_spread`, `gather_standoff` and the four jam fixtures above.
   This is the step everything else waits on, and it is the owner's own instruction ("cut the book entirely").
2. **P5** - the order-finish goes, its witness restored from this note's description.
3. **P4** - the re-plan throttle goes with the goal fallback it sat beside (no longer needed once there are no off-grid goal
   points).
4. **P3** - touching collides, in one slice with the plan/step agreement (`leg_is_clear` as strict as the step), then re-point
   `building_passage::a_big_unit_walks_through_a_one_tile_gap` to the game's answer (D117 withdrawn).
5. **P11** - the levelled search; then **P1** (the budget) lands with it.

## The fork that the owner has already ruled (P3, kept for the record)

The binary's collision test is four `<=` (`FUN_00840830`), and its search grows every object by the mover's radius
(`FUN_007f9d80`). Under those, a body of radius **half a tile** - a packed trebuchet, the widest in the pack - **cannot thread a
one-tile gap**: its box exactly fills the lane, so it touches both walls and touching is a collision. `building_passage::
a_big_unit_walks_through_a_one_tile_gap` asserts the opposite, and that fixture is not ours: it is **D117, the owner's rule**,
2026-11-04 - *"a one-tile gap is a gap, whatever the circle"*.

So the two cannot both stand:

- **(a) the game wins**: touching collides, D117 is withdrawn, and a half-tile body stops at a one-tile gap;
- **(b) the rule wins**: a body is refused only where it would *penetrate* a static object (the step's current behaviour), and
  that is the one place in the pathing where his ruling overrides the binary - it must then be written as such, in D117.

P4, P5 and P6/P7 do not depend on the answer.

## The sequence

1. **P1 + P2** - the budget and the tie-break, one slice: both are inside `Ground::search`, and P1's bound is what makes P4
   affordable.
2. **P3** - touching collides, with the cut landing a raw unit short. Small, and it changes the step everywhere, so it lands
   before the throttle comes off.
3. **P4 + P5** - cut the throttle and the forty-tick finish, together: both are "ours" rules that exist to paper over a slow
   search, and with P1 in place the honest measurement is possible.
4. **P6 + P7** - the object query and the shared tile, one slice: both replace the tile-as-obstacle assumption.
5. **P11** - the hierarchy, when it is priced (needs the HD read).
6. **P12** - read `zone_spatial_pyramid_check_tile` in HD, then adopt or cut.
7. **P8, P9** - his call, then whatever he decides.

## What "proved" means here, every step

- A **conformance row** for a rule the game states, or a **fixture** for behaviour, **watched failing before the change**.
- The acceptance fixture's time (`the_acceptance_match_trains_more_villagers`), and the suite's red set **diffed by name**
  against the previous commit's, not by count.
- The step written into `docs/decisions.md` and this file, committed and pushed per step.
