# Parity plan: no deliberate difference from the original (owner, 2026-11-19)

> *"I want no deliberate differences to original game. If there is any, I want that adjusted to meet the original or cut off.
> Please plan the sequence of implementation steps to reach parity with original game and decomp."*

> **Reaffirmed 2026-11-21**: *"Please remove all deviations. I want this to be blunt copy."* The answer for every row below is the
> **original's behaviour**, including the rows that were marked FORK because they encoded the owner's own earlier rulings - he has now
> ruled the other way (D330). Where a row cannot be reached from this box, that is said in the row rather than in a note.
>
> **The order the rows are taken in, with each step's witness and size, is `docs/conformance-sequence.md`** (written 2026-11-21 at
> the owner's ask, crosschecked against the first-party talk and openage). This file is *what* differs; that one is *in which order*,
> and it also carries what "100%" cannot mean (bit-exact routes).

This file is the register of every place this engine's pathing and obstacle avoidance is **ours** rather than the game's, what the
original does there (with its address or module), and which of the two answers the owner's rule takes: **adopt the original**, or
**cut ours**. Each row is a step with a proof - a conformance row or a fixture watched failing - and a measurement.

Order is by dependency: the search's own rules first (they bound everything after), then the collision query, then our
inventions on top of it. The two rows marked **FORK** cannot be settled by me: they are places where the owner's own ruling
ordered behaviour the game does not have, so "meet the original" and "the rule I gave you" disagree.

## The register

