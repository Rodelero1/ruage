# No-overlap survey: how OSS projects and published sources keep units out of buildings, trees and mines

*2026-11-28. Question: given the DE New System adoption (owner scope 2026-11-28: HD-exe
findings are HD-only, DE behaviour stays open until read from DE) and the HD finding
(search = point-in-tile, movement = grown-box —
`docs/reviews/2026-11-28-point-expansion-crosscheck.md` §5), what are the live options to
guarantee no unit↔building/tree/mine overlap? Per-claim citations below; disagreements
parked in §4, not smoothed. No recommendation — evidence only.*

*Heads read 2026-11-28 via raw-file fetch, SHAs unpinned: re-pin before quoting any of
this as a fixture. Licence: openaok is MIT; genie is AGPL-3.0 (read-only, nothing
crosses); openage is GPL-3.0 (docs read for facts, no code).*

## 1. OSS code (primary)

### 1a. `Zaydme/openaok` — search bends round standing bodies, movement bends its own step

Survey: every tick builds a per-tile `Traffic` index of `Body{entity, x, y, radius,
moving}` for every animated non-building, non-dying, non-corpse, non-flier unit in
bounds (`src/ecs/Navigation.cpp`, `Survey`). Radius source is the dat: `RadiusOf` reads
the unit rules' `size_x` for its owner/player (`Navigation.cpp`, `RadiusOf`; speed
likewise from the rules' `speed`, dat tiles/sec, in `MovementSystem.cpp`,
`StepLength`).

Search-time: `Route` collects a "standing" list from tiles within `kStandingReach = 3`
of the start–goal box, **skipping self, the order's target, every body whose `moving`
flag is set, and every member of the marching group** (`Navigation.cpp`, `Route`; same
skip rule recorded 2026-11-16 at `Navigation.cpp:230`). Each survivor becomes a box from
**its own radius** (`standing.push_back({b.x - b.radius, b.y - b.radius, b.x + b.radius,
b.y + b.radius})`) — box-from-circle-centre, not grown by the mover. The mover's own
radius travels separately as `FindRoute`'s `radius` argument, whose contract is
"keeping off the edges of closed ground" (`src/pathfinding/Pathfinder.h`, `AStar::FindRoute`).
So: tiles are searched with mover-radius clearance, standing units are searched **around**
("round the units `standing` where that costs little and through them where it does
not", `Pathfinder.h`) — i.e. search-time treatment is circle-clearance vs ground plus
soft box obstacles for bodies, and bodies are explicitly pass-through-able.

Movement-time: `KeptApart` (`MovementSystem.cpp`) bends **the walker's own candidate
point only**, by `need = (b.radius + radius) * kKeepApart` with `kKeepApart = 3/4` —
a tolerance, not a clearance: "a crowd overlaps a little, never stands one over
another". The push is capped at the step length, never backwards (turned aside
instead), and discarded unless the kept point is itself passable and line-clear
(`MiddleClear`). **No cross-entity position write exists**: `Update` writes only the
walker's own `transform.position`; a blocked walker stops against the occupant
(`OnTheGoal`), moves on to a free ring spot (`PlaceRound`, rings of `kRingSpacing =
0.7` out to `kRings = 4`, at most `kMostMovesOn = 4`), or replans at most
`kMostReplans = 4` times per order. Arrival at something is box-vs-grown-box:
`GapAt` grows the target's box by the mover radius (`reach{min_x - r, …}`,
`Navigation.cpp`); `Taken` refuses a spot within `b.radius + radius` of anyone
standing still (moving bodies exempt).

### 1b. `librematch/genie` (DE port) — standing things are mesh geometry, moving things are not

Search-time: the resource search is a tile BFS from the unit tile (or the drop site's
footprint, gated on zone/dock, then the unit tile) testing `standable` — a per-unit
terrain-mask tile test — plus explored-ness and a per-candidate `reaches` reachability
test, 30-test cap (`crates/engine/genie-core/src/gathering/gather_search.rs`,
`closest_resource_at`, `more_work`; zone/footprint gate recorded 2026-11-14). No
radius growth at search time: standable is a tile property, reach is a slot-0x3d0
predicate.

