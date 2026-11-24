# Conflict table — draft for owner ruling

**Rule:** agent drafts, owner rules (agreed 2026-11-24). No live file is authoritative until each row below is ruled. Proposed dispositions use the truth hierarchy in `docs/live/README.md`.

Format: conflict → bank rows → evidence → proposal → needs-owner.

## C1. Gather standoff distance (3 numbers, 1 live)

- Rows: D186 (1.156-tile ring, superseded) / D192 (tile border) / D199 (touch, owner's saves) / D191 (border) / D169 (0.7 tile) / D182 (body radius off square) / D285 (box gap, game's rule, deletes 4 of ours).
- Evidence: `FUN_00840c60` segment test, box-gap arrival (D285 review 2026-11-19), owner saves.
- Proposal: live = D285 + D199. Demote D186/D169/D182/D191/D192 to archive with supersede links. D285's box-gap is the arrival; touch is the measurement.
- Needs owner: confirm D285's box-gap kills D169's 0.7 and D186's 1.156 for good.

## C2. Seat book / pile model

- Rows: D293 (press in, book stops being read) / D294 (book gone) / D271 (first man takes spot, superseded by D289) / D289 (crowd jams, owner) / D351 branch (taken place is not a place, not landed) / D357 (arrival is placement, pile halves) / D354 (picker breaks owner's rule).
- Evidence: pile census 2026-11-22 (11 bodies, 317k pairs), D357 review (144,534 → 74,717 pair-ticks).
- Proposal: live = D289 + D293 + D294 + D357. Park D271 as superseded, D351 as branch-not-landed, D354 as UNVERIFIED until anchor lands.
- Needs owner: C1's last inch — place model stays withdrawn (D293) or pile accepted as game behavior?

## C3. Coarse cost table

- Rows: D298 (10/300/2000) / D327 (100/600 are terrain flags, 2000 is not bodies) / D342 (300 row is a mine) / D328 (2000 row priced a rule the original does not have; terrain bits unread).
- Evidence: `FUN_007ed280` reads 2026-11-21 (D327/D328 reviews).
- Proposal: live = D327 + D328 + D342 as GAME with `FUN_007ed280` pointer. Demote D298 to superseded-in-part.
- Needs owner: none on facts; confirm demotion wording.

## C4. Tier activation / floor

- Rows: D302 (line→hierarchy→tile) / D305 (hierarchy never ran for units, D302 withdrawn) / D300 (correct but 60x too slow) / D301 (pyramid cached, D300 withdrawn) / D344+D345 (floor 2 restored, tier worse than flat) / D346+D347+D348 (follow game, level 0 is game's, window is per-unit margin).
- Evidence: `FUN_007ee980`, `FUN_007f0be0`, TierShadow/funnel measurements 2026-11-22.
- Proposal: live = D346 + D347 + D348 (GAME, with unread-margin caveat) + D338 (avoid gate dropped). Archive D300/D302/D305 as superseded chain.
- Needs owner: confirm floor stays at game's (level 0 answered) per D346 ruling.

## C5. Body collision / solidity

- Rows: D189 (pass through, superseded by D200) / D200 (friends solid, owner) / D200a (parked branch) / D236+D237+D238 (walk through / solid / shuffle) / D244 (both red fixes starved match, reverted) / D290 (no press floor) / D339 (body is a box) / D281 (body collision is original's).
- Evidence: `FUN_00844360` epilogue (step taken, higher id paused, mover re-plans), `FUN_008aa990` discards depenetration (D330 blunt copy).
- Proposal: live = D200 + D281 + D290 + D339 (GAME) + D200/OURS wrapper (owner). Archive D189/D200a/D236 as superseded.
- Needs owner: none new; re-affirm D200.

## C6. Defeat / victory

- Rows: D46 (match ends when seat gone, supersedes D59) / D59 (defeat is absence, supersedes D46) / D59a (counts only if produces/takes in, owner amendment) / D59b (dat building classes, supersedes D59a).
- Proposal: live = D59b. Archive D46/D59/D59a as chain.
- Needs owner: confirm D59b ends the chain.

## C7. Housing / production ceiling

- Rows: D31 (housing + exemption from tree) / D34 (ceiling pauses, correcting D31 refusal) / D66 (second order queues) / D157 (one queue) / U24 (housing grant unread, action Housing never fires on 0-work-rate house).
- Proposal: live = D34 + D157 as Decisions, U24 as UNVERIFIED. D31 refusal clause archived.
- Needs owner: confirm pause-not-refuse stands.

## C8. Garrison / building volley

- Rows: D36 (garrison is data, superseded by D38) / D37 (correcting D36: ejected at 20%, volley is dps) / D38 (production holds what it trained).
- Proposal: live = D37 + D38. D36 archived.
- Needs owner: none; confirm.

## C9. Gather radius / search

- Rows: D141 (radius bound, adopted then refused by measurement) / D149 (nearby bounded by sight, superseded by D150) / D150 (anchor was the problem) / D175 (dat's own number per unit) / U22 (game: LOS+5, 30 reach-tests, drop-site first pass; ours unbounded).
- Evidence: `gather_search` port notes in mechanics README U22.
- Proposal: live = D150 + D175 as GAME-cited, U22 as DEFECT in divergence register (already there in spirit). D141/D149 archived as refused/superseded.
- Needs owner: confirm U22's LOS+5 + 30-test contract is the target (test-first, in order).

## C10. D240 (never delete) vs cleanup

- Row D240 says retire-never-delete, supersede-link one side only.
- Proposal: amend D240 for one migration only — live may rewrite, archive preserves every old row. After migration, D240 resumes.
- Needs owner: approve one-time break.

## C11. Movement-layer ownership

- Rows: D98 (two owner rulings) / D98a (superseded by original's mechanism) / D279 (movement layer is original's, blunt port) / D330 (blunt copy, register was lying) / D247 (original's rules are separate conformance set).
- Proposal: live = D279 + D330 + D247 (GAME/OURS split). D98a archived.
- Needs owner: re-affirm blunt-copy (D330) still holds.

## C12. Unsourced GAME claims (burn-down 5)

- Rows per index: D151 (AI leaves binary) / D166 (marker clipped) / D184 (reproducer/tree model) / D226 (farm reseed auto/manual) / D228 (min-1 once per attack).
- Proposal: demote all 5 to UNVERIFIED in live with settle-condition; do not build on them. D228 already has 3 sources named in title — promote to GAME if sources check out, else UNVERIFIED.
- Needs owner: confirm demote-not-delete.

## What happens after ruling

Owner replies per-C (accept / amend). Agent then writes `docs/live/<area>.md` files, moves bank to `docs/archive/decisions-2026-11-24.md`, archives sprawl, regenerates indexes, and records the migration in SPEC changelog.

## C13. Numbering collisions (blocks every index)

- Rows: D88/D89/D90/D167/D168 each head two different decisions (`docs/decisions.md:2288/2325/2343/2357` vs `:2325/2343/2357/4129/4186` area). 2026-11-01 batch restarted numbering while D88–D90 already existed. Suffixed IDs (D24a…D210b) and split headings (D77–D82) explain the 264-headline vs 345-indexed gap.
- Proposal: live files use new unique IDs with `supersedes D88a/D88b` mapping table; bank frozen as-is for provenance.
- Needs owner: accept renumber on migration.

## C14. Area taxonomy drift

- `docs/decision-areas.toml` (15 authored areas) vs `decision-index.md` (10 heuristic buckets, reads no TOML). Vision & fog, Production & queues, Data & tooling, Licensing & publication vanish; D50/D214 sit under Combat, D152 under Buildings, D307 alone under Other.
- Proposal: live uses the TOML's 15 areas. Index generator switched to read TOML or live files replace it.
- Needs owner: confirm TOML wins.

## C15. Conformance verdict drift

- New rows D353/D356/D357/D358 carry proper `GAME - <pointer>`; D354 carries nonstandard `OWNER` which `conformance_audit.py` does not count (reports GAME 4, OURS 3, UNVERIFIED 1, pending 0).
- Proposal: `OWNER` → `OURS - owner ruling <date>`. Audit schema stays 3 verdicts.
- Needs owner: none; editorial.