| # | status | ours today | the original | step |
|---|---|---|---|---|
| P1 | **DONE** | a search with a budget: `h` expansions, hard cap 20,000, then the best settled node answers | same | landed; conformance row green |
| P2 | **DONE** | the nearest-cell answer picks by octile distance, first node found wins ties | same | landed; conformance row green |
| P3 | **LIVE - blocked on one read** | `object_depth > 0`: a landing that **penetrates** a new object is refused, and one that touches an already-touched object may stay or slide out but not go deeper | `FUN_00840830`: an **inclusive AABB** (`min <= other_max && other_min <= max`, so touching counts) applied to the mover's box - **optionally shrunk on all four sides** by `+0x268` when `+0x266` is set. Read 2026-11-21 | land the inclusive test **with its inset**: without the inset a half-tile body is refused every one-tile gap, contradicting D306 (the owner measured a packed trebuchet through one). **The inset's formula is not in hand**, and its writer is not identified in the corpus. **The set cannot see this row**: `conformance/contact.rs` pins `Ground::touches_object`, which has no production caller (D334) |
| P4 | **DONE** | re-plans on **every** ground refusal (D332) | re-plans on **every** terrain collision (`0x122f`, `0x128b`) | landed `844d3e7`, witness `immediate_replan::a_wall_in_the_lane_is_replanned_the_tick_it_is_seen`; measured 1,918 -> 1,727 re-paths for -10% |
| P5 | **DONE** | no order is abandoned for being blocked (`finish_blocked_orders` and `beside` are both gone) | the original has no such rule; the pause is the only brake | landed (D292), with the seat book (P8) and the no-coincidence guard behind it |
| P6 | **NOMINAL (measured)** | `Ground` holds `occupants: Vec<u16>` - a **count** per tile - so a blocked tile is one whole tile wide | per-object **radius pairs**, and a radius query over the tile's objects (`FUN_00844310`, `FUN_00840830`) | **every object our content carries is a whole tile wide**: all four resource kinds have `radius = 0.5` (a tile's half-extent) and buildings take the dat's radius, which is 1.0, 1.5 or 2.0. A tile-granular index *is* the object's box for this content, so the deviation is nominal; it becomes live the day an object narrower than its tile exists |
| P7 | **DONE** | the planner includes every body the mover is not standing on, by id (D331) | the search starts on the shared tile and includes **every stander the mover does not ignore** (`FUN_007f9d80`) | landed `0ca81ba`, witness `planning_round_bodies::a_body_the_mover_overlaps_is_still_planned_round`; measured 1,362/724/510 -> 1,949/960/130 at 2,400 s |
| P8 | **DONE** | no seat book: gatherers spread by collision and pause only | the original has no seat book | cut (D293/D294), fixtures re-pointed |
| P9 | **DONE** | nothing: `beside` (D200) was cut in D292 and the crowd ring point (D271) was withdrawn in D289 - `grep` finds neither rule in the tree | a group is sent to the **clicked point**; no "beside a body holding the goal" rule | already cut, at the owner's instruction, before this directive; the row was stale |
| P10 | **KEEP** | the pause is drawn from our seeded stream, 0.25-1.50 s | the game's `rand()`, whose own comment calls it *"not considered sync safe"* | same interval, same distribution; only the random *source* differs, and a reproducible one is what makes the acceptance hash possible |
| P11 | **HALF** | one flat A* for short/medium walks, and the levelled search for long routes | **three**: levelled, tile short-range, and a **polygonal** one for combat (dynamic convex hulls) | the levelled one landed (D296-D302, D322). The polygonal one is **not built** - there is no hull code in the tree - and it is a project, not a step. **The target is the legacy algorithm** (slides 21-38: AABBs, gift-wrapped hulls, cardinal rays, edge walk, 64-iteration cap, smoothing), not the DE-era fixed-point edge walk the talk presents as its replacement - HD predates that rewrite (crosscheck, `docs/conformance-sequence.md` §2) |
| P12 | **UNREAD** | a tile is walkable or not; no per-edge passage | a **four-bit N/S/W/E neighbour mask**: a step is legal per **edge** (`zone_spatial_pyramid_check_tile.cpp`, `func_001D29F0`) | no HD address pins the mask, so the first move is a **read**, not a patch. **`docs/pathfinding-conformance.md` reads the mask as a derived cache of `!blocked(neighbour)` and says nothing needs porting** - the two files disagree and one of them is stale (D334) |
| P13 | **LIVE** | the flat search's heap key is `g + 25 x euclid` (`ground.rs`) | the key is `g + octile`; the 25 is the **expansion budget** only (`FUN_008007f0`, lines 177, 198, 312-316) | measured A/B (D334): greedy key 28.9 expansions/search and 54% of searches exhausting the budget, octile key 11.8 and 5.8%; the match itself changes (`players=1` -> `players=2`). Needs a witness and a suite run before it lands |
| P14 | **LIVE** | tier 2 runs only on `avoid.is_empty() && manhattan >= 16 && level >= 2` | the land default **is** the levelled search (`FUN_008000d0` -> `FUN_007ee980`), and it **ignores moving objects**: `FUN_007ee980`'s body contains no stander insertion (the inserter `FUN_007f9d80` is reached from `FUN_007f96b0`/`FUN_007fbab0`, not from the levelled search), and openage's reading of the classic engine says the same ("it ignores moving objects, e.g. units or wildlife") | measured (D334): 121 long walks, **121 carrying a stander, 0 coarse searches**; lift the gate -> 6 answers, unchanged hash. **The gate is backwards**: the game runs the coarse search *and* ignores bodies; ours falls back to the flat search instead. The 16-tile floor is D306's and is ours too. Order and witnesses: `docs/conformance-sequence.md` steps 2-3 |
| P15 | **LIVE (no witness)** | `EXPANSION_CAP = 20_000`, and no fixture can reach it | `20000 < local_46c` at `0x800d57` | the row builds a 480x480 map whose own estimate is 16,850 and spends **642** expansions; `EXPANSION_CAP = u32::MAX` leaves it green (D334). Needs a scene past ~800 tiles of diagonal walk, or the row is decoration |
| P16 | **LIVE** | a body refuses a landing whose **centre** is inside the pair's reach - a **disc** (`motion.rs:505`) | four `<=` on two axis-aligned **boxes**, the query grown by the mover's radius (`FUN_00840830`) | measured (D334): 4,741 of 218,920 body-steps (2.17%) end with both boxes overlapping and the discs clear; the original counts those as collisions and answers with the repath and the pause |
| P17 | **MISSING** | a waypoint the mover cannot reach is returned by every re-plan and stands there | **substituted**: the log names `mConsecutiveSubstituteCount` - *"We won't reach our waypoint either this or the next update, but we have a count of %d, faking it"* - so the waypoint is skipped and the next is reached | `grep substitut crates/` finds only a comment; the gap register listed it and no live row carried it (D334). Witness and order: `docs/conformance-sequence.md` step 5 |
| P18 | **MISSING** | the order has one goal point; nothing else is carried | the order's **path vocabulary**: a forced waypoint, a teleport-to-waypoint, a per-path hit (`ADDmove`'s fields) | unread; the gap register listed it and no live row carried it (D334). A read first, then a fixture (`docs/conformance-sequence.md` step 9) |
| P19 | **MISSING** | the levelled search has no distance-from-the-edge term | open cells cost `10 + (coord - window_lo) + 10` in the direction of travel, and a border cell gets `cost_mod` 0/-100 (`FUN_007ed280`, `zone_spatial_pyramid_search_radius`) | parked on the window's padding, which is unread (D299) - but the term itself is read and is a row, not a note (`docs/conformance-sequence.md` §3) |

**Rows this box cannot finish, said here rather than at the bottom**: the **100/600 cost rows** (D327) need the terrain descriptor table, and
this install's `AoK HD.exe` is **a different build** from the decompiled corpus (the table's address is outside its image, and the corpus's
strings sit `0x8346` lower, non-constantly). **P12's mask** and **P11's polygon pathfinder** both need reads the corpus may not hold.