Movement-time: the obstruction mesh holds one rect per standing object —
`shape_rect`: position ± radii **grown by half a tile**; `refreshed_rect`: same at each
action-object update's end (`crates/engine/genie-core/src/pathing/mod.rs`, `Pathing::moved`,
`Pathing::refresh`, `shape_rect`, `refreshed_rect`). The standing/moving split is the
whole mechanism: a tall object with a recorded obstruction holds a shape **while it is
not moving** and it is **removed the instant it moves** (`moving == MoveState::Moving →
mesh.remove`), so a standing unit is real geometry and a walking one is not in the mesh
at all (recorded 2026-11-16, `pathing/mod.rs:343-360`). On a kept hit the walker either
pauses the met unit — ally 0.1 s, anyone else 0.5 s, written into that unit's walked
level (`movement/walk.rs`, `Mover::go_round`; `objects/release.rs:209-223`) — or drops
its own walked level so the parent replans around the temporary shapes; **the pause
timer is the only field ever written into a met body**. Radius source is the dat via
the master: `SearchSite` carries "its master's radii (`+0x5d8`, `+0x5dc`)", spawn
places on the square at `site.radius + unit radius + tuning gap`, stepping by the
unit's x radius (min 0.1), first free point wins
(`production/spawn/closest_spot.rs`, `closest_spot`, `edge_points`).

### 1c. `SFTtech/openage` — no unit↔unit answer; documents the game's 3+2

`PathRequest` carries grid, start tile, target tile and time — nothing about occupancy
(`libopenage/pathfinding/path.h:17-26`); cost fields are terrain-only, and
`Position::set_position` appends a keyframe to that entity's own curve
(`libopenage/gamestate/component/internal/position.cpp:53-55`). Units are never
obstacles to each other: not applicable, not solved (recorded 2026-11-16). Its
reverse-engineering notes state the published scope: AoE2 uses **three pathfinders**
(mip-mapped long-range on passability ignoring moving objects but taking static
objects like buildings; tile-based short-range; polygonal convex-hull for combat and
crossing paths) **plus two obstruction systems**
(`doc/reverse_engineering/game_mechanics/pathfinding.md`, verified verbatim
2026-11-28; sources named: Gamasutra postmortem p.2, Pottinger GDC-2000 `.doc`,
Shelly 1999).

## 2. Published and online sources

- **Klinger deck (Meeting C++ 2025, first-party).** One line: original algorithm "Shrink
  A to a point and expand all other obstructions" (p. 23), replacement "Shrink ourselves
  to a point and expand the obstructions" (p. 83), with p. 65 "compatible with the
  radius expansion approach" as the only width word
  (`docs/reviews/2026-11-28-point-expansion-crosscheck.md` §2).
- **Pottinger/Shelly 1999 (first-party published).** Dave Pottinger, "Coordinated Unit
  Movement" (gamedeveloper.com, 1999-01-22): pathfinding vs movement split ("movement,
  the execution of a given path, is the other half"), collision-resolution rules for
  two-unit collisions, and the policy quote "the restriction for zero collision overlap
  often makes units walk well out of their way… decide how much collision overlap is
  acceptable". Companion "Implementing Coordinated Movement" (1999-01-29): hierarchical
  pathfinding via path stacks. Trajectory-overlap prediction with formation-level
  priority (highest priority passes, others stop/slow) is Pottinger's via the secondary
  cite (Christensen/Eiserloh thesis §2, citing Pottinger [6]) — read, not verified
  against the original.
- **Configuration-space theory (high-trust, no game weight).** kindatechnical.com,
  "Configuration Space and Minkowski Sums" (2026): "for a disc centred on the reference
  point… `O ⊕ R` is the set of points within distance r of `O`… Growing every footprint
  by the robot's radius and planning for the centre point is the whole method for a
  round robot"; "the radius and not the diameter"; gap rule — two obstacles at distance
  w leave a passage for a disc of clearance c iff w > 2c, w = 2c treated as closed.
- **Geldreich 2018 (ex-Ensemble engineer blog, adjacent).** "We ported Age2's entire
  obstruction manager into DE… Units use circular obstructions, and buildings use
  square obstructions" (richg42.blogspot.com, 2018-02 "On Age DE's pathing/movement"):
  circle-vs-box as the shipped split.
- **Dre Dyson Godot tutorial (tertiary).** One line, retelling only: shrink unit to a
  point, expand obstacles by unit radius ("margin"/"Minkowski sum"), raycast + edge-walk
  with A* fallback (dredyson.com, 2025 Godot RTS tutorial, labelled tertiary).

