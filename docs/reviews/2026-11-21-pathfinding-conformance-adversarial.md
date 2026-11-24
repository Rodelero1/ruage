# Pathfinding and obstacle avoidance, adversarially: what the game does, what we do, and where our own notes are wrong

*Asked for: an adversarial review of pathing and obstacle avoidance conformance against the original game, the decompilation
and the first-party sources. Read this session: `docs/parity-plan.md`, `docs/pathfinding-*.md`,
`docs/movement-in-the-binary.md`, `docs/decisions.md` (D275, D279, D296-D333), the code in `crates/sim/src/{ground,levels}.rs`
and `crates/sim/src/world/motion.rs`, the HD corpus at `/opt/data/tools/rea/pseudo-aokhd`, and the AoC decompilation's
`src/zone/`. Everything below was checked against the symbol or the line, not against a decision row.*

**The short answer.** The port is closer than the register says and further than the notes claim, in different places. Three
findings change behaviour or invalidate a documented reason:

1. **The 25 is the expansion budget, not the heuristic's weight.** The binary's frontier is ordered by `g + octile` (weight 1);
   ours is ordered by `g + 25 x euclid` (weight ~1.56). Six files assert the opposite, from one line read out of context.
2. **The levelled search never runs in a match**, and the gate that stops it is ours: all **121** long walks in the acceptance
   match carry a stander, so the coarse search is never asked. Lift the gate and only **6** answer, and the acceptance hash does
   not move - the tier is inert in both directions.
3. **Body collision is a disc where the game's is a box.** Measured: **4,741 of 218,920 body-steps (2.17%)** end in a state whose
   two boxes overlap on both axes while the discs are clear - a collision the original counts and we take.

The 20,000-expansion cap has no witness at all (642 expansions into a 20,000 assertion, and `EXPANSION_CAP = u32::MAX` leaves
the row green), and one conformance row pins a rule no production code implements.

## The findings

| # | finding | severity | evidence |
|---|---|---|---|
| 1 | the heuristic weight is **1** (octile) in the binary, **~1.56** (25 x euclid) here | **high** | `FUN_008007f0` 177, 312-316 |
| 2 | the levelled tier is **gated off for every long walk in a match** | **high** | probe, 121/121 avoided |
| 3 | the 20,000 cap is asserted by a scene that cannot reach it | medium | 642 of 20,000; cap->MAX green |
| 4 | body-body collision is a **disc**; the game's is a **box** | medium | `FUN_00840830`; 2.17% of steps |
| 5 | `contact.rs` pins `touches_object`, which no production caller uses | medium | `grep -rn touches_object crates/` |
| 6 | the register is stale again: P4 and P7 read LIVE, both landed | low | `git show 844d3e7 0ca81ba` |
| 7 | the 2026-11-20 crosscheck's rows #5, #6, #7 are refuted by later reads | low | D322, D328, D306 |
| 8 | nine small divergences are ours and in no register | low | see below |
| 9 | `docs/pathfinding-conformance.md` carries stale counts and contradicts P12 | low | 21/0/0 measured |

### 1. The 25 is the budget. The heuristic is the octile, weight 1.

`FUN_008007f0` computes `iVar20 = (int)euclid * 2500 / 100` (or `* 0x19` on the other arm - both 25) at lines 186-191, and
`iVar20` has **one reader**: line 198, `if ((DAT_00aad420 != -1 && iVar20 < local_46c) || (20000 < local_46c)) break;`, against
`local_46c`, the expansion counter incremented at 542. That is the budget, and `docs/movement-in-the-binary.md` calls it one
("the budget is the estimate") everywhere it discusses termination.

The **frontier's key** is somewhere else. Line 177 stores `FUN_007ffd10(start, goal)` - the octile - as the start's value at
`0x38414`, and line 316 computes a neighbour's value as

```c
iVar17 = (stored_current - octile(current, goal)) + octile(neighbour, goal) + step(current, neighbour);
```

which is `g + octile`, because `stored_current` *is* `g + octile`. A worked example on (0,0) -> (10,0): `stored[start] = 160`;
the neighbour (1,0) gets `160 - 160 + 144 + 16 = 160 = g + h`. Under a 25 x euclid heuristic the same node would be `16 + 225 =
241`.

