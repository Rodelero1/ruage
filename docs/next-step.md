# OPEN — ADR 0018: crowd/collision batch (D200, D204–D205, D289–D295)

**Where this stands:** ADRs 0001–0017 signed; index at 315 rows; `origin/adr-migration` in sync. This tranche covers the crowd/collision line: D200, D204–D205, D289–D295. D175–D179, D187, D193–D197 stay for their area tranches. D183–D185 (foundation) stay with buildings.

**Prior implementation (code frozen, docs-only so far):**
- Take-gate live: `crates/sim/src/world/step.rs:1638` (`at_node = box gap <= one step`, D285/D286); touching test + ring fallback beside it (`:1605-1624`, D192 remnants, `let _ =` inert).
- Cap from geometry: `lib.rs:1350 touching_place_count`; `gather.rs:338-371` spreads past cap via `nearest_like_free` (`gather.rs:41`, D182/D204/D291 line).
- Movement grown-query comments (`motion.rs` ~:642-673, box not disc, `reach` halves, strict-not-touching ~:706-711); search-grown rects (`geometry.rs:33-38`).
- Tests pinning it: `gather_press_in`, `tile_geometry`, `gather_room`, `acceptance_economy` (guard).
- Measures today: not re-run (docs-only); last full-suite at 0016 sign-off: green except 2 pre-existing `modifiers` Town-Watch failures. Verification runs at sign-off.

**Decided 2026-11-28 (owner, binds 0018):** no-overlap guarantee follows DE — conditional grow+test+reset (flag+margin, box-vs-grown-box, terrain point-in-tile); working margin 0.95 gather / 1.0 walk, `+0x78` identity as revisit trigger.

**Evidence already in tree (D203 order):** HD binary — search point-in-tile, movement grown-box, multiplier 1.0 (crosscheck §5); DE exe static — terrain point-test confirmed, object-box encrypted (crosscheck §6); DE live dump — conditional Minkowski, flag `[ctx+0x57]` + margin `[ctx+0x58]`, 0.95-scaled gather / unscaled walk, `PathingSystem.cpp` found (crosscheck §7); OSS survey — openaok/genie/openage mechanisms + O1–O5 options (no-overlap survey §3); deck — shrink-to-point/expand one-liner only.

**To do, in order:**
1. Draft ADR 0018 (retirement record for D200, D204–D205, D289–D295), plain language, conditional-grow as the adopted rule.
2. Owner verdict: sign / amend / rework (one word signs).
3. On sign: delete bank rows; MANIFEST per-D-id rows → 0018; `make decision-index`; regen live files; review note with Performance + Stability; commit; push.

No SPEC file in this tree (cited by section only); no changelog file (review notes + bank rows are the changelog). No TDD slice until production code is touched (named, not slipped).
