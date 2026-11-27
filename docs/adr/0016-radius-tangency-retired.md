# ADR 0016 — Radius, tangency, stamp, refine, line test and avoid-cap rules are retired; geometry answers instead (draft)

- **D-id alias:** D303 + D306 + D307 + D308 + D309 + D310 (2026-11-19/20). D322 (refine port) and D329 (cap measurement) folded in as follow-ups. D323's dormancy finding recorded, not re-litigated.
- **Status:** Draft — needs owner sign-off. Signed 0014/0015 are the pattern: this is a retirement record, not a live behavior claim.

## Context

Six bank rows described how the old tile engine handled body size and passages: where the mover's radius was applied (D303), that touching a wall counts as passing it (D306), that the radius growth landed on single tiles rather than whole 8-tile blocks (D307), that a coarse step was refined to the tile where the passage is (D308), that no straight-line shortcut was ever found in the binary (D309), and that our avoid list had no 255-entry cap on purpose (D310).

The engine they described is gone. The New Way slice (`aafb740`) deleted the flat search, the level maps, the pyramid, the coarse search and the refinement. Every mover now routes through `geometry::solve`: shrink the mover to a point, grow each blocked box by the mover's own reach, shoot a ray, walk the hit edge, re-ray per vertex, smooth. So none of the six rows can land as code. What survives is the game fact underneath each one, restated as a geometry rule — or, in one case, a correction.

## Decision

There is no radius-in-searches question, because there are no searches. The six rules retire as follows:

1. **D303 (radius in line test and step, not searches):** moot. True of the code it described, confirmed against the crosscheck table — and the code is deleted. The radius now lives in one place: every blocked box is grown by the mover's reach before the solver ever sees it (`ground.rs:423 route_with`, per-rect `max` so already-grown lists are never doubled).
2. **D306 (tangency is a passage):** survives as a geometry rule. Touching a grown face is contact, not collision; a packed trebuchet walks a one-tile hole because 0.5 radius in a 1.0 gap leaves zero clearance and zero is passable. The owner's HD measurement and the deck's "compatible with the radius expansion approach" agree with the solver's face-contact rule and diagonal-thread refusal.
3. **D307 (stamp resolves at the finest level):** conclusion survives, label corrected. The bank said bit `0x20` means "this cell contains an obstacle". Re-read in the evidence file: `0x20` means "this cell is passable" (`pathfinding-levels-evidence.md:277-293`; the 2026-11-20 entry read the branch backwards and withdrew itself). Growth is still resolved per tile — a one-tile gap is never washed into an 8x8 block — but the next reader must not quote the old gloss.
4. **D308 (refine to the tile, not the cell centre):** survives as solver behavior by construction. The solver emits waypoints on grown faces at the crossing, never cell centres. Ported to the old engine in D322 (`Levels::refine` emits boundary crossings); the one known residual (binary scans outward from the interpolated point, ours scanned from the low end — `conformance-sequence.md:97`) died with the old engine and is recorded, not carried.
5. **D309 (no pre-search line test found):** stays open, explicitly. Three probes found no straight-line shortcut before the search, and the crosscheck table agrees the negative is genuine — but a negative cannot be confirmed without the binary corpus, which is absent on this box. The prescribed settler is unchanged: trace the caller of `FUN_008000d0`, inspect `FUN_007ffae0`. Under geometry the question is reframed, not answered: the solver *is* the line test (ray first), so a "shortcut" would be a no-op. No parity claim is made either way.
6. **D310 (avoid list has no 255 cap, on purpose):** survives as a recorded difference. The binary's ignore list caps at 255 (`movement-in-the-binary.md:260`); ours is unbounded by decision, for the two stated reasons (our truncation order would be id order, not the binary's colliders-first; no witness could see it). D329 later measured reason 2: 0–400-body wall, routes bit-identical with and without truncation; largest live list in the acceptance match is 17 entries. What remains unfaithful below 255 is selection order only, and it matters exactly as much as the cap does.

## Evidence

- **Game (rung 0, recorded pseudocode — no binary corpus on this box):** `FUN_007f0da0` (growth), `FUN_007eeda0` (refine to tile), `FUN_007ee980` (descent from start cell), `FUN_008000d0` (no line test before registry), `FUN_008007f0` + `local_404[255]` (ignore-list cap), `FUN_00801a20` (fail-closed passability: moving mates free, stopped bodies blocked). Each cited from `docs/archive/research/` as recorded, not re-read.
- **Owner (rung 0 by rule):** packed-trebuchet one-tile hole and bamboo corridor walks (D306 review); "blunt copy" order (D330) sets the bar these rows are judged against.
- **Slides (cross-check, deck read directly from the PDF 2026-11-27):** ray → hull traverse → re-ray per vertex → brute-force smooth; expand search area once, else Closest; 64-iteration bound; "compatible with the radius expansion approach". Our suite re-derives the described edges, never copied.
- **Neighbouring OSS:** no openage/genie statement found in-repo on a straight-line shortcut (D309) or a 255 cap; recorded as absent, not as support.
- **Own suite (what pins the geometry half today):** `crates/sim/tests/geometry_*.rs` (verdict contract, ray hit, edge walk, re-ray with bounds, land wiring, walkability, movers); `crates/conformance/tests/{budget,walkability}.rs` as geometry rows. Full suite green except 2 pre-existing `modifiers` Town-Watch failures (reproduce on clean HEAD).

## Consequences

On sign-off, bank rows D303, D306, D307, D308, D309, D310 are deleted (6 rows). D322, D323, D329 stand — they are later measurements on the old engine, referenced here, retired by a later tranche if at all. D311+ untouched.

Open HD divergences (accepted, not corrections — the binary is still right about HD):

- **Replays / formations / 100-600 rows:** same as 0014/0015 — no cost table left for terrain flags to live in; geometry ignores them until a later decision says otherwise.
- **D309 line test:** the solver rays first, so our open ground answers in one waypoint where HD may search. Until the caller trace lands, this is a declared possible speed difference, not a parity claim.
- **D310 selection order:** below 255 bodies the only difference is *which* bodies are considered, in id order vs colliders-first. Unobservable in every scene measured (D329); a 256-body wall with the decisive body last would be the witness if it ever lands.
