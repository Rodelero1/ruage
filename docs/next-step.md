# OPEN - R1 continues, and two questions are waiting on the owner

**Where things stand (2026-11-17).** The owner ruled that the game is the source of truth on pathing (D239) and answered the four
forks it cannot settle (D241). Since then: **R1 measured and split** (D242 - both halves built, both reverted, the numbers below);
the four **body rules** green and unskipped (D243/D244/D248); **R10**, the game's pathing answers, built and **parked on one
crowd fixture** (D249); and the **second four rules** green with four defects found in one mistake - a rule asking about the *tile a
body's centre sits in* rather than the body (D250). That last step's acceptance is **67.6/68.2 s -> 28.4/28.5 s**, **154 -> 137
entities**, **1540 -> 710 repaths**, because **the match is now won at 2,381 s** where the control is undecided at 48,000. R7's
first half is settled by reading (below). The narrative lives in `docs/history/next-step-2026-11-17-d250.md` and in D242-D250.

**The rule, corrected by the owner (2026-11-17).** *"man stands or walks. No acceleration or deceleration"* - and *"follow the
game. No sliding."* The fixtures say it in the observable form (one step a tick or a stand), and the slide is what breaks it.
**D255** is where that lands: deleting the slide makes the corner fixture green (567 of 1,398 short ticks -> 0) and reddens
**14 tests across 8 binaries**, and the single refusal behind them is a **164-raw (0.008-tile) brush** against a blocked tile that
`a_villager_walking_beside_a_building_keeps_its_body_clear` forbids on every tick. Both fixtures are honest, so the next slice is
**the model, and it is three pieces (D256/D257)**: the **step** half is written - ground at a point, a building's footprint as a box,
no slide - and the **pathfinder** half is the one that is missing: `path.rs` has **no radius in it**, so a villager's 0.2-tile body
plans a *point* route that hugs the town centre and the mover then refuses the step. The original shrinks the walkable area by the
unit's radius before the search (its own string: *"the waypoints outside the boundaries of where this unit's radii will go"*).
**PROVED (D260, corrected by the owner): turns are instant** - `calculateYawAngle()` limits the **facing**, not the step.

**PROVED (D258)**: the game collides **boxes, not tiles** - `FUN_00840830` builds each object's box from its radius table and
collides on `<=` per axis; `func_002690E0` measures box-to-box, overlap clamped to zero (the half-diameter rule).
Our tree is *not* the wrong size (`TREEA` is `radius = [0.5, 0.5, 2.0]`, so its box is its tile); the 164-raw refusal was against
the town centre's footprint, which the game would refuse too.