## 3. Synthesis: live options for our engine

Given: HD search tests terrain point-in-tile with no radius (`FUN_008aa480`) and bodies
by single-tile occupancy; HD movement queries grown-box at the new position
(`FUN_00840830` grows by mover radius, `param_7` = 1.0 at both collision sites);
DE New System behaviour open. Our tree already mirrors the HD split: movement-time
box-vs-grown-box at the landing (`crates/sim/src/world/motion.rs`, grown-query
comments ~:642-673; box not disc, `reach` = halves summed, overlap strict-not-touching
~:706-711; clearance-not-tile ~:737) and search-time grown rects
(`crates/sim/src/geometry.rs:33-38`, `grown` = half + reach, touching not inside).

| # | Option | Shape | Cost |
|---|---|---|---|
| O1 | Search-time footprint/occupancy test: refuse or route round tiles/boxes held by buildings, trees, mines at plan time (genie `standable` + zone gate; openaok standing boxes; HD pointedly does *not* do this) | plan-time | route invalidation when objects appear/move; HD-divergent for units (HD search ignores moving bodies) |
| O2 | Movement-time grown-box refuse-and-replan: keep the HD query (new-position box vs object box grown by mover radius), refuse the step, repath with the hit avoided (genie: hit → pause met unit or drop walked level → replan with temporary shapes; openaok: replan ≤4) | step-time | replan storms near crowds (our D52 history: 24 → 529 acceptance repaths when clearance entered the step); needs avoided-set state |
| O3 | Separation steering on the walker's own step: bend (openaok `KeptApart`, capped at step, never backwards, no cross-writes) or pause/slow by priority (Pottinger; genie 0.1/0.5 s pauses) | step-time | overlap becomes bounded tolerance (3/4 radii), not zero; priority needs an ordering both sides agree on |
| O4 | Take-gate / placement search at the ends: spawn/arrival searches for a free point (genie `closest_spot` square at building + unit radius + gap; openaok `PlaceRound` rings + `Taken`; `GapAt` standoff) | order-time | claims/release bookkeeping (D53 declined this); deadlock rule when every spot is taken |
| O5 | Foundation/building placement eviction: a site appearing over a unit moves the unit out first (genie `move_aside` for foundations; ours: D102 evicts before ground turns solid) | placement-time | choosing the escape tile; ours already pays this |

## 4. Disagreements parked

- D1: openaok grows standing-body boxes by the *body's own* radius and passes the mover
  radius as clearance; HD grows the query box by the *mover's* radius
  (`FUN_00840830`). Both can't be the game; HD leg is binary, openaok is peer design.
- D2: openaok's `kKeepApart = 3/4` *permits* slight overlap; a strict no-overlap
  guarantee needs 1.0, which re-opens the corridor deadlock D131 settled as correct
  behaviour. Tolerance vs guarantee is owner's call, not evidence's.
- D3: genie grows every mesh shape by half a tile on top of radii (`shape_rect`,
  `refreshed_rect`); our touch rule is strict-not-touching on the grown box. Same
  question as D2 one level down.
- D4: Pottinger's "acceptable overlap" and formation-commander routing vs Klinger's
  2025 "no pushing allowed" (deck p. 9): different eras, possibly different games'
  answers (AoE1-era articles vs DE-era talk). Do not mix without a build tag.

## 5. Performance

No new measurement taken (survey only; no code touched). Numbers on record, for sizing
the options: openaok full-game budget ~100 ns/entity/tick at ~3,000 entities (prior-art
2026-10-31); our geometry solve 40.2 µs in a base-sized window (D113); genie caps
plans per player per turn (`PlanBudget::allows`, `pathing/mod.rs`). O2's failure mode
is measured in our history (D177: repaths 24 → 529); O1's is per-plan µs.

## 6. Stability

Attacked the failure each source's mechanism leaves: openaok — a walker replans ≤4
then gives up (`Replan` false → `arrived`), and `Taken` exempts moving bodies, so a
goal can be held by traffic rather than geometry; genie — a moving unit leaves the
mesh, so two walkers can agree on one free spawn/stand point in the same tick
(`closest_spot` "first free" is per-search, not claimed); HD reading — rests on the
crosscheck's static disassembly, DE side open. Heads unpinned (§0): any of the OSS
file cites can drift; re-fetch before building on them.