So the original's flat search is an **octile A\*** (weight 1), and the sentence in `docs/movement-in-the-binary.md` - *"16 is the
step cost and 25 is the heuristic weight ... weighted A\* / best-first with a weight of about 1.56"* - is a misreading of `iVar20`,
propagated into D275, `pathfinding-gap-register.md` row *cost*, `crates/conformance/tests/cost_model.rs` and the D275 review.
`ground.rs` implements the misreading: `open.push(Reverse((step + estimate((nx, ny)), ...)))`, where `estimate` is `25 x euclid`.

**Measured, one variable, same tree and data, seed 7, 48,000 ticks, release, idle box** (the A/B changed only the two pushes;
the budget still uses `estimate`):

| | `g + 25 x euclid` (now) | `g + octile` (the binary) |
|---|---|---|
| searches | 5,989 | 45,088 |
| tile expansions | 172,818 | 530,962 |
| mean expansions/search | **28.9** | **11.8** |
| searches that **exhausted** the budget | **3,235 (54%)** | 2,594 (5.8%) |
| re-paths | 1,727 | 41,557 |
| acceptance hash | `e248486298910e28` | `9d5fea513f3f8efb` |
| entities / players at 48,000 | 127 / 1 | 133 / 2 |
| wall, median of 5 | 10.04 s | 9.92 s |

Two things follow, and the second is the surprise. The greedy key is **more** expensive per search than the admissible one (28.9
against 11.8 expansions) and burns its whole budget on **more than half** of all searches; "weighted A\* is faster" is false for
this cost table, because a budget sized at `25 x distance` is generous enough that a greedy key spends all of it before settling.
And the match is a different match: with the game's key both players are still alive at the end of the tick budget, where ours
loses at 2,348.6 s.

**Not landed.** This changes every route in the engine; it owes a witness (a route the two keys answer differently, which is cheap
to build now that the number is known) and a suite run before it can be called a fix. What is refuted today is the *reason* the
greedy key is in the tree.

### 2. The levelled search never runs in a match - and the gate is ours

`route_with` takes tier 2 only when `avoid.is_empty() && long_walk && coarse.level >= 2` (`ground.rs:596-600`). Instrumented for
one 48,000-tick acceptance match (env-gated prints, removed before this commit):

- **121 long walks** (Manhattan >= 16) reached the gate.
- **121 of 121** carried a non-empty `avoid` list (1 to 36 standers). `coarse_route` was **not called once**.
- With the gate lifted, the same run: **6** coarse searches, **6** answers (all `level=2`), **0** rejected legs; the other 115
  are refused by `descent(start) < 2` (78 at level 0, 12 at level 1) or by `coarse_route` returning `None`.
- **The acceptance hash was bit-identical with the gate lifted** (`e248486298910e28`): those six hierarchy answers changed
  nothing.

So the tier is inert on this match with the gate and inert without it, and the conformance row that carries it
(`divergences::the_original_solves_a_long_walk_in_its_hierarchy`) calls `Levels` directly, which is why a green set cannot see
any of this. D323's account ("the descent is the cause") describes the 1,677 long *calls*, most of which tier 1 answers before
the gate; on the 121 calls that reach the gate the first refusal is `avoid.is_empty()`, and the descent is never consulted.

Both of the gate's other clauses are ours and in no register: the **16-tile floor** (D306) and **"no standers in the plan"**.
The binary's land-unit default *is* the levelled search (`FUN_008000d0` -> `FUN_007ee980`), and it has neither.

### 3. The 20,000 cap has no witness

`conformance::budget::the_budget_never_exceeds_twenty_thousand` builds a 480 x 480 empty map and walks (1,1) -> (478,478). That
scene's own estimate is `25 x 674 = 16,850`, below the cap, and the search spends **642** expansions (measured by forcing the
assertion to fail). The row therefore asserts `642 <= 20,000` and cannot see the cap at all:

- `EXPANSION_CAP = u32::MAX` -> **all four budget rows green**.
- The cap binds only when `25 x euclid > 20,000`, i.e. a diagonal walk past ~800 tiles, i.e. a map about 1,140 tiles a side.
  Nothing in the tree builds one.

