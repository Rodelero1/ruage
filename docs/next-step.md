# OPEN - R1: adopt the game's pathing answers, starting with the step through a body at rest

**The rulings are in (D239 and D241, 2026-11-16).** The owner, over the movement rows of `docs/behaviour-diff.md`: *"Resolve the questions by
adopting from the game. That is the source of truth."* The register is now **30 rows, 19 ruled**; rows 13-30 are pathing and
obstacle avoidance - **18 rows, 17 ruled: 6 same, 11 different, 1 unknown-static** - and **every divergent row becomes a slice**,
with two exceptions that need a read first (R7). Where the game's side is unread, the read comes before the code; nothing is
guessed in the game's name. The register was corrected the same day (rows 18, 20, 21; two new rows 29 and 30) and its recount is
the authority, not the morning's prose.

**The previous open step (T16, and everything carried with it) is archived at `docs/history/next-step-2026-11-16-t16.md`.**
Nothing in it is withdrawn; it is simply not this step.

## Now: the seat, once (D245) - the one mechanism that gates rules 1, 3 and 4

The owner asked how the original meets his four body rules and for them to be implemented. All four were read (D245), rule 3 was
built and it **passes its own fixture** - `invariants`: 3 passed, 1 ignored - but it cannot land, because it re-times every
delivery and `gather_motion::three_gatherers_on_one_bush_do_not_pile_up` then goes **red at 0.236** (bodies touch at 0.400).

**Three attempts now name the same defect.** D236 (path-ignore mates, put-back-on-sink), D238 (three seat fixes) and D245 (delivery
timing, `is_past` narrowing, wall aim) all end at it: the ring round a node is **maintained by stepping men who are already
working**, so removing the stepping removes the arrangement and the ring piles at 0.204-0.236. D245 also *rules out* the passer-by
explanation: excluding a working gatherer from D210's "not moved is past" left the number unchanged at 0.236, so it is two
**seated** men drifting.

**The change**: a node hands out a **place**; the man keeps it until he leaves (node empty, full hands, new order); a man with no
place **waits outside the ring**, at a distance that keeps **0.400** from every body, and does not displace anyone. Then:

- rule 4 (no movement while gathering) is a property of the placement, not of a per-tick correction;
- rule 1 (never closer than a diameter) is a property of the ring and the waiting spot;
- rule 3 can finally land - `footprint_gap(position, drop) <= body_radius + one step`, measured **4,155 raw** against a **4,000**
  radius on the reverted build, with acceptance 157 entities and 649 re-paths.

Red-first fixtures that must bite: `three_gatherers_on_one_bush_do_not_pile_up` (already red under rule 3's fix - use it as the
mutation proof), `invariants` rules 1, 3 and 4, `overlap::a_passer_by_does_not_move_the_seated_gatherers` (ignored since D238),
`gather_motion`, `gather_ring`, `gather_standoff`.
## The queue, in the order a player would see it

Each slice ships test-first, with its measurement in a review under `docs/reviews/`, and **the gate is the acceptance match at
48,000 ticks**: entities at or above **155**, `repaths` near **380**, plus the slice's own fixture. A hash moves whenever behaviour
moves; say so rather than treating it as a defect.