## Status, verified against the code on 2026-11-21 (D330, re-checked by D334)

Five rows had gone stale in **both directions** - marked open after the work landed, and marked ruled where the code still carried the old rule.
Each line below was checked by symbol, not from memory:

- **Done since the register was written**: P1, P2 (budget and octile tie-break), P5 (`finish_blocked_orders` and `beside` no longer exist),
  P8 (no seat book), and the levelled half of P11.
- **Done since D330's own check, and the register had not noticed**: **P4** (D332), **P7** (D331), and the two rows D333 closed by measurement
  (P6 nominal, P9 already cut). D330's bullet list below is kept as written and is now itself out of date - that is the fourth and fifth time a
  row's status survived the commit that landed it.
- **Live and unblocked**: P13, P14, P15, P16 - all four found by the D334 review.
- **Live and blocked on a read**: P3 (one float), P12 (the mask), and P11's third pathfinder.
- **Stale comment fixed at the same time**: `motion.rs` still said *"PENDING A RULING (parity-plan P3)"* after D306 had ruled; it now carries
  the ruling and the constraint the ruling puts on P3's implementation.

## What D330's status block said (kept as the record of how it went stale)

- **Done since the register was written**: P1, P2 (budget and octile tie-break), P5 (`finish_blocked_orders` and `beside` no longer exist),
  P8 (no seat book), and the levelled half of P11.
- **Live and unblocked**: P4 (its blocker was the seat book, cut by D293), P6 (`occupants: Vec<u16>` is still a per-tile count), P7 (the
  quarter-tile skip is still in `standing_bodies`), P9 (both fork rules are still in the engine).
- **Live and blocked on a read**: P12, and P11's third pathfinder.

## Work order (owner's directive, 2026-11-21) - what is left

1. **P13** - the flat search's key: `g + octile` instead of `g + 25 x euclid`. The number is measured; it owes a witness (a route the
   two keys answer differently) and a suite run. It moves every route, so it is a step, not a constant swap.
2. **P14** - the levelled tier's two gates. The 16-tile floor and the "no standers in the plan" clause are both ours; with them the
   hierarchy answered 0 of 121 long walks in a match, and without them 6, for an unchanged hash. Deciding *whether the game plans
   round standers at coarse level* is the open read here.
3. **P15** - a witness for the 20,000 cap, or the row is decoration: a scene whose own estimate exceeds the cap (a map about 1,140
   tiles a side, sealed) and an assertion on the number of expansions, not on a bound recomputed from the same function.
4. **P16** - the body rule as boxes. Four `<=` where we test one distance; 2.17% of body-steps are in the difference. It lands with
   the same arrival arithmetic (`reach_between`, `target_contact`) or the fixtures will not converge.
5. **P3** - needs one read (the inset behind `+0x266`/`+0x268`) before the inclusive test can land without breaking D306's measured
   one-tile passage. Until then our `>` test is the same rule from the other side.
6. **P12** - the four-bit per-edge mask: a read, then the mask - or `pathfinding-conformance.md`'s reading is right and the row closes.
7. **P11's polygon pathfinder** - a project, to be scoped separately; there is no hull code in the tree.
8. **P10** - kept deliberately: same interval, same distribution, reproducible source. It is the one row where a difference is a
   requirement rather than a choice.

Small alignments that ride with whichever step touches them: the **search window** (D299), the **refinement's interpolation** (D322), the
**descent's start-only test** (D323), and the **body list's order and 255 cap** (D329).

## History: rulings and blockers as measured 2026-11-19

*Kept for the record. Several are now superseded - see the status table above.*

- **P3's fork is ruled**: *"the game wins (a): adopt `<=`, a half-tile body stops at a one-tile gap, and D117 is withdrawn."*
  D117 no longer stands; `building_passage::a_big_unit_walks_through_a_one_tile_gap` must be re-pointed to the game's answer
  **when the rule lands**, not before (the engine cannot assert a rule it does not follow).
- **P1 is blocked on P11**: at `h = 25 x tiles` a flat search truncates a legitimate detour - measured, a one-tile walk answered
  with a node fifteen tiles east. The budget lands with the levelled search, or not at all.
- **P3 is blocked on the plan/step agreement about touching**: a route certified by `leg_is_clear` still asks for a landing
  that touches two blocked tiles, and the body stands in `WaitGround` for 3,000 ticks.
