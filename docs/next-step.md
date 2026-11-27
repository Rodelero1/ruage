# OPEN — ADR 0020: D285/D286 take-gate (implementing)

**Where this stands:** ADRs 0001–0019 signed; index at 302 rows; `origin/adr-migration` at `69d6a5c`. Plan saved at `docs/superpowers/plans/2026-11-27-take-gate.md` (writing-plans skill). This compact is the agent's handoff.

**Prior implementation (code frozen, docs-only so far):**
- Take-gate live: `crates/sim/src/world/step.rs:1638` (`at_node = box gap <= one step`, D285/D286 — stands, later tranche); touching test + ring fallback beside it (`:1605-1624`, D192 remnants, `let _ =` inert).
- Cap from geometry: `lib.rs:1350 touching_place_count`; `gather.rs:338-371` spreads past cap via `nearest_like_free` (`gather.rs:41`, D182/D204/D291 line).
- Forty-tick re-target live: `count_refused_squares` (`step.rs:88,335`), `STALLED_TICKS = 40` (`step.rs:922,1727`); `TakeAtRest` deliberately absent (`motion.rs:17`) — ADR 0019's standing half.
- Movement grown-query comments (`motion.rs` ~:642-673); search-grown rects (`geometry.rs:33-38`).
- Tests pinning it: `gather_press_in`, `tile_geometry`, `gather_room`, `acceptance_economy` (guard).
- Measures today: not re-run (docs-only); last full-suite at 0016 sign-off: green except 2 pre-existing `modifiers` Town-Watch failures. Verification runs at sign-off.

**Adopted and binding:** adopt-from-game (0019) + conditional grow+test+reset (0018) — box-vs-grown-box, flag+margin, terrain point-tested; past stands for every body; forty-tick same-kind re-target, displacement gone; working margins 0.95 gather / 1.0 walk; `+0x78` identity, 0.95 rationale, `+0x84` meaning all open; DE cost ×75 / bitmask / no-substitution / no-reseed / no-parent-path recorded.

**Candidates for the next tranche (owner picks one):**
1. ~~D285/D286 — the box-gap take-gate~~ → **taken: ADR 0020, implementing now (this step).**
2. D183–D185 — foundations; D175–D179, D187, D193–D197 — area leftovers.
3. D242–D245 — R1/invariants follow-ups to 0019 (R1a/R1b split, four-body invariants).

**To do, in order:**
1. Owner names the tranche.
2. Draft the ADR (retirement record), plain language, test-first if code is touched.
3. Owner verdict: sign / amend / rework (one word signs).
4. On sign: delete bank rows; MANIFEST per-D-id rows → new ADR; `make decision-index`; regen live files; review note with Performance + Stability; commit; push.

No SPEC file in this tree (cited by section only); no changelog file (review notes + bank rows are the changelog). No TDD slice until production code is touched (named, not slipped).
