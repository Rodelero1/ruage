# OPEN - R1: adopt the game's pathing answers, starting with the step through a body at rest

**The ruling is in (D239, 2026-11-16).** The owner, over the movement rows of `docs/behaviour-diff.md`: *"Resolve the questions by
adopting from the game. That is the source of truth."* The register is now **30 rows, 19 ruled**; rows 13-30 are pathing and
obstacle avoidance - **18 rows, 17 ruled: 6 same, 11 different, 1 unknown-static** - and **every divergent row becomes a slice**,
with two exceptions that need a read first (R7). Where the game's side is unread, the read comes before the code; nothing is
guessed in the game's name. The register was corrected the same day (rows 18, 20, 21; two new rows 29 and 30) and its recount is
the authority, not the morning's prose.

**The previous open step (T16, and everything carried with it) is archived at `docs/history/next-step-2026-11-16-t16.md`.**
Nothing in it is withdrawn; it is simply not this step.

## The queue, in the order a player would see it

Each slice ships test-first, with its measurement in a review under `docs/reviews/`, and **the gate is the acceptance match at
48,000 ticks**: entities at or above **155**, `repaths` near **380**, plus the slice's own fixture. A hash moves whenever behaviour
moves; say so rather than treating it as a defect.

1. **R1 - the step through a body at rest, and the re-plan's obstacle set** (rows 29, 30). Delete `collide`'s rule 6
   (`TakeAtRest`): an idle friend who is not past is a non-past collision, answered by a re-path and the pause. Stop folding
   `route.avoiding` into the search's **obstacle tiles** (`step.rs:1325`); the classifier keeps the set, the search loses it.
   *Fixture*: a walker ordered past a friend who stopped **before** the route was planned (`clearance::`'s existing one), and a
   walker whose route has met three bodies in a row - assert his second re-plan is not longer than the first by tiles.
2. **R2 - the pause is re-drawn, and a paused mate is not a mate** (rows 18, 20). `setPause` has no "already paused" guard, so a
   fresh collision draws again; and the mate exemption is re-tested on motion, not on the route's existence.
   *Fixture*: the head-on pair, asserting the **first** pause is inside 0.25-1.50 s, the waypoint list is unchanged across it, and
   **both arrive** - never "longest continuous stand", which measures the end of the match.
3. **R3 - the seat book is authoritative** (rows 24, 25). `maybe_respread` retires and D204's next-node re-target retires; a
   newcomer with no free square keeps circling the same node. **The mechanism is an inference, not a read** - the first act is a
   test that pins what the game looks like (his sentence: a newcomer "will be walking around until he finds a spot"), not a
   guess at the internals. D238's "a seated body must not be past" is **withdrawn**: under the game's own past test a stopped man
   *is* past, and the seats are what keep bodies apart.
   *Gate*: the D238 fixture's 0.204 never appears, `overlap::a_passer_by_does_not_move_the_seated_gatherers` comes off ignore, and
   entities do not fall below 155.
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
