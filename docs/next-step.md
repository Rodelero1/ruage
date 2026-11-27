# OPEN — next tranche after ADR 0018 (owner picks)

**Where this stands:** ADRs 0001–0018 signed; index at 304 rows; `origin/adr-migration` in sync (push pending for 0018 at time of writing — see below). This compact is the agent's handoff; the next tranche is the owner's call.

**Prior implementation (code frozen, docs-only so far):**
- Take-gate live: `crates/sim/src/world/step.rs:1638` (`at_node = box gap <= one step`, D285/D286 — stands, later tranche); touching test + ring fallback beside it (`:1605-1624`, D192 remnants, `let _ =` inert).
- Cap from geometry: `lib.rs:1350 touching_place_count`; `gather.rs:338-371` spreads past cap via `nearest_like_free` (`gather.rs:41`, D182/D204/D291 line).
- Movement grown-query comments (`motion.rs` ~:642-673); search-grown rects (`geometry.rs:33-38`).
- Tests pinning it: `gather_press_in`, `tile_geometry`, `gather_room`, `acceptance_economy` (guard).
- Measures today: not re-run (docs-only); last full-suite at 0016 sign-off: green except 2 pre-existing `modifiers` Town-Watch failures. Verification runs at sign-off.

**Adopted and binding:** conditional grow+test+reset (ADR 0018) — box-vs-grown-box, flag+margin, terrain point-tested; working margins 0.95 gather / 1.0 walk; `+0x78` identity, 0.95 rationale, `+0x84` meaning all open.

**Candidates for the next tranche (owner picks one):**
1. D239/D241 — the idle-body-nudge withdrawal that partly superseded D204; natural follow-up, closes the D204 partial.
2. D285/D286 — the box-gap take-gate (live at `step.rs:1638`); the other half of the gather end-state.
3. D183–D185 — foundations; D175–D179, D187, D193–D197 — area leftovers.

**To do, in order:**
1. Owner names the tranche.
2. Draft the ADR (retirement record), plain language, test-first if code is touched.
3. Owner verdict: sign / amend / rework (one word signs).
4. On sign: delete bank rows; MANIFEST per-D-id rows → new ADR; `make decision-index`; regen live files; review note with Performance + Stability; commit; push.

No SPEC file in this tree (cited by section only); no changelog file (review notes + bank rows are the changelog). No TDD slice until production code is touched (named, not slipped).