- **P4 is blocked on P8**: the goal fallback that made off-grid gather/attack points survivable is part of the book's furniture.
- **P5 is NOT free after all - it is the third step blocked on P8.** Cut alone (witness written first and watched failing:
  `crates/sim/tests/order_never_given_up.rs`, "the task is Idle after 120 ticks"), the order-finish turns out to be what breaks
  every crowd jam in the engine today: four fixtures went red - `a_mate_who_is_stopped_becomes_solid`,
  `the_loser_stands_for_the_drawn_pause`, `a_man_who_cannot_get_his_square_takes_the_next_tree`, and
  `collision::a_walker_never_ends_a_tick_inside_a_friend` (which presses one step deeper once nobody gives up). None of them is
  wrong: they describe a crowd whose only escape from a jam is that some men abandon their orders. With the order-finish gone,
  the collision pause has to do that work - which is exactly what the seat book's removal (P8) is supposed to install. So **P8 is
  the keystone**: P4, P5 and (after the plan/step agreement) P3 queue behind it, and P8 lands first.

## P8 - the seat book, cut: the design, with its surface named

**The owner's instruction:** *"cut the book entirely and let collisions and pauses do the spreading, re-pointing the
fixtures."* Reading the code before cutting shows the book is **narrower than the name suggests**, and one part of it is *not*
ours:

- **Kept - the order-time spot** (`World::standoff_point`, `crates/sim/src/lib.rs:3794`). It picks, once per order, the nearest
  free place touching the node's tiles (`tiles::touching_places`), refusing places another body holds, another order has taken,
  or that the man cannot reach. That is the game's rule as the prior art states it (`docs/prior-art.md:1220`: *"the planner walks
  the target's perimeter at a standoff, starts at the caller's own face, nearest usable spot"*), so it is not the thing to cut.
- **Cut - the registry that outlives the order.** `World::gather_place` (`lib.rs:1134`), `reserve_gather_place` (`lib.rs:3949`),
  `gather_place_of` (`lib.rs:3967`), `release_gather_place` (`sites.rs:208`), `place_is_live`, and the two places the step reads
  it (`step.rs:1844` `owned_place`, `step.rs:2080`). Nothing in the original keeps a man's seat: he stands where he stands, and
  the body is solid.
- **Cut - the re-spread.** `respread_at` (`lib.rs:967`), `maybe_respread` (`step.rs:933`), the `respreads` turn bucket
  (`step.rs:82, 2274ff`), and the refusal-counted re-lay at `step.rs:1939`. Re-laying a ring because a man was refused is the
  per-tick band correction D200/D202 introduced; the original has no equivalent, and it *moves working men*, which the owner's
  rule 4 forbids anyway.
- **Cut - the angular ring.** `ring_point` (`lib.rs:3904`), `GATHER_RING_MILLI` (`lib.rs:1363`), `GATHER_PLACES` (`lib.rs:1369`),
  and the ring sampling at `lib.rs:3770`. Nothing in the binary places men at two-degree granularity; `touching_places` (per-tile,
  at one diameter) is the generator that stays, and `crowd_ring_point` (`lib.rs:2923`) belongs to **moves** (his D271) and is
  untouched by this slice.
- **Kept for now, judged afterwards**: `settle_workers` (`step.rs:1069`) - it is the step's own landing, not the registry, and
  with the target cut (D286) a man already comes to rest at contact.

**The fixtures that will re-point, by name, and the claim each has to change:** `overlap::a_newcomer_does_not_move_the_working_gatherers`
(the newcomer no longer displaces anybody - that is the cut), `gather_ring`, `gather_spread`, `gather_standoff`,
`gather_standoff_rule`, `gather_touching`, `gather_contact` (each asserts a ring/place number that the cut removes; the claim
becomes "men stand touching the node, a body apart from each other"), plus the four jam fixtures named under P5 - they should
*stay* green once the order-finish is cut, and if they do not, P8 is not finished.

**Acceptance, in this order:** the four jam fixtures stay green with P5 applied; ten men work one bush with no two bodies
overlapping; a man who is working does not move; the acceptance match's time is within the +1.1 s measured for P4+P5.


## P8b - CORRECTED 2026-11-19: the taken list is *ours*, and the game re-targets (see `docs/gather-seating-evidence.md`)

**This section was wrong when it was written**, and the crosscheck the owner asked for overturned it. It said the taken list is the
game's mechanism, quoting a *neighbouring open-source project's* approach planner. The game's own players say otherwise:
*"If you send them to different trees within the same tile, they will usually stand still. This will also happen if you send 3
villagers to the same tree"* (Capoch's build-order guide), and a blocked gatherer is **re-targeted to the nearest usable resource**
(*"if a villager collides with an object near its target it will retarget to something else"*, Age of Empires forum 289465;
*"the vills go to nearest available tree if they can't path to an assigned one"*, r/aoe2). So:

- **our `standoff_point` taken list is a deliberate difference** - it fans men out so that nobody ever bumps, which the game has
  never done - and keeping it is a choice, not parity;
- **D204 stays**: re-targeting a blocked gatherer is the game's rule, not ours. `finish_blocked_orders` and `beside` still go,
  because the game's best-reported bug is wedged villagers that never resolve.