**Attempted, reverted (D261)**: five pieces at once, **still 3 red** at **`next=164`**: a tile-centre diagonal past
a corner passes *through* it, so the box overlaps by the whole radius - a town centre's `radius` is `[2.0, 2.0, 0.0]`, so its box
**is** its 4x4 footprint.
**Next: trace the planned polyline leg by leg, then delete the slide.** D264 found why the good planner was dead here - it was
given a goal *inside* an obstruction (the node's own tile) and returned `None` - and **D265 refutes D264's own explanation**: the settle
writes positions directly, so the step rule never sees it, and the mover refuses its own route by **164 raw** at a fixed point.
Given `legal_standoff` the planner answers and the animals family is **11 of 11**; delete the slide and it is **3 red** again. The
polyline's leg-by-leg trace names the stage. **Measured**: the endpoint projection (D262) took the family **3 red -> 1**.

**Waiting on the owner - the only open question.** The queue's floor is **entities >= 155**. The control reads **154** and never
resolves; the D250 build reads **137** and **wins**. Either the floor is about a *contest* and needs its own fixture, or the win is
the better outcome and the floor should be restated. Nothing else in the queue is blocked on a decision.

**Second question, and it is new (2026-11-17, R1a measured on `wip/r1a-at-rest`).** Deleting the step-through arm no longer
deadlocks - a one-tile corridor pair and the head-on pair both pass, so the *past* chain D242 blamed needs no code - but it
costs the match: **40 s -> 154 s**, entities **138 -> 159**, re-paths **761 -> 423 (body 445->251)**, and the **enemy survives**
(`players=1` -> 2) where the control eliminates him. So: take the pass-through out and pay 3.9x the wall time, or keep the arm
and keep the ~0.09-tile walk-through it hides. The numbers, and the two fixtures that had to be restated to what survives D241,
are in `docs/reviews/2026-11-17-r1a-at-rest.md`. Also named there: **one property is now unpinned** - "a man who is not past is
never crossed" cannot be observed from outside the world without a route accessor, which is why the slice is parked.

**Third question - answered by the owner and landed (2026-11-17, D251/D252).** May a villager ever change speed? The owner:
*"villagers never slow down ... so either they walk, or stand"*, verified first and then ruled: **fixture and fix in one slice**.

- **The game** (`docs/movement-in-the-binary.md`): `getMaxVelFwd` (`008a2a30`) returns the unit's own dat speed, multiplied only
  by a motion-object factor (`[move+0x14]+0x6c`) when that is `> 0`, and **exactly 0.0 on a pause**. No terrain factor, no ramp,
  no partial arm; the factor's writer is unread and only a formation needs one. **The decomp cannot help** - it has no
  speed/velocity code at all (`docs/prior-art.md`).
- **The defect**: 15 of 20,000 villager body-ticks moved 0.11-0.95 of a step, all decided `Take`, fourteen of them at a **route
  transit** - the mover landed exactly on a corner and spent the rest of that tick's step on the gap.
- **It is fixed and live**: `crates/sim/tests/whole_steps.rs`, 2 tests, **neither ignored** - every walking tick covers one whole
  step *along the route* (or stands), the last tick lands on the goal, and a walk costs the ticks its geometry owes. Red before the
  fix at **tick 158, 86 raw (0.107 of a step), 14 of 1,174 walking ticks**, green after.
- **The fix is `World::fold_the_step`**, and it **only folds `Move` and `Hunt`**. That narrowing is measured, not taste: folding a
  gather's approach made the acceptance match **bit-identical** (fold off 39.5 / 40.3 s, fold on 40.0 / 39.7 s,
  `entities=138 players=1 hash=01a0fb771ee2e75b repaths=761`) and starved a 400,000-tick fixture - one gatherer alternating every
  tick between **(394394, 313527)** and **(395101, 313901)** from tick 38,000 to 400,000, the mover walking him 800 toward his
  route on odd ticks and `settle_workers` walking him 800 back on even ones, so the bank stuck at **200 of 400** with **1 repath**.
- **The attempt before it**: spending corners *before* the step cuts them - a walker wedged for **68 ticks** at **1 raw** a tick.
- **Standing rule from the owner**: a fixture he asked for is **never `#[ignore]`d** without his explicit word. When the fix will
  not land, find out why.
- **The adversarial review** (`docs/reviews/2026-11-17-whole-steps-adversarial.md`) prices it: **no measurable CPU cost** (a
  120-villager, 6,000-tick march: 4,301 ms off against 4,235 ms on, medians of three), but **deterministic crowd churn** - the same
  men spend **340 more body-ticks walking (+0.6%)** and take **10 more repaths (61 -> 71)** because a whole-step tick ends them up
  to 0.04 tiles further; all arrive. And **116 of 116 folds in the suite are taken under `Take`**, so the "slide-fold" hole is not
  observed anywhere.
- **Following the game on the slide (D254): attempted, and the wall is elsewhere.** The slide was deleted with the owner's
  corner fixture written first - red at **567 of 1,398** walking ticks, **82, 81, 80... raw** a tick, then green - and the suite
  went red: **14 tests across 8 binaries** (192 of 200 green), animals and gathering first, then fights, a line upgrade and a
  whole scenario. Traced: a gatherer onto a sheep stops at **(316980, 316980)** with
  `WaitGround` and `stalled` at **267**; his aim is refused by `step_is_walkable` (landing `next < 0`, body clear) although the
  pathfinder called it reachable. Raising the ground budget from **2 to 40** changed nothing - the re-plan returns the same route.
  So **we have two measures of "walkable" and the game has one**, and the slide papers over the disagreement. Unifying them
  touches **D176** (the disc rule the owner asked for) and needs its own fixtures on both sides.
