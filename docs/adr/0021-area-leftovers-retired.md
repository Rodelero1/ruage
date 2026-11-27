# ADR 0021: foundations stand, the body is drawn where it stands — area leftovers retired

**Status:** Signed 2026-11-27. Deleted 15 bank rows on sign-off: D175, D176, D177, D178, D179, D183, D184, D185, D187, D189, D193, D194, D195, D196, D197. D190 never issued — recorded, not deleted (`docs/dangling.toml:15` stays).

## What this batch is

Sixteen ids from the frozen bank's oldest areas, all already built and pinned except three paper rows: D189 died when D200 was signed (0018), D190 is a numbering gap that was never a decision, and D197's counters were removed by D279's movement re-port (D279 itself still pending — this ADR records the removal, it does not judge it). Everything else retires as built. No code moves in this step; the one thing found — orphan comments where D197's fields used to be — is recorded below and left for D279's ADR to sweep or keep.

## The story, in the order it happened

1. **D175 — the search radius is the unit's own number.** The dat gives every unit a `task_info.search_radius` (villager 4 tiles); `gather_reach` uses it when the pack carries it, older packs keep sight-plus-five. Mutation-proved: disabling the branch reports "looked 9 instead".
2. **D176/D177 — collision is the body's, drawn and tested.** The ring shrank to the collision circle (scale 1.5 → 1.0; today the ring is `ring_scale` from content, D125 — same shape, owned by data now). The step refuses landings with negative clearance unless already inside and not digging deeper, with one exception for a unit entering its own building. Cost measured at the time: repaths 24 → 529, because the planner still smoothed corners blind to the radius.
3. **D178/D179 — the drawing caught up with the body.** The silhouette's anchor moved to the base point, the ring rose to embrace the feet, the health bar went above the head. Three view tests learned the new anchor with reasons written down, not deleted. Acceptance byte-identical.
4. **D183/D185 — foundations, the owner's model, built in order.** Walkable invisible unstarted ghost, cost at placement with full refund on cancel, builders working from outside the footprint, no start while any unit stands on site (allies and animals told to leave, enemies stall as a tactic), first stroke takes the ground, farm the walkable exception. Eleven tests in `foundation_model.rs` plus the D185.1 defect (a cancelled site resurrected by its builder — fixed with "a dead site is not worked"). D185.2 says plainly what was not built: resume covers gather and hunt only, a fighting unit blocks its own step-aside, an unexitable site stalls on one message, farms seed from beside not from on.
5. **D184 — the guard that never fired.** Written test-first in two shapes before any code moved, and it passed on the shipped engine: our forests are terrain, so eviction physically cannot choose a tree. It stays a guard in `eviction.rs`. Conformance UNVERIFIED — the accident needs the object-on-grass tree model this tree still does not have.
6. **D187 — the click follows the drawing.** `body_contains` is the renderer's own geometry, front-most by `depth_of`, own-first then id so snapshot order never decides. Deliberately not covered: `pick_at` (right-click orders) is still tile-based — an order is a place, and that question has its own evidence to gather.
7. **D189 — dead link.** "Own units pass through each other" was withdrawn by D200, signed in 0018. Recorded here so the id resolves; nothing to pin.
8. **D193/D194/D195/D196 — the process rules that held.** Tests assert relations and quote measurements in failure text only (D193); errors classify themselves with no wildcard arm so a new variant does not compile unclassified (D194); this file is capped at 16 KB starting with `# OPEN` and the tooling enforces it (D195); movement decisions are values with a per-body trace behind a flag (D196 — 0.4 µs of a 6.7 µs tick, off by default, and the trace caught a walker passing an idle enemy at 0.237 where 0.4 was owed).
9. **D197 — built, then removed by later work.** The by-reason repath split (`repath_by_body/_ground/_failures`) went out with D279's movement-layer re-port (`220a222`); only the total `repath_count` survived. Its doc comments linger orphaned at `lib.rs:985-993`, describing fields that no longer exist — flagged for D279's ADR, not moved here.

