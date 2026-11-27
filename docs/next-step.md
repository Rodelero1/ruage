# NEXT — sign ADR 0016, delete D303 + D306–D310 (budget + tangency batch)

**Where this stands:** ADR 0016 drafted as retirement record (`docs/adr/0016-radius-tangency-retired.md`, draft, unsigned). Movement tranche closed (0014/0015 signed, `b0003b1`); live files compacted (`2cea22f`, 493 stale lines removed); index at 334 rows. Crosscheck complete: `docs/reviews/2026-11-27-tranche-crosscheck.md` — D303 CONFIRMED/moot, D306 CONFIRMED/mechanism replaced, D307 conclusion CONFIRMED bit-gloss REFUTED (`0x20` = passable, `pathfinding-levels-evidence.md:277-293`), D308 CONFIRMED (+D322 port, residual in `conformance-sequence.md:97`), D309 UNVERIFIABLE (no corpus), D310 CONFIRMED (+D329 measurement). Deck verified direct from PDF 2026-11-27: ray → hull → re-ray → smooth, expand-once-else-Closest, 64-iteration bound, "compatible with the radius expansion approach".

**Prior implementation (what the draft retires — nothing live to change):**
- Solver: `crates/sim/src/geometry.rs` (`solve`: ray + edge-walk + re-ray + smooth, fixed-point only, Reached-vs-Closest contract).
- Wiring: `crates/sim/src/ground.rs:423 route_with` — blocked tiles as grown `Clear` rects (per-rect `max` reach), plus map-edge strips; all movers via `plan_path` / `plan_path_for` / replan arms.
- Tombstone: `ground.rs:89-93 pyramid_builds()` zero-always (deletion pinned by test).
- Tests pinning it: `crates/sim/tests/geometry_*.rs` (12 deck-edge in `geometry_solve`, verdict/ray/walk/land/movers), `crates/conformance/tests/{budget,walkability}.rs` as geometry rows.
- Measures today: full suite green except 2 pre-existing `modifiers` Town-Watch failures (reproduce on clean HEAD); acceptance 16.54→71.94 s debug / burst 13.7→161.2 ms release (reported, non-blocking under functional gates).

**To do, in order:**
1. Owner verdict on 0016 (sign / amend / rework). No code change in any outcome — worst case is a draft rewrite.
2. On sign: delete bank rows D303 (l.8237), D306 (l.8256), D307 (l.8277), D308 (l.8291), D309 (l.8309), D310 (l.8330); MANIFEST per-D-id rows → 0016 (D309 marked open-but-signed as retirement record); `make decision-index`; regen live files (`python3 tools/live_migrate.py` — expect D8/D9 REFUSED lines, pre-existing); review note; commit; push.
3. Next in trunk order after this: crowd/standoff; then D311+ (reseed area, untouched here). D322/D323/D329 stand (later measurements, referenced not retired).

No SPEC file in this tree (SPEC cited by section only); no changelog file — changelog is the review notes + bank rows. No TDD slice: docs-only, no production code (named, not slipped).
