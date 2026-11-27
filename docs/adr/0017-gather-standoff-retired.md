# ADR 0017: the gather standoff — thirteen rulings, one mechanism gone

**Status:** Signed 2026-11-28. Deleted 13 bank rows on sign-off: D169, D170, D171, D172, D173, D180, D181, D182, D186, D188, D191, D192, D199.

## What this batch is

Between 2026-11-08 and 2026-11-13 the engine learned where a working villager stands. It went through four answers in six days: 0.7 tiles out (D169), a 1.156-tile ring with nine seats (D186), 180 sampled places with no cap (D188), and the tile's border with the capacity falling out of the geometry (D191/D192), verified against the owner's saves (D199). Then the owner withdrew the whole seat mechanism: no man owns a place any more, crowds press in, and the take is gated on the boxes touching (D293, recorded here, retired there).

This ADR retires the placement line as a mechanism and records what survives as rules.

## The story, in the order it happened

1. **D169 — 0.7 tiles.** Two screenshots from the owner: villagers stood half a tile from their work, the game holds them closer. The dat gives it: villager radius 0.2 + tree/bush/mine radius 0.5 = 0.7, the two bodies touching. The walk still ended at a tile centre, so a `settle` placed the worker on the standoff on arrival — a placement, not a walk, and the eye saw the jump (D173).
2. **D170 — a mine is a pile per tile.** The owner's answer: each tile holds its own pile with its own amount. Four independent tiles, not one 2x2 object. So the 0.7 rule holds per tile.
3. **D171 — fan out round a node.** Four axis sides at the touching distance, best-facing first, free and reachable ground only. Ceiling: four sides, a fifth man keeps the centre. Proven failing first (two men at 0.00 apart).
4. **D172 — nodes stand at tile centres.** The seed put nodes on tile corners; half a body spilled into each of four tiles and the four working sides were not symmetric. Fix: `tile_centre`, the same convention D114 and D168 already used. Entities 137 → 153, repaths 144 → 24 — the crowding was never a gather rule.
5. **D173 — OPEN: the last third of a tile is a placement.** The jump reproduced (0.6 tiles in one tick at 0.04 speed). Two walks tried and rejected (stranded worker; 225 repaths against 24). Third shape promising but needs wider "beside the node". Berry-patch piling, distance foraging, and flicker not reproduced — left open with the owner.
6. **D180 — the settle walks.** The jump's cause was `entity.speed` read per-second where a step is per-tick (16,000 vs 800). Fix: one `step_towards`, speed-capped, position only. The ignored test runs since.
7. **D181 — standoff when too close, not only when stood on.** "Held" became *closer than two bodies wide* with same-pass sides counting. Distance half fixed (0.92 vs 0.70: two answers to one question); gating on touching turned six tests red, unexplained — shipped the overlap half, left the distance half open. **Already marked superseded by D293 in the bank.**
8. **D182 — one gatherer a square, at a body's radius off its edge.** The owner's rule verbatim. Approach side only; a taken square answers with the nearest same-kind source or idles. Player orders only (internal re-tasks cost 16 s → 180 s). Price: entities 160 → 137, repaths 254 → 618 — recorded, not argued.
9. **D186 — the 1.156 ring, nine men.** Measured from six men in the owner's save (means, spread 0.084). Nine slots at 40°, walkable/free/unclaimed, integer trig. Later: buried-ring fallback, nearest-claimant ownership, walk-to-the-ring-not-the-band, nine-is-the-cap addenda. **Already marked superseded by D199 in the bank** — the 1.156 reading measured the tile grid (cardinal centres at exactly 1.000, diagonals at 1.414), not the men.
10. **D188 — no cap; eighteen places from the geometry.** The owner's answer to nine-vs-eighteen-vs-eleven: the count is not a rule. 180 samples every two degrees, one body clear; capacity derived (`2·π·1.156/0.40 = 18`). Reverted the same hour at eighteen slots (settle vs collision fight, 0.243 apart); nine stands as a packing defect, not a rule.
11. **D191 — tile geometry module.** `tiles.rs`: corners, `distance_to_tile`, `touching_places` in fixed order (hashed simulation; a reordering geometry is a desync). Capacity falls out with no table: eleven places for a 0.2 body, six for 0.4, forty-one for 0.05. Added as geometry only; the standoff switch was its own slice.
12. **D192 — the standoff is the tile's border.** Circle against the tile (`distance_to_tile <= radius`), nearest-first, eleven places; buried nodes fall back to the open tile beside them. Landed with a deadlock it exposed (friend on the goal tile) fixed by scoping the blocker rule to hostiles. Repaths 240 → 409 (+70%), recorded not smoothed.
13. **D199 — the touch, measured on the owner's saves.** Tree/mine/bush saves: seven bodies each at 0.641/0.70x3/0.935x3-ish, means 0.793/0.793/0.811, closest pair 0.661–0.663. D186's ring withdrawn (artefact band). Capacity open: game shows ~0.66 spacing, geometry gives eleven places — permissive side, no test fails, numbers not the same claim.