**Old text, kept so the error is visible:** what is left of the book is *the game's own design*

Reading the remainder of the book against the evidence, before cutting any of it:

- **The taken list is the game's.** `standoff_point` refuses a place that another body stands on, another *order* has taken
  (`gather_place`), or that cannot be reached - and that is exactly what the prior-art reading of the original says it does:
  *"the planner walks the target's perimeter at a standoff ... nearest usable spot"*, and *"they decide a spot by a deterministic
  scan of a target's perimeter with a taken list, at the moment the order is given"* (`docs/prior-art.md:1220`, `:1256`). Cutting
  the registry would therefore be a **new** difference, not parity: the original carries "who else is coming" in the request.
- **The angular ring is legacy, not the seating rule.** `ring_point` / `GATHER_PLACES` / `GATHER_RING_MILLI = 1156` survive in
  **one** place, `ring_open` (`lib.rs:3763`), a 36-sample walkability test that gates the settle. Seating comes from
  `tiles::touching_places` (D192's 0.70 rule). The ring's 1.156 circle lands in the same eight neighbours the touching places do,
  so `ring_open` and "is any touching place walkable" are near-equivalent - removing it is a cleanup with no measured behaviour
  change, and it is correctly *last*.

**So P4 and P5 cannot be unblocked by finishing P8**: their blocker was never the book's *existence* but the two jobs the book
does for the engine today - the off-grid goal point each order carries, and the ability of a jam to clear by men abandoning
orders. Cutting the taken list would not remove either.


## P5 - measured once more, and the blocker is not the one we thought (2026-11-19)

The owner chose the original's answer: *"no engine help: the order stays live and the man stands; only sliding and the pause
rule act."* Cutting `finish_blocked_orders` and measuring it turned up a **second** rule of ours that ends a walk, and the
fixture cannot separate them:

- a walk whose route is **exhausted** while still short of its goal is ended by the task layer (`Idle`) - the route-done arm,
  not `finish_blocked_orders`. Two scenes were built to isolate the forty-tick rule and both ended `Idle` **with the cut in
  place**: (a) a friend standing on the goal tile, (b) a friend standing in the only one-tile gap between two forest blocks with
  the goal beyond it - in (b) the search answers *the closest reachable tile* (`conformance::obstacles::a_walled_in_goal_is_answered_with_the_closest_tile`),
  so the route is short, it is consumed, and the order completes legitimately.
- so P5 is **two cuts in one slice**: the forty-tick order-finish *and* the route-done arm must both be replaced by the original's
  behaviour (a walk that cannot arrive stands, with its route intact, until the player re-tasks), and the witness has to be a
  scene where the goal is reachable *by the search* but **not by the body** - a lane exactly one body wide with the goal on the
  far side is not enough (the search refuses it); the scene needs a blocked-later-by-a-body case, i.e. the friend walks into the
  lane **after** the order is given, which is how D98a's fixtures already force an encounter (`collision::a_walker_never_ends_a_tick_inside_a_friend`
  places the friend once the walker is under way).

**A third rule, found by probe, and the witness's shape.** `Command::Move`'s acceptance already resolves a click onto a tile a
body holds to a **free neighbouring tile**, so a walk onto a held tile cannot be ordered; the engine sends the man to the tile
beside it and he arrives. Measured with both cuts in place (`ZZREACHED EntityId(9001) at (249620,209621) to (250000,210000)`).
So the jam case only arises when a body **moves into the goal after the order** - and a `Command::Spawn` aimed at the goal tile
is displaced by the placement rules, which is why two attempts at the fixture arrived instead of jamming. The witness must
therefore make a **body walk in** (`collision::a_walker_never_ends_a_tick_inside_a_friend` does exactly this, placing the stander
once the walker is under way), and the three cuts go in one commit: `beside`, `finish_blocked_orders`, the D204 re-task.
**A draft of the fixture is `docs/p5-witness-draft.rs`** (not compiled): two orders to one tile, the nearer man ordered a tick
later. It does **not** jam - the engine's own acceptance resolves both - and the `Idle` at tick ~144 is still unattributed, so
the next session's first move is to instrument `work.finished` (`step.rs`, the `Task::Move` arm) and `end_walk` (`motion.rs`)
with the tick and read which arm fired.

**The four fixtures the cut reddens, and what each has to become** (all measured with the cut in place): `a_mate_who_is_stopped_becomes_solid`
(overlap), `the_loser_stands_for_the_drawn_pause` (head_on), `a_man_who_cannot_get_his_square_takes_the_next_tree` (the D204 rule,
which is also ours and goes in the same slice), `gather_standoff::two_woodcutters_on_one_tree_both_touch_it_and_stand_a_body_apart`,
and `collision::a_walker_never_ends_a_tick_inside_a_friend` (whose monotone-separation claim held only *because* some walker gave
up: with the cut the pair presses **781 raw** deeper, against an **800 raw** step - one step is the whole allowance, and the claim
becomes "no deeper than the step that was already being taken").