1. **R1a - the past-rule chain, then the step through a body at rest** (rows 29, 30; **measured and split, D242**). Both
   halves were built and reverted: deleting `TakeAtRest` fixes the encounter (8,000 raw in an 8,000 reach against 7,200) but
   **deadlocks two soldiers in a lane** - `group_overlap`'s head-on pair stands for 2,002 ticks - because the chain the game
   uses to end a stand (paused man stops moving, the other's re-plan is stamped later, the stopped man becomes **past**) does
   not complete here. **R1b** (the re-plan's obstacle set) is worse still: entities **155 -> 143**, `repaths` **497 -> 649**,
   because `route.avoiding` was the only memory of the men already met; the game has a second memory we lack (its search
   re-inserts standing objects, `FUN_007f9d80`). Order: complete the past chain, then delete the arm, then re-measure the avoid
   set. The specification fixture is `crates/sim/tests/replan.rs::a_stopped_friend_is_not_walked_through`, `#[ignore]`d.
2. **R2 - the pause is re-drawn, and a paused mate is not a mate** (rows 18, 20). `setPause` has no "already paused" guard, so a
   fresh collision draws again; and the mate exemption is re-tested on motion, not on the route's existence.
   *Fixture*: the head-on pair, asserting the **first** pause is inside 0.25-1.50 s, the waypoint list is unchanged across it, and
   **both arrive** - never "longest continuous stand", which measures the end of the match.
3. **R3 - the incumbents' squares** (rows 24, 25; D241). `maybe_respread` retires, and the idle-body nudge with it. The
   newcomer's half is **already built** and is the owner's rule verbatim - the nearest free node of the same kind within
   `gather_reach`, else wait (`step.rs:2178-2212`) - so this slice is one deletion, one guard, and one **restated fixture**.
   **The fixture must assert what he objects to** (a seated man does not move), not that no two bodies ever overlap: with the
   past rule standing (D241 ruling 1) a route planned through a stopped man *is* walked through, and the 0.204-tile pile D238
   caught is the game's behaviour rather than a defect.
   *Gate*: `overlap::a_passer_by_does_not_move_the_seated_gatherers` off ignore with incumbents moving no more than one step;
   entities at or above 155.

4. **R4 - an unreachable waypoint is substituted** (row 26). `mConsecutiveSubstituteCount`: *"we have a count of %d, faking it"*.
   `finish_blocked_orders` retires in its favour. *Read first*: the counter's own semantics beyond the log line (how many counts,
   what "faking" does to the waypoint index) - `FUN_008a4360`.
5. **R5 - passage is the body's** (row 21, **supersedes D117**). Grow the tile graph by the body's radius, as the step layer
   already does (D176/D192). `a_big_unit_walks_through_a_one_tile_gap` **inverts**: a 0.5-radius unit has no passage. The trap is
   measured: a clearance with any **margin** put every gather ring's places inside a neighbour's disc and reddened 18 tests
   (D211) - the clearance is the pair's reach **exactly**.
6. **R6 - the caps go, and the price is measured** (row 22). Remove `MAX_GROUND_REPLANS = 2` and `MAX_REPATHS_PER_TICK = 4`; the
   game has no counter. The 3.5-against-0.3 ms/tick figure predates the sub-tile pencil, so it is re-measured here. A regression is
   a finding for the owner, not a licence to reinstate the caps.
7. **R7 - two reads before two ports** (rows 13, 27). **The cost model and the frontier**: settle the 2500/100 estimate against a
   16-per-step cost - one of them is wrong, and neither is ported until it is - then adopt 16 with the octile table and the
   generation-stamp array rather than a heap, because equal-cost ordering shapes the route. **The two-level path**: read
   `AddFuturePath`'s parent derivation, then build it.

8. **R8 - the start re-seed, and the path vocabulary** (rows 33, 36, from the sibling thread of 2026-11-16). When the mover
   begins **inside** a body the original offsets the start by **2 x radius** along a fixed direction ring and re-seeds the search,
   returning "no low-level path" only when every direction is blocked (`FUN_007fb220`); ours lets a route leave a disc it already
   stands in, but only outward, and that is a stand-in rather than the rule. And the order's path vocabulary is narrower: forced
   waypoint, teleport-to-waypoint, per-path hit (patrol we have, D47). *Fixture*: a body spawned inside a stander reaches its goal
   with its first waypoint off the disc; a forced waypoint is passed without a re-plan.

9. **R9 - a thing that cannot be reached** (row 32; D241 ruling 3). Today the order is refused at `orders.rs:52`. The owner's
   rule: **accept it**, walk as close as the ground allows, and **idle** when a wall is what blocks the way; when the obstruction
   is other resource - a tree behind trees - prefer a reachable node of the same kind within `gather_reach` and work that. The
   helper exists (`nearest_like_free`), the trigger does not. *Fixture*: a tree ringed by a house is approached to the closest
   standable tile and the man idles there; a tree ringed by trees sends him to the nearest reachable tree with a gather task.
   **Villagers are solid and formations are deferred** (D241 ruling 2): the same-tick mate stand-in for soldiers stays, documented
   in `stamp_group` as a stand-in for a formation that does not exist yet.

10. **R10 - the nearest standing tile is the tile nearest the click** (D247, found by the conformance set). `nearest_reachable`
    answers **one step from the start**: on empty ground from (1,1) toward (10,10) it returns (2,2), because every tile its
    frontier pushes is already free, so the first pop is a neighbour of the start. `crates/conformance/tests/findings.rs` is the
    red fixture (`a_distant_click_into_a_wood_lands_on_the_woods_edge` is the player-visible one); the fix owes its own slice.
    **The trap**: `click_into_the_woods.rs` passes today because it spawns the villager one tile from the wood, so "a legal
    tile within one tile of the block" is satisfied by the bug - start the walker further away when rewriting it.

**Still open, and it needs an artefact rather than work:** row 23, the exclusive goal. **Forty units ordered to one tile**, one
recording. No read decides it and no decision should stand in for it.

## Carried, not dropped

The full text of each is in the archive above; none of them is this step.

- **T16 / T12 stage 3** - the per-player clock (`pturn`, the nine accumulator words) and the two-tier unit AI; stage 3 is priced
  and not worth it yet (D229-D232).
- **P1b, the save converter** (D235): sections 2-4 of the save, a `tools/` converter, `--load`, and a version stamp. P1a landed.
- **The effect interpreter's remaining kinds** (T2): 0 / 101 / 103.
- **D225's two probes**: whether a blind placement over an enemy unit stalls the builder, and the search cap (LOS + 5 tiles, 30
  reach tests) that only matters if the stall is real.
- **R6's second half** (D234): index each effect by attribute once instead of cloning modifiers per player - the seam costs
  +2.6% until it lands.
- **Civ and team bonuses** stay deferred by ruling (D219); the design is `docs/civ-bonuses.md`.

## Hazards a fresh session must not re-learn the hard way

- **A missing pack passes every world test silently.** `config/gamedata/` is gitignored; copy the four `.toml` files in, never
  symlink them.
- **One `CARGO_TARGET_DIR` per checkout** - two worktrees sharing one swaps binaries under a running suite.
- **A doc that promises a mechanism is not the mechanism.** "Eight shoulder points round every body" lived in a review for a day
  and did not exist. When this file claims a mechanism, `grep` for it.
- **A test whose route is planned *around* the body it means to collide with measures nothing.** With R1 deleting the accumulated
  avoid set, four fixtures that currently move a body *after* the route exists must keep doing so - the failure reads "the scene
  never met the body", and that is a stale fixture, not a broken rule.
- **A number that is right does not mean the mechanism is.** Four times in the D214-D218 run the fix was the *second* thing.
