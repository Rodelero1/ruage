# ADR 0018: the crowd — eleven rulings, seats gone, conditional growth adopted

**Status:** Signed 2026-11-28. Deleted 11 bank rows on sign-off: D200a, D200, D204, D205, D289, D290, D291, D292, D293, D294, D295.

## What this batch is

Between 2026-11-13 and 2026-11-19 the engine learned what a crowd is. It went through three answers in six days: friends pass through each other so a crowd can deliver (D189, withdrawn), friends are solid with nudges and slides to keep the crowd moving (D200/D200a), and friends are solid with only the pause and the re-path acting — crowds jam, press to any depth, and never lose their orders (D289–D295). Then the owner settled the remaining question — how the no-overlap guarantee is actually computed — from the DE live process: conditional growth, box against grown box, terrain point-tested (owner decision 2026-11-28).

This ADR retires the crowd/collision line as argued and adopts the conditional idiom as the rule.

## The story, in the order it happened

1. **D200a — the first landing, parked.** Owner-blind collisions built on a branch (`d189-owner-blind`): no alliance test anywhere, repath round a friend instead of passing through. Parked on a measurement, not landed: our working places sat exactly one reach apart, so a man could not stand next to a standing friend. Kept because its failure set the landing shape.
2. **D200 — friends are solid.** The owner's rule verbatim: no friend-or-foe difference, as in the original. With it came the rest of the machinery: forty-tick nudge, stale places, comfortable-then-packed placement, yard aims, finish-where-you-stand — and the Rule 4 slide fix, when seven villagers waited a whole match on their own tiles and banked 0 food. Price recorded: slower matches, enemy still standing at 40:00.
3. **D204 — no pushing.** The owner, playing the D200 merge: a man refused his square takes the next node of the same kind, never shoves the holder. Built with a counted refusal (`square_refused`, re-task after `STALLED_TICKS`) and two gates in the old nudge. The carrier half parked: `made_room` counts the walker's step, not the shove, so no fixture could witness it. **Partly superseded by D239/D241** — the idle-body nudge is gone, the re-target stands.
4. **D205 — right of way is the lower id.** Read out of the HD binary byte for byte: each side calls `[vtable+0x2b4]`, the higher handle pauses 0.25–1.50 s keeping its path, the lower proceeds. Built — then measured **not landable as written**: the pair stops at exactly the two 0.2 radii, the winner's step is zero, and a tile-set repath cannot route round a body. The decision's own claim (a bounded pause means no standing forever) is false on a tile router. Recorded, with the two alternatives named and neither taken.
5. **D289 — crowds jam.** The owner, asked whether followers keep private ring places: drop it, let them jam, start as close to the game as sensible. Every man's goal is the ordered tile. `crowd_ring_point` deleted.
6. **D290 — no press floor.** Read from the binary, not chosen: the depenetration is computed in full and then discarded (never read back into position). Friends may overlap to any depth; only the pause and the re-path act.
7. **D291 — same kind only.** The engine never moves a man across resource types — that is the AI's job (`sn-retask-gather-amount` and kin). The bush-to-farm arm cut; a forager with no bush in reach stands.
8. **D292 — orders are never given up.** `beside` and the forty-tick finish go. Added because the cut exposed it: a mover re-armed his friend's pause every tick (1,876 ticks of standing against the game's 0.25–1.50 s). A no-two-bodies guard tried and withdrawn — the game has no press floor, so fixtures ask for convergence.
9. **D293 — gatherers press in.** The gather arm hands out no seats; a man goes to his node and takes when his box is against it. The spreader is D204's re-target with a new trigger: the refusal count itself (route spent and still not taking). Suite 897/11/21, four greens.
10. **D294 — the seat book is gone.** Deleted, not left inert. Suite byte-identical before and after (895/11/21) — which is what "inert" has to mean.
11. **D295 — same-tile gathers are accepted.** D293's press lets crowds stand on kept-clear ground; three fixtures spawned on the bush's own tile and were refused as `NoPath` on a zero-length route, then remembered as walled-off. Gather validation now skips the reachability search in exactly that case.

## What survives (live rules, with witnesses)

