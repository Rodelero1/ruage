# ADR 0014 — The flat tile search is retired; geometry answers instead (superseded by New Way)

- **D-id alias:** D15 third clause (2026-10-23) — the half parked in ADR 0012's Consequences. Scoping half lives in 0012; this was the search half.
- **Status:** Superseded — owner decision 2026-11-26 (New Way design, full replacement, Option B). This draft was never signed; it is kept as the record of what was decided against.

## Context

HD 5.7 plans land routes with a flat tile A*: an octile estimate scaled by 16, an expansion budget derived from the straight-line distance and capped at 20,000, and the best node reached as the answer when the budget runs out. The binary picks per object between this flat search and a four-level search (the split 0015 used to pin). The owner has now chosen the deck's "New Way" geometry end-to-end instead: one geometric solver for every range, no tile grid consulted at route time. So the flat search described here no longer runs. Nothing below is a claim about HD — the binary remains the source of what HD does. This ADR records what we stopped doing and why.

## Decision

There is no tile search. Every route comes from `geometry::solve`: shrink the mover to a point, grow every obstruction box by the mover's own radius, shoot a ray at the goal, walk the hit shape's edge, re-ray per vertex, smooth what survives. The budget rule (20,000 expansions, best-node answer) retired with the search — an expansion counter has nothing to count in a geometric router. Its job, never failing the caller, is now the Reached-vs-Closest contract: the solver always answers, either "I am there" (Reached) or "I got as close as I could, here is where I stopped" (ClosestPoint). Callers must handle both and never mistake Closest for arrival.

D304's budget rule lived here, and it is retired with the search for the same reason: no counter, no cap, no best-node — the no-failure answer is structural now (fuel cap, expand-once, turn count), not budgeted.

## Evidence

- **Game (rung 0):** `FUN_008007f0` (flat search: 480×480 cost array, estimate `FUN_007ffd10`, neighbour cost `FUN_007ffc80`, budget `FUN_007ffcd0`, 20,000 cap, best-node answer); trapped-start reseed `FUN_007fb220`; standers inserted per search `FUN_007f9d80`; velocity caps `008a2a30`. This is what HD does; we chose not to do it.
- **Owner:** New Way design, Option B full replacement, 2026-11-26. Functional gates, not the deck's stopwatch: a route never gets stuck and never passes through a blocked area; the deck's 2x/40x numbers are motivational only.
- **Slides (verified direct from the PDF 2026-11-26):** the deck's 100+ case evolving suite is slides-only (no artifact ships with it) — our suite re-derives its described edges (aligned overlap, hole-vs-open, whole-map guard, surrounded-once-then-closest) from our own conformance rows, never copied.

## Consequences

The engine this ADR described is deleted: no tile search, no expansion budget, no best-node tail. The conformance rows that pinned it (`crates/conformance/tests/budget.rs`, `walkability.rs`) are rewritten as geometry rows — the rule the budget protected (a reachable goal is answered at the goal) stays, the counting is gone. The geometry witnesses live in `crates/sim/tests/geometry_*.rs`.

Open HD divergences (accepted, not corrections — the binary is still right about HD):

- **Replays:** paths differ from HD's tile routes, so a replay of our game on HD (or HD's on ours) will not match footstep for footstep.
- **Formations:** crowd pushing and formation movement beyond bump/stop/repath are out of scope for this design (later batch); the deck numbers say nothing about them.
- **100/600 terrain rows:** our tiles cannot express the terrain flags those coarse cost-table rows need, and there is no cost table any more for them to live in. If terrain flags ever land, geometry ignores them until a later decision says otherwise.
- **D305 body-width gate:** closed by geometry. The old gate refused any width and the hierarchy never answered unit routes; there is no gate now — every box is grown by the mover's reach (per-rect `max`, so already-grown lists are never doubled), and touching a face is contact, not collision. What the gate was protecting (units squeezing where they should not) is pinned by the diagonal-thread refusal and the face-contact rules instead.