- **The gap D253 found, attempted, and handed back (D253)**: with a body half a tile past a corner the walk breaks the rule **567
  times in 1,398 ticks** - the man creeps at **82, 81, 80... raw units** a tick - and the cause is the **ground slide**, not the
  fold: `StepDecision::Slide` takes the step's axis component, a fraction of a step. Both fixes are refuted by measurement:
  a full-length slide regresses **five** fixtures (walks further than planned), standing instead of sliding regresses **twelve**
  (animals, gathering, combat - a man who cannot slide wedges, which is D188's finding). **The question for the owner**: does a
  slide count as walking? The original has no slide at all - speed or 0.0, and a setter that clamps against the border alone - so
  the slide is ours. Review: `docs/reviews/2026-11-17-the-slide-and-the-rule.md`.

## The queue, in the order a player would see it

Each slice ships red-first, with its measurement in `docs/reviews/`, and **the gate is the acceptance match at 48,000 ticks**:
entities at or above the floor the owner restates, `repaths` near **380**, plus the slice's own fixture. A hash moves when
behaviour moves.

1. **R1a - the past-rule chain, then the step through a body at rest** (rows 29-30; D242). Deleting `TakeAtRest` fixes the
   encounter (**8,000 raw** in an 8,000 reach against 7,200) but **deadlocks two soldiers in a lane** - the head-on pair stands
   **2,002 ticks** - because the game's chain (paused man stops -> the other's re-plan is stamped later -> the stopped man is
   **past**) does not complete here. **R1b** (the re-plan's obstacle set) is worse: entities **155 -> 143**, `repaths`
   **497 -> 649**, because `route.avoiding` was our only memory of men already met; the game has a second one (its search
   re-inserts standing objects, `FUN_007f9d80`). Spec fixture: `replan.rs::a_stopped_friend_is_not_walked_through`, `#[ignore]`d.
2. **R2 - the pause is re-drawn, and a paused mate is not a mate** (rows 18, 20). `setPause` has no "already paused" guard; the
   mate exemption is re-tested on motion, not on the route's existence. *Fixture*: the head-on pair - the **first** pause inside
   0.25-1.50 s, the waypoint list unchanged across it, **both arrive**. Never "longest continuous stand"; that measures the end.
3. **R3 - the incumbents' squares** (rows 24-25; D241). `maybe_respread` retires, and the idle-body nudge with it. The newcomer
   half is already the owner's rule verbatim (`step.rs:2178-2212`: nearest like node within `gather_reach`, else wait). The
   fixture must assert **a seated man does not move**, not that nobody overlaps - under D241 ruling 1 a route planned through a
   stopped man *is* walked through.
4. **R4 - an unreachable waypoint is substituted** (row 26). `mConsecutiveSubstituteCount`: *"we have a count of %d, faking it"*;
   `finish_blocked_orders` retires. *Read first*: the counter's semantics beyond the log line - `FUN_008a4360`.
5. **R5 - passage is the body's** (row 21, supersedes D117). Grow the tile graph by the radius, as the step layer already does.
   `a_big_unit_walks_through_a_one_tile_gap` **inverts**. Trap: a clearance with any **margin** put every ring place inside a
   neighbour's disc and reddened **18** tests - the clearance is the pair's reach **exactly**.
6. **R6 - the caps go, priced** (row 22). Remove `MAX_GROUND_REPLANS = 2` and `MAX_REPATHS_PER_TICK = 4`; the **3.5 vs 0.3
   ms/tick** figure predates the sub-tile pencil, so re-measure. A regression is a finding for the owner, not a licence to restore.
7. **R7 - the port is not done, the read is** (rows 13, 27). Settled: **16** per cardinal step (`FUN_007ffc80`, octile table) and
   **25** as the heuristic **weight** (`FUN_008007f0`: `euclid*2500/100` == `euclid*0x19`), i.e. **weighted best-first at ~1.56** -
   my "not admissible" was my assumption, not the game's inconsistency. The frontier is **a heap as well as** a stamped cost array
   (`FUN_00801e90` sifts every push). Unread: `local_46c` (it counts **expansions**, and the break compares it with a **distance**)
   and the equal-`f` tie-break. A weighted search returns **longer routes everywhere** - one slice with its own measurement.
8. **R8 - the start re-seed, and the path vocabulary** (rows 33, 36). A mover beginning **inside** a body: offset the start by
   **2 x radius** on a fixed ring and re-seed (`FUN_007fb220`), "no low-level path" only when every direction is blocked. And the
   order vocabulary we lack: forced waypoint, teleport-to-waypoint, per-path hit (patrol exists, D47).
9. **R9 - a thing that cannot be reached** (row 32; D241 ruling 3). Accept the order, walk as close as the ground allows and
   **idle** behind a wall; when the obstruction is other resource, work the nearest reachable node within `gather_reach`
   (`nearest_like_free` exists, the trigger does not). Villagers stay solid and **formations are deferred** (D241 ruling 2) - the
   same-tick mate stand-in for soldiers is labelled as such in `stamp_group`.
10. **R10 - parked on one fixture, not dropped** (D249). The closest-point answer and the game's cost model are built on
    `wip/follow-the-game`; conformance is **32 green, 1 red** and the red is a soldier boxed in by 18 mates at submit. Its
    prerequisites are rows 30 and 33 - i.e. **R1b and R8**. Its fork, still his to take: adopt the whole-region closest point and
    **retune `config/acceptance.toml`** (ours to retune; the game has no one-step answer), or keep the one-step answer and say so.
    Trap: `click_into_the_woods.rs` passes only because it spawns one tile from the wood - move the walker out when rewriting it.

**Still an artefact, not work:** row 23, the exclusive goal - **forty units ordered to one tile**, one recording. No read decides
it and no decision should stand in for it.

## Carried, not dropped

- **T16 / T12 stage 3** - the per-player clock (`pturn`, nine accumulator words) and the two-tier unit AI; stage 3 priced, not
  worth it yet (D229-D232).
- **P1b, the save converter** (D235): sections 2-4, a `tools/` converter, `--load`, a version stamp. P1a landed.
- **The effect interpreter's remaining kinds** (T2): 0 / 101 / 103.
- **D225's two probes**: whether a blind placement over an enemy unit stalls the builder, and the search cap (LOS + 5 tiles, 30
  reach tests) that matters only if the stall is real.
- **R6's second half** (D234): index effects by attribute once instead of cloning modifiers per player - the seam costs **+2.6%**
  until it lands.
- **Civ and team bonuses** deferred by ruling (D219); design in `docs/civ-bonuses.md`.

## Hazards a fresh session must not re-learn the hard way

- **A missing pack passes every world test silently.** `config/gamedata/` is gitignored; copy the four `.toml` files in, never symlink.
- **One `CARGO_TARGET_DIR` per checkout** - two worktrees sharing one swaps binaries under a running suite.
- **A doc that promises a mechanism is not the mechanism.** "Eight shoulder points round every body" lived in a review for a day
  and did not exist. When this file claims one, `grep` for it.
- **A test whose route is planned *around* the body it means to collide with measures nothing** - four fixtures move a body *after*
  the route exists for exactly this reason. "The scene never met the body" is a stale fixture, not a broken rule.
- **A number that is right does not mean the mechanism is.** Four times in the D214-D218 run the fix was the *second* thing.
- **This file is a step, not a log.** It is capped at 16 KB (`python3 tools/next_step.py --check`); a landed block moves to
  `docs/history/` the moment the next one starts.