### P5 measured again: three rules end a walk, and arrival is generous

The crowded-goal witness (eight men ordered to one tile, `docs/p5-witness-draft-crowd.rs`) **bites on the unmodified engine** -
`ZZREACHED EntityId(9103) tick 94` and `ZZFINISHED tick 94` - and it shows there are **three** ways a walk ends, not two:

1. **`beside`** (D200) - ours, cut.
2. **`finish_blocked_orders`** (D200) - ours, cut.
3. **`reaches(position, to, one step)`** - the arrival test, which is **generous**: a man *within one step* of the goal counts as
   arrived, so in a crowd several men satisfy it and the order completes legitimately. With 1 and 2 cut the fixture stayed red and
   the man it named had **reached** at tick 94 - while an earlier run had shown him `Idle` at 86, i.e. **something re-issues his
   order** after the engine stood him down.

**The witness as it must be written** is in `docs/p5-witness-draft-crowd.rs`, and its claim is not "every man stays `Move`" - the
one man who arrives may legitimately stand down. It is: **a man may stand down only standing on the goal**; every other man keeps
his order. Written that way it **bites on the unmodified engine** (`at tick 104 EntityId(9102) was given up away from the goal:
Idle`), which is what a witness has to do.

**Still open, measured**: with `beside` and `finish_blocked_orders` both cut, that same fixture **still** reddens with the identical
message at the identical tick, so a **third** writer stands a `Move` down away from its goal. The instrumentation that would name it
did not complete (a scripted edit corrupted `share_failed.insert(node, self.tick)` and the tree was reverted), so the next session's
first move is that probe done by hand: tag each of the nine `entity.task = Task::Idle;` sites, capture the tick **before** the
`find_mut` borrow, and read which one fires in this scene. Until it is named, `beside` and `finish_blocked_orders` stay in the
engine - cutting them alone makes the suite worse without proving anything.


### P5 measured a third time: the forty-tick rule carries three other rules on its back

Cutting `beside` and `finish_blocked_orders` together was measured against the seven fixtures that could notice. **Four redden,
and three of them are not about the cut at all** - they are rules that silently depended on it:

| fixture | what the cut did |
| --- | --- |
| `collision::a_walker_never_ends_a_tick_inside_a_friend` | the walker presses **781 raw** deeper after contact, against an **800 raw** step - one step is the whole allowance, so this one is a re-point |
| `group_overlap::a_mate_who_is_stopped_becomes_solid` | **a stopped body is walked over**: the walker comes **4576 raw** from him, inside an **8000 raw** reach. The old rule ended the walker's order before he could press through; without it the collision rules let friends pass through each other |
| `head_on::the_loser_stands_for_the_drawn_pause` | the loser's longest stand is now **0 ticks**, against the **0.25-1.50 s** the original draws - the drawn pause fires off `stalled`, and nothing reaches forty any more |
| `no_pushing::a_man_who_cannot_get_his_square_takes_the_next_tree` | the D204 re-target no longer fires: the man stays at (5.151, 7.199) on a tree with no square for him while the free tree of the same column is untouched |

**So the forty-tick counter is load-bearing for three rules that have nothing to do with finishing an order**: the body press-through
guard, the head-on drawn pause, and the D204 re-target. P5 is therefore not a cut but a **re-plumbing**: `stalled` must become a
counter each of those three rules reads on its own terms (a press-through guard that holds a body at contact, a pause draw, a
re-target), *then* the order-finish can go. That is a slice of its own and it is the honest next step, not a deletion.

**D271 is now withdrawn (D289)**: the owner ruled *"Drop it and let the crowds jam. I want the start as close to the game as
sensible. Upgrades may only come later."* `start_walk` sends every man to the ordered tile, so a crowd ordered to one spot presses
and jams - the precondition P5 was missing. Suite after the cut: **890 / 17 / 21**, no new red and one baseline red turned green
(`docs/reviews/2026-11-19-d289-the-crowd-jams.md`). **P5 is now the next slice** and its shape is the re-plumbing above: give the
press-through guard its own hold, the head-on draw its own roll, and D204 its own refusal count, then delete the forty-tick finish.


### P5 re-plumbing, first probe: the press-through diagnosis is wrong

Cutting `beside` + `finish_blocked_orders` again (now with D289 landed) reproduces the same four reds. The most alarming one -
`group_overlap::a_mate_who_is_stopped_becomes_solid`, *"a stopped body was walked over: he came **4576 raw** from him, inside the
**8000** reach"* - is **not a pass-through**, and the probe says so: with a trace on both skip clauses of `refusal`
(`is_mate` and `is_past`) in that fixture, **neither fires even once** (`ZZSKIP` count zero on a full run). The stopped mate blocks
on every tick the walker presses him, so the 4576 is the pair **mid-exchange**, not the walker inside a ghost.

