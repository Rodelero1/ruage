# ADR 0015 — The three-tier route is retired; geometry answers instead (superseded by New Way)

- **D-id alias:** D296 + D297 + D298 + D299 + D301 + D302 (2026-11-19/20). D300 superseded by D301 (premise and number both withdrawn there). D298's 100/600-row correction (D327) folded in. D304's budget lives in 0014. D305's gate hole is closed by geometry — see 0014.
- **Status:** Signed 2026-11-27. Signed as the retirement record: the hierarchy is gone from the engine (`aafb740`, New Way slice), and this ADR is the trace of what it was and why it went. It is not a live behavior claim.

## Context

HD 5.7 answers land routes in three tiers: the straight line first, then a four-level search over level maps (cells of 8, 4, 2 and 1 tiles), then the flat tile search from 0014. The binary carries the level maps beside the tile grid and searches them first for land units; the coarse search ran one level over four directions with a Manhattan estimate and refined to one waypoint per cell centre. The owner has now chosen the deck's "New Way" geometry end-to-end instead: one geometric solver for every range, no levels, no tiers. So the hierarchy described here no longer runs. Nothing below is a claim about HD — the binary remains the source of what HD does. This ADR records what we stopped doing and why.

## Decision

There are no tiers. Every mover — land, ships, flying things, projectiles — routes through `geometry::solve`. Land feeds its blocked tiles as grown boxes, ships feed the water-layer tiles, flying things and projectiles pass an empty list (direct). The level maps, the pyramid cache, the coarse search, the refinement, the tier certification and the fallback chain are all deleted; `Ground::pyramid_builds` stays as a zero-always tombstone so the deletion is pinned by a test instead of a memory. The search-area bound the tiers used to provide (stay near the walk) is now the solver's own search-area rect: the from/to box, then once more after one expansion — a walk that cannot progress inside it answers Closest beside itself instead of wandering.

## Evidence

- **Game (rung 0):** switch `FUN_008000d0` (levelled search as the land-unit default); descent `FUN_007ee980`; coarse search `FUN_007ef170` with pooled nodes; refinement `FUN_007edb80`/`FUN_007eeda0`; stampers `FUN_007f1100`/`FUN_007ede40`; cost table `FUN_007ed280`. This is what HD does; we chose not to do it.
- **Owner:** New Way design, Option B full replacement, 2026-11-26. Scope is all movers and terrains; formations and crowd pushing stay a later batch.
- **Slides (verified direct from the PDF 2026-11-26):** slide ~38 pairs short-range geometry with hierarchical long-range — we follow the geometry half and drop the hierarchy half by owner decision, not because the deck says to.

## Consequences

The engine this ADR described is deleted: `levels.rs` is gone with the pyramid cache, the tier funnel and shadow, the coarse search and the refinement, and the tiered tests are rewritten as geometry witnesses or deleted with reasons. What the tiers were for — long walks without searching every tile — is now the search-area rect plus the turn count (4 more lefts than rights is the inside of a hole) plus the fuel cap: bounds, not budgets.

Open HD divergences (accepted, not corrections — the binary is still right about HD):

- **Replays:** same as 0014 — our routes are not HD's routes.
- **Formations:** same as 0014 — beyond bump/stop/repath is a later batch.
- **100/600 terrain rows:** same as 0014 — no cost table left for them to live in.
- **D305 body-width gate:** closed by geometry — see 0014. No width is ever refused at a gate; width is grown into the boxes.
- **The one-line fix and tangency entanglement** the old draft left for the owner: moot. There is no hierarchy for the fix to land in and no tier for tangency to entangle with; the passage rule (touching is clear, the diagonal thread is refused) is pinned by the geometry witnesses instead.

## Fixtures

- `crates/sim/tests/geometry_movers.rs` (every mover routes through the solver; the pyramid builds nothing, pinned at zero).
- `crates/sim/tests/geometry_*.rs` (verdict contract, ray hit, edge walk, re-ray loop with bounds, land wiring, walkability).
- `crates/conformance/tests/{budget,walkability}.rs` (rewritten as geometry-bound rows with reasons; the tile-counting rows retired with the tile system).