## What survives (live rules, with witnesses)

- **`search_radius` per unit** (`crates/data/src/units.rs:25`, `crates/sim/src/lib.rs:1587`): the dat's number bounds the next-source search — pin `search_radius.rs`.
- **Clearance as one integer expression** (`lib.rs:3827`, `:3911`): distance to nearest blocked tile minus radius; the Forgotten Empires shrink-the-unit idiom — pins `body_clearance.rs`, `collision.rs`.
- **Base-point drawing** (`crates/view/src/lib.rs:1032`): ring and body agree where the bottom is — view tests.
- **Foundation lifecycle** (`world/build.rs:188,261,280`, `error.rs:241`, `lib.rs:1139`): outside approach, empty-site gate, walk eviction, first-stroke ground, full-refund cancel — pins `foundation_model.rs` (11/11), `site_clear.rs`, `eviction.rs` guard.
- **Body-first click** (`crates/view/src/ui.rs:49`): renderer geometry, depth order, deterministic ties.
- **Classified errors** (`error.rs:542,559`): Refused / BadRequest / Internal, no wildcard — pin `error_class.rs`.
- **Valued movement decisions** (`lib.rs:672,178,1701,984`): thirteen outcomes, per-body trace behind a flag — pin `step_decision.rs`.
- **Relation-asserting tests, capped next-step** (D193, D195): this file complies; SPEC cited by section only (no SPEC file in this tree).

## Evidence (D203 order)

1. **Original game first.** D175's numbers come from the dat file itself (`task_info.search_radius`, v5.7.2 — the install's own data, read not copied). D183/D187 rest on the owner's verbatim model statements, which for behavior-only rules are the primary source the bank cites. Decomp cross-checks from sibling tranches (box geometry, waypoint discipline) contradict nothing here.
2. **Adjacent sources second.** Nothing in openaok/genie or the published tables contradicts the survivor list; the clearance idiom is the Forgotten Empires talk's own shape. No new cross-check run: nothing in this batch ports game geometry.
3. **Disagreements parked, not smoothed.** D184 stays UNVERIFIED (guard, not claim). D197's removal is D279's to justify. The D152/D183 link direction follows the bank (D183 supersedes D152's foundation-blocked part), correcting the index row that reads inverted. `pick_at` tile-based picking and farm-seeding animation stay open with their owners named (D187, D185.2).

## What this commits us to

Foundations behave as the owner described, bodies collide and draw at the same place, clicks select what is drawn, errors say whose fault they are, tests state relations, and removals are recorded where they happened rather than silently absorbed.

## Sign-off checklist (pins green, verdict owed)

- [x] Pin tests 22/22 green this step, in the working repo (`/app/ruage` on `adr-migration`): `search_radius` 2, `body_clearance` 3, `site_clear` 1, `foundation_model` 11, `eviction` 2, `step_decision` 1, `error_class` 2
- [x] Full suite on the sign-off tree: **942 passed / 2 failed / 23 ignored** on the clean tree (reds: 2 known pre-existing `modifiers` Town-Watch — merge baseline, zero new) + `cargo fmt --check` clean (clippy not run: no Rust file changed), recorded in `docs/reviews/2026-11-27-0021-signoff.md`.
- [x] `make ai-review`: not run — no OCR provider config on this box after the reboot (same as 0020; recorded in the review note, not skipped silently)
- [x] Owner verdict: sign (2026-11-27)
- [ ] On sign: delete 15 bank rows (done — D-sections incl. `###` D187/D189/D193–D197 subs; D190's dangling row stays), MANIFEST per-D-id rows → 0021 (done), `make decision-index` (done, 300 → 285), regen live files (done, 7 files) + site (refused by shrink ratchet by design, recorded), review note with Performance + Stability (done), commit by name, push