**What that changes:** the old forty-tick finish did not so much protect the collision rules as *end the exchange before it pressed
this deep*. So the P5 slice is smaller than the re-plumbing note above made out - the press-through guard does not need inventing -
and its first task was to decide, with the game in hand, how deep two friends may press - **the binary answered (D290,
`docs/press-depth-evidence.md`): there is no floor.** `FUN_008aa990` computes a full depenetration and its only caller never reads
the result back into the unit's position; what stops a unit is the pause and the re-path. So the fixture's `>= reach` is replaced by
"friends may press", and the quarter-footprint floor proposed before the reading is withdrawn.


### P5 attempted with D289 + D290 in hand: three of four reds are re-points, the fourth is a scene problem

Cutting `beside` and `finish_blocked_orders` with the crowd now jamming (D289) and no press floor anywhere (D290) reddens four
fixtures. **Three are re-points, measured and applied in the attempt:**

- `group_overlap::a_mate_who_is_stopped_becomes_solid` -> *pressed into, not walked past*: the walker comes **4576 raw** from the
  stopped man inside an **8000 raw** reach, and the claim that survives is "he never ends up on the far side" - the binary has no
  press floor to hold him further out.
- `collision::a_crowd_sent_to_one_place_*` -> *presses together*: with the order-finish gone the closest pair presses to **0 raw**,
  and the claim becomes "the crowd converges", not "it never merges".
- `no_pushing::a_man_who_cannot_get_his_square_takes_the_next_tree` -> *a man who can press in keeps the node he was given*: with no
  press floor, "no square for him" is not a reason to move him; the D204 re-target is for a man who cannot reach the node at all.

**The fourth is the scene, not the rule.** `head_on::the_loser_stands_for_the_drawn_pause` measures the loser's *first stand* between
first contact and his next move. Sent past each other, the pair never touches - our re-plan goes round the body
(`replan_around_bodies`), measured **0 ticks**. Sent to the **same** tile (D289's own rule), whoever arrives first is **idle at his
goal** and the stand measured is 93.8 s of idling, not a pause. The scene that can measure it is a pair whose bodies **exactly fill**
the gap - knights (radius 0.25, so two of them are 1.0 of a 1-tile corridor) - and that fixture is the next session's first job.

**A real defect found on the way, and fixed in the attempt:** a mover re-arms his friend's pause on **every** tick he is refused,
because the pause is written after the pass (`pauses` vec) and `pause_remaining` is still zero while the pass runs. The guard wants
both checks (`other.pause_remaining == 0 && !pauses.contains(&other.id)`); without it a head-on's loser never resumes. It was
reverted with the rest of the slice, so it is **not in the tree** - it has to go in with P5.


### P5, third attempt: five fixtures re-pointed and green, two economy reds left

The slice was carried far enough today to know its exact shape. With both cuts in, a profile guard (a pause that is running is not
re-armed) and a no-coincidence guard (two bodies may never share one point - the binary's own resolver exists to prevent exactly
that, even though its result is discarded), **five fixtures went green and stayed green**:

- `head_on::the_loser_stands_for_the_drawn_pause` - rewritten as **two knights in a one-tile corridor** (0.25 + 0.25 is exactly the
  width, so a pair cannot slip past). It measures the drawn stand correctly once the window starts *after* the contact tick, and it
  passes: the loser stands inside **0.25-1.50 s**.
- `group_overlap`, `collision`, `no_pushing`, `overlap`, `click_spot`, `group_move` - re-pointed to the press model the binary
  actually has.

**Two economy reds remain, and they are the real blocker:**

| fixture | what it says |
| --- | --- |
| `retarget::a_forager_whose_bush_is_finished_goes_to_a_farm` | after the bush is spent the man is in `Return` (a *carrier*, `slot: 1`) and never reaches the farm inside 3,000 ticks |
| `head_on::the_higher_id_pauses_for_a_quarter_to_one_and_a_half_seconds_and_keeps_its_route` | passes alone and reddened in the full run - **order-dependent**, not yet explained |

The first is the interesting one: neither cut touches the `Return` arm, whose arrival is `footprint_gap <= radius + step`
(D246/D248), so the carrier's failure is a *consequence* - the man gathered, went to bank, and the retarget to the farm never
happened. It needs one session with the economy fixtures and the withdrawal path, not a guess.

**Where the work stands:** everything in the slice is known and measured; the tree is left at `9a8800f` with the plan, two witness
drafts under `docs/`, and no engine change from this attempt.


## Found by the 2026-11-19 crosscheck, and then withdrawn: there is no nudge left to remove

The crosscheck's first draft claimed the gather arm still pushes a body 0.15 tiles after forty refusals (quoting a *comment* at
`world/step.rs:1807`). **It does not**: no assignment to an entity's position exists outside the movement pass, the only `scale / 20`
left is a tolerance for "on his own place", and `no_pushing::a_crowd_at_a_tree_is_never_pushed` - which asserts that nobody in the
crowd is moved - is green. D204 removed the nudge; the comment outlived it. `docs/crosscheck-2026-11-19-rulings.md` carries the
correction.

