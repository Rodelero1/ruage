# ADR 0019: adopt the game's pathing, keep the re-target — two rulings, nudge gone

**Status:** Signed 2026-11-28. Deleted 2 bank rows on sign-off: D239, D241.

## What this batch is

On 2026-11-16 the owner settled how pathing disputes get decided: adopt from the game, and where the game cannot be read, read first — never guess in the game's name. D239 is that order plus the nine answers the game did settle and the two it could not yet. D241 is the four forks the game leaves open, answered by the owner the same day, one of them already built.

This ADR retires the pair as argued, and closes the D204 partial left open by 0018: only the idle-body nudge went; the same-kind re-target is the owner's rule and stands.

## The story, in the order it happened

1. **D239 — the game is the source of truth.** Every divergent row in the behaviour register becomes a slice; where the game's side is unread the read comes first. Adopted as read: a body at rest is not walked through (rule 6, `TakeAtRest`, goes — an idle friend who is not past is a non-past collision; the epilogue has no step-through arm, only target / non-past-repath / pause-inside-repath / don't-know, and the repath gate reads the non-past *unit* counter only — pure-terrain falls to don't-know); the re-plan stops folding met bodies into the search (search loses the avoid set, classifier keeps it); the pause is re-drawn on a fresh collision (`setPause` has no already-paused guard — single store `path+0x28`, null-path guard only); a paused mate stops being a mate on the next (re)search (exemption is a search-time predicate on live `isMoving`, HD `0x800b5e` / `0x7f9e30` — not a per-tick re-test; the pause loop itself re-filters neither past nor ignore-list); a man with no seat follows D204 as built (another bush nearby → go work there, none → wait) — **only the nudge retires**; an unreachable waypoint is substituted, not waited on (`finish_blocked_orders` retires for it); passage is the body's (supersedes D117 — grown ground, grown graph); plus the sibling thread's start re-seed and narrower order vocabulary. Blocked, not portable on a guess: the cost model (16/step vs 2500/100 estimate — DE answers half: estimate args ARE tiles so the tension is real and larger, live default ×75 at `0x14435d6a0` = 7500 not ×25; the 16/octile half has NO DE counterpart, neighbour key is squared-distance arithmetic) and the two-level parent path (HD idiom refuted in DE: zero `AddFuture*`/parent strings, movement calls PathingSystem directly; `0x141077870` parked as the one unread post-search candidate). Substitution has no DE counterpart (no `faking`/counter strings or code; single call, no retry). Start re-seed likely absent-by-design in DE (neither search entry tests the start tile). Nothing ruled out of adoption, including both re-plan caps. Left open: the exclusive goal — forty units ordered to one tile, the cheapest artefact.
2. **D241 — four forks the game cannot settle.** A stopped man is walked through (*follow game*): the past rule stands for every body, D238 step 1 stays withdrawn, and visible overlap of a routed-through stopped man is the pause's job, not a shove's. Formations deferred, villagers solid: the mate exemption keeps its same-tick stand-in for soldiers as a documented stand-in, villagers join no formation so D237 already is the game's rule. A thing that cannot be reached: accepted, planned to the nearest reachable point, and for a resource node preferring a reachable same-kind node within `gather_reach` — the no-seat helper at a different trigger. A crowd at a bush: already built — after forty refused ticks `count_refused_squares` asks `nearest_like_free` and re-targets, or stands the man down. The decision corrects its own predecessor: D239 and the D204 marker said the re-target was retired; it is not — only the displacement goes. Names the fixture consequence: with the past rule standing, the 0.204-tile pile is the game, so the fixture must assert nobody is shoved, not that no two bodies overlap.

## What survives (live rules, with witnesses)

