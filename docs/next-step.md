# OPEN — next tranche after ADR 0021 (owner picks)

**Where this stands:** ADRs 0001–0021 signed; index at 285 rows; `adr-migration` ahead of `main` by the 0021 sign-off (commit lands this step). Working repo `/app/ruage` on `adr-migration`, with `config/gamedata/` and `Age2HD` present. Post-reboot rule: `cargo clean` before trusting any suite number after a move (stale `target/` baked `/app` paths → 151 phantom reds); suite numbers only count from a tree with gamedata present (19 missing-file reds without it).

**Prior implementation (ADR 0021, signed 2026-11-27, docs-only):**
- Retired as built: D175 (dat `search_radius`), D176/D177 (`clearance_at`/`body_clearance`), D178/D179 (base-point drawing), D183/D185 (foundation lifecycle), D184 (eviction guard, UNVERIFIED), D187 (body-first click), D193–D196 (process rules), D189 (dead since D200), D197 (removed by D279's re-port; orphan comments at `lib.rs:985-993` flagged for D279's ADR).
- Pins 22/22 in 27 s; full clean suite 942/2/23 (reds: 2 known `modifiers` Town-Watch); fmt clean; clippy not run (no Rust touched).
- Deletion: 302 bank lines, 15/15 header-asserted; MANIFEST → 0021; areas stripped (15); index 300 → 285; 7 live files regenned; site refused by shrink ratchet by design (as 0017–0020); tools tests 228/230 (shrink-clears-on-commit + pre-existing plan_check path).
- No ai-review: OCR provider config lost in reboot (same as 0020, recorded in signoff note).

**Adopted and binding:** adopt-from-game (0019) + conditional grow+test+reset (0018) + one box ruler (0020); margins 0.95 gather / 1.0 walk. Evidence order unchanged: original game, open-source/published, disagreements parked not smoothed.

**Candidates for the next tranche (owner picks one):**
1. D242–D245 — R1/invariants follow-ups to 0019.
2. D287/D288 — relay cut + gather re-target rule.
3. Foundations-adjacent remainder: D174, D201–D203, D206–D238 (large; suggest splitting).

**To do, in order:**
1. Owner names the tranche.
2. Draft the ADR (retirement record), plain language, test-first if code is touched.
3. Owner verdict: sign / amend / rework (one word signs).
4. On sign: delete bank rows; MANIFEST per-D-id rows → new ADR; `make decision-index`; regen live files; review note with Performance + Stability; commit; push.

No SPEC file in this tree (cited by section only); no changelog file (review notes + bank rows are the changelog).