## What is done, what is next (rewritten 2026-11-19, after D289/D290/D291)

**Done and pushing the engine toward the original:** D285/D286 (one box ruler; a click names its object), D287/P8a (the node re-lay
deleted), D289 (a crowd goes to the tile it was ordered to - D271 withdrawn), D290 (no press floor anywhere), D291 (the engine never
crosses resource kinds), P2 (the octile tie-break).

**Next, in this order.** Each item names the fixtures that carry it, so "done" is visible in the suite.

1. ~~**P5** - cut `beside` + `finish_blocked_orders`: **DONE** (D292), `docs/reviews/2026-11-19-d292-orders-are-never-given-up.md`.
   Suite **893 / 14 / 21** against 890 / 17 / 21. The "unexplained" `right_of_way` failure was **not** order-dependent - it
   reproduces on every run - and it was the no-coincidence guard deadlocking the crossing; the guard is withdrawn (the binary has no
   press floor) and the fixture passes.~~
2. ~~**The gather seat book** - DONE end to end: D293 stopped the engine reading it (896/12, six fixtures re-pointed), the next commit
   took the fixtures off it (895/11, two seat-capacity fixtures deleted with their subject), and **D294 deleted it** - suite unchanged
   at 895/11/21 with the red set byte-identical.~~
3. **The same-kind re-target and the reseed payment** - `gather::a_depleted_node_sends_the_villager_to_the_next_one_of_its_kind`,
   `reseed::a_queued_reseed_pays_the_farms_price_and_replaces_the_spent_plot`,
   `reseed::auto_reseed_is_a_player_setting_and_pays_when_the_plot_runs_out`. All three were red before D289 and are untouched by it;
   they are the economy's own parity holes and they are cheap.
4. **P4** - the terrain re-plan throttle goes, then the goal fallback it sat beside. That second half needs the order's goal to be a
   **tile**, as the search's answer already is, so it lands in two steps rather than one.
5. **P3** - touching collides, in one slice with the plan/step agreement (`leg_is_clear` as strict as the step), then re-point
   `building_passage::a_big_unit_walks_through_a_one_tile_gap` to the game's answer (D117 already withdrawn by the owner).
6. **P11 then P1** - the levelled search, and the budget lands with it (measured: a flat search at `h = 25 x tiles` truncates a
   legitimate detour).
7. **The AI layer** - now a *requirement*, not a nicety: D291 took cross-kind repurposing out of the engine, so idle villagers are
   re-tasked by the AI or not at all. That is the layer the game ships `sn-retask-gather-amount` for, and it needs its own evidence
   from the AI scripts before a line is written.
8. **Left over, unexplained, and not to be forgotten**: `both_sides_field_an_army`, `the_skirmish_only_refuses_what_the_fight_has_taken`,
   `a_walled_off_wood_is_tried_once_and_then_left_alone`, `a_villager_drops_only_within_half_a_diameter_of_the_drop_site`.

## The fork that the owner has already ruled (P3, kept for the record)

The binary's collision test is four `<=` (`FUN_00840830`), and its search grows every object by the mover's radius
(`FUN_007f9d80`). Under those, a body of radius **half a tile** - a packed trebuchet, the widest in the pack - **cannot thread a
one-tile gap**: its box exactly fills the lane, so it touches both walls and touching is a collision. `building_passage::
a_big_unit_walks_through_a_one_tile_gap` asserts the opposite, and that fixture is not ours: it is **D117, the owner's rule**,
2026-11-04 - *"a one-tile gap is a gap, whatever the circle"*.

So the two cannot both stand:

- **(a) the game wins**: touching collides, D117 is withdrawn, and a half-tile body stops at a one-tile gap;
- **(b) the rule wins**: a body is refused only where it would *penetrate* a static object (the step's current behaviour), and
  that is the one place in the pathing where his ruling overrides the binary - it must then be written as such, in D117.

P4, P5 and P6/P7 do not depend on the answer.

## What "proved" means here, every step

- A **conformance row** for a rule the game states, or a **fixture** for behaviour, **watched failing before the change**.
- The acceptance fixture's time (`the_acceptance_match_trains_more_villagers`), and the suite's red set **diffed by name**
  against the previous commit's, not by count.
- The step written into `docs/decisions.md` and this file, committed and pushed per step.