- **Adopt-from-game binds** (D239's order): divergences become slices, unread sides get read first.
- **The past rule stands for every body** (D241.1, HD `0x8a8044`–`0x8a81d2`): an idle friend is not a wall to a later route; overlap there is answered by the pause, which re-filters neither past nor ignore-list (a higher-handle mate is still paused).
- **Pause is re-drawn, never guarded** (D239.3, HD `0x8aa6d0` six instructions): fresh collision overwrites `path+0x28` unconditionally.
- **Villagers are solid; soldiers keep the documented stand-in** (D241.2): `stamp_group` later, D237 now.
- **Unreachable is accepted and approached** (D241.3): nearest reachable point; same-kind node within `gather_reach` preferred.
- **Forty refused ticks re-target same-kind or wait** (D239.5 via D241.4, D204's standing half): `count_refused_squares` (`step.rs:88,335`), `STALLED_TICKS = 40` (`step.rs:922,1727`), `nearest_like_free` (`gather.rs:41,361`). `TakeAtRest` deliberately absent (`motion.rs:17`); avoid set out of the search, classifier-kept.
- **Substituted waypoints, body-grown passage, re-seeded starts** (D239.6–7 + sibling 33, HD-side): `finish_blocked_orders` gone; one-tile gap rule inverted by the D239 test. DE side: no substitution counterpart, no re-seed (start never tested), both parked — HD mechanisms, not New-System reads.
- **Ignore list is bitmask rows, not an id array** (DE correction): set/clear helper `0x141075b80`, per-tile qword tests (`0x14107993f`); search vs classifier are separate regions sharing the mask vocabulary.

## What is gone (mechanism deleted, rows deleted with it)

- `TakeAtRest` (rule 6), the search-side avoid set, the already-paused guard assumption, the route-held mate exemption, the idle-body nudge, `finish_blocked_orders`, D117's tile-only passage.
- D240's retirement mechanics note: D240 (retire-never-delete, superseded by D192, itself retired in 0017) is not re-deleted here — recorded as already superseded, no row to delete.

## Evidence (D203 order)

1. **Original game first.** HD collision epilogue `0x8a7cb0`–`0x8a83b3` (target `0x8a7fe7`, non-past repath `0x8a8044` gated `test %esi` at `0x8a8051`, pause inside `0x8a80a0`–`0x8a818b`, don't-know `0x8a81dd`); `setPause` (`0x8aa6d0`) no-guard; `isMoving` re-test per-search (`0x800b5e`, `0x7f9e30`), log-only in-tick sites (`0x8a44bc`, `0x8a7d69`); start re-seed `FUN_007fb220` outward-16 over table `0x9e2528` (entries 10–11 duplicated, −67.5° absent); stander direct-insert `0x7f9e42` via chain `0x8007f0→0x7f7460→0x7f9f10→0x7fbab0→0x7f9d80`; substitution count `unit+0x148` threshold >2 (`0x8a7b28`), string VA `0x9ea318`, increment `0x8ade14`. Drive-by: 0018's third depenetration site is live (`0x8adec0` via `0x8a61d2`, not dead `0x8ade90`) — discard half unaffected, recorded for the 0019 crosscheck note, no signed-ADR rewrite.
2. **Adjacent sources second.** Openaok (`95cb8a05`): standers boxed from own radius plus `kPastStanding = 40` search cost and per-route `Walker::passed` memory — structural precedent for search-loses-set / classifier-keeps-it, not proof. genie (`6eb73570`): hit→pause/replan, pause timer the only write into the met body, every retarget kind/group-scoped. Klinger deck (104 pp.): "No pushing allowed", shrink/expand — motivational only. Decomp (`84f354ff`): zero `AddFuture*` in 8,021 entries; cost file is PS2-matched asm with control-flow-only tests — silent on exactly the two fork questions. Community: forum "Unit stacking" + Steam gold-pile stacking — overlap where routes met stopped men, no shove; consistency only, both DE regression-era. No contradictions: no cross-kind in-engine, no place books.
3. **In-tree cross-checks.** D242 measured both R1 halves and reverted (TakeAtRest delete deadlocks the lane pair; avoid-set delete costs 155→143 entities, repaths 380→649) — adoption needs its paying mechanism (past-rule chain; stander re-read). D238's 0.204 pile reclassified by D241.1 from defect to game. HD leg: epilogue/repath-gate/`setPause`/re-seed/stander/substitution re-derived; "every tick" corrected to per-search; pause loop re-filters neither past nor ignore. DE leg: vocabulary retained (`doIntelligentMovementUpdate` + `isMoving=%d` literal `0x1432b22e8`, 2 decrypted refs); per-obstacle predicate `call *0x220` velocity-shaped but speed-vs-isMoving undecided; exemption transient, no route-held flag. Full note: `docs/reviews/2026-11-28-0019-crosscheck.md`.
4. **Disagreements parked, not smoothed.** Cost model: 16/octile not portable (no DE counterpart), estimate ×75 live with tiles-args confirmed — budget-vs-counter reading dissolves the tension the HD way. Parent path: HD idiom refuted in DE, `0x141077870` parked. Substitution/start-reseed: HD-only, no DE counterparts (`*0x220` identity, mask-row meanings, 7500 writer, frontier-key formula, `0x141077870`, start-inside dynamic — each with its next read). Exclusive goal (forty to one tile): open, cheapest artefact. R1a/R1b split: past-rule chain unblocks the arm; avoid set needs the stander re-read first.

## What this commits us to

- No game-named rule without a read: the register, the binary, the talk, or a save — in that order.
- The re-target trigger stays counted (forty refused ticks), never per-tick; the displacement stays gone.
- Fixtures assert no-shove, not no-overlap, wherever the past rule applies.
- The two blocked adoptions stay blocked until the numbers fit; the caps stay adopted (re-measured, regressions reported as findings).

## Sign-off checklist

- [x] Owner verdict (sign / amend / rework): signed 2026-11-28.
- [x] On sign: bank rows D239, D241 deleted from `docs/archive/decisions-2026-11-24.md` (D240 kept: already superseded by D192, no row to delete).
- [x] `MANIFEST.md`: per-D-id rows → 0019 (D204 partial closed by this; D240 noted already-superseded, no row).
- [ ] `make decision-index`; regen live files (`live_migrate.py` — expect clean, no new orphans).
- [ ] Review note with `## Performance` (no engine diff) + `## Stability` (what was attacked); commit; push.