The row's arithmetic also duplicates the crate's own: the test recomputes `estimate().min(20_000)`, so a wrong **cap** is
invisible to it and only a wrong **estimate** could fail it.

### 4. The body rule is a disc where the game's is a box

`motion.rs:505` refuses a landing when `dx*dx + dy*dy < reach*reach` - centres within the summed radii. `FUN_00840830` is
four `<=` on two **boxes**, each grown by the mover's radius (and optionally inset by `+0x268`, P3). A disc is inside its own box,
so ours can never refuse something the box takes; the interest is the other way, and it is measurable.

New probe, `collision_probe::what_a_box_would_refuse_that_a_disc_takes`, acceptance match, 36,000 ticks:

```
218,920 body-steps ended somewhere new
  4,741 of them (2.1656%) end with both boxes overlapping on both axes and the discs clear
  closest pocket: 7,974 raw against an 8,000 raw reach (26 raw short of a straight overlap)
end-of-tick disc overlaps by the exemption still standing: mates 0, past 15,597, neither 3,568
```

An end-of-tick audit cannot attribute a step exactly (a pair's exemption can lapse inside the tick when the other man moves after
the mover), so the 3,568 is an upper bound and the probe says so; the 4,741 is a state count and is not affected by that. What the
number means: about one step in fifty ends in a pose the original's query would have refused, and the game's answer to that
refusal is the repath plus the pause, not the step.

### 5. A conformance row for a rule the engine contradicts

`contact.rs` pins `Ground::touches_object` with *"the engine must say so"* about a body exactly at the grown face. `grep -rn
touches_object crates/` returns only those two rows - **no production caller**. The step uses `object_depth > 0`
(`motion.rs:486-490`), i.e. touching is allowed, which is D306's ruling from the owner's trebuchet measurement. So the set is
green on a rule the engine does not follow, and P3's open question ("our `object_depth > 0` is the same rule from the other
side") is not tested by anything: the row tests the query, not the step.

The same file's neighbour, `obstacles::the_search_has_no_body_layer`, states *"the planner has no body layer - a `Ground` cannot
hold one"*. That was D279. Since D282/D331 the planner carries every stander as a `Clear` rectangle, so the sentence describes
a `Ground` API and not the planner.

### 6. The register is stale again, in both directions

`docs/parity-plan.md` as of `2a00824`:

- **P4** reads *"LIVE, unblocked"* - landed by D332 (`844d3e7`), whose witness `immediate_replan::a_wall_in_the_lane_is_replanned_the_tick_it_is_seen` is in the tree.
- **P7** reads *"LIVE"* - landed by D331 (`0ca81ba`), witness `planning_round_bodies::a_body_the_mover_overlaps_is_still_planned_round` in the tree.
- The *"Status, verified against the code on 2026-11-21 (D330)"* block still lists P4, P6, P7 and P9 as live; D333 closed P6 and P9 in the same session.

D330's own message calls this "the third time this register pointed at covered ground". It is now the fourth and fifth, and the
cause is the same each time: a row's status is written in the commit that *plans* it and not in the commit that *lands* it.

### 7. The crosscheck's rows have been overtaken

`docs/pathfinding-crosscheck-2026-11-20.md` still lists as live differences, one day later:

