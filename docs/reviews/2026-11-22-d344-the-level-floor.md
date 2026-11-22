# 2026-11-22 - D344/D345: the level floor, the funnel, and the acceptance that put the floor back

The change that **landed**: measurement only. `TierShadow` (every tier answer priced beside the flat search's answer for the
same walk - waypoints and polyline length in raw units), `TierFunnel` (why the tier did not answer: long walks, floor
refusals, no-coarse, one-cell chains, refused legs split body / object / tiles), `Ground::compare_tier_routes` as the probe's
switch, and `flat_route` extracted from `route_with`'s tail so both answers come from the same code. Probe-only, never hashed.

The change that **did not land**, with its numbers: the level floor moved from 2 to 1 (D344), and the chain's body
certification was dropped (D345). Both were landed, measured against the acceptance the slice was for, and put back.

## Performance

**The floor alone is inert** (D344). Acceptance, 48,000 ticks, release, median of three, back to back on an idle box, tier
forced to each floor by edit on the same tree:

| floor | wall | band | hash | entities / players | re-paths |
|---|---|---|---|---|---|
| 2 | 12.00 s | 11.90 / 11.90 / 12.09 | `1e1fcc3cf2b05a4c` | 133 / 1 | 36,567 |
| 1 | 12.16 s | 11.93 / 12.16 / 12.19 | `1e1fcc3cf2b05a4c` | 133 / 1 | 36,567 |
| 0 | 11.96 s | 11.85 / 11.96 / 12.14 | `1e1fcc3cf2b05a4c` | 133 / 1 | 36,567 |

Bands overlap, every counter identical: **no floor value moves this match**, which withdraws D338's 13/108-answer figures for
this tree (they predate D341).

**The funnel says why** (`tier_probe`, 48,000 ticks): 91 long walks, floor-refused 60, no coarse route 13, leg refused by a
body 17, by geometry 1, **answered 0**. So the floor was never the gate - two of our own gates behind it were.

**With both gates opened** (D345), the tier answered and lost. Same tree, floor 1 and the body veto dropped:

| | tier | flat search, same walks |
|---|---|---|
| waypoints | **172** (12.3/walk) | **67** (4.8/walk) |
| length | **5,180,218 raw** | **4,719,155 raw** |
| | | tier **+9.8%** ground, **2.6x** the waypoints |

and the match: **13.71 s** median of three (13.48 / 13.71 / 13.76), `2999657a804e913b`, **134 entities**, 1 player, 36,569
re-paths - against 12.16 s and 133 entities with the floor at 1 but the veto in place. The tier's routes cost **13% of wall**
for **14 of 50,022** routes, hand the mover 2.6x the waypoints, walk 9.8% further, and change the match by one entity. The
owner's acceptance - *"is the tier's route better than the flat search's on the same walk ... if it is not better, the finding
is the numbers and the floor stays at 2"* - is answered **no**, so the floor is 2 and both gates are back.

The instrumentation's own cost: off in production, one `Cell` read per tier answer; the shadow's second search only while a
probe has asked for it. The wall at floor 2 is unchanged from before the slice, which is the check that the extraction of
`flat_route` did not move the flat path.

## Stability

Attacked, and what each attack found:

- **Every floor value, by edit**: identical hash and counters at 0, 1 and 2; the wall differences are the band. Nothing to
  attack in the gate itself.
- **The two gates, in order**: opening the body veto changed the hash (`2999657a804e913b`), kept 1 player and moved one
  entity - so the match is sensitivity-tested, not assumed.
- **`flat_route` extraction**: the tail of `route_with` moved, not rewritten; the acceptance hash at floor 2 is identical to
  D343's, and every route-shape, straight-line and route-avoiding fixture passes.
- **The funnel and shadow are off**: `watched` is one `Cell` read per long walk; `no_float`, `state` and `reseed` pass, so
  nothing leaks into the hash.
- **A goal in an occupied cell** falls through to the flat search (13 walks) - no panic, no loop.
- **An open map** still descends to 3 and answers from the hierarchy (`descent.rs`, `tier_ignores_bodies` green), so the
  floor change did not move the level choice for a start that already passed.
- **Parked fixtures**: both witnesses are in `docs/parked/` with the un-park condition in a header comment, and the suite
  does not compile them - the failure mode that parking creates is named where the next session reads it.

**My errors, in the same breath:**

1. The first fixture put a wood resource on grass; the loader refused it (`"wood" at (11, 43) is not on a forest tile`).
2. The same fixture's descent premise read 3 instead of 1 because `Ground::from_map` carries terrain only - resources are
   stamped by seeding. The premise now stamps them the way seeding does.
3. **The witness I first wrote for the body veto could not bite**: a villager's half is 0.5 of a tile, so his grown box
   (0.7) always contains a tile centre near him; my "box clips the line, no tile does" scene was refused by both models. The
   witness was rewritten to a stander on the corridor - a case where the two models genuinely disagree.
4. **D338's answer counts**, quoted in this tree for a session as the floor's price, do not reproduce after D341. The rows
   now say so.
5. The first cut of the instrumentation had eight arguments per function and `clippy -D warnings` refused it; bundled into
   `Walk`.

**The landed tree, after the withdrawal**: acceptance `1e1fcc3cf2b05a4c`, 133 entities, 1 player, 36,567 re-paths, wall
**12.18 s** median of three (11.94 / 12.18 / 12.21) - bit-identical to the tree before the slice, which is the proof that what
landed is instrumentation and an extraction, not a behaviour.

**The AI review (OCR, D209), and what I did with it.** `make ai-review` on `7acffe6` ran 3m8s and returned **two findings and
a cut**: it hit the 700k-token budget after four files, so `crates/sim/src/lib.rs` and `crates/sim/tests/tier_probe.rs` were
**not read** - both are thin pass-throughs and a print, reviewed by hand instead, and said so here rather than implied.

- **Accepted**: the parked fixture would have **panicked** on a tree without the generated pack (`common::present` refuses a
  load error, and `config/gamedata/units.toml` only exists after `cargo xtask gamedata`, D8) instead of skipping as
  `tests/common/mod.rs`'s first rule says. Both parked files now check the marker file and return.
- **Accepted**: `LegVerdict::Clear` was folded into the `Tiles` counter in a block only entered on a refusal - an impossible
  state that would have counted a clear leg as a blocked tile if the guard ever moved. It is its own arm now, with the reason.
- **Refused**: nothing.

**Not attacked, and named**: the game's coarse *costs* (`FUN_007ed280`'s 10/100/300/600 rows) are ours to port but not
measured against the flat search's octile costs as a route-quality question; that is the fork in `docs/next-step.md`. The
search window (D299) is still unread, and a windowed search would answer fewer walks, not more.