## What survives (live rules, with witnesses)

- **Bodies touch at the edge.** 0.7 at an edge (`0.5 + 0.2`), 0.94 at a corner (`0.5·√2 + 0.2`) — D169's number inside D199's measurement. Witness: `tile_geometry`, `gather_press_in::two_gatherers_on_one_tree_press_in`.
- **A mine is piles per tile** (D170). Map model; `seed.rs` places nodes per tile at `tile_centre` (D172, `seed.rs:412`).
- **Capacity comes from the ground, not a table** (D188's rule, D191's module, D192's border): `touching_place_count` (`lib.rs:1350`) over `touching_places`; the cap in `start_gather_with` (`gather.rs:338-371`).
- **A refused square answers same-kind, never with a shove** (D182's second half, kept by D204, D291): `nearest_like_free` (`gather.rs:41`); refusal counted, re-tasked after `STALLED_TICKS` (`step.rs:922,1722-1740`). Full story belongs to ADR 0018.
- **No man owns a place** (D293, retired there, recorded here): `owned_place: None` (`step.rs:1668`); the take is the box gap within one step (`step.rs:1638`, D285/D286 — stand, later tranche).

## What is gone (mechanism deleted, rows deleted with it)

- `standoff_point`, `Standoff`, the `gather_place` book, `GATHER_RING_SLOTS`, the settle walk, the ring band as a gate (D171/D173/D180/D181/D182-cap/D186/D188-seats/D192-gate). Deletion executed in D294 (suite 895/11/21 byte-identical before and after); the settle arm in `step.rs` is inert comments and `let _ =` bindings.
- D181 and D186 were already marked superseded in the bank (by D293 and D199); this ADR deletes those rows with the rest.

## Evidence (D203 order)

1. **Original game first.** D199's saves (positions above; one body at 1.054 unexplained, recorded). D169/D173/D180/D181/D182 owner's screenshots and verdicts, quoted in the rows.
2. **Adjacent sources second.** Decomp (`samuelcaldas/age-of-empires-2-decomp`): silent on gather/work/speed — stubs only (`prior-art.md:38`). Nothing found, recorded. Openaok (`librematch/genie`): `gather_search.rs`/`closest_resource.rs` give the same-kind next-source search shape (D291's basis); no seating or standoff mechanism anywhere in it. Nothing found, recorded. Community: eleven-on-a-bush observed, no cap documented (D186 addendum 4).
3. **In-tree cross-checks.** `gather-seating-evidence.md` Q1: the taken list is ours — the game smooths nothing, men pile and stand pressed (basis for D293). `movement-in-the-binary.md`: collision block has no seating. Deck (Klinger's): pathfinding only, silent on seating — nothing found, recorded.
4. **Disagreements parked, not smoothed.** Game spacing 0.66 vs geometry eleven places (D199) — open, carried. D192's gate vs D285's box gap — D285's row stands for its tranche; this ADR retires D192's gate claim only.

## What this commits us to

- No code path may keep a per-man record of where round a node he stands (D294's rule, kept).
- Any future capacity rule argues against the game's 0.66 spacing first (D199), not against a constant.
- Functional gates bind: never stuck (D292: no order ends on a refusal), never through blocked (border geometry + box gap).

## Sign-off checklist

- [x] Owner verdict (sign / amend / rework): signed 2026-11-28.
- [x] On sign: bank rows D169, D170, D171, D172, D173, D180, D181, D182, D186 (+4 addenda), D188, D191, D192, D199 deleted from `docs/archive/decisions-2026-11-24.md`.
- [x] `MANIFEST.md`: per-D-id rows → 0017 (D181/D186 noted superseded-retired, D182/D192 noted partial).
- [ ] `make decision-index`; regen live files (`live_migrate.py` — expect clean, no new orphans).
- [ ] Review note with `## Performance` (no engine diff) + `## Stability` (what was attacked); commit; push.