- **#7 "the radius lives in the step"** - fixed by D306: `fits`, `pyramid_for(reach)` and `leg_is_clear(..., reach, ...)` all carry
  the body's radius into the plan. The row's *direction of the error* ("every route in this engine is a route a point could
  walk") is no longer true of the planner.
- **#6 "bodies are priced at 2000"** - refuted by D328: `[0x4d]` is the eight-direction run-mask grid built by `FUN_007ef3f0`,
  and the only body-aware test (`FUN_00801a20`) is in the flat search. There is no 2000-per-body row to adopt.
- **#5 "cell centres"** - fixed by D322 (boundary crossings).

### 8. Nine small divergences that are ours and in no register

Each is a line in the tree with a source line beside it; none changes the match today, and each is one line in a register:

1. **The coarse 300 row.** Ours: "any orthogonal neighbour is blocked" (`levels.rs:366-376`). The binary's 300 is an object of
   **type 8 or 0x20** on the cell or beside it (`FUN_007ed280` via `FUN_007f11f0`, which tests exactly those two type words).
   The predicates differ both ways: next to a cliff or water - blocked, no object - ours charges 300 and the binary 10 + the
   distance term; next to an object of another type our cell is blocked anyway (`walkable` counts every occupant), so the
   second difference is not reachable in this content.
2. **The descent takes the goal too.** `descent(from).min(descent(to))`; the binary tests the start only (`FUN_007ee980:54-59`).
   Recorded (D323) as different in 0 of 1,677 long calls, not in the register.
3. **The flat search's tie-break.** Ours is `(f, g, y, x)`; the binary's sift moves an equal-`f` child up (`FUN_00801e90`), i.e.
   newest first. Deterministic both ways, different expansion order.
4. **The flat search's goal test.** Ours stops on the goal **tile**; the binary accepts a node inside a box around the target
   or within the order's range `param_6` (`FUN_008007f0:201-233`), so "close enough" is the search's answer there and the task
   layer's answer here.
5. **The budget's comparison.** Ours breaks at `expanded >= budget`; the binary at `budget < expansions`, one expansion later.
6. **The octile table's cap.** `FUN_007ffd10` caps the minor axis at 479 (`FUN_007ffc80` masks to 255); our `octile` has no cap -
   different only past 479 diagonal steps, so not on a 64 x 64 map.
7. **The mover's radius by position.** `reach_at` (lib.rs:1969-1976) finds the body whose `position` equals the plan's start, not
   the mover id it was handed. In the state D289/D290 make reachable - two bodies on one point - the planner can take the other
   body's radius.
8. **A waiting walker is invisible.** `standing_bodies` skips every body with a route and no pause; the binary's test is velocity
   zero, so a walker blocked and waiting is an obstacle to the next plan.
9. **The pause's loser.** Ours compares `EntityId`; the binary's `[vtbl+0x2b4]` returns the object handle, whose order is not
   established (`docs/movement-in-the-binary.md`).

### 9. The conformance document's own numbers

`docs/pathfinding-conformance.md:5` says *"13 tests green and 3 `#[ignore]`d"*; measured today the set is **21 passed, 0 failed,
0 ignored** over seven binaries. Its rule table still lists rows 26-29 red, which they are not. And its P12 section ("the mask is
derived, nothing needs to be ported") contradicts `parity-plan.md` P12 ("UNREAD ... the first move is a read"). The decomp's C
body supports the document; the register row is the stale half - but the register is the file that decides what is left.

## Performance

All figures: release build, seed 7, `config/acceptance.toml`, 48,000 ticks, this box (2 vCPU, 4 GiB), idle unless said.

- **The acceptance match, unmodified:** `hash=e248486298910e28 entities=127 players=1 repaths=1727`, **10.04 s median** of five
  (10.01-10.13). This matches D332's 10.26 s and D331's 10.2 s. **My own error, reported:** a first five-run batch measured
  16.11 s median and I nearly attributed the difference to the engine; the batch had been started while `cargo test --workspace`
  was still running on the same two cores. Every number here was re-taken on an idle box.
- **Per-search cost, the A/B above:** 28.9 expansions/search and **54% of searches exhausting the budget** with `g + 25 x euclid`,
  against 11.8 and 5.8% with `g + octile`. This is the one comparison the two runs share: the matches themselves differ
  (`players=1` against `players=2`), so the wall times (10.04 s against 9.92 s) are **not** a like-for-like cost and are quoted
  only to show that the honest key is not slower.
- **The tier-2 accounting:** 121 long walks, 0 coarse searches with the gate, 6 with it lifted, 0.0 s difference in the hash.
- **The collision probe:** 4,741 of 218,920 body-steps (2.17%) in a box-overlap-only pose; 36,000 ticks in 9 s.
- **Allocation per search**, named and not measured as a cost: `search_reporting` allocates `best: Vec<u32>` and
  `from: Vec<Option<usize>>` per call - 49 KB on a 64 x 64 map, i.e. ~290 MB of allocation and clear per match at 5,989 searches
  and ~2.2 GB at 45,088. The binary's generation-stamped per-tile array exists precisely to avoid this; it is a difference to
  price if plan cost ever becomes the bottleneck.

## Stability

- **Determinism:** three consecutive runs of the unmodified tree gave the identical hash, entity count and repath count
  (`e248486298910e28`, 127, 1,727); five runs for timing did the same. The A/B variant was likewise bit-identical across five.
- **The suite:** `cargo test --workspace --no-fail-fast` on `2a00824`, **933 passed, 0 failed, 19 ignored** over 213 `test result`
  lines (binaries plus their doc-test runs)
  (before the probe row was added; the probe is `#[ignore]`d, so the tree now reads 933 / 0 / 20).
- **The conformance set:** **21 passed, 0 failed, 0 ignored**, seven binaries, `--no-fail-fast`.
- **Attacked, and what held:** the expansion cap (mutation to `u32::MAX` - the rows stayed green, which is the finding, not a
  pass), the tier-2 gate (lifted - the hash did not move), the heuristic (swapped - the match changed shape, so the suite would
  have to be run before that lands, and was not), the body rule (probed, not changed), and the register (every row checked by
  symbol: `finish_blocked_orders`, `maybe_respread`, `push_the_standing`, `TakeAtRest`, `reserve_gather_place`,
  `crowd_ring_point` appear only in comments or not at all, which is what P5, P8 and P9 claim).
- **What was not run:** the full suite on the octile-heuristic variant, because the variant is not a landing candidate until it
  has a witness; and `make ai-review` on this docs+probe diff (the OCR pass reads code, and no production line changed).

## What is still unknown, said plainly

- **Whether the binary short-circuits a straight line before the search** (D309's open question) - unread, and our tier 1 stays
  unverified. Nothing in this review moves it.
- **The `[0x42..0x45]` search window's padding** (D299) - unread, and our superset behaviour is a recorded deviation.
- **The 100/600 terrain-bit rows** (D327) - blocked by the build mismatch D328 found; unreachable for a land unit anyway.
- **The octile table's exact values** - `DAT_00afd2a0` is runtime storage; our 23-per-diagonal is an inference that matches
  `16 x sqrt2` to 1.6%, and no source in hand can confirm it. It is not a finding, it is a gap.
- **Whether the diagonal inset (`+0x266`/`+0x268`) is a body-radius inset or something else** (P3) - still one number short.

## What this review corrected in the tree

Documentation only, plus one probe; no engine line changed:

- `docs/movement-in-the-binary.md` - the "cost model settled" paragraph now states what the two numbers are for, quotes
  `FUN_008007f0:312-316`, and keeps the retraction visible.
- `docs/decisions.md` - **D334** records the review, the three headline findings and the measurements.
- `docs/parity-plan.md` - P4 and P7 marked landed with their witnesses (D332, D331); new live rows **P13** (the heuristic), **P14**
  (the tier-2 gates), **P15** (the cap's missing witness), **P16** (the disc-vs-box step rule); the P12/P-conformance contradiction
  named.
- `docs/pathfinding-conformance.md` - the counts corrected to 21/0/0 and the red rows closed.
- `docs/pathfinding-crosscheck-2026-11-20.md` - rows #5, #6, #7 marked superseded, with the decisions that closed them.
- `crates/sim/src/ground.rs`, `crates/sim/src/world/motion.rs`, `crates/sim/src/lib.rs` - comments that described a port the code
  has moved past: "no hierarchy, no radius, no avoid set, no expansion budget" (three of the four now exist), "the re-plan is
  throttled" (P4/D332), "`finish_blocked_orders`" (deleted in D292), "no radius argument, no second solver, no avoid set" on
  `plan_path`.
- `crates/conformance/tests/{cost_model,contact,obstacles}.rs` - the weight sentence, the `touches_object` note (no production
  caller) and the "no body layer" claim, each corrected in place with the finding number.
- `crates/sim/tests/collision_probe.rs` - the box-vs-disc probe, kept so the 2.17% can be re-measured rather than re-argued.