- **Friends are solid** (D200's core; D189 withdrawn, D200a parked). A body blocks whoever it belongs to.
- **A refused square answers same-kind, never with a shove** (D204 via D291, trigger via D293): `nearest_like_free` (`gather.rs:41`), refusal counted, re-tasked after `STALLED_TICKS` (`step.rs:922,1722-1740`).
- **The lower id proceeds, the higher pauses 0.25–1.50 s keeping its path** (D205), and the pause plus the re-path are the only collision answers (D290/D292). D205's "never stands forever" claim does not hold on our tile router — recorded open, carried.
- **Crowds jam on the ordered tile** (D289); **no press floor** (D290); **orders are never rewritten** (D292); **no seats** (D293/D294, 0017's commitment kept); **same-tile gathers accepted** (D295).
- **Adopted 2026-11-28 (owner, binds): the no-overlap test is conditional growth.** Box against grown box: when the flag is set the tested box grows by the margin on every side, the test runs, then flag and margin reset. Terrain tiles are point-tested, never grown. Working margins (ours): gather ends × 0.95, search walk × 1.0 — the game's own menu is wider (×0.5 and ×4.0+0.1 sites seen; 0.95 occurs at exactly one site), so these are working values, not the game's constants. Witnesses: flag `[ctx+0x57]` + margin `[ctx+0x58]`, grow+test at `0x14111d1b4` / `0x14111d91f` / `0x14111c785` (all three cmps against zero — flag-0-means-ungrown is global, not per-site), recompute `0x14111cd27`, setter `0x141121350` (26 callers) called with `dl`=1 at ActionGather `0x140b7b44e` (margin = `[X+0x78]` × 0.95), test `0x140b7b4cc`, reset `0x140b7b4dd` (flag and margin both zeroed, straight-line set…reset, terrain early-exit precedes the set); unscaled setter calls in the MapAnalysis walk (`0x1410598c3`, `0x141059bbf`); terrain point-test `0x140b7b3f3`. One setter site takes the flag conditionally (`seta`, `0x1410612f8`) — still reset-paired.

## What is gone (mechanism deleted, rows deleted with it)

- Pass-through friends (D189, via D200), the D200a attempt, D200's nudge/slide/give-way mutex/stale-place/comfortable-packed/yard/finish machinery (replaced by the pause).
- The per-follower ring goal (D271 moving side, via D289), `beside`, `finish_blocked_orders`, the cross-kind arm (via D291), the no-two-bodies guard (tried, withdrawn).
- The seat book (D293/D294): `standoff_point`, `Standoff`, `gather_places`, reservations, releases — suite byte-identical across the deletion.

## Evidence (D203 order)

1. **Original game first.** HD binary: collision block with no alliance test, owner sentinel query, lower-handle-wins pause (`008a80ed`–`008a8141`, `path+0x28`, uniform 0.25–1.50 s); depenetration computed in full and discarded at every call site (`FUN_008aa990`; live callers `FUN_008a4360` + `FUN_008accc0`, plus dead duplicate `FUN_008ade90` — none writes the result into a unit position). DE live dump: conditional Minkowski growth as cited above; `[X+0x78]` identity open (chains pass through string-decrypt idiom); 0.95 gather / unscaled walk as working margins; per-site `+0x84` multiplicand open. Owner's screenshots and verdicts quoted in the rows.
2. **Adjacent sources second.** Openaok (`Zaydme/openaok`, MIT): per-tick per-tile `Traffic` survey, standing boxes, bend-round-standing-boxes routing, capped separation steering; genie (`librematch/genie`): boxed search avoidance, hit→pause/replan, `closest_spot` placement search; openage: classic-engine tier description. Replay-derived DE model: same-kind next-source search within a radius; AI retask params as the cross-kind mechanism. Decomp: silent on gather/work/speed — stubs only. Community: men piling on one tree, standing pressed. Deck (Klinger's): shrink-to-point/expand one-liner only — motivational, never a gate.
3. **In-tree cross-checks.** Point-expansion crosscheck §§5–7 (HD search point-in-tile, movement grown-box, multiplier 1.0; DE static terrain point-test confirmed; DE dump conditional growth settled). No-overlap survey §3 (OSS mechanisms, O1–O5 options; O2's failure mode measured in our history: repaths 24 → 529 when clearance entered the step). D205 addendum (pause measured exact, pair still stuck at 8000 raw). D292 guard withdrawal (0-raw press vs convergence).
4. **Disagreements parked, not smoothed.** `+0x78` field identity (revisit trigger); 0.95-vs-unscaled rationale; per-site `+0x84` multiplicand; D205's pause-vs-tile-router gap (winner step zero, repath inexpressive round a body — lateral step vs step-through-paused, both need the owner's word); 4576-raw press vs the game's unbounded press (ours is the tighter bound; fine).

## What this commits us to

- Every set of the grow flag is paired with its clear; a stuck flag silently strictens everything after it. Grow+test+reset is one idiom, not three steps to pick from.
- Flag policy is the tolerance call: flag 1 where overlap is forbidden (buildings, trees, mines at rest), flag 0 where crowds may press.
- Friends block; the pause and the re-path are the only answers; no order is ever rewritten; no code path keeps a per-man place (0017's commitment, kept).
- Any future crowd rule argues against the game's press (unbounded) and spacing (~0.66) first, not against a constant.
- Functional gates bind: never stuck (every crowd scene banks), never through blocked (grown-box refusal + box-gap take-gate).

## Sign-off checklist

- [x] Owner verdict (sign / amend / rework): signed 2026-11-28.
- [x] On sign: bank rows D200a, D200, D204, D205, D289–D295 deleted from `docs/archive/decisions-2026-11-24.md`.
- [x] `MANIFEST.md`: per-D-id rows → 0018 (D200a noted superseded-retired; D204 noted partial: nudge half superseded by D239/D241).
- [ ] `make decision-index`; regen live files (`live_migrate.py` — expect clean, no new orphans).
- [ ] Review note with `## Performance` (no engine diff) + `## Stability` (what was attacked); commit; push.
