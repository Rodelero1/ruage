# OPEN - D186 landed: nine men a square on a 1.156 ring

**Landed on `main`.** The owner's save gave the number the engine was guessing at: a working man stands **1.156 tiles** from a
source square's centre (six woodcutters at 1.105-1.189, spread 0.084), and **nine** men work one square - *"max 9 per each. 4 on
corners and 4 on sides"*, which his own file agrees with (sixteen orders over two adjacent trees came out **9 and 7**; a cap of
eighteen would have sent all sixteen to one tree). The ring is nine slots at forty-degree steps, a work band of 1.056..1.256, and
a neighbouring-tile fallback for a ring buried in a forest. Tests in `crates/sim/tests/gather_ring.rs`, decision **D186**, review
`docs/reviews/2026-11-11-the-gather-ring.md`.

**The picker's own cap: fixed** (D186 addendum 2). `claimed_by_other` still said *"any man on the node means taken"*, so a
woodcutter whose patch ran dry stood idle four tiles from a patch with two men of nine on it, and the eleventh man ordered onto a
full square stood still instead of joining the square next door. Both are measured in `crates/sim/tests/gather_room.rs`.

**Two men overlapping: fixed** (D186 addendum). The place now belongs to its *nearest claimant* (ties by id), the walk ends on
the ring rather than at the band's outer edge, and both tests that were `#[ignore]`d pass with no tolerance.

**Two other defects the work found, both fixed:** a ring slot could land on a **building site** (a foundation is walkable since D183), so
a gatherer stood on it and the empty-site gate waited for him - a barracks frozen at **871 of its build ticks**; and
`config/skirmish.toml` pulled that barracks' only builder off at 380 s where the ring makes it finish at **~395 s** - the step is
460 s now, with both numbers beside it.

**OPEN - the cap is not asked where villagers are *moved*.** `start_gather_internal` skips the nine-man gate (`if spread &&
on_node >= GATHER_RING_SLOTS`), and `nodes_of_slot` - the picker the villager-share pass uses - returns the nearest source of a
column without asking whether it has room. Under D182 that was harmless (a square held one man); under the ring the tenth man
moved to a full square keeps a `Task::Gather` he cannot work, so the share pass counts the column as manned and nobody looks
again. Two callers are exposed: `world/shares.rs` (`step_shares`) and `pending_gather` (a moved villager resuming after a
deposit). **Not fixed, and not guessed at**: a fixture with two patches (nine men on the first, the second empty) and
`set_villager_shares(&[(1, vec![(wood, 100)])])` left the idle tenth villager untouched - `Task::Idle` after two seconds - so the
share pass's own preconditions (anchor, reach and the retry window) are not understood well enough here to write a test that
fails for the right reason. That fixture is the next session's starting point; it is three paragraphs up in this file's own
history and in `docs/decisions.md` under D186 addendum 2's neighbour.

**The three saves read (2026-11-12).** `mine.aoe2spgame`, `bush.aoe2spgame`, `house.aoe2spgame`, kept at
`/opt/data/cache/saves/owner-{mine,bush,house}.aoe2spgame`.

- **The mine measures, and it agrees with the tree.** Four men round the gold at (66, 81): from the mine's tile centre they
  stand at **1.020, 1.041, 1.061 and 1.190** tiles (mean **1.078**, spread 0.17), and the closest pair is **0.400** apart -
  two villager bodies, the same signature as the tree's 0.404. A free circle fit puts them at **1.125** round a centre 0.11
  from the tile centre (residual 0.004), which is what a ring does when its centre is not exactly the tile's. **1.156 is not
  refuted**: the mine's own spread (0.17) is wider than the difference between the two means (0.08), and with four men a free
  centre and a free radius are one measurement short of separable.
- **The bush and the house do not measure.** Their orders were given while the men were still **walking**: the bush's
  from-points sit 0.3-1.9 tiles from the bush (and its later destinations are sub-tile, at (64.60, 79.60) and (64.60, 80.00),
  two bushes 0.4 apart), and the house's four build orders point at (76, 78) from **2.1, 2.4, 3.1 and 3.7** tiles away. A
  from-point is where a man stood when the order was given, so these record the approach, not the post.
- **To re-take them**: let the men reach the bush/site and *work for a few seconds*, then click the bush (or the house) once
  more and save immediately - the same trick that made the tree readable. For the house, a site that is **half built with its
  builders already standing round it** is the one that answers whether construction rings at the same distance.

**OPEN - the nine-man cap is contradicted by the owner's own game.** `owner-bush2.aoe2spgame` (2026-11-12) carries **eleven**
orders to one bush - `(64.50, 78.50)` eleven times - and the owner's words with it: *"Bush has 11 villagers on it all foraging."*
D186's cap is **nine**, taken from his earlier *"max 9 per each"* and from the 9 + 7 split over two trees in `owner-test2`. Eleven
on one bush refutes it: either the cap is the geometry (eighteen at body spacing on the 1.156 ring), or it is a number above
eleven, or the 9 + 7 split was the click's own distribution and not a cap at all. **The engine has nine; the game shows eleven.**
It is one constant (`GATHER_RING_SLOTS`) and needs the owner's call, not a guess.

**The two later saves still do not give standing positions.** In `owner-bush2` the freshest orders (stamps 1024-1028) come from men
at **1.21 to 2.57** tiles of the bush, and in `owner-house2` the four build orders (stamps 494-500) come from **2.06 to 3.73**
tiles of the site: both groups were still walking when the click was made, so the from-points are the approach again. What worked
for the tree was a click made while the men were *already standing at it* - the six radii there were 1.105-1.189.

**Still owed, in order:**

1. **The builder's ring is unmeasured.** The owner expects *"the same pattern"*; no save with a site under construction has been
   read.
2. **Mines and bushes were not separated** from the trees in his save, so the engine uses 1.156 for every node kind.
3. **`start_gather` routes to the neighbouring tile, not to the slot**, so a worker walks in and then out again - up to two seconds
   per trip, which is why the felling window had to widen from 15 s to 16 s.


# DONE - D183, built (D185): a foundation is walkable until somebody works on it

Session 51. `crates/sim/tests/foundation_model.rs`, 11 tests, each watched failing first (three of them - the first
stroke taking the ground, the eviction walking, and the farm exemption - were written after their code and proved by
mutation instead; the review says which). Acceptance: entities 137 -> **143**, repaths 618 -> **517**, hash
`dc9cf9905f65b61c` -> **`3aaf6037ff27d283`**, 10.14 s -> 10.73 s on a *different* match; the tick itself 6.8 us on both
sides, the snapshot +6.5% because the view now asks `visible_to` per entity. The ghost is
painted in `palette::FOUNDATION` (light grey, every tile) - without that branch the renderer drew **nothing**, because its
rule is "paint the tiles the engine blocks" and an untouched site blocks none. Review:
`docs/reviews/2026-11-11-a-ghost-is-a-drawing.md`; the rule is D183, the implementation D185, the defect it found D185.1.

**Two defects fixed on the way, both reported in the review:** a builder used to be sent *inside* the walkable ghost by
`work_position` (a house took 1 builder-tick in 500), and a **cancelled site was resurrected by its own builder** - a
pre-existing bug, `work_one_tick` overwrote the zero health `Delete` had written.

# NEXT - the D183 tail, in the order it matters

1. **A step-aside resumes a gather and a hunt, and nothing else** (D185.2). A `Return`, `Build` or `Attack` interrupted by a
   foundation stands down when the walk ends. Extending it is a slice of its own: `Return` needs its drop site re-resolved,
   `Build` a fresh sight of the site, and `Attack` a target that may be dead.
2. **A hunt blocks a step-aside.** `resuming` is both "the order to put back" and "this unit is already leaving", so a unit
   fighting when a foundation lands on it is never told to leave. One more field, or a second set, would separate them.
3. **The enemies' invisible ghosts and the AI.** The AI's placement search now cannot see an enemy's untouched foundation,
   which is the rule - but the first-stroke contest has never been exercised in a match where both sides want one tile.

# OPEN - a scout that stops, and cannot be nudged (owner, 2026-11-08)

The owner, with a screenshot of a scout cavalry standing at the forest's edge over an order marker: *"I noticed scout cavalry got
stuck in the forest. It can be unstuck by ordering to move somewhere."*

**Not reproduced yet**, and not guessed at either. The hypothesis to test first is that D177's clearance rule opened a new door onto
D127's old "stuck scout": a route whose *last leg* (or whose step off a corner) lands where `clearance_at(landing, radius) < 0` is refused,
the mover burns `MAX_GROUND_REPLANS` (2) trying the same geometry again, and then stands still for ever - while a **new order** resets
`stalls`, which is exactly why the owner can unstick it by clicking.

**The fixture to write first:** a scout ordered across a forest belt, with the goal such that the route's final leg passes within a body's
radius of a blocked corner, and an assertion that the unit's distance to the goal falls at least once every N ticks (a "never stalls" test,
not an "arrives" test - arriving can be true of a unit that teleported).

**Evidence that would settle it in one read:** the owner's save with the stuck scout in place (positions, task and route at the tick it
stopped) - see the save-game item below. Do not "fix" it by relaxing the clearance rule: that rule is the owner's collision complaint.

# NEXT - the save-game object walk: the owner's file is here, sections 1-3 are located, section 4 is not

**The file**: `test.aoe2spgame`, sha256 `af5282fb1ddd9caa96e0d3e4bf1b69e3c0739334a94de2bad5ac5e60d95be872`, 289,740 bytes on
disk, **3,290,468 undeﬂated**, `VER 9.4`, save version **12.5**, word at 12 = 1006.0 - the same family as `brit1`-`brit13`.
Kept at `/opt/data/cache/saves/owner-test.aoe2spgame` (outside the repo; nothing derived is committed, D8/D22). It carries three
groups the owner named: **gold top right, one tree left, berries bottom right - and the berries are TWO bushes**, not one
(corrected by the owner 2026-11-11), with a screenshot of the same moment
(`docs/`-less, held in the session).

**What is established with evidence, and what is not.**

- **The map is 144×144.** Two 16.16 fixed-point words `0x00900000` (144.0) sit at **0x100a3c** and **0x100a40**, and the tile
  layers follow: the longest run of low-valued bytes in the file is **82,967 bytes at 0x115861** = four 20,736-byte layers
  (144×144) plus 23, and a second 41,523-byte run (two layers) at **0x10b5c9**. This matches `FUN_00836830`'s field order
  (`& map_width`, `& map_height`, `& world_width`, then the per-tile arrays).
- **The tail of the file is a stream of per-object records** (from ~0x3228A0 to EOF 0x323564): a repeating 28-byte header
  whose fields include a small increasing id (26, 27, 31, 35, 43, 44, 48, ... 355) and a waypoint count, followed by that
  many triples `(x, 0.0, y)` in **tile coordinates** (57.5, 93.5), (57.5, 90.5), ... - a moving object's path. That is the
  object list or its tail, and it is why this is a bounded job rather than a research question.
- **Not done, and deliberately not guessed at**: no record layout is verified against the game's loaders, so **no object,
  unit, villager or distance has been read out of this file**. Two structural probes were tried and both failed to chain to
  EOF (a 24/28/32-byte header with 8- or 12-byte points), and scanning for a per-tile object layer as a plain `u16[144×144]`
  array found only zero-filled layers. A parse that "looks plausible" is exactly what `docs/save-format.md` records as the
  way a fixed offset misled it before.

**The walk that is owed, in order:**

1. `FUN_00758030` reads `"& type"` (a 4-byte word) and dispatches to a creator (`FUN_007570e0`), which then calls the
   class's own `load`. The per-player section (`FUN_00730600` around line 530) loops `numPlayers` times and calls it, so the
   object list lives **inside each player's loader**, not in one flat list at the end.
2. Class loaders read their base first (`TribeCombatObject` -> `RgeCombatObject` -> `RgeActionObject` -> `RgeMovingObject` ->
   `RgeAnimatedObject` -> `RgeStaticObject`), and `RgeStaticObject` is where `id`, `x` and `y` live. Reading that chain is
   the actual work: the same shape `librematch/genie`'s `genie-save` models for **DE** (66.x) - its `generated/world.rs`
   is a good map of the classes, but **not** of this format's field widths, and must not be used as one.
3. Then the four checks the register already names: object count equals what the scenario declares; every position inside
   the map; the towns land on the tiles the map section gives them; the same save's terrain reproduces what the map reader
   yields.
4. Then the payoff: villager -> node -> exact distance for every gatherer at once, which is the oracle D169/D182
   approximated from `radius[0]`, and what the owner's three groups were sent for.

**The order section at the tail, read (2026-11-11).** From `0x322783` to `0x3234cf` (149 bytes short of EOF) the file
holds **53 records of `[u32 type][u32 1][u32 0][u32 id][u32 n]` + `n` triples of `(f32 x, f32 0, f32 y)`**, `type` being 4 or
6, size `28 + 12n`, ids ascending (7, 9, 13, ... 355). The points are in **tile coordinates**. They are **orders, not
positions**: ids 169, 170 and 171 carry the identical from/to pair, which three separate men standing in one spot cannot do.
Their destinations cluster exactly where the owner's screenshot has his patches:

- tree: **(57.5, 90.5) x9** and **(56.5, 90.5) x7** - two adjacent squares, sixteen orders;
- berries: **(54.5, 74.5) x6**, plus **(54.10, 73.38)** and **(54.10, 75.62)** - **two bushes about 1.2 tiles apart**, which
  agrees with the owner's correction that the berries are two bushes and not one;
- gold: **(66.5, 81.5) x4**, plus (65.38, 81.10), (66.10, 80.38), (67.68, 81.10).

The 9-and-7 split over two adjacent squares is the first quantitative support for the owner's *"max 9 per each"*: sixteen men,
nine to one square and seven to the other. The from-points of those same records sit two to four tiles away, so the men were
**not** standing round the tree when the order was given - which is why this section cannot answer "how far does a working man
stand": the standing positions are in the object records, and those are not yet located.

**The standoff, measured (2026-11-11, second save `owner-test2.aoe2spgame`).** The owner re-saved with the men standing
at the tree and one more click given, so the tail section's *from*-points are standing positions. Reading them
(`[type][1][0][stamp][n]` + `n` triples, 39 records, `0x32ceb4`..149 bytes short of EOF):

- **The working men sit on a circle around the source's tile centre at radius 1.156 tiles** - six of them, at 1.105, 1.142,
  1.160, 1.170, 1.170, 1.189 (spread 0.084). Not 0.70, which is what `radius[0]` gives us and what D182 implemented.
- **They pack along that circle as close as their bodies allow**: the three closest pairs are 0.404, 0.455 and 0.710 apart,
  and 0.404 is exactly two villager radii (2 x 0.2) - body touching body.
- Behind the ring: 1.357, 1.555, 1.598, then the men who are not working at 2.236, 3.000, 3.000, 3.162.
- Arithmetic worth keeping: the ring's circumference at 1.156 is `2*pi*1.156 = 7.26` tiles, and `7.26 / 0.404 = 17.97` - so
  **eighteen men fit on one ring at body spacing**, which is the owner's count of his group and the "max 9 per source" over
  the two adjacent source squares the tail names (9 + 7 destinations). Treat this as an observation, not a rule, until the
  object records confirm the assignment.
- **The two adjacent squares** (57.5, 90.5) and (56.5, 90.5) again appear as the destinations, so the left patch is two
  source squares, the same shape as the berries.

What this changes in the engine: the gather standoff should be a **ring at ~1.16 tiles from the source's tile centre, packed
at body spacing**, not one man per square at 0.70. That is a D-level change (it supersedes D182's standoff) and it needs a
test written first.

**What the owner says the picture is for** (2026-11-11): *"I'd focus instead on how close men can stand to the bush, tree
or mine. The ones behind them don't work. Only first row ever works."* So the question the save answers is the **standoff**,
not the crowd: how far from a node's tile the men who are actually working stand. And the crowd question is answered the
other way - **only the front row works**, so D182's one-worker-a-square stands unchanged and the men behind are idle.

**A cheaper intermediate the owner would accept, if the chain takes too long:** the *scenario* section (section 2,
`FUN_00545460`) and the *map* (section 3) read first - the map alone can be drawn and checked against the owner's own
screenshot, which is a real result even before an object is read.

# The step walks the line, and the route bends where the ground does

> # Session 50, 2026-11-09 - D168: the decisions are browsable, and reviewable in batches
>
> **The owner's ask:** one page that shows every decision, **how it was made**, and what is still missing - "help me to
> spot gaps and bad decisions". Built: `tools/decisions.py`, 37 tests, output committed at `docs/site/index.html`,
> rebuilt by `make decisions` (760 ms).
>
> **What it says about the corpus today:** 165 decisions, 5 ids carrying an amendment section, 4 corrected by a later
> decision, 5 superseded, **33 with no stated basis**, 24 paragraphs where a decision names its own limits, 12 open
> questions in the U-register.
>
> **The finding to act on:** ten ids are cited in the docs or the code and never written - **D78, D79, D81, D82** (a gap
> in the numbering four documents still reference), **D115, D124, D126, D136**, and the lettered **D122b, D125b**.
> `docs/site/index.html#dangling` gives the file and line of every mention. Each is either a lost decision or a
> citation that was never true.
>
> **Next:** triage the ten (write the rule or fix the citation), then look at the 33 uncited decisions the page names.
> **Not next:** linking a decision to the test that pins it - no such map exists and inferring one from comments would
> be a guess.

> # Session 6, 2026-11-04 - straight-line walking, the pass the owner has asked for three times
>
> **Where session 5 ended:** `docs/pathfinding-from-the-talk.md` (pushed as `3cfd783`) - the short-range algorithm from
> the Forgotten Empires talk, mapped onto this tree, with the one decision it forces left to the owner there. The owner's
> complaint is on record three times, the third with a screenshot: *"Walking still follows squares and goes through their
> centers - units go either vertically, horizontally or diagonally, if unit has to go from point A to point B and point B
> is 60 degrees in some direction, the path is not a straight line but instead 45 degree walk followed by horizontal
> walk."* That sentence names the bug exactly: `step_towards` clamps **each axis independently**, so every heading that is
> not a multiple of 45 degrees becomes a 45-degree leg plus a cardinal leg.
>
> **The step, in two slices, one commit each:**
>
> | # | slice | state |
> |---|---|---|
> | M1 | **the step walks the segment**: `step_towards` moves `delta` along the line and lands exactly on the point when the remainder is one step; a repath's route is pulled the way a planned route already is | **landed** - `docs/reviews/2026-11-04-the-step-walks-the-line.md` |
> | M2 | **the route's corners are the obstacles'**: `short.rs`, blocked tiles merged into rectangles and expanded by the body's radius, a visibility graph over their corners (the talk's edge walk, as a graph), so a walk bends where the ground does and keeps its body's clearance | **next** - the design is `docs/pathfinding-from-the-talk.md`; M1 is the half that could be landed without it, and it is what the owner's screenshots are about |
> | M3 | **arrival and collision read the same line**: `step_is_walkable` asks the segment rather than the landing tile alone, and the shape question in `docs/pathfinding-from-the-talk.md` (bump-and-repath against `push_units_off`) is answered by the owner | after M2 |
>
> **D110 records the deferral**: slide is kept for M1/M2 (a step refused by a body still slides, `collide` untouched), and
> the original's "units bump into others, stop and repath around. No pushing allowed" (slide 9 of the talk) is **not**
> adopted until the owner says so - it is a gameplay rule, not a correctness one, and it would change the acceptance hash
> for a reason nobody asked for. Recommended, if asked: keep sliding friendly bodies (our units keep lanes), and revisit
> when formations exist.
>
> **Standing rules for each step, unchanged:** compact (this block), test-first with the failing run shown, an
> adversarial review in `docs/reviews/` with a measured `## Performance` number and a named `## Stability` attack, then
> commit and push.

> # Session 49, 2026-11-07 - D152: priority order, and what "functionality" actually lacks
>
> Order: **1 functionality, 2 UI, 3 graphics, 4 AI, 5 music.** AI deferred (D151 is the plan, not the work).
>
> Pathfinding audited with numbers: 8.9 us corner-to-corner (64x64), both-corners diagonal rule, occupied-start refusal,
> world-level smoothing, and `src/short.rs` for the local case. The review's "sealed map expands the grid" is ~9 us, so it
> is *not* the next fix. **The functional gaps are the documented ones**: no gates (D54), no unit pushing (D52), fog without
> occlusion (D50).
>
> **Next: gates** - a wall that lets its owner through. Then unit pushing, then fog occlusion.

> # Session 48, 2026-11-07 - D150: +5 was never the problem
>
> Instrumented answer to the owner's two hypotheses: 586 failed picks, **all gold**, nearest known gold 9.2-18.5 tiles; the
> map's gold sits 9.4 tiles from the town centre. No wood/food pick ever failed - the forest was not depleted. The depletion
> path already chooses at the node (his rule, implemented and reverted as unprovable), and the real fault was the share
> table's anchor. **With it fixed, sight + 5 resolves the match**: 135 entities, 1 player alive, hash
> `45596da62c9e3668`, and **5 is the shipped constant**. Review:
> `docs/reviews/2026-11-07-the-anchor-was-the-problem.md`.
>
> Then: `decide` at 432 lines if shape work is wanted; the 8 parked owner questions; D141's two-stage gather search is no
> longer needed for the radius (it may still help a *scarcer* map).

> # Session 47, 2026-11-07 - D149: "nearby" has a number
>
> The owner picked sight + 5; **measured, 5 breaks the match** (129 entities, both players alive, 83 repaths). Shipped
> **sight + 8 = 12 tiles**, the smallest bound that resolves: **152 entities**, 186 repaths, 159 made-room, hash
> `82a5063dcca63108`. The share table now anchors on the town centre (a test caught villagers spawned far from base never
> getting a share row). Review: `docs/reviews/2026-11-07-nearby-has-a-number.md`.
>
> **Open question asked, awaiting his number**: keep 12, or 10 (reach 14, also resolves, hash `2adb19a4`), or the two-stage
> gather search that walks outward from the worker instead of measuring from it (D141's queued design - the only version
> where a small radius is safe).
>
> Then: `decide` at 432 lines if shape work is wanted; the 8 parked owner questions.

> # Session 46, 2026-11-07 - D148: only where it has looked
>
> The owner's report (villagers crossing the map to unscouted berries) is fixed at its cause: the picker ranked every node
> in the world; it now asks `World::knows(player, entity)` (the per-player fog), in `nodes_of_slot` (script action + shares)
> and `nearest_like` (node ran dry). RED watched, control passed before the change. **New acceptance baseline
> `dfc835be5d880a47`** (was `f328ebc8d4d34b78`): counts identical, `--dump` diff is one line - node 117 keeps 90 raw units.
> Suite 735/0/19. Review: `docs/reviews/2026-11-07-only-where-it-has-looked.md`.
>
> **OPEN AND ASKED: the "nearby" radius** - the second half of his rule. Among known nodes the picker still takes the
> nearest however far. Options: (A) none, today; (B) the worker's line of sight, 4 tiles; (C) `los + 5`, 9 tiles (D141's
> number, which broke a match when applied to *all* nodes - unknown whether it does with the fog filter). Measuring any of
> them costs one acceptance run plus the two knowledge tests.
>
> Then: the D141 two-stage gather search (bounded BFS from the worker, then the picker), and `decide` at 432 lines.

> # Session 45, 2026-11-07 - D147: a length that is a length, and half a table
>
> Second review [74] [173] [184] [112], the deferred items with a named remedy, both fixed test-first: `duration_seconds`
> was cast with `as i64` (which saturates - `nan` was a zero-tick match that exits 0, `1e300` a match that never ends), and
> `--techs` without `--effects` skipped research in silence. Now `Scenario::ticks(tick_rate_hz)` is the one definition, used
> by both runners and by `Runner::new`, and half a research table is an error naming the missing flag. RED watched on both
> (3 of 4 duration tests; the CLI test with the run exiting 0). Review: `docs/reviews/2026-11-07-a-length-that-is-a-length.md`.
>
> **The review's 205-finding JSON is gone** - `/opt/data/cache/documents/` is pruned. 41 loader + 63 sim findings survive in
> `docs/reviews/`; the **49 tests / 26 config / 16 other** ones were never written out and cannot be recovered verbatim.
> Lesson recorded in D147.5: commit findings as they are triaged.
>
> **Queued with reasons, not lost**: [70] `--dump` CSV escaping (no value it can produce today contains a comma, so no test
> can be watched failing); [71] `players=` is the highest id, not the count (unobservable while seeding gives `1..=n`; needs a
> sparse-player or mass-death fixture); [125] [134] [135] the path/gather search questions, still open.
>
> **Next, in order**: (1) the D141 two-stage gather search - bounded BFS from the worker, then the picker; (2) `decide` at
> 432 lines, if shape work is wanted over correctness; (3) the 8 parked owner questions, which still need an owner.

> # Session 44, 2026-11-07 - D146: stage 2 done, `step` is 30 lines
>
> `decide()` (432 lines, the one pass over the entity list) fills a `TickWork`; nineteen named phases follow it in the order
> the old statements ran. Hash `f328ebc8d4d34b78` unchanged; measured 11.5 s -> 10.8 s over 36,000 ticks (five runs each side,
> non-overlapping). Review: `docs/reviews/2026-11-07-step-from-1562-lines-to-30.md`.
>
> **Options from here, in the order I would take them:**
> 1. **Back to the second review's queue**: 118 of 205 findings verified; tests 49, config 26, other 16 left. That is real
>    work on correctness rather than shape.
> 2. `decide` per task (`decide_attack`, `decide_gather`, ...) - the last big function, 432 lines, one `match` with eight
>    arms. Modest gain, real churn.
> 3. The queued **D141 two-stage gather search** (bounded BFS from the worker, then the picker) - the design the refuted
>    radius bound was standing in for.
>
> **Open owner questions (8)** still parked: loader strictness for wrong-but-harmless values; `Civ::bars` stub; `pack.rs`
> unused `bytes` map; pack hash FNV vs SHA-256; duplicate `at_villagers` rows; army size 8/12 vs 16/36.

> # Session 43, 2026-11-07 - D145: `lib.rs` 8,731 -> 4,106
>
> Sixteen subject files under `crates/sim/src/world/` plus `crates/sim/src/error.rs`. `lib.rs` now holds the crate's nouns
> (types, `World`, constants, free functions, re-exports). Hash `f328ebc8d4d34b78` after each of twelve moves; suite
> 728/0/19. Review: `docs/reviews/2026-11-07-lib-rs-down-to-4106.md`.
>
> **The next job is stage 2, and it is the one that matters**: `crates/sim/src/world/step.rs` is **1,581 lines in a single
> function**. Cutting it into named phases - order intake, walking, gathering, building, training, research, combat,
> bookkeeping - is behaviour-preserving only if done one phase at a time with the hash checked each time, and it is not a
> mechanical move. Suggested first cut: the combat/battle phase, because `world/battle.rs` already holds every helper it
> calls.
>
> Secondary: `battle.rs` 665 could split into skirmish / projectiles / aiming; `orders.rs` and `sites.rs` overlap on
> placement. Neither is urgent.

> # Session 42, 2026-11-07 - D144: four more subjects out of `lib.rs`
>
> `lib.rs` **10,216 -> 8,731** with `world/{seed,train,build,research}.rs`; hash `f328ebc8d4d34b78` checked after **each**
> move, suite 728/0/19 unchanged. Review: `docs/reviews/2026-11-07-four-more-subjects-out-of-lib-rs.md`.
>
> **The remaining split work is two different jobs, and the second is the one that helps a reader:**
> 1. **Stage 1, next slices**: `orders` (`apply` 223 + `validate` 101 + `submit*`), `state` (`state_hash` 343 + `stamp_*` +
>    `bookkeeping_*`), `combat` (`swing`, `volley`, `spawn_projectile`, `true_aim`, `lead_shot`, `blast_victims`,
>    `gap_squared_to`, `in_strike_range`), `sites` (`site_refusal`, `site_has_lane`, `site_fits`, `build_site_tiles`,
>    `footprint_area`, `solid_tiles`, `nearest_drop_site`). Each is a hash-checked mechanical move.
> 2. **Stage 2, the real one**: `step` is **1,562 lines** in a single function - cutting it into named phases is not
>    mechanical and is worth more than the six remaining stage-1 slices together.

> # Session 41, 2026-11-07 - the four are done: two adopted, one reverted, one narrowed
>
> 1. **D140 the binary's module map** - 84 paths, 541 classes, 31 `Comm*`; U12 has a shape. 2. **D141 the gather radius
> bound - reverted by measurement** (141 -> 129 entities, match undecided); the corrected **two-stage** search is queued.
> 3. **D142 reach is box to box** - both call sites, one existing test re-stated. 4. **D143 flat arrays** - 5.7x on long
> searches, routes byte-identical, match unchanged.
>
> **Next, in order of what it unblocks:**
> 1. **The sim-block review's queued head** (24 cheap findings): the enemy selectors can aim at a Gaia town centre or a
>    foundation; a rule naming a role the pack lacks never fires and says nothing; `duration_seconds = 1e300` is a 9.2e18
>    iteration hang in both binaries.
> 2. **The two-stage gather search** (D141's corrected shape): the AI picks an area, the unit picks a node within sight of
>    it. Needs the area concept, so it is a design step.
> 3. **The stall class test** the deck's own regression loop suggests: a unit with a live order that has not moved for N
>    ticks fails, whatever the mechanism.
> 4. **Path budgets** (D143's refused half) as decisions with numbers, not as optimisations.

> # Session 40, 2026-11-07 - steps 1-3 of the four, and the one that failed its measurement
>
> **1. DONE** - the binary's module map (D140): `tools/exe_map.py`, 84 source paths, 541 classes, 31 `Comm*`, U12 given its
> shape. **2. ADOPTED, MEASURED, REVERTED** - the gather radius bound (D141) starved the base: 141 -> 129 entities and both
> players alive at 30 minutes. The corrected shape is queued: **two stages**, the AI choosing an area and the unit a node in
> sight of it. **3. DONE** - reach is box to box (D142), both call sites, one existing test re-stated with its reason.
>
> **4. NEXT - the four pathing techniques** (landmark A* estimate, budget by route length, per-tick search budget, re-sort
> last tick's order). These answer the deferred `path.rs` findings of the second review (a `BTreeMap` per search, no node
> budget). Do it as its own performance step: measure the acceptance match before and after, and put the numbers in the
> review. **Not a behaviour change** unless a budget truncates a search - if it does, that is a decision, not an
> implementation detail.
>
> Also queued from the sim-block review: the enemy selectors (Gaia / foundations), rule conditions naming roles the pack
> lacks, and the batch/headless duration hang.

> # Session 39, 2026-11-07 - the four adopted from the prior-art watch, one at a time
>
> **The owner's instruction:** *"Adopt as proposed one after another. Each step - compact, implement, adversarial review
> on performance and stability."* The order, from the digest's own ranking:
>
> 1. **DONE - the binary's module map** (`tools/exe_map.py`, `docs/binary-class-map.md`, D140). 84 source paths, 541
>    classes, 31 `Comm*`; the DE build root as provenance; U7's six-pather shape confirmed from a second source; U12 given
>    its shape (turn sync + cmd count + checksum) with the rate left open and the arithmetic written down.
> 2. **NEXT - the gather search's radius bound** (genie's `LOS + 5`, at most 30 objects tested). Ours has no radius bound
>    at all after D138, so a villager's "nearest" can be across the map. **Gameplay: the owner's call** - he adopted the
>    list as proposed, so implement it; the test is a node beyond the bound not being chosen.
> 3. **Attack reach box-to-box** (openaok). We measure centre-to-centre; the original measures between the dat's boxes,
>    and since D76 we *have* building footprints, so the digest's "missing datum" caveat has expired. Changes the
>    acceptance match's combat, so measure the hash and report it.
> 4. **Four pathing techniques** (landmark A* estimate, budget by route length, per-tick search budget, re-sort last
>    tick's order) - the answer to the deferred `path.rs` findings, as its own performance step with numbers before/after.
>
> Each step: compact (this file), implement test-first, adversarial review with Performance **and** Stability sections,
> commit and push.

> # Session 38, 2026-11-07 - D138: one picker, and it asks nothing
>
> **The owner chose B.** The script's `nearest_resource` no longer runs a path search per candidate node (90 wood nodes on
> the acceptance map) and no longer asks the wrong question (`can_reach` on a tree's own blocked tile); it delegates to
> `World::nodes_of_slot`, which is the economy pass's picker and already carried the measured rule. Failures are booked in
> `share_failed` by `start_gather` **and** by `submit_checked` - the second matters because a walled node is refused in
> `validate`, which is `&self` and cannot book. Two REDs, both watched (pre-filtering, then 29 refusals in 30 s).
> Acceptance byte-identical; **720 passed / 0 failed / 19 ignored**.
>
> **Next, in order**: (1) the sim block of the second review (63 findings, still unread); (2) the remaining 34 loader
> findings; (3) the stall class test the deck's regression method suggests (a unit with a live order that has not moved for
> N ticks). **Owner questions still open**: `Civ::bars`, `pack.rs` bytes map, FNV vs SHA-256, duplicate `at_villagers`,
> army size, loader strictness.

> # Session 37, 2026-11-07 - D131 decided: no pushing allowed
>
> **The owner chose A.** `bound_to_reach` in `world/movement.rs`: rules 4 and 9 no longer step a body into another's reach.
> Scoped by measurement twice - to **two players' units** (bounding every pair cost 23 entities and left both players alive:
> 164 vs 141, refusals 574 -> 1,021) and **never to a body at rest** (a node's fallback reach is a whole tile; it stopped a
> villager 5 tiles from its tree and failed seven tests). With both scopes the acceptance match is byte-identical
> (`f328ebc8d4d34b78`, 141 entities, 188 repaths, 11.7 s).
>
> The corridor test is renamed `..._never_overlaps`, spawns **two players'** villagers, and asserts the invariant **and** that
> they met. **719 passed / 0 failed / 19 ignored.** Review: `docs/reviews/2026-11-06-no-pushing-allowed.md`.
>
> **Next, in order**: (1) the sim block of the second review (63 findings, unread); (2) the remaining 34 loader findings;
> (3) D131's honest follow-up - an **enemy** pair that jams in a corridor now waits forever unless the repath finds another
> way, so a "unit with a live order that has not moved for N ticks" class test is the cheap guard the deck's own regression
> loop suggests. **Owner questions still open**: 2 (`Civ::bars`), 3 (`pack.rs` bytes map), 4 (FNV vs SHA-256), 5 (duplicate
> `at_villagers`), plus reachability option 2 and army size 3.

> # Session 36, 2026-11-07 - D137, every name must exist
>
> The loader block's best finding, implemented test-first: **six places accepted a role or tech-role name the pack does not
> have** (`build.requires` both sides, `build.blocking_roles`, `footprint` keys, the four `hunt` lists, `age.requires` keys
> and `of`/`single`), while four others were already refused. Seven tests, **watched failing first** (30 passed / 7 failed),
> each swapping one real name in `config/economy.toml`; the refusal names the key and the name. `[metrics] military_roles`
> is **refused** as a finding - the pack's own comment says a metric is a measurement, not a rule (D137.4).
>
> **The shipped pack was clean before the check existed** (0 mismatches over 40 roles + 6 tech roles), so the acceptance hash
> `f328ebc8d4d34b78` does not move. Review: `docs/reviews/2026-11-06-every-name-must-exist.md`.
>
> **Open**: the other 34 findings of that block (17 confirmed, 15 low, 3 partly, 2 refuted - verdicts in
> `docs/reviews/2026-11-07-second-review-the-rest.md`), then sim 63, tests 59, config 26. **Owner questions still open**:
> loader strictness for wrong-but-harmless values; `Civ::bars`; `pack.rs`'s `bytes` map; FNV vs SHA-256; duplicate
> `at_villagers` rows.

> # Session 35, 2026-11-07 - the second review's loader block, and the split
>
> **Two things, both pushed.** (1) **D136**: `lib.rs` split, three slices - `world/movement.rs` (369), `world/memory.rs` (280),
> `world/gather.rs` (357); `lib.rs` **11,073 -> 10,103**; the acceptance hash `f328ebc8d4d34b78` identical after every slice,
> which is the proof that a move changed nothing. Remaining files: `orders`, `production`, `combat`, `state`, `selection`;
> then stage 2, cutting `step` (1,559 lines) into phases. (2) **the second review's loader block**: 41 findings against
> `crates/data/src/` given a verdict each - **17 confirmed, 15 confirmed but low, 3 partly, 2 fixed tonight, 4 refuted** -
> in `docs/reviews/2026-11-07-second-review-the-rest.md`. The best find is one defect in three hats, narrower than filed: the economy
> loader refuses a `build.farm`, a `build.spaced_roles` entry and an `age.order` name that are not in its tables, and accepts
> the same mistake in `build.requires`, `build.blocking_roles`, `footprint` keys, the four `hunt` lists and `age.requires`.
> My first draft of the review said it checked none of them - corrected before the commit.
>
> **Still to read from that review**: sim 63, tests 59 (10 already settled), config 26, other 16.
>
> **Open questions for the owner** (asked with no deadline, in the review's last section): loader strictness for wrong-but-
> harmless values; delete or implement `Civ::bars`; wire `pack.rs`'s `bytes` map through or delete it; FNV vs a cryptographic
> pack hash; refuse duplicate `at_villagers` rows or let the later one win.

> # Session 34, 2026-11-06 - the 17x tick was a failed re-plan a tick (D135), and D134 merges
>
> **Session 33's open question, answered the same evening.** The D134 match ran at 3.5 ms a tick against 0.3 with the same
> 23 buildings / 31 units / 101 nodes, the same tasks, path lengths and plan count. Splitting the headless loop put it in the
> **simulation** (28.9 s against 1.7 s; AI unchanged), and the instrument that named it was the re-plan path: **2,559
> attempts, 2,533 failures, 8.1 s of a 20 s run, 3.2 ms per failed search.**
>
> **Why the cap never engaged:** D127's two-per-route budget counts `Route::stalls`, incremented **only on success**. Ground
> that answers "no" was asked again every tick - and for a unit with **no route entry** at all (`paths.get_mut` → `None`,
> the case this match was in) there was no counter to fail to increment.
>
> **D135:** a failed re-plan spends one of the two attempts, and creates the route entry when there is none. Measured:
> attempts **11,260 → 192**, failures **11,070 → 4**, re-plan time **35.4 s → 13.3 ms**, match **47.5 s → 11.8 s**, and the
> **hash does not move** (`f328ebc8d4d34b78`, 141 entities, 187 made_room) - every search removed decided nothing.
> Test first: `crates/sim/tests/repath_budget.rs` asserts at most two failed re-plans in 600 ticks, watched failing at
> **177**. Review: `docs/reviews/2026-11-06-a-failed-replan-costs-a-tick.md`.
>
> **D134 merges with it** (`d134-click-accepted` → `main`): the click rule, its four tests, and the walls test that now
> asserts the ring is not crossed rather than that the order is refused. Acceptance baseline moves by design -
> **`f328ebc8d4d34b78`** / 141 / 190 repaths - with walled-off refusals **203 → 1** and the wall clock back at ~11.8 s.
> **711 passed / 0 failed / 20 ignored.**
>
> **Two lessons worth keeping:** a cap that counts successes caps nothing on the case it exists for; and a fixture that does
> not reproduce the bug passes for free - the first `repath_budget` map (walls five tiles wide) could be walked around and
> went green before the fix.
>
> **Still open:** the circle-clearance rule (*"the circle has to stay away from tree square"*, every body, tangency allowed -
> the other half of the owner's bug report), D131's head-on overlap, and the army-size pair.

> # Session 33, 2026-11-06 - every click is accepted (D134), and the tick that costs 17x (open)
>
> **The owner, on a wedged scout:** *"clicking into the middle of the forest ends up with error message. It has to be
> allowed. Scout cavalry will simply try to close to the point as close as it can without running into trees."* Nine
> questions were answered the same evening; the answers are in D134.
>
> **Landed on the branch `d134-click-accepted`, not on `main`**: `World::walk_goal` turns a walk order's target into the
> **nearest legal standing point** when the order is given (blocked *or* unreachable), `nearest_reachable` in `path.rs`
> answers it, and `Move`/`AttackMove`/`Patrol`/`WalkTo` are never refused for the ground. Gather, build and attack keep their
> refusals - two existing tests (`a_node_with_no_path_is_refused_at_order_time`,
> `an_unreachable_site_is_refused_before_the_wood_is_spent`) fail if the snap goes into the shared planner, which is how the
> first attempt was built. **710 passed / 0 failed / 20 ignored**; the review is
> `docs/reviews/2026-11-06-every-click-is-accepted.md`.
>
> **Why it is a branch.** In the acceptance match the click costs nothing measurable (the fallback ran **twice**, 16 node
> expansions) but the match it produces runs **3.5 ms a tick against 0.3** from tick ~10,000: **simulation 28.9 s against
> 1.7 s**, AI unchanged. At tick 12,000 both worlds have 23 buildings / 31 units / 101 nodes, the same tasks, the same path
> lengths and the same plan count - nothing countable differs, only where the bodies stand. **That is a defect of the
> engine, not of the click**: a tick whose cost is not a function of the entity count. `perf` is not installed here and
> every counter I could add without it is equal between the runs.
>
> **Next step is that slow state**, not D134: dump the world at tick 12,000 of the branch's match (hash `e78d90b6aae58e74`)
> and bisect the per-tick cost by entity, task and tile. When it is understood, D134 merges and the acceptance baseline
> moves by design (`f328ebc8d4d34b78`, 141 entities, 190 repaths, walled-off refusals **203 -> 1**).
>
> **Still open from the owner's list:** D131's head-on overlap (his own argument - a villager is thinner than a tile - favours
> letting bodies take a side), the army-size pair, and the circle-clearance rule (*"the circle has to stay away from tree
> square"*, every body, tangency allowed), which is the other half of this bug report.

> # Session 32, 2026-11-06 - passage is the surface (D132) and the match is thirty minutes (D133)
>
> **The owner answered the two content questions that needed no research** (of the six in Session 31's list):
> *"If dirt is a surface after the tree was cut and fully gathered from to have 0 wood, then grass and dirt passable, wood
> impassable"* and *"30 minutes is enough"*. Both landed test-first, both watched failing before the code moved.
>
> **D132 - two predicates, told apart.** `Map::is_walkable` (a town-centre site, a seeded body) accepts **grass or dirt**;
> `Terrain::walkable()` (a route) keeps grass, dirt **and** forest, because terrain is ground and the tree is the obstacle
> (D84, the owner's 2026-10-31 rule). A cut wood needs no terrain change: `Walk` counts **occupants**, so the tile frees when
> the node goes - which is the mechanism the owner described and the code already had.
>
> **D133 - one number in three places.** `duration_seconds = 1800.0` (was 2400), the prose already said thirty, and the
> canonical command plays 36,000 ticks = 1,800 s at 20 Hz. A test now ties the file to the command.
>
> **Measured**: acceptance **unchanged** - hash `394c4892aac776d2`, 141 entities, 184 repaths, 187 made_room - because the
> seeder's candidate tiles were grass already; the headless refusal log is identical line for line before and after, and the
> match is still decided at **1,527.6 s**, so the shorter limit truncates nothing. Timing 10,936 / 11,346 / 11,345 ms, median
> **11,346** - not a claim, the same hash means the same match. **707 passed / 0 failed / 20 ignored**, clippy and fmt clean.
>
> **Still open from the six**: 1 (D131 - the three options, with the owner's own point that a villager is thinner than a
> tile, which argues for B), 2 (chop pace: already the dat's - VMBAS `attacks = [3, 3, 6, 0]`, `reload_time = 2.0`, so
> 3 hp / 2 s = 1.5 hp/s), 3 (army size: the game's own scripts say `sn-minimum-attack-group-size 10`,
> `sn-maximum-attack-group-size 80`, and `Dr3/shared.per` attacks with **16-36 in four groups**), 6 (reachability in the
> node lookup - the bounded designs are priced in the reply).

> # Session 31, 2026-11-06 - the harness's first dividend: a head-on pair overlaps by one step (D131)
>
> **Step 1 of the evening is pushed (`62fa76a`)**: D130, the loader that no longer turns "I could not read the content" into
> "the test passed". Its first dividend arrived within the hour - see Session 30 above - and this block is the finding it
> produced, which needs one decision before any code moves.
>
> **What fails.** `crates/sim/tests/footprint.rs::a_head_on_pair_in_a_one_tile_corridor_passes`, a test that had never
> executed because its inline map was refused by the loader (wood on grass) and `.ok()?` ate the refusal. With the map fixed
> it runs and breaks its own invariant: two villagers walking head-on come **7,372 raw units** apart on tick 66, against the
> **8,000** their footprints need - an overlap of **628 raw**, one step (1,000 raw, 0.05 tiles) minus the 372 they were
> already inside by.
>
> **The mechanism**, read in `collide` and checked against the arithmetic: rule 9, `wait_or_give_way`, returns the **full**
> step for the unit that gives way, so the giver closes from exactly `reach` to `reach - step`. Rule 8 (a body at rest) is
> not the cause: at tick 66 both are mid-route.
>
> **The invariant itself is sound and asserted elsewhere**: `collision.rs` checks every pair of a three-unit converging
> crowd after 600 ticks, and passes. What breaks it is a **head-on pass in ground too narrow to pass in**.
>
> **Three options, priced in D131 §7.** A bounds the give-way step (no overlap, but a one-tile corridor deadlocks and the
> open jams after rule 6's single nudge); B makes bodies slip past laterally (a gameplay change, with a measured mosh-pit
> regression attached to removing the nudge cap); C keeps the behaviour and bounds the invariant at `reach - one step`
> (no engine change, no hash change, and it writes the 0.05-tile overlap down as a rule). **Recommended: C.**
>
> **This is the owner's call, not mine** - D110 already recorded that the original's "units bump into others, stop and
> repath around; no pushing" is a gameplay rule and is not adopted until he says so, and B is the first push-like mechanic
> the engine would own.

> # Session 30, 2026-11-06 - a fixture that cannot read its pack says so (D130)
>
> **Where session 29 ended:** `24f72cc` pushed, **702 passed / 0 failed / 19 ignored**, and five questions waiting on the
> owner - none of them answerable by me (chop pace, `army_at_least` against its own prose, Grass-only `is_walkable`,
> `duration_seconds`, reachability in the node lookup). The owner, in a new thread: *"Please overtake [...] and continue
> with findings."* So this session takes the first item the triage left unfixed and needs no decision: **the harness.**
>
> **The hole.** 36 of the 82 `crates/sim/tests` files load the pack through `ruage_data::load_*(..).ok()?` - **220** loader
> calls, plus the `World::new` / `seed_from` / `load_*` calls beside them - so **every** load error becomes `None`, `fresh()`
> returns `None`, and each test takes the `else { return }` at its head and reports **passed**. Truncate
> `config/economy.toml` and a third of the suite goes *greener*, not red. The other 46 files in the same directory already do
> it right (`expect("economy.toml")`), which is what makes the minority a defect rather than a house style.
>
> **What the drift had already built.** Eight hand-copied `fresh()` bodies differing only in which tables they load, and
> `farm_ring.rs` still carrying a comment - *"Enough wood and food that a rule is refused for placement, never for price"* -
> over `for slot in 0..4 { let _ = slot; }`. The stockpile it describes was deleted; a no-op loop compiles, so nothing said so.
>
> **D130 - the rule.** The generated pack's *absence* skips a fixture; a pack that is *present and will not load* fails it,
> with the file's name and the loader's error. One loader in `crates/sim/tests/common/mod.rs`, five entry points for the five
> shapes the copies had, `present()` for every other table.
>
> **Slices, one commit:** (1) the extraction, behaviour unchanged, suite still **702 green**; (2) the strictness, with a
> meta-test over a **broken copy** of the pack and over a **directory with no pack**, RED first; (3) the suite-level A/B -
> truncate `economy.toml` and run one test binary: **green before, red after**.
>
> **Found on the way (D131).** The first fixture that ran once the harness stopped skipping failed: `footprint.rs`'s
> corridor map put its wood nodes on grass, the loader refused the map, `.ok()?` ate the refusal, and
> `a_head_on_pair_in_a_one_tile_corridor_passes` had **never run**. With forest rows under its trees it runs, and it fails -
> the pair overlaps by **628 raw units** at tick 66 (7,372 against the 8,000 their footprints need). Ignored with that
> reason, and the repair is the next step, not this one.
>
> **Then:** adversarial review with a measured number (nothing on the tick path changes, so the measurement is the suite's
> own wall time and the acceptance hash, which must not move), then push. The five session-29 questions stay open and are not
> guessed at.

> # Session 29, 2026-11-05 - a tree took one blow; and the second review, triaged (D129)
>
> **Second external review**: 205 findings over 176 files, `kimi-k2.7-code`, 42m45s, **partial** (107 of 176 items failed).
> Triaged in `docs/reviews/2026-11-05-second-review-triage.md` - checked, right; checked, wrong; and what was not checked.
>
> **The catch of the day, filed as a performance complaint**: `fell_hp_per_tick` **multiplied** the per-second chop rate by
> the tick rate instead of dividing. 1.5 hp/s at 20 Hz became **30 hp a tick**, so a 20 hp tree fell to its **first** blow -
> 0.05 s against an intended 13.3 s - and the felling rule has never run in a match. No test asserted the rate; the depletion
> tests ask with `amount`, which instant felling satisfies.
>
> **Fixed (D129)**: the rate is converted once at load to `Gather::fell_milli_per_second`, and `World::fell_carry` keeps the
> remainder per node (one whole point is `1_000 * hz` of it), hashed as state. Three tests watched failing first; a tree now
> falls on tick **267** at 20 Hz - 13.35 s, the seventh chop.
>
> **Acceptance changes by design**: `e4bdd521acaaf342` / 152 entities / 237 repaths -> **`394c4892aac776d2`** / **141** /
> **184**. Timing unmoved: **12,383 ms median against 12,391**. **702 passed / 0 failed / 19 ignored**.
>
> **Five questions for the owner** (they need his decisions, not mine): the chop pace; `army_at_least = 3` vs a comment that
> says eight; Grass-only `is_walkable` against D117's dirt and forest; `duration_seconds = 2400` in a file that says
> "thirty-minute"; and whether to revisit reachability in the node lookup, which two reviews now want and which our own
> measurement says cost 37x the run.

> # Session 28, 2026-11-05 - the external review, checked (D128)
>
> **The owner handed over a 141-line external static review and said "please scrutinize".** Fifteen findings, checked one by one
> against the tree: **nine confirmed, two wrong in severity, four low-value.** Line numbers were accurate; the reviewer could not
> compile anything and says so.
>
> **The two overstatements, with the arithmetic:** `raw * 2` needs a position above **4.6e18** (no map at a scale that fits
> `i64` can hold one); squared distances overflow only above **scale 32,768 on a full 65,535-tile map**, and the shipped pack is
> 64 tiles at 20,000 = **3.2e12** against `i64::MAX` **9.2e18**. Right in principle, not live.
>
> **Landed:** a **load-time refusal** for a pack whose geometry cannot be squared (`GeometryOverflow`, `max_scale_for`, tested
> against the arithmetic itself); panics -> errors in `headless`, `batch` and `frame`; **one lookup instead of four** in the share
> pass; `i128` for the reflection and the stat percentage.
>
> **Measured:** acceptance **13,520 -> 12,391 ms (-8.3%)** with the hash **unchanged** (`e4bdd521acaaf342`), which is what makes
> the comparison a like-for-like - the same match, faster.
>
> **Deferred with reasons:** the `EntityId -> index` map (the real fix for the linear `find`, and a slice of its own);
> `nearest_resource`'s A* per node - the review's best performance finding, needing the short-list treatment and a measurement;
> the footprint-mask assert; the O(n²) RNG check.
>
> **What the review could not see** - behaviour, and it missed the three defects that running found this week (the O(n²) husk
> pass, the stale-route freeze, the minimap seams). That is the case for this project's method, not a mark against its.
>
> **699 passed / 0 failed / 19 ignored**; clippy and fmt clean.

> # Session 27, 2026-11-05 - a stale route is re-planned (D127)
>
> **The owner:** *"I still see scout bumping into trees and getting stuck. Can we learn something from
> https://github.com/glouw/openempire?"*
>
> **The gap, precisely:** `collide` re-planned when a **body** blocked a step and did nothing when **ground** did - slide on one
> axis if possible, otherwise stand still, for ever. A route goes stale for ground reasons constantly: a house built across it, a
> nudge off the line, a pulled corner clipping a tree.
>
> **The fix:** a ground refusal asks for a re-plan, **at most twice per route** (`MAX_GROUND_REPLANS`), through the existing
> per-tick budget; the count is on the route and in the state hash. Test: a villager walking a clear lane with a house dropped
> across it twenty ticks in - it stopped at `(659850, 810000)` for 2,400 ticks before, and arrives now.
>
> **From `openempire` (`src/Units.c`)**, notes in `docs/sources.md`: it re-plans **every path every 25 cycles**
> (`CONFIG_UNIT_PATHING_TIMEOUT_CYCLES`), ten slices a tick, unconditionally - no reason needed - and its boid layer adds a
> **wall push** that slides units off obstacles. Copied: the principle. Not copied: the timer (a search per unit per 25 ticks,
> more work than a stall check) and the wall push (float steering, forbidden in the tick by D9, and it would let a body through a
> gap D117 closes).
>
> **Cost, measured:** 36,000-tick match, three runs each: **11,944 ms -> 13,520 ms (+13%)**, with 154 -> 152 entities and
> **211 -> 237 repaths** - the match now goes further because stuck villagers arrive. Hash `4568cef16368bd5a` ->
> `e4bdd521acaaf342`, still a win.
>
> **696 passed / 0 failed / 19 ignored**; clippy and fmt clean.

> # Session 26, 2026-11-05 - the minimap's crossing lines were seams (D126)
>
> **The owner:** *"Minimap has 2 crossing lines artifact."*
>
> **The cause, exactly:** the minimap painted one diamond per tile with `ry = step / 2`. On a 64-tile map in a ~230-pixel box the
> step between tile centres is **1.82 px**, so `ry = 0.91` - under one pixel - and `fill_diamond` then draws **a single row** per
> tile. Tile rows are 1.82 px apart, so about every other row was painted; the missing rows are the gaps, and they run along both
> diagonals of the grid: the two crossing lines.
>
> **Fix:** `ry` floored at **1.5 px**. `rx` keeps the full step, as before, so for any minimap whose step is 3 px or more the
> radii are identical and the work is unchanged.
>
> **Two fixes measured and rejected:** a per-pixel fill (correct, **4.4 ms** HUD band against 2.0 ms - `minimap_tile` is a float
> inverse per pixel, 29k times) and a half-pixel overlap on every diamond (**+1.5 ms**: a third more pixels each, 4,096 tiles).
>
> **Measured by seam count, not by time:** **182 -> 0** panel-coloured pixels inside the map diamond, plus a second test at the
> owner's own box size. No timing claim, because for the shipped map the work is identical and `frame_cost`'s difference method
> swings between 746 and 1,847 us on the same binary.
>
> **694 passed / 0 failed / 19 ignored**; acceptance unchanged `4568cef16368bd5a`; clippy and fmt clean.

> # Session 25, 2026-11-05 - the ring is a curve, not a diamond (D125b)
>
> **The owner, looking at the rendered frame:** *"I see a diamond, not elipse in the example"* - and he was right. D125 claimed an
> ellipse and drew `outline_diamond`: a rotated **square** projects to four straight edges, and a square is not a circle. The
> reasoning in the first version - "a circle in tile space projects to a diamond" - was wrong, and it is corrected in the review.
>
> **Landed:** `Frame::outline_ellipse` - `half = rx * sqrt(1 - (dy/ry)^2)` per row and the mirror per column, both axes walked so
> a shallow slope does not leave the curve dashed. Units use it; **buildings keep the diamond**, because a footprint *is* a tile.
>
> **The test pins the difference where the shapes disagree most**: at half height an ellipse is still **86.6%** of its width and a
> diamond has closed to **50%**. RED, verbatim: *"at half height the ring is 19 px from the centre; an ellipse would be 31, a
> diamond 18"*. Measured on the rendered frame after the fix: half-width **14 px** at the middle, **12 px** at 50% height -
> ratio **0.86**.
>
> **693 passed / 0 failed / 19 ignored**; acceptance unchanged `4568cef16368bd5a`; clippy and fmt clean.

> # Session 24, 2026-11-05 - a ring the size of the body (D125)
>
> **The owner:** *"circle enveloping the units ... Smaller units have smaller circle ... bigger units have bigger circle ...
> centered on the center of the base of the unit"*, then the answers: **selected only**, **the pack's radius times `k`**
> (chosen `k = 1.5`), **an ellipse on the ground plane**.
>
> **Landed:** a selected **unit** wears a ring of `radius * ring_scale` tiles centred on its feet, drawn as a diamond outline
> (a ground circle under the isometric projection). The size was already in the pack - 0.2 infantry/villager, 0.25 cavalry,
> 0.3 some archers, 0.5 siege/ships - and it is the same number the simulation uses for standoff (D117). `k` is content:
> `[ring] scale = 1.5`; `1.0` makes the ring *be* the collision circle.
>
> **Buildings keep their footprint ring** - a farm's radius is 1.5 and would draw a ring inside its own walls.
>
> **`no_float` caught my first accessor** (`World::ring_scale() -> f32`); it now answers `ring_scale_percent() -> i32`, and the
> guard is green. That is the rule doing its job on a *display* value, which is the case most likely to slip through.
>
> **One of my measurements was wrong, again a measurement and not the code:** `ring_width` counted the longest horizontal run
> of ring pixels; an outlined diamond has two pixels on its widest row, so it read 7 px for every unit and "proved" a bug that
> was not there. It measures the extent between the leftmost and rightmost ring pixel now.
>
> **692 passed / 0 failed / 19 ignored**; acceptance unchanged `4568cef16368bd5a`; frame 7,972 µs at 960x720 / 128 entities,
> HUD band 2,336 µs - unchanged in kind, because the ring is still one outline per selected unit.

> # Session 23, 2026-11-05 - the husk is drawn as its farm, crossed out (D124)
>
> **The owner:** *"As of now, husk can get the same pictogram as farm but crossed over."* The card of a spent farm now draws the
> building's own square with **two diagonals** in the refusal colour, one pixel at a time - there is no line primitive and a
> diagonal is not a rectangle.
>
> The chain is the one D122 built: `is_expired_farm_ref` (sim) -> `EntityView::is_husk` -> `SelectedRow::is_husk` -> the cross, so
> the picture cannot disagree with the rule about what a husk is. The pictogram itself is still the empty framed square until the
> husk unit (`FARM_`, 357) has art; the flag is what will tell the card which pictogram to reach for when it does.
>
> **689 passed / 0 failed / 19 ignored**; acceptance **unchanged** at `4568cef16368bd5a` (a drawing-only slice); clippy and fmt
> clean.
>
> **Not measured, and said so in the review:** the tick (6.2 us) and the snapshot (95.3 us) are unaffected by construction - no
> simulation code and no per-entity view work changed - and `hud_cost` selects a villager, so it would report the same number
> with or without the cross. Running it to "confirm" would be measuring nothing.
>
> **Still ahead:** the farm **replaced by unit 357** on exhaustion, so the husk is the game's own unit rather than a flag on the
> farm (evidence in D123); and the husk's art.

> # Session 22, 2026-11-05 - the names were already there (D123)
>
> **The owner:** *"1 A. I may rename them later on. IIRC language DLL files should contain proper names. That way it was possible
> to translate the game in hex editor. Age2HD could do it differently though. If none is found there or in other files, follow
> A"* and *"2 can you find in original files? It must be somewhere there. Otherwise OK."*
>
> **1. The names exist, and were already in use.** `resources/en/strings/key-value/key-value-strings-utf8.txt` says in its own
> header *"Strings from Game/en/language.dll"*; the dat gives every unit a `name_str_id` (83 -> 5121 "Villager", 50 -> 5149
> "Farm", 359 -> 5409 "Halberdier"); and `tools/names.py` (D97) has been joining them into **`config/names.toml` - 1,390 keys
> for 1,389 units, no duplicates, no gaps**. **Option A is unnecessary and nothing needs deciding.**
>
> **2. My D119 inventory measured the wrong column** - the dat's five-letter codes in `units.toml`, not `names.toml`. "56
> unnamed, 28 duplicated" was a fact about codes. Corrected in the plan, in D123, and in the tool, which now reads
> `names.toml` and takes `--root`.
>
> **3. The gate had no tests, and one of them found a real bug**: the cycle walk was bounded by the parsed file's top-level key
> count (1), so a **two-unit** cycle - the smallest, and the shape the real pack has - was never reported. Six fixture tests
> now pin: no key, duplicate key, a role naming a missing unit, a cycle, and a unit whose dat code is unreadable but whose key
> is fine. The real pack still reports its own cycle (`RMSHP` 527 / `SDGAL` 1104).
>
> **4. The husk is in the dat.** The farm (50) has **`dead_unit_id = 357`** - `FARM_`, string 5407, hp 0,
> `resource_decay_rate = 1.0` - the "effect only" husk exactly as described. The next slice is to **replace the farm with unit
> 357** on exhaustion and let its decay remove it; *what* its 1.0 decays is not in the dat, so **30 s stays ours**.
>
> **5. "Nearby" has no constant in the original files** - only `sn-lumber-camp-max-distance` / `sn-mining-camp-max-distance`
> (commented out) and `sn-camp-max-distance`, nothing for farms. **12 tiles stays ours.**
>
> **No engine code changed**; **688 passed / 0 failed / 19 ignored** unchanged, acceptance hash `4568cef16368bd5a` unchanged.
> New: `tools/test_vocabulary.py` (6 tests, python).

> # Session 21, 2026-11-05 - the adversarial review that found five things
>
> **The owner:** *"Please do complete adversarial review on performance and stability"* over D120 (the card), D121 (job names)
> and D122 (farms). Attack pass, not summary: **five defects, four fixed here, one a rule the owner had stated and the code had
> not.**
>
> 1. **"Nearby" was never implemented** - the second farmer was sent to the nearest free farm *anywhere*. Now `[farm]
>    free_farm_radius = 12` (content) and a test with the only free plot 40 tiles away, where he stands.
> 2. **The husk pass was O(n^2)**: `is_expired_farm(entity.id)` does a linear `find`, per entity, every fourth tick. Measured
>    **step 5.9 -> 7.4 us (+23%)**; fixed to test the entity in hand, back to **6.0-6.2 us**.
> 3. **The snapshot was O(n^2) too**, and re-derived every card per entity (cloning attack/armour lists, resolving fractions
>    through decimal text). Fixed: entity-based husk test plus a per-`(owner, unit)` cache. **Snapshot 111.4 -> 95.3 us
>    (-14.4%)** - cheaper than before the card existed.
> 4. **A husk removed under a live task**: tested that the villager is not removed with it and does not keep the dead node.
> 5. **Two orders in one tick** (a box selection): tested, two plots, never one crop shared.
>
> **My own correction:** the D122 review blamed the whole +14% match time on "the match doing more work". Half right - the match
> *is* bigger (154 vs 148 entities, 211 vs 124 repaths), **and** the pass cost +1.4 us a tick. The microbenchmark separated
> what the wall clock could not.
>
> **New tool:** `crates/view/examples/step_cost.rs` - times `World::step` and `Snapshot::of` separately, so a slice that touches
> either can be priced rather than described. `crates/view/examples/hud_cost.rs` does the same for the HUD.
>
> **Still open, named:** `nearest_free_farm` is O(entities) per redirected order; `is_farm_role` costs a role lookup per entity
> in the husk pass (a cached farm-unit set would remove it); the husk has no art; and no test replays a match across a husk's
> expiry - the replay suite should own that.
>
> **688 passed / 0 failed / 19 ignored**; clippy and fmt clean; acceptance hash **unchanged at `4568cef16368bd5a`** (the radius
> did not change this match, which is worth knowing before assuming a correctness fix is behaviourally free).

> # Session 20, 2026-11-04 - one farmer a farm, and a husk that goes (D122)
>
> **The owner:** *"Only 1 farmer per farm (can be constructed by multiple, but only 1 can farm once construction is finished).
> Once farm expires, it decays and can/needs to be replanted. It will cost 60 wood"*, then *"1 A but vanishing takes some time,
> husk first ... Husk is effect only and only serves the player who can left click villager, right click on the husk and
> villager starts planting new farm. It obstructs nothing as if it was not there"*, and *"2 C if there is no empty farm nearby,
> villager becomes idle."*
>
> **Landed:** one farmer a farm (the second is sent to the nearest free farm, or stands idle); an empty farm becomes a **husk**
> tracked in `World::husks` and removed after `[farm.depletion] husk_seconds = 30`; a husk is **walkable** (tested, not
> trusted) and **clickable with a villager to replant** - `EntityView::is_husk` + a `Build` order for the farm the husk was, at
> the pack's 60 wood. An enemy's husk is not ours to replant.
>
> **Checked and cleared:** the farm's price was already right. `FARM` (unit 50) costs `[[1, 60]]` = 60 wood. My first "50 food"
> came from reading unit **259**, the farmer *villager variant*, not the building.
>
> **Cost, measured:** whole 36,000-tick match, `HEAD` against this tree, three interleaved runs: **10,389 ms -> 11,887 ms
> (+14%)**, and the +14% is the match doing more work (148 -> 154 entities, **124 -> 211 repaths**: farms vanish, so they are
> rebuilt). The new passes are O(entities) every fourth tick.
>
> **The hash moved, as it must:** `82efabd6f614c798` -> `4568cef16368bd5a`, still a win (`players=1`).
>
> **685 passed / 0 failed / 19 ignored**; clippy and fmt clean.
>
> **Next, and not done here:** the husk is a *drawing* the engine keeps alive; the renderer has no husk art yet, so a spent plot
> draws as the farm it was until it leaves. That belongs with the asset pack, like the portraits.

> # Session 19, 2026-11-04 - a villager is called a Forager (D121)
>
> **The owner chose the key** for the job words: **A**, the target's **role name** in `economy.toml`. Landed as a superset of
> that, and the reason is in the data: the reference pack has **40 roles and none of them is a tree, a bush or a mine** - they
> cover buildings, animals and soldiers - so a role-only key would have left the woodcutter, the forager-by-bush and both miners
> with no name. `config/jobs.toml` therefore accepts **`target`** (a role: `farm` -> Farmer, `sheep` -> Shepherd, `deer` ->
> Hunter) **or `column`** (a resource: `wood` -> Lumberjack, `food` -> Forager, `gold`/`stone` -> their miners); a role is
> checked first so a pack that later adds a `tree` role overrides the column entry without moving anything; an entry with
> neither key is refused at load, because a rule that can never match is a rule the author meant to be a rule.
>
> **The engine says which target, content says the word.** `World::work_target(id)` -> the target's role and column; `Jobs::
> name_for(role, column)` -> the word; `ruage_view::label_for(job, role, fallback)` -> one precedence rule used by the window
> and the diagnostic picture alike.
>
> **Verified end to end, not asserted:** the diagnostic binary prints what it called the thing it drew (a 7-pixel font cannot
> be read back reliably). Tick 400, the villager the AI put on berries: `selected 2 is "Forager"`. Tick 60, before any work:
> `selected 2 is "villager"`. Both the job path and the fallback, from the real pack.
>
> **Process error, reported:** the loader and the accessor were written before their tests in this slice, which is not the order
> D7 asks for; the tests were written immediately after and do catch regressions, but the rule was still skipped once.
>
> **680 passed / 0 failed / 19 ignored.**

> # Session 18, 2026-11-04 - a card in the bottom bar (D120)
>
> **The owner:** *"let's add a functionality where unit stats are shown in the bottom central part. the big long health bar can
> be removed. For Villager: renamed to match the task he is doing and show stats and carried resources ... For units see
> Selection-Bar.png for ui example"* → **landed**: cards, the wide bar gone, the numbers on the card.
>
> **Answers taken before coding (the clarify was unreadable in Discord, so they were restated as text):** 2A job names come from
> content, 3A a card per selected unit, 4A the wide bar goes and the small one under the portrait stays, 5A attack + both
> armours + range.
>
> **What the card is:** name, an **empty** portrait frame (no art yet - an honest gap, not an invented silhouette), the small
> health bar with `hp/max`, then `ATK`, `ARM m/p`, and `RNG` (or `WRK` for a worker) plus `WOOD 7/10` for a load in hand. A
> resource node prints `HP n` and its remaining amount.
>
> **Where the numbers come from:** `World::stats_of(player, unit)` → `UnitStats`, **after the owner's upgrades** (D112's rule
> applied to every number). Attack is `max(class 3, class 4)`: an archer reads 4, a villager reads 3 and not the 6 of its
> anti-building bonus. Fractional values cross the boundary as **hundredths** (speed 96 = 0.96), so no float can reach a tick.
>
> **Cost:** measured with `cargo run --release -p ruage-view --example hud_cost` against a `HEAD` worktree, interleaved, 2,000
> draws each: **168.5 µs → 170.8 µs** a frame (median, +1.4%, inside this machine's ±12 µs noise). My first single runs
> (263.9 vs 291.4) were cold-cache and wrong; the review says so.
>
> **Not in this step, and the next thing to do (Villager's job name):** the pack has no vocabulary for *what a villager is
> working on*. The engine can name the **target** (a bush, a farm, a sheep, a tree, a mine) but the word "Forager" belongs to
> content, and the key it hangs on is the open question: **the target's role name** (`farm`, `berry_bush`) is readable and
> moddable; its **dat name** (`VMBAS`-style) is not. Until that is decided, the card says "Villager", which is what the pack
> says. Asked, not assumed.
>
> Also deferred by the owner's own words: *"in the future at least shepherd uses different sprites (carrying staff), hunter
> carries bow..."* - the portrait is where that lands, and the portrait needs the asset pack.
>
> **669 passed / 0 failed / 19 ignored**; acceptance `82efabd6f614c798` unchanged (no tick code).

> # Session 17, 2026-11-04 - the unit vocabulary: plan written, inventory tool landed
>
> **The owner accepted the proposal** ("1 I accept the proposal. Proceed"), with the modding requirement attached: *"What if I
> chose that scout cavalry will be upgrade of hussar, not the other way around?"*
>
> **Landed (no engine change):** `docs/unit-vocabulary.md` - the plan, phases and risks - and **`tools/vocabulary.py`**, the
> inventory that every number in it is measured against (exits non-zero while the pack would block a name-keyed schema).
>
> **What the inventory changed about my own proposal, which is the point of writing it first:**
>
> - **A per-unit `upgrades_from` cannot express this pack.** **30** units have *two* possible predecessors, and the graph has
>   **3 cycles** (527, 528, 1104). The unit of content is therefore the **edge**: `[[upgrade]] from`, `to`, and the `tech`
>   that performs it - which is also the modding answer: reversing a line is a data edit, and the loader must *detect* a
>   cycle rather than assume acyclic.
> - **28 duplicate names**, **56 unnamed units** of 1,389 - so phase 2 is a generator change (names must be keys), not a
>   hand edit of generated files.
> - Roles are already clean: **40**, all naming real units, no two roles on one unit - the engine's rule vocabulary is in
>   better shape than its keys.
>
> **Phases, each landable:** (1) inventory tool - *done*; (2) unique names for everything, in `tools/genconfig`;
> (3) `[[upgrade]]` edges emitted from the 223 effect edges, with the cycle detector refusing the three; (4) `Entity.unit`
> becomes a **pack key** (an index the loader assigns, so the tick still compares integers); (5) the hash migration, landed
> alone and re-baselining the acceptance test in the same commit.
>
> **Do not start phase 2 in a session that is already long** - it touches the generator, the pack schema, every test that
> names a unit, and the state hash.

> # Session 16, 2026-11-04 - two vocabularies, one bug: the owner asks for units without dat ids
>
> **The report:** *"player trains champions in dark age and has hussar instead of scout cavalry"*. **It is a name bug, not a
> simulation one.** `role_name` looked a unit up by family root and had no tie-break, so `find` returned the alphabetically
> first member of the family - and for an upgrade line that is the **top** tier. Proven by a failing test: `role_name(74)`
> was `Some("champion")` and the opening scout (448) read `hussar`. A headless dump showed the simulation is right: unit
> **448** spawns, **74** is what the barracks trains. **Fixed** - exact id first, then the family's base role (which is what
> the fallback exists for: the Feudal town centre 71), then the family's lowest id, so the order is not alphabetical luck.
> The tests missed it because they compared `role_name` against `role_name`.
>
> **The owner's next question, and it is the right one: get rid of dat unit ids.** *"Each unit having clear parameters -
> starting availability and which unit it upgrades."* It is **doable**, and it is the end point of D17 (the engine knows
> kinds, not particulars). The shape:
>
> - **pack:** units keyed by **name**, with `upgrades_from = "militia"` (empty for a base unit) and
>   `available_from = "dark_age"` (or `requires = [...]`). The dat id survives only as provenance the importer writes and
>   nothing reads.
> - **engine:** `Entity.unit` becomes a **pack key** - an index the *loader* assigns, not a dat id - so the tick still
>   compares integers and never strings. `role_unit(name)` -> key; `root(key)` follows `upgrades_from` edges;
>   `role_name` becomes a field read, and the bug fixed today dies **by construction** (a key *is* a name; there is no family
>   to disambiguate).
> - **importer:** the only place dat ids remain: units, the 227 upgrade edges, and **305 `unit = <id>` fields in
>   `effects.toml`**. An id it cannot name must be a hard failure, not a guess.
> - **blockers, measured:** **1,389** units of which **28 names are duplicated** (`DOLP3`, `THSWD`, `MPCAV`, `ARTCT`,
>   `WCTW3`, `SMOKE`, ...), so the pack needs unique keys (a disambiguation rule is a content decision); and the vocabulary is
>   in the state hash, so **every acceptance hash and recording changes** (named migration, and the hash is the test).
> - **size:** 150 `role_unit(` call sites (already names - the good news), 108 `entity.unit` uses, 52 equality comparisons,
>   58 `units.root(` calls.
>
> **Order:** this is the next big slice, and it wants its own plan note (`docs/unit-vocabulary.md`) before a line of code:
> schema, importer refusal rules, the duplicate-name decision, and the hash migration. **Do not start it in a session that is
> already long** - it touches the importer, the pack schema, the state hash and every test that names a unit.

> # Session 15, 2026-11-04 - the owner's rule ends M3: passage is the ground's
>
> **The owner, in one message, killed the model three sessions had gone into:**
>
> > "Either fully occupied tiles touch (even by corners) and there is no passage ever at all, or [they] don't, and then there
> > is passage for anyone. No matter the size of the enveloping circle or number of units."
>
> **D117, landed.** Ground passability is decided by **tiles**; a body's circle governs **bodies** (`collide`) and the
> drawing, never the ground. **The engine already worked this way** - `Walk::blocked` is per tile and never consults a radius,
> and the corner rule has been pinned since the passage slice - so the slice is a guard test and the decision, not a change.
> `a_big_unit_walks_through_a_one_tile_gap` walks a `trebuchet_packed` (radius **0.5**, the biggest land circle in the pack)
> through a one-tile gap and passes unchanged. **663 / 0 / 19**.
>
> **What it cancels, explicitly: M3c.** Judging the ground by the disc means a half-tile circle touches both walls of a
> one-tile gap, so any deviation from the exact centre line increases the overlap and the body jams - the arithmetic behind
> the prototype's eight stuck workers. Not deferred: **cancelled.** The prototype stays in `/opt/data/cache/m3-findings/` for
> the record only.
>
> **My error, for the next session's benefit:** I built `ground_overlap`, a radius parameter through the mover and a back-off
> over three attempts, and the question that settles all of it - *does a circle decide passage?* - is one line to the owner.
> Ask before the second attempt at a movement model, not after the third.
>
> **Still open, and now the only movement item that matters:**
>
> 1. **Spawns sit on tile corners** while walks aim at centres (D114's named asymmetry): a freshly trained unit appears at a
>    corner. The rally ring decides where, and it is content-dependent.
> 2. **`plan_path_for` expands by the body's radius.** Harmless while every moving body is under half a tile (a one-tile gap
>    stays passable, exactly), but it must become a follower margin if one ever is not - named in D117 so it is found before
>    it bites.
> 3. The blind approach walk (`approach_point`): no test can fail on it alone, so it stays unlanded until something needs it.

> # Session 14, 2026-11-04 - M3c a third time: the rule works, the *route* is what stops a body
>
> **The ask:** "proceed" after D116. Took M3c again with the prototype, now that D114 (centred goals) and D116 (no automatic
> moving target) are in.
>
> **The rule works, and its three tests are green** wrote-first: a villager ordered **into** a house had its disc **538 raw
> units** inside the wall with a 4,000 body (RED) and never touches it (GREEN); a body spawned on a house tile was frozen for
> ever (RED) and walks out (GREEN); a one-tile doorway stays walkable. `building_passage` 11/11.
>
> **What still fails, and the measurement:** eight tests across `animals`, `build`, `rally` and `research`, all of the shape
> "a worker never arrives". Probe of `the_villager_follows_the_sheep`: after 300 ticks the villager is **8.7 tiles** from the
> sheep with `Task::Gather`, **`path_length = 5`** - a route, five waypoints, none of them consumed. It is not stuck without a
> plan; it cannot take the **first step of the plan it has**.
>
> **Diagnosis, and it is a mover problem, not a planner one.** The plan's margin (`radius + one step`) guarantees the *planned
> corridor* is walkable for a follower that is already inside it. It guarantees nothing about the **first leg from where the
> body actually stands**: a body at or inside that margin - a spawn on a tile corner, a body that slid into it before the
> plan - has a first waypoint it cannot reach in one straight step, and every candidate step is refused. Under the old
> tile-level rule the slide eventually pushed it through; with a body's shape it has nowhere to go.
>
> **The fix shape is the deck's own rule for this** ("units bump into others, stop and repath around"): **when every
> candidate step is refused and the body has a route, re-plan from the position it is standing at.** That is a change to the
> mover with its own tests (a body that cannot take its first step must end up with a *different* first waypoint, and must
> consume it), and it is the right next slice - not another attempt at the shape rule.
>
> **State:** reverted, clean at `1a8910d`, **662 / 0 / 19**. Prototype v3 at
> `/opt/data/cache/m3-findings/{lib.rs.m3-v3,building_passage.rs.m3-v3}` - the whole change, including the `at_node`
> "reach plus one step" fix and the `approach_point` blind-walk fix, both of which are needed *with* the shape rule.
>
> **Order from here:**
>
> 1. **Bump-and-repath in the mover** (above), with its own test: a body whose first waypoint is unreachable must get a route
>    it can start on.
> 2. M3c again on top of that - it has been one mechanism away three times, and each mechanism is now named and fixed.
> 3. Spawns on tile centres (D114's named asymmetry).

> # Session 13, 2026-11-04 - herding dropped (the owner's call), and M3 unblocked
>
> **LANDED** - `docs/reviews/2026-11-04-a-sheep-stands-still.md`, **D116** (supersedes D70's movement). A gathered herdable
> is worked **where it stands**; moving livestock is an ordinary order. The strike now fires when the shepherd is within
> reach of the animal, not when the animal reaches a drop site - and "within reach" has to be **reach plus one tick's step**,
> because a body walking at another rests a step short of the standoff (measured **10,399** against **10,000**).
> **662 / 0 / 19**; acceptance **unchanged** (`82efabd6f614c798`, 10.39-10.77 s, 148 entities) - that scenario gathers no
> herdable, which is the honest reading of a change the benchmark cannot see.
>
> **Why this matters beyond sheep:** the only *automatic* moving gather target is gone, so the blind-walk hazard
> (`next_waypoint(id).unwrap_or(target)`) is no longer in M3c's way. It is still real for a player who walks a sheep away
> mid-gather - keep it in mind, do not chase it now.
>
> **Next, in order:**
>
> 1. **M3c, the body's shape against the ground** - the prototype at `/opt/data/cache/m3-findings/*-v2` (`ground_overlap`, the
>    disc rule, the back-off, the `radius + step` plan margin) plus the three tests. Two of the three things that blocked it
>    are gone: corner goals (D114, landed) and the automatic moving gather (D116, landed). **Expect the plan margin to matter
>    most** - it is what makes a geometric route followable.
> 2. **Spawns still sit on tile corners** while walks aim at centres (D114 named it): a freshly trained unit appears at a
>    corner. The rally ring decides where, and it is content-dependent.
> 3. The blind approach walk (`approach_point`), which now has no test that fails without M3c - land it *with* something that
>    does, or leave it.

> # Session 12, 2026-11-04 - M3c attempted again and **not landed**: the blocker is the gather approach, not the mover
>
> **The ask:** "Proceed" after M3a. Took M3c (the body's shape against the ground) with the prototype from Session 10 plus
> M3a now in.
>
> **What passed with the change in** (all three written first, RED shown, GREEN after):
>
> - `a_body_never_overlaps_the_wall_it_leans_on`: a villager ordered **into** a house had its disc **538 raw units** inside
>   the wall with a 4,000 body; with `ground_overlap` + "a step may not make it worse" it never touches it.
> - `a_body_standing_in_a_wall_walks_out_of_it`: a body spawned on a house tile was **frozen for ever**; with the relaxation
>   it walks out.
> - `a_one_tile_doorway_is_still_walkable`: the shape rule does not brick a gate.
> - Everything in `building_passage`, `passage`, `collision`, `evasion`, `rally`, `gather`, `build` - **except three animal
>   tests**.
>
> **The blocker, now measured properly instead of guessed.** With the change in,
> `the_shepherd_kills_the_sheep_and_the_gathering_resumes_on_the_body` fails with the villager **5.32 tiles** from the sheep,
> `task = Gather { node: 129 }` (the sheep's own id - not an id mix-up), and **`path_length = 0`**: it has no route at all
> and `move_body` is never called. So this is **not** the mover, and it is not the shape rule: the **gather approach** plans
> once, loses its route when the sheep is herded, and never replans - it stands there with the order still live. That is the
> deck's own failure class, "unit stuck", and the deck's own answer applies: *"expand search area once, otherwise give the
> closest point path"* - a planner that always answers a route turns a silent stall into a visible walk.
>
> Two smaller fixture truths the same run established: `a_live_animal_is_a_gather_source` places its sheep at `centre + 1`,
> which is **inside the town centre's 2x2 solid core** - unreachable by any rule, and it only ever passed because a body's
> *centre* could stand at the core's corner; and `a_sheep_stops_at_the_drop_site` measures the sheep against the town
> centre's **anchor tile**, where the rule it means is "beside the building" (its own footprint edge).
>
> **State:** reverted, clean at `af41441`, suite green (662/0/19). Prototype updated at
> `/opt/data/cache/m3-findings/{lib.rs.m3-v2,building_passage.rs.m3-v2}` - `ground_overlap`, the disc rule, the back-off, the
> plan margin of `radius + step`, and the three tests.
>
> **Order for the next session:**
>
> 1. **The gather approach to a moving target** (M3b, now the blocker for everything else) - **mechanism found, fix written
>    and reverted because it cannot be proved on its own.** The line is
>    `let next = self.next_waypoint(entity.id).unwrap_or(target)` in the `Task::Gather` and `Task::Return` approach branches:
>    when the route is spent the unit walks **straight at the target, ignoring ground**. That is why the gatherer freezes
>    under M3c (the blind line crosses the town centre's core and every step is refused) and why it merely *presses* into
>    walls today. The fix is an `approach_point(id, target)` helper: follow the route while it has a waypoint, otherwise
>    **plan again for where the target is now**, and stand still if the plan fails.
>    **Why it is not landed:** at HEAD the tile-level ground rule masks the blind walk, so no test fails before M3c is in -
>    and D7 forbids landing production code without a test watched failing. **Land this fix and M3c as one slice**, with the
>    three animal tests as the acceptance: `a_live_animal_is_a_gather_source`, `a_sheep_stops_at_the_drop_site` and
>    `the_shepherd_kills_the_sheep_and_the_gathering_resumes_on_the_body` are exactly the three that go red, and they go
>    green when both are in. The helper's body is in this session's record and takes ten minutes to retype.
> 2. **The two animal fixtures above** (a sheep inside a building's core; "beside" measured from the anchor tile rather than
>    the footprint).
> 3. **M3c** again, unchanged, once 1 and 2 are green - it was one mechanism away from landing.
>
> **Still the owner's call:** bump-and-repath versus the pushing this engine does now.

> # Session 11, 2026-11-04 - M3a landed: the goal is the middle of a tile (the owner: "Proceed worth m3")
>
> **Landed** - `docs/reviews/2026-11-04-the-goal-is-the-middle-of-a-tile.md`, D114, commit on `main` after `91e9236`.
> `tile_centre()` is the walk-goal convention; `tile()` keeps placement. Measured before it: a villager stopped **10,000 raw
> units** from the middle of its goal tile and **0** raw units from a house it was sent beside. 661/0/20; acceptance
> `players=1`, 36,000 ticks, 10.54-10.81 s, hash `82efabd6f614c798`.
>
> **What this slice cost, and why it was worth it:** five assertions across four test files were measuring a proxy for the
> corner convention (tiles where the rule is a standoff, a row change where the rule is "went round"), and one test's *name*
> said "centre" while its assertion demanded the origin. All five now measure the rule. That is the strongest argument that
> the convention was wrong: the suite had been agreeing with it.
>
> **Open, and the next three items in order:**
>
> 1. ~~A body on the walk's line no longer forces a repath~~ **RESOLVED, same day, no defect:** the mover slides past a
>    body on the other axis before it ever replans (D98a), and a walk from a corner to a centre always has that other axis.
>    Centre to centre, the slide is the step already refused and the repath happens. The fixture now spawns the walker in the
>    middle of its tile; the test is `#[ignore]`-free again and passes. **Lesson for the next fixture:** a walk's *shape*
>    decides which mover rule answers it, so a body-on-the-line fixture must be exactly on the line.
> 2. **Spawns still sit on tile corners** while walks aim at centres: a freshly trained unit appears at a corner. Unify, and
>    the rally ring (which is content-dependent) decides where.
> 3. **M3c, the body's shape against the ground**, unchanged from Session 10: `ground_overlap`, `step_is_walkable`
>    refusing a step that makes the overlap worse, the back-off when neither forward nor sideways is legal, and the plan
>    margin of `radius + one tick's step`. The prototype is at `/opt/data/cache/m3-findings/`. With M3a in, the
>    `two_trees_touching_at_a_corner_leave_no_gap` failure it used to trigger should be gone - that was the corner goal.

> # Session 10, 2026-11-04 - M3 attempted and **not landed**: the body's shape against the ground is not a one-file change
>
> **The ask:** the owner said "Proceed" after M2 landed (`ddcad16`). M3 was scoped as "`step_is_walkable` asks the shape,
> not the landing tile".
>
> **What was built and measured, before it was reverted:** a `ground_overlap` measurement (sum of `radius^2 - d^2` over the
> blocked tiles a body's disc overlaps, zero when clear), `step_is_walkable` refusing any step that makes that number
> worse, and a back-off candidate when neither forward nor sideways is legal. It fixes the bug it was written for and it was
> verified doing so:
>
> - `a_body_never_overlaps_the_wall_it_leans_on` (new): a villager ordered **into** a house had its disc **630 raw units
>   inside the wall** with a body of 4,000; with the rule it never touches it. RED before, GREEN after.
> - `a_body_standing_in_a_wall_walks_out_of_it` (new): a body spawned on a house tile used to be **frozen for ever**
>   (`step_is_walkable` refused every step whose landing tile was still the house, and a step never crosses a whole tile);
>   with the "may not make it worse" rule it walks out. RED before, GREEN after.
> - `a_one_tile_doorway_is_still_walkable` (new): the shape rule must not brick a gate; it does not.
>
> **Why it is not landed:** it changed the outcome of **18 existing tests** across gather, build, animals, rally, research
> and passage, and the causes are not one bug. Four distinct mechanisms were found, in this order:
>
> 1. **A goal point is a tile's *corner*.** `World::tile()` returns `x*scale, y*scale`, and it is used as the point a walk
>    aims at. A body with a radius can never stand on the point where four cells meet when one of them is blocked, so an
>    order whose goal tile touches a tree presses the body into the clearance ring and it stands at the mouth of the gap
>    (`two_trees_touching_at_a_corner_leave_no_gap`: stuck at (5,3) aiming at (100000, 80000), the shared corner of the tree
>    it may not touch). A `tile_centre()` for **walk goals only** (not for placement, which is what `tile()` is for) fixes
>    that one test and breaks two animal assertions that encode the old corner convention.
> 2. **The mover has no escape from a "cannot advance, cannot slide" corner.** With a radius the diagonal step into a corner
>    is refused and *both* axis slides are refused too; the three legal directions are back, back-left and back-right. A
>    villager that walked to 0.18 of a town centre's core froze for the rest of the match. A back-off candidate fixes that
>    case in isolation.
> 3. **The plan and the follower disagree about the clearance.** A route that keeps 144 raw units of clearance is legal and
>    unfollowable: one diagonal step from it lands 3,144 raw from the corner and is refused. Planning with `radius + one
>    tick's step` as the margin is the right answer (it made `two_trees` and `rally` pass).
> 4. **Not everything is the mover.** After 1-3, `the_shepherd_kills_the_sheep...` still froze with the villager 5.37 tiles
>    from the sheep and **neither of the mover's stop branches firing at all** - `move_body` was never asked to move it, so
>    the freeze is in the **task layer** (a gather whose target is a moving body, whose approach route is never replanned),
>    not in the step. That is the piece that needs to be understood before the shape rule can land: it is not a movement
>    slice, it is the gather-approach slice.
>
> **State of the tree:** reverted, clean at `ddcad16`, suite green (660/0/19 at that commit). The prototype is preserved at
> `/opt/data/cache/m3-findings/{lib.rs.m3,building_passage.rs.m3}` for the next session - a working `ground_overlap`,
> `step_is_walkable` and back-off, plus the three new tests, none of which need to be rewritten.
>
> **How to scope M3 next time, in this order** (each independently landable, each with its own review):
>
> - **M3a - the goal is the middle of a tile.** `tile_centre()` for walk goals; fix the two animal assertions that compare
>   a tile to a corner-anchored position (`a_sheep_stops_at_the_drop_site` wants "beside the town centre", not "exactly
>   tile (16,16)"). Small, testable, and a prerequisite for anything that reads the body's shape.
> - **M3b - the gather approach to a moving body.** A `Gather` whose target is a unit (a sheep, a boar) that walks away
>   leaves the villager standing with no route and no refusal. Find out why the task does not replan; pin it with a test
>   (villager + a sheep herded 6 tiles away, assert the villager follows) **before** touching the shape rule.
> - **M3c - the body's shape and the back-off.** The prototype above, with the plan margin (`radius + step`), landed on its
>   own once a-c are green.
>
> **Still the owner's call, unchanged:** bump-and-repath versus the pushing this engine does now (`D110`'s deferral, slide 9
> of the talk).

> # Session 7, 2026-11-04 - resource nodes in the panel (the owner's ask, questions open)
>
> **The ask, in his words:** *"I would like trees to be clickable showing hit points and volume of wood they have. Same
> for farm, can I see amount of food it has left? Same for berries, stone and gold. These resources need to be visible in
> the central part of the bottom bar."*
>
> **What is already true, verified in the tree rather than assumed:**
>
> - `Snapshot`'s `EntityView` carries `hp`, `amount` (whole units, `entity.amount / scale`) and `player`, so the data
>   reaches the view already - this is a **panel and selection** change, not a simulation one.
> - `Selection::click` filters to `player == snapshot.player` and to `Unit | Building`, so a node (player 0, Gaia) can
>   never be selected; a farm can, and is.
> - `ui::SelectedRow` has no amount field, and `hp_fraction` is zero for a node, so even a selected one would draw nothing.
> - A map's nodes carry **no unit id** (`unit: None`), so the pack cannot name a tree: a node knows its resource column
>   (`Resource(slot)`), its hp (20/25/30 for a standing tree, `0` for a bush or a mine) and its amount.
> - A farm is a building whose `amount` is the food left, with hit points of its own.
>
> **LANDED** as D111 - `docs/reviews/2026-11-04-a-node-in-the-panel.md`. The owner's one clarification: **"left click on
> resource (berry bush, farm, mine...) just shows its statistics. Nothing else."** So: a node is selectable whoever owns it
> (they are all Gaia's), the click only selects, `HP n` is a number because a node has no maximum on record, and the holdings
> line reads `WOOD 100` / `FOOD 213` in the pack's own word for the column (`World::held_slot` -> `resources.name`). The
> unasked-for half of the work was the font: **every word in the interface had been running together** because a space took
> no room - `font::SPACE` fixes that, and it was found by rendering a frame and reading it, not by a test.
>
> Still open on this subject: **enemy** hit points on click, a band or double-click that sweeps nodes, and a node's maximum
> hit points (a new simulation field, not a drawing change).
>
> # Session 8, 2026-11-04 - enemy hit points on click (the owner's next ask)
>
> **The ask:** *"Enemy hit points on click"* - the board item that D111 deliberately left out.
>
> **Where the boundary already falls, verified:** the snapshot only carries what the player can see (`visible_to`; a unit
> out of sight is gone, a building or node seen before stays as a remembered one), so "select an enemy" cannot be a peek at
> the fog. Orders are built from the entity **under the pointer** and only from the player's own units, so a selected enemy
> cannot be ordered. `World::card` already refuses a building that is not the player's, which is the one thing that would
> matter once an enemy building can be selected - and it is pinned by a test in this slice rather than trusted.
>
> **The rule:** own unit, **enemy unit**, own building, **enemy building**, node - units before buildings because that is the
> order the picture is drawn in, and ours before theirs among equals. An enemy standing on a farm or a node is therefore what
> the click means, which is the one a player is reaching for. The panel then shows what it shows for anything else:
> the pack's name and a health bar, because an enemy unit and building both have a maximum the pack knows.
>
> **LANDED** - `docs/reviews/2026-11-04-enemy-hit-points-on-click.md`, D112. The enemy selects, the panel shows its name and
> health, and the fog stays the filter. **The slice's real find was a bug older than the slice:** `hp_fraction` divided an
> enemy's hits by the *viewing* player's maximum health, so once player 1 had Loom a full-health enemy villager read **0.625**
> - and the same wrong bar had been drawn over the enemy's head on the map all along. The maximum is the owner's now.
> `World::card`'s refusal of another player's building is pinned by a test, because with enemy buildings selectable that
> guard is load-bearing; orders were never at risk (`order_at` commands only the player's own units).
>
> **Named rather than changed:** a *remembered* enemy building (seen before, not watched now) carries **live** hit points in
> the snapshot, and its health bar is drawn live on the map. Freezing a memory would be a change in `Snapshot::of` (a captured
> hp at the moment of last sight), which is a different slice with its own question - what a player should be told about a
> building they are not watching.
>
> **Next on the board:** the movement pass's M2 (route corners at the obstacles' edges rather than tile centres), and the
> farm/nodes items the owner listed earlier (band selection of nodes, a node's maximum hit points).

> # Session 9, 2026-11-04 - M2: the route's corners are the obstacles' (the owner: "Proceed with M2")
>
> **What is left after M1:** the step walks the segment, so a walk is straight *between waypoints* - but a waypoint is a
> **tile's origin**, so a unit still turns at the corner of a cell and can brush a wall by up to half a tile. The owner has
> seen that as "villagers went through the house" (a corner graze).
>
> **The slice:** `crates/sim/src/short.rs` - blocked tiles merged into rectangles, each expanded by the body's radius, a
> visibility graph over their corners, shortest path through it. Every edge must also be clear at tile level (sampled), so the
> line the geometry chooses obeys the same rule `step_is_walkable` enforces; the start's and the goal's own tiles are not
> obstacles, exactly as the search and the goal rule already say. `Route.steps: Vec<(u16, u16)>` becomes
> `Route.points: Vec<(Fixed, Fixed)>` - the mover already aims at points (M1), so this is a data change at eight call sites,
> not a new movement rule.
>
> **The bounds, and what it will not do:** the solve is per route (never per tick), over the bbox of the route expanded by
> three tiles, and it **falls back to the pulled tile chain** when the window or the rectangle count exceeds its budget - a
> long strategic walk keeps tile corners rather than paying for a map-sized graph. Bodies are not obstacles: the avoid set is
> passed in for repaths, because a route planned around a body that the geometry then straightened through it is worse than
> no smoothing at all.
>
> **LANDED** - `docs/reviews/2026-11-04-a-route-bends-where-the-ground-does.md`, D113. The route is a polyline of **points**
> (`Route.points`), the geometry is `crates/sim/src/short.rs`, and the measured clearance is the body's own radius: the
> mutation that removes it puts a villager **1 raw unit** from the house's wall with a body of 4,000.
>
> **Cost, measured honestly:** 40.2 us a solve (`short_probe`), once per route and never per tick. The acceptance match is a
> win at 36,000 ticks in 10.54-10.65 s with 148 entities (M1: 8.73-8.91 s, 126) - two different matches, so that pair is not
> a like-for-like cost; an ablation with the geometry forced off played a *larger and slower* match (12.4-13.0 s, 156), which
> is the evidence that the tile-centre fallback, not the visibility graph, is where the time goes.
>
> **What is left of the movement pass:** M3 - `step_is_walkable` still tests the landing *tile* rather than the segment, so a
> body may still overlap a wall by up to half a tile even though its route keeps a radius (the collision half of the pass);
> and the bump-versus-push question in `docs/pathfinding-from-the-talk.md`, which is still the owner's to answer.

## Earlier queue entries (history)

> # Session 5, 2026-11-03 - the interface, after openaok
>
> **The owner's answers (this thread):** (1) the bottom bar takes **the game's own shape** - command grid on the left, the
> selected object in the middle, the minimap on the right, all in one bar; (2) placeholders are **labelled buttons** (a short
> word inside each), not bare rects; (3) hotkeys **wait** until the grid exists. And: **the game must run on wide screens.**
>
> Evaluation: `docs/ui-from-openaok.md` (what a mature AoE2 engine does, what we already have in 2,808 lines, the gap, what
> not to copy). Order, each a normal step:
>
> | # | step | state |
> |---|---|---|
> | U1 | anchored layout, widget states, widescreen | **landed** - `docs/reviews/2026-11-03-the-bar-gets-a-shape.md` |
> | U2 | the 5x3 command grid with pages, selected-object area | **landed** (grid + named selection; pages wait for a pack that needs them) - `docs/reviews/2026-11-03-what-is-selected-named.md` |
> | U3 | age readout, five top-bar buttons, rollover help box | **landed** - `docs/reviews/2026-11-03-the-age-and-the-help.md` |
> | U4 | input occlusion from what was painted | **landed** - `docs/reviews/2026-11-03-one-answer-to-the-interface.md` |
> | U5 | hotkeys (content table) | waits for U2 |
>
> **Two things taken from openaok on the way** (`docs/ui-from-openaok.md` §6): "drawing and clicking both go through this"
> as an explicit rule of `ui.rs`, and **refusal as an id** rather than the engine's `Display` sentence.
>
> # Session 4, 2026-11-03 - the playable core, on the owner's order
>
> **The owner's order (this thread):** postpone saving and recording; fix what is broken, then shape the engine to be
> playable - a proper GUI and HUD, no sound, no real graphics, no real AI, the one pregenerated map. Take the work in this
> order: **A5/6, then A3, P1, P2, P4**, one step at a time, each with a compact before it, an adversarial review after it,
> and a push. D1 (the command card) must be **moddable**: units, buildings, techs, descriptions, AI and civs all changeable
> by a pack. D2: keys 1-5 stay as they are.
>
> | # | step | state |
> |---|---|---|
> | A5/A6 | the gate stops lying; the 14 red tests | **landed** - `docs/reviews/2026-11-03-the-suite-that-was-green.md` |
> | A3 | the HUD that exists gets drawn | **landed** - `docs/reviews/2026-11-03-the-hud-is-drawn.md` |
> | P1 | feedback: refusals on screen, a menu on Escape | **landed** - `docs/reviews/2026-11-03-the-hud-learns-to-speak.md` |
> | P2 | band select, double-click, minimap click | **landed** - `docs/reviews/2026-11-03-input-that-was-never-run.md` |
> | P4 | the command card (moddable) | **landed** - `docs/reviews/2026-11-03-the-card-is-a-query.md` |
>
> **A5/A6 in one line, with the numbers:** `tools/require_data.py` refuses to call a run meaningful without the generated
> data (7 tests; `make test` depends on it, `make check` runs 96 tool tests), and the **14** red tests - not the 8 I reported -
> are green: 584 passed / 0 failed / 17 ignored. The engine fixes: a spawned building evicts units off its footprint
> *before* the ground turns solid, `push_units_off` matches by tile, and a carrier's route home is the anchor's first with
> the footprint's walkable edge only as a fallback (D102). **Measured:** aiming every trip at the yard cost the match -
> 176 entities and a win became 127 and a draw. The acceptance still wins: `players=1`, hash `ab314fb1969fde95`,
> 160 entities, 29 s.
>
> # Session 3, 2026-11-03 - the queue, and the baseline it is measured against
>
> **The acceptance baseline, measured three times at HEAD `55fcc2f` (not remembered):** 36,000 ticks, seed 7 -
> **`players=2` (a draw)**, hash **`91a8ad6e2cbd01f6`**, **15.2-15.4 s** (0.42 ms/tick), 131 entities, 177 repaths,
> 214 made_room; at 1,600 s **1,914 wood / 416 food / 512 gold**, 35 units, 24 buildings. The run is byte-identical
> across repeats. `make headless` adds the build and reports 44 s; quote the direct run.
>
> **The "won" claim is withdrawn, and the disagreement is explained as far as the tree allows.** The winning hash
> `51992525a3b1c3fb` is not reproducible at HEAD, at the merge `a782a47` (built in a clean worktree by the previous
> session) or in three fresh runs here. What is left is an **uncommitted intermediate build** - a binary that was never
> in any commit - so there is nothing to bisect to: the hash is an oracle only for a tree that can be rebuilt. The
> corrected numbers in `d00ee7a` stand and are the baseline above.
>
> **Where session 3 ended, in one line each - six of the seven, with the numbers:**
>
> | # | item | state | evidence |
> |---|---|---|---|
> | 8 | gather an army, then attack | **landed**, and the match is won at 36,000 ticks | `069d6c7`, `docs/reviews/2026-11-03-the-army-and-the-red-suite.md` |
> | 7 | soldiers attack consistently | **first half landed** (reload and route kept on a re-order); stances/attack-move untested | `8d2d1fe`, `docs/reviews/2026-11-03-a-reissued-attack.md` |
> | 4 | drop sites at the mine | **capability + content landed; 0 camps stand** (one miner, so the trigger is met late) | `8d2d1fe`, `docs/reviews/2026-11-03-the-work-a-rule-can-see.md` |
> | 6 | pictograms | **landed** (`[mark]`, five glyphs, a load error for a bad key) | `04a53a9`, `docs/reviews/2026-11-03-what-a-glance-shows.md` |
> | 5 | silhouettes | **landed** (square / triangle by row width) | `04a53a9`, same review |
> | 5b | speed selector | **landed** (`Speed`/`Pace`, five HUD boxes, keys 1-5; D101) | `04a53a9`, same review |
> | 9 | save games and recordings | **NOT STARTED** - see below | - |
>
> **The suite at HEAD was red, and that is the other finding of the day.** A clean worktree at `55fcc2f` with the generated
> `config/gamedata/` copied in reports **525 passed / 40 failed / 18 ignored**; 32 of the 40 are repaired (all the
> fixed-tile staging the footprint slice invalidated), and the eight left are `research.rs` (7) and `walls.rs` (1), all
> pre-existing. **The reason nobody saw it is D8's skip**: every suite returns early without `config/gamedata/`, so a clean
> worktree is "green" because the tests do not run. Until that is fixed, `cargo test` on a fresh clone proves nothing.
>
> **Item 9 is the head of the queue, and it is a format walk, not a feature.** 10 real `SaveGame/*.aoe2spgame` files are in
> the install and **zero** recordings, so the save half is verifiable and the recording half is not. The named gap is
> `docs/save-format.md`'s sections 2-4: `player_info[i].{Food,Wood,Gold,Stone}` in the scenario section
> (`FUN_00545460` / `FUN_00567630`) and the object loop in the world section (`FUN_00730600`), with the pseudocode corpus
> at `/opt/data/tools/rea/pseudo-aokhd`. Read the two functions, find the offsets, then a `stockpiles()` reader with a
> real-file assertion - the same shape as `find_game_clock`.
>
> **The queue as it stands** (owner, this thread: implement all of it, stop only for a decision):
>
> 1. **A - "gather an army first, then attack" (item 8). LANDED.** `When.army_at_least` / `army_at_most` and
>    `World::army`, plus `RuleAction::Attack` ordering the **whole** family rather than the newest unit. **Correction to
>    the entry this replaces:** the reverted version was blamed on `World::army` counting wrong; the cause was the *test*
>    naming one id three times (`max + 1` per iteration without a step), so two spawns were `DuplicateEntity` refusals and
>    one soldier existed. `army()` was never wrong. Content: the feudal barracks trains the militia line to twelve
>    (`army_at_most = 11`) and at **three** soldiers the army attacks the town centre (`army_at_least = 3`). Measured:
>    the town centre is at 19 hp at 36,000 ticks, gone at 36,500, `players=1` at 37,000 and at 48,000 ticks. The step also
>    closed the `farm_husk` leftover (a spawned farm's amount, the husk's ground, the lane rule), killed D58's dead blind
>    overbuild, and repaired **28 tests that were already red at HEAD** - see `docs/reviews/2026-11-03-the-army-and-the-red-suite.md`.
> 2. **B - "soldiers attack buildings consistently" (item 7). LANDED (first half).** `start_attack` no longer resets a
>    running reload and no longer re-plans the route when the order is the **same fight** - a soldier's attack runs on its
>    own reload, not the AI's twelve-second beat. Retaliation was checked and cleared: it answers only animals, so it never
>    pulled a soldier off a building. Measured, and it is a caution: with the same content the town centre stood at **385 hp**
>    at 36,000 ticks where the bug had it at 19 - the reset had been giving the army free swings. Review:
>    `docs/reviews/2026-11-03-a-reissued-attack.md`. **Second half open:** stances and attack-move are untested against a
>    building; nothing in the match uses them yet.
> 3. **C - "villagers build drop sites near the resource" (item 4). LANDED, with the camp itself unbuilt.** `When.working`
>    counts the villagers on a resource (gather **and** trip home, column from `gather_source`); a gold rule sends a
>    villager when the bank is empty and a mining-camp rule waits on one villager there. Placement now rejects a site the
>    builder cannot walk to - the first camp ordered was inside a wood pocket. **The match is now won at 36,000 ticks**
>    (`players=1`, hash `c12bdc86aab31513`, 29.4 s) because the gold column the army eats finally fills; but **0 camps
>    stand** - the personality sends one miner, so the camp's own precondition is met only late. Review:
>    `docs/reviews/2026-11-03-the-work-a-rule-can-see.md`. **Next thing this queue owes:** the share pass does not re-task
>    a base with no idle villagers, which is why the gold column needed a rule of its own.
> 4. **D - pictograms on buildings (item 6). LANDED with E and F in one view slice.** `[mark] <role> = "<shape>"` in
>    `economy.toml` (13 roles), `World::mark_name`, five glyphs drawn on the footprint; a mark naming a role the pack lacks
>    is a load error. Review: `docs/reviews/2026-11-03-what-a-glance-shows.md`.
> 5. **E - silhouettes. LANDED.** A square for the start villager's family, a triangle for `[metrics] military_roles`, a
>    dot otherwise; the test compares the **row widths** a villager and a militia paint at one pixel (constant against
>    growing), so the two cannot be confused for a colour difference.
> 6. **F - a speed selector (1x, 1.5x, 2x, 3x, 5x). LANDED.** `Speed` + `Pace` (thousandths accumulator, so 1.5x runs
>    1, 2, 1, 2 ticks and never drifts), five clickable boxes at the right of the HUD, keys 1-5 in the window, the pace in
>    the title and a log line on every change. **Decision (D101):** allowed mid-match, not match state - the sim is
>    fixed-rate (D16), a replay is a tick stream, and how fast a person watches is not in it.
> 7. **G - save games and recordings (item 9).** Stockpiles, population and entities from a save; the action bodies
>    (type byte, unit ids, coordinates) in a recording.
>
> **Standing rules for each step, unchanged:** compact (this block), test-first with the failing run shown, an
> adversarial review in `docs/reviews/` with a measured `## Performance` number and a named `## Stability` attack, then
> commit and push. Nothing below this block is current - it is history, and where it disagrees with the baseline above,
> the baseline wins.

## Earlier queue entries (history)

> **2026-11-03, added to the queue by the owner this turn, and one of them done:**
> - **DONE - panning is screen-aligned** (item 5). `Camera::scroll_screen` converts a screen direction into tile space
>   (the map is a diamond, so a tile step moved the picture diagonally); the arrow keys use it. Review:
>   `docs/reviews/2026-11-03-panning-the-screen.md`.
> - **NEW - a speed selector** (1x, 1.5x, 2x, 3x, 5x) in the window: the sim is fixed-rate, so this is the *window's*
>   tick pacing (`window.rs`'s loop) plus a dropdown in `ui.rs`. Decide whether a speed change is allowed mid-match and
>   whether it is recorded; nothing in the sim needs to know.
> - **NEW - silhouettes: villagers a square, military units a triangle** (or another shape), so a glance tells them
>   apart. `crates/view` draws entities as blocks today; a shape per class, from content's roles, with a test that a
>   villager and a soldier at one pixel differ.
> - **Reference worth reading: `https://zaydme.github.io/openaok/engine/`** (OpenAoK, a from-scratch C++20 AoE2 engine,
>   MIT). Same architecture family as ours - 32.32 fixed point, a fixed tick, no floats inside the tick, one-way
>   presentation, commands stamped with a player - and its pages on pathfinding, physics and the AI `Brain` are the
>   closest thing to a second opinion on items 4, 7 and 8. Ideas, not code: our licensing rule (D8) is unchanged.
>
> **2026-11-03, CORRECTION - the acceptance match DRAWS, it does not win.** The entry below claims `players=1`, hash
> `51992525a3b1c3fb`, from a single run that I could not reproduce. Re-running the merged binary, and a binary built from
> the merge commit `a782a47` in a clean worktree, gives **`players=2`, hash `91a8ad6e2cbd01f6`, 14.8 s** on every run
> (1,914 wood / 416 food / 512 gold at 1,600 s, 24 buildings, 177 repaths, 214 nudges). The "48.4 s against 19.9 s"
> comparison in the two reviews below also mixed a build-and-run figure with direct-run figures; back to back the same
> tree is 14.8 s against the baseline's 9.8-10.0 s - and the baseline no longer wins either.
>
> **Before anything is built on top of this, explain the disagreement.** Same binary, same pack, same seed, different hash:
> either the winning run used a stale binary, or something changed under it. The hash is the oracle and the bisect starts
> at the three commits between `97958a8` and the merge.
>
> One thing learned and not landed: `When.army_at_least` (a rule condition on the military count) was written, watched
> fail without the condition, and **reverted** - `World::army(player)` returned fewer than three for three spawned
> militia, and the cause was not found before the session ran out. That is the first thing item 8 needs.
>
> **2026-11-03, the owner's nine observations: 1-3 are landed, 4-9 are the queue.**
>
> **Standing measurement:** acceptance match at 36,000 ticks, seed 7 - **`players=1`** (won), hash
> `51992525a3b1c3fb`, bank **1,294 wood / 1,209 food / 175 gold at 1,600 s**, 136 entities, 34 buildings, 160 repaths,
> 143 nudges, **48.4 s** (1.34 ms/tick, 2.7% of the 50 ms budget). The 2.4x wall clock against `c979ba7`'s 19.9 s is the
> base's size and the pathing around 34 blocking buildings, not the movement pass - measured in the two reviews below.
>
> 1. **DONE - buildings block their footprints, and the base leaves lanes.** Rule: every building blocks its whole
>    footprint from its foundation, a farm is a floor, the town centre keeps its solid core and walkable yard, no overlap
>    except a spent farm. Placement follows the game's own manual - a lane around every building, farms included
>    (*"leaving some space between them to avoid 'peasant traffic jam'"*), the ground within 3 tiles of the town centre
>    left for farms (*"do not build your first house too close to the Town Center"*). Review:
>    `docs/reviews/2026-11-03-buildings-block-their-ground.md`.
> 2. **DONE - no passage between two trees touching at a corner.** The both-corners rule for raw-space diagonals is
>    `World::step_is_walkable`, and `passage::two_trees_touching_at_a_corner_leave_no_gap` proves it (trees at (4,4) and
>    (5,5), a villager at (4,5) sent to (5,4) goes round; the test fails if the rule is removed).
> 3. **DONE - a worker stands beside its node.** Every resource node now closes its tile while it holds anything (the old
>    rule keyed on hit points, which only a tree has, so a bush or a mine was a floor - and a felled tree with wood in it
>    stopped blocking). **This is what brought the win back**: a worker standing on the node was standing in every other
>    worker's arrival, and the food column starved at 609. Review: `docs/reviews/2026-11-03-ground-and-arrival.md`.
>
> **Next in the queue:**
> 4. **Villagers should build drop sites near the resource they are mining.** Content/AI: a rule that raises a mining camp
>    beside gold or stone once a villager is working one there (`where = "gold"` exists; the trigger does not).
> 5. **Panning is vertical because the map is rotated 45 degrees.** `crates/view`: screen-aligned up/down and left/right,
>    which is the camera basis rather than the tile basis.
> 6. **Pictograms on buildings**, so a glance says which is which. `crates/view`: a mark per role, from content, drawn on
>    the footprint.
> 7. **Soldiers walk up and down instead of consistently attacking enemy buildings** (they strike, then withdraw). AI:
>    stance/attack-move rules and the retreat path.
> 8. **Gather an army first, then attack.** AI: a rule that attacks at a unit count rather than per unit.
> 9. **Save games and recordings should give a lot of this information.** `tools/savegame.py` reads the clock; stockpiles,
>    population and entities are the named gap, and the recordings walk to EOF with an op grammar - the action bodies
>    (type byte, unit ids, coordinates) are next.
>
> **Proposed order:** 4 and 8 together (both content/AI rules, and 8 is what makes 7 worth fixing), then 7, then 5 and 6
> (view), with 9 available whenever a long build is running.
>
> Superseded: **C1b + C2 landed, and the acceptance match is WON.**> Superseded: **C1b + C2 landed, and the acceptance match is WON.** A body blocks a disc of `radius[0]` summed
> for the pair on **every** step, and gather / build / deposit / attack arrival are distances in the same units
> (`World::reach_between`). Review: `docs/reviews/2026-11-03-c-footprints-and-distance-arrival.md`.
>
> **The standing measurement has moved, and the next session measures against these:**
> - **`players=1`** - the enemy town centre is gone at **1,667 s**, where the landed baseline (`1a31830`) was `players=2`
>   and had never won;
> - hash **`912386556784c88f`**, 36,000 ticks in **18.7-19.1 s** (0.53 ms/tick, 1.1% of the 50 ms budget; baseline
>   9.8-10.0 s), 153 entities, **repaths 100 / nudges 166**;
> - the bank at 1,200 s is **752 wood / 825 food / 370 gold** - the baseline's 1,600 s figure, eight game-minutes early -
>   and by 1,600 s it is spent on **49 buildings** (baseline 22) and the won match.
>
> **The garrison eject fixture is unparked** - same fixture, same twelve melee attackers, `garrison::
> a_damaged_building_ejects_its_garrison` now passes: a melee attacker strikes from where its footprint touches the
> target's, so the bodies behind the first rank also land blows. That item is off the queue.
>
> **Next, in the order I would take them:**
> 1. **N1 - names for the ~30 attack/armour classes.** The last numbers the tables quote; it unblocks flipping those
>    tables to name keys. No movement dependency, so it can slot in anywhere.
> 2. **G1 - groups, then spatial awareness, then raiding.** The AI queue, and it is finally unblocked: raiding is
>    movement behaviour, and units now block each other and pass.
> 3. **The two leftovers named rather than fixed:** `farm_husk` (a command-spawned farm is not `is_expired_farm` - the
>    known defect it parks) and the 0.25 ms/tick the collision pass costs, of which ~4.4 s of the 36,000-tick run is the
>    match doing more work rather than the query (both measured in the review).
>
> **Three deadlocks this slice had to answer, worth knowing before touching movement again:** two bodies side by side in
> one tile wait for each other forever; a body **at rest** never moves aside, so waiting on it is waiting for ever; and a
> repath around a body standing on the mover's **own tile** produces a route that walks backwards. The rules that answer
> them are in `collide` (steps 2-8) and the fixture evidence is in the review.
>
> Superseded: **C was tried a second way and the two variants bound the problem exactly. Start the next session at
> "C1b + C2" below - the tree is clean at `1a31830` with B landed and C out.**
>
> - **C1b (footprint on *every* step) works mechanically.** Tests green, distance assertions in place, mutations caught
>   (tile-only: *closest 500 raw where the footprints need 8,000*; no make-room: *both have to arrive: a at (4,1), b at
>   (4,1)*). It needs a **make-room** step for head-on pairs, because a one-tile row gives no sideways component to slide
>   on. Uncapped, that nudge turned the acceptance match into a mosh pit: **89 wood / 20 food / 50 gold at 1,600 s**
>   against B's **729 / 792 / 370**, rule firings `900,6,7,1,9,0,0,0,0,23,0,...`. Next attempt caps it with a per-route
>   `gave_way: bool` (set once, then refuse and re-plan), which is the difference between "a way past one obstruction"
>   and "a way of life".
> - **C1a (footprint only inside the tile-entry gate) is a dead end, measured.** Keeping the gate so a unit can always
>   leave its own tile sounds safe and is *leaky*: a step that stays inside a tile is untested, so a body at 0.98 in one
>   tile and another at 1.02 in the next end **0.04 tiles** apart. Measured violations in the two evasion tests: *500 raw*
>   and *7,000 raw* against an 8,000 footprint. Do not re-try it.
> - **So C is all-or-nothing:** every step tested, slide on one axis, capped make-room, and **C2 distance arrival** in the
>   same slice - without C2 a body stops 0.4 tiles short of a tile-centre goal and every gather/build/deposit stalls,
>   which is exactly the 89-wood run. The acceptance test for the slice is B's bank at 1,600 s (**729 wood / 792 food /
>   370 gold**) coming back, plus `players=2` still, with the tick cost measured.
>
> Superseded: **B is landed (`f0fe9af`) at a measured 4.94% of walking distance; C was built, measured and
> reverted, because on its own it kills the economy** (89 wood at 1,600 s against B's 729) - collision stops a body 0.4
> tiles short while the arrival predicates are still tile-based. **The next step is C1 + C2 in one slice:** the footprint
> collision as built (with the make-room step and live positions) *and* distance-based arrival for gather, build, deposit
> and attack, with the 1,600 s bank as the acceptance test. The build is described in
> `docs/reviews/2026-11-03-b-string-pull.md`, "C was built, measured, and reverted".
>
> Superseded: **prove B and C by measurement before writing either (owner's instruction).** Both are priced with
> probes first: **B** - does a pather route carry slack a string-pull would remove? Measured as polyline length against a
> greedily pulled route over 200 pairs of the project map. **C** - do bodies overlap without sharing a tile? Measured on
> the acceptance match as pairs closer than `radius[0] + radius[0]` split by whether they share a tile, which is exactly
> what the tile predicate cannot see. Whatever the numbers justify gets written; a step whose worth measures at zero is
> dropped with the number rather than built on a hunch. Then the garrison fixture restage and the acceptance victory.
>
> Superseded: **A (exact order points) is scoped to the last line and NOT started - the context ran out, not
> the plan.** [A has since landed, `238ae56`.] Owner: "implement as agreed and then follow with remaining previous steps". What the scoping pass settled,
> so the next session does not rediscover it:
>
> 1. **The view half is nearly free.** `Camera::pick` already computes the continuous tile coordinate (`x`, `y` from
>    `sx`/`sy`) and only *then* rounds; a `pick_point(pixel) -> (f32, f32)` is that function without the round, with a
>    test beside the existing round-trip one in `crates/view/tests/camera.rs`.
> 2. **The engine half is `Route.end: Option<(Fixed, Fixed)>`** - set from the ordered point, returned by `next_waypoint`
>    when one step remains - plus a way to *say* a point: an additive `Command::WalkTo { unit, to: (Fixed, Fixed) }`
>    rather than retyping `Command::Move.to`, which 34 test call sites construct with tiles. One internal path (positions),
>    two ways to say where: a tile means its centre, and that keeps scenario files and the AI's rules readable.
> 3. **The trap, found before writing it:** the waypoint-spent test compares the new position *for equality* with the
>    tile centre. A free point is never hit exactly by a fixed step, so the last waypoint would never be spent and the
>    order would never end - the mover needs to clamp the final step to the remaining distance and snap (the original's own
>    "Teleporting to exact waypoint" is the same idea).
> 4. **Crowd slotting is tile-based** (`slot_for`): a clicked point must survive as the exact goal for the first unit and
>    be replaced by a tile centre only when the crowd takes a slot.
>
> **A is landed (`238ae56`) and B is dropped, measured rather than argued.** A probe asking whether an open-ground route is
> a staircase of centres measured **1.218 tiles** of deviation - and the reason was not a staircase at all: the goal
> crossed forest, so the route bent round it, which is the pather doing its job. On a corridor that *is* open there is
> nothing to straighten: eight-way A\* steps diagonally, so the polyline through centres is the line. B would only cut
> corners on routes that bend, which is a cosmetic change with a real risk (a chord through a diagonal gap). Left out of
> the queue; the owner can put it back if a bent route looks wrong on screen.
>
> Then B (string-pull the route so a clear run is one line), the garrison fixture restage, the radius predicate with
> distance-based arrival, and the acceptance victory.
>
> Superseded: **the owner's observation - movement is continuous in the original, and ours is tile-centre to
> tile-centre.** `Command::Move/AttackMove/Patrol { to: (u16, u16) }` takes a **tile**, `check_order` turns it into
> `self.tile(to)` (the centre), and `next_waypoint` returns `self.tile((x, y))` for every leg - so every leg ends at a
> centre and the destination is a centre. The original disagrees, and its own strings say so: waypoints and goal points
> are floats ("Waypoint[%2d]=(%f, %f, %f)", "Adding goalPoint of (%f, %f, %f)", "Setting position to exact waypoint").
> Tiles are terrain, resource placement and building placement; **movement is free of them**. The slice, in order:
> (1) orders carry an exact point, not a tile (the route's last leg aims at it); (2) the route is string-pulled - steer
> at the furthest waypoint the unit can see, so a clear run is one line and not a staircase of centres; (3) arrival and
> stand rules become distance-based, which is what E1b's radius step wanted anyway. This supersedes the "E1b, in two
> steps" ordering below; the fixtures parked by E1a/E1b stay parked until (3).
>
> Superseded: **E1b, in two steps. First the adjacency-ring arrival** - an order against a *blocked* target tile
> (a building, a node) must end the route on a tile from which the order can actually be satisfied, or a melee army
> stops 1-4 tiles off and never strikes (measured in E1a: twelve attackers at 1,066 of 2,400 hp, all alive,
> `cooldown_ticks: 0`). That is what un-parks `garrison::a_damaged_building_ejects_its_garrison` and may well be the
> throughput lever the acceptance match needs, since a villager ordered at a node has the same shape. **Then the radius
> predicate** (E1b proper): collision by overlapping `radius[0]` circles on the candidate position, with a tile->bodies
> index for the 3x3 query. Baseline: acceptance `players=2`, hash `c4e63f8ff29c8b0a`, 12.66 s at 36,000 ticks.
>
> Superseded: **S1 is the work in hand - the original's straight-line shortcut.** `doIntelligentMovementUpdate`
> tries a straight line to the goal before any path (move_obj.cpp 4,079-4,245): "no non-target collisions there, so we
> can do the straight line path", else "we can't do the straight line path" and the route is planned. Touch points:
> `path::line_clear` (a supercover line over tiles, excluding the **goal's own tile**, since a node or a body may stand
> there), `World::plan_path` tries it before A\*, and the tests are one waypoint on open ground, a fallback when a wall
> crosses the line, and the goal tile not counting. Baseline to beat, from E1a: acceptance **players=2**, hash
> `7c0b9132ed2074f8`, 36,000 ticks in 10.8 s. Then E1b (radius predicate + adjacency-ring arrival).
>
> Superseded: **E1a has landed, and it cost the acceptance victory - that is the next thing to face.** Units no
> longer share a tile; a blocked unit repaths around the body (D98a), with a tick budget of 4 searches and a body the
> route already avoids treated as past. Four tests in `crates/sim/tests/evasion.rs`; review in
> `docs/reviews/2026-11-03-e1a-evasion.md`. **The match at 30:00 now ends with both players alive**
> (`players=2`, hash `7c0b9132ed2074f8`) where it won for player 1 before (`98c1c2a051d044f6`) - villagers queue at
> bushes and drop sites. **Next, in order:** (1) measure and fix the economy's loss of throughput (is the 4-search budget
> starving a busy base? is a villager blocked by another villager that is about to move?), (2) S1, the straight-line
> shortcut, which is the original's answer for open ground, (3) E1b, the radius predicate and the adjacency-ring
> arrival - which also un-parks the two fixtures named below.
>
> **Parked by E1a, with diagnosis:** `garrison::a_damaged_building_ejects_its_garrison` (12 melee attackers, all alive,
> stand 1-4 tiles off a blocked goal tile: 1,066 of 2,400 hp instead of 480 - needs the adjacency-ring arrival) and
> `retarget::a_forager_whose_bush_is_finished_goes_to_a_farm` (picks node 103, another food source, no refusal
> recorded - cause unread). Both are `#[ignore]`d with that reason in the attribute.
>
> Superseded note, kept for the sections below: **evasion adopts the original's shape (D98a) and it is the work in hand.** The owner ruled **B**:
> a blocked unit does not pass through and does not merely wait - it **repaths**, exactly as `doIntelligentMovementUpdate`
> does (move_obj.cpp 4,600-4,760: collision at the new position, a collision with the target means arrival, a *new*
> obstacle means repath, an obstacle the path already knows about is ignored, terrain collision means repath). The design,
> the call sites, the tests and the one predicate still unread are in "Evasion, the original's shape" at the tail.
> **Do this before** names for the attack/armour classes, groups, spatial awareness and raiding. `[policy]` / D94a is
> done and pushed (v1.57). The order below this line is history; the tail is the queue.

> **Queued 2026-11-03 by the prior-art watch — two small fixes, both test-first.** (1) **The attribute ids 1 and 12 are the
> wrong way round.** `librematch/genie`'s unit-attribute table names 1 Line of Sight and 12 Maximum Range; our importer calls
> 1 `range`, so Town Watch's +4 to classes 3/52 lands on tower *attack* range (8 → 12) and Greek Fire's +1 (attribute 12
> only, fire galleys 529/532/1103) lands nowhere. Two tests: a galley with Greek Fire outranges one without, and a tower after
> Town Watch keeps range 8. Then the importer's name table, 1 `line_of_sight` and 12 `range`. (2) **A drop site with no
> route.** `Zaydme/openaok` remembers the distance of the site it found no way to and looks only further out, and prefers a
> site whose footprint borders the ground the worker stands on; ours picks the nearest by straight distance with no path test
> and re-picks the same unreachable site every tick — a candidate cause of the queued villagers the E1a regression left.
> Evidence and the never-copy line: `docs/prior-art.md`, 2026-11-03.

Written 2026-10-26, after the resource enum and the unit roles became content (SPEC v0.54).
**Kind 3 landed the same day** (v0.55): research plus "every unit `a` becomes unit `b`". This file
keeps the census that decided the order of work and records what is left - the question it answers
is not "what would be nice" but **what each increment costs and what it buys**, measured against
`config/gamedata/effects.toml` rather than remembered.

## Done: kind 3 (81 of 93 carrying effects are *only* kind 3)

`Command::Research` + a per-player researched set + `[3, from, to]` applied as a health-fraction
preserving swap, with one order per tech in flight at a time (`TechInProgress`). The tech the tests use is Man-at-Arms (tech 222, effect 211, `74 → 75`, 100 food +
40 gold, 40 s), identified by cost and time rather than by id. Not done, and named: prerequisites
(`required`) are loaded and *not* enforced - tech 222 requires tech 101, the Feudal Age, whose own
effect is a kind-102 gating command - a unit already inside a production queue is not upgraded, and
`location` reads as "any building" when it is `<= 0`, which the data leaves ambiguous: 458 techs
carry `-1` and 136 carry `0`, while only 13 name a barracks explicitly.

## The census, re-measured

`config/gamedata/effects.toml` (generated from the user's own dat):

| | |
|---|---|
| effects | **798** |
| commands | **3,306** |
| kind 4 (add) | **1,091** |
| kind 5 (multiply) | **576** |
| kinds 4 + 5 | **1,667 — 50.4% of every command in the game's data** |
| effects made *only* of kinds 4 and 5 | **270** |
| next largest kind | 102 (tech-tree gating), 942 |

Attributes those 1,667 commands touch, most common first: **9 attack (321), 0 hp (220), 8 armour
(200), 1 range (177)**, then 23 (152), 12 (127), 13 gather rate (117), 100 (88), 5 speed (47),
10 (45). The `class` field (`b`) is `-1` — "every class" — in **1,229 of 1,667**, and the target
unit (`a`) is `-1` — "every unit" — in **440**.

## Done next: kinds 4 and 5, for attack and armour (2026-10-26, v0.56)

The importer now decodes the packing, so an effect carries `modifiers` beside its raw `commands`, and
the engine applies **attack and armour** changes where damage is resolved: additions before
multiplications, in researched-tech order, filtered by the unit id or class the modifier names.
Checked against published numbers: Forging (+1 melee attack, blow 4 -> 5), Scale Mail (+1 melee and
+1 pierce armour, blow 4 -> 3), and a class filter that keeps Forging off a class-4 villager.

Done since, in a second slice (v0.57): **hit points, speed, gather rate and carry**.

Two rules were needed, and both were decisions rather than code:

- **Health and speed are state, so a research grants them to units that already exist.** A villager at
  25 health becomes 40 when Loom lands (and one wounded to 10 becomes 25 — the grant is health, not a
  reset). Masonry is the same shape: a house at 550 becomes 605, one at 100 becomes 155.
- **Gather rate and carry capacity are resolved when an order lands.** A load already in hand keeps
  the numbers it started with, so Wheelbarrow's +25% carry changes what the *next* trip carries and
  not the load being carried now. This is why the interpreter needed no re-pricing pass over
  in-flight tasks.

The maximum is not stored per entity: it is derived (`max_hp(player, unit)` = the table's value with
the researching player's modifiers), which keeps `Entity` unchanged — the previous slice learned what
a wider `Entity` costs.

Two defects the review of this slice found, and what is left of them:

- **Speed and gather-rate additions were applied in the wrong unit** (fixed, tested). The data has nine
  speed additions and two gather-rate additions in tiles/work per second, while the engine holds those
  as fixed-point numbers at `scale`; the code added the raw value, so a +0.35 tiles/s modifier moved the
  unit by 0.35 scale units. `scale_fixed` now scales an addition before adding it, and
  `a_speed_addition_arrives_in_tiles_per_second_and_not_in_scale_units` pins it (a free, instant tech
  that gives a 1.2-tile scout +0.35, so the speed must read 31000 at scale 20000).
- **A build site was quoted the table's health, not the player's** (fixed **and now tested**). A house
  ordered after a health tech finished at 550 rather than 605, because `check_build` copied `stats.hp`.
  Both halves are closed, and the tests exist because the test debt was paid by building the content the
  test needed rather than by lowering the claim: a purpose-built pair of tables (a free, 1-second tech
  that doubles building health) removes the race against the pack's 50-second Masonry, and the scenario
  runner supplies the legal build site the seeded fixture would not.
  - `a_house_ordered_after_a_health_tech_is_quoted_the_new_maximum` - the order-time quote.
  - `a_house_finished_after_a_health_tech_reaches_the_new_maximum` - and the same defect one layer
    down: a site already going up when the tech lands used to finish at the maximum it was quoted,
    because `Construction` stores that number and nothing refreshed it. `finish_research` now updates
    `construction.max_hp` alongside the entity's health.

**`[[train.entry]]` stopped quoting ids** in the same pass: `building = "town_center"`,
`units = ["villager"]`, resolved through `[role]` at load, with an unknown role refused by name and an
integer refused outright — accepting both spellings is how the numbering creeps back in. A role the
table lacks is an error the author sees at load, not a building that quietly makes the wrong unit.

## Done next: tech-tree gating, and the age-up identity defect it uncovered (2026-10-27, v0.60)

**Prerequisites are enforced.** Every id a tech lists in `required` is a *tech* id - all 614 entries
across the table resolve to techs, even where the number also names a building (tech 50 is Masonry, unit
50 is a farm) - so the gate is "has this player researched it", refused as `MissingPrerequisite` and
named with the pack's `[tech_role]` word when it has one. `World::grant_tech` completes a research
instantly for a world that begins later in the game, and it goes through `finish_research` so a granted
tech applies exactly like a paid one.

**That gate could not be tested without confronting an age-up.** The Feudal Age effect carries 14 kind-3
commands that *replace building ids*: town centre 109 -> 71, barracks 12 -> 498, blacksmith 103 -> 105,
house 70 -> 463. The engine applied them literally, so after an age-up:

- villagers could no longer deposit anything (the drop list names 109, the town centre was 71),
- Forging was refused at a "not a blacksmith" (the blacksmith was 105),
- `[train.entry]` no longer matched a barracks (12 vs 498).

Three separate breakages from one mechanism, all in a real match, none in any test because no test had
granted an age. The fix is a **family**: `ruage-tools` derives `variant_of` from the kind-3 pairs (190
units have a parent; the dat's own `base_id` points at itself and is useless), `Units::root` walks it,
and the four places that name a building - drop sites, research location, training, and the scenario's
role search - compare families rather than ids. Mutation-proved: restoring `unit == wanted` in
`is_family` fails `an_age_up_does_not_take_the_blacksmith_away`.

**What is still not modelled, and is the natural next increment**: a match does not *start* with the
Dark Age tech granted - a scenario's first step has to research it (it is free, instant, and legal at
any finished building), and the tests do it with `grant_tech`. Content should say `[start] techs =
["dark_age"]` and seeding should grant it, which is the same place kind-6 tech-tree effects (what a
building may train, per age) will need to land.

## Done next: a match opens with its starting techs (2026-10-27, v0.61)

`[start] techs = ["dark_age"]` and `[tech_role] dark_age = 104`, granted by the engine when the world
is seeded *or* when the research tables are loaded - either order, because a headless runner and a
scenario runner call them the other way round from each other. The point is not bookkeeping: every tech
in the table lists 104 as a prerequisite, so the previous state of the world ("nothing researched") was
one in which **research was impossible**. Tests had been granting it by hand, which is exactly the
smell: a test doing what the game should.

Two fixtures had to learn the same thing, and their failure was informative - a purpose-built tech
table with no 104 made `load_research` fail with `MissingTech { id: 104 }`, which is the right answer:
content that says "start with dark_age" and tables that cannot supply it are inconsistent, and the
loader says so rather than starting a match that silently cannot research.

**Still missing from "what a match begins with": resources.** Every player starts with nothing, so the
ten-minute script has to gather before it can build and cannot afford a 50-gold tech at all. `[start]
stock = { wood = 200, food = 200, gold = 100, stone = 200 }` (the standard opening) is the same shape of
content, but it will move every test that asserts "the player starts broke", so it wants its own step.

## Done next: the standard opening stockpile (2026-10-27, v0.62)

`[start] stock = { food = 200, wood = 200, gold = 100, stone = 200 }`, resolved through the pack's own
column names and applied once. With it, the project scenario can research Loom at t=0 (50 gold) *and*
still afford its spearman, so **the ten-minute script now demonstrates the interpreter end to end**
instead of demonstrating a queue.

The churn was the interesting part, because every test that said "the player starts broke" was
asserting a *content* fact as an engine rule:

- three gather tests now count from `opening` rather than from zero;
- `a_build_without_the_resources_is_refused` and `research_is_refused_without_the_money` ask for a pack
  with `start.stock` cleared, which is how content says "this match opens broke";
- the two own-pack tests (five own resources) had to clear the project's stock too, and their failure
  was the mechanism working: `UnknownResourceName { name: "food" }`, because a grain/iron/salt pack
  cannot start with 200 food;
- the build-time test lost an assertion that measured wall-clock ticks between "arrival" (inferred from
  position) and the finish, which moved by one because the villager no longer walks to a node first. It
  now asserts the rule - the house needs exactly 500 **builder-ticks**, the last of which is the one
  that finishes it - and says why the wall-clock version was a fixture artefact.

What is left of "what a match begins with" is nothing; the next increment is the **tech tree proper**:
kind-6/kind-101/102 commands decide what a building may train *per age*, which is the other half of what
`[train.entry]` means today (it says a barracks trains spearmen, and nothing says a Feudal barracks
cannot train something else).

## Done next: the civ tech tree as a restriction list (2026-10-27, v0.63)

The dat gives every civilisation a "technology tree" effect whose commands are all kind 102 with nothing
but an id (`[102, -1, -1, -1, 428]`) — no state field, so the file does not say whether an entry permits
or forbids. That ambiguity was the last fork worth asking about, and the data answered it (D23):

- **no civ's list contains anything every civ has** — Loom, the ages, Wheelbarrow and Forging appear in
  **0 of 32** lists, which a permission list could not survive;
- **Britain's 31 entries are its real omissions** — the Hussar (428), the Eagle line (384, 434), the
  Siege Onager (320), the Paladin (265).

So a civ's tree is a restriction list. `civs.toml` gains a loader; `[start] civ = "British"` names the
civilisation by the table's own word; `check_research` refuses a barred tech **before** the prerequisites
(a tech you can never have should say so, not ask for an age) with a refusal that names the civ; and the
civ id is hashed, because it changes what the world will allow. Mutation-proved: disabling `barred_by`
fails `a_barred_tech_is_refused_by_the_civilisation` with the prerequisite message instead.

What that leaves, in the order it now makes sense:

- **unit availability (kind 101 and 6)** — a civ's tree also governs which units and buildings exist for
  it, and those commands name an id that is *sometimes a unit and sometimes a tech* (U16), so each needs
  the same evidence treatment the tree effect just had;
- **per-player civs** — one civ for the match is honest but crude; `[start] civ` should become a list;
- **team bonuses** (`team_bonus_effect`, carried and unapplied) and the 211-number per-civ `resources`
  array, both of which need names before they mean anything.

## Done next: unit availability (2026-10-27, v0.64)

Kind 2 with a positive flag is "make this unit available", and the effect names say so themselves
("Barracks (make avail)", "Castle -- Age Three", "Trebuchet"). 109 units are gated that way by 81 techs
that do nothing else, cost nothing, take no time and have no home building - so the engine treats them as
bookkeeping and applies them as their prerequisites are met (D24). The consequences are visible rather
than theoretical:

- a **castle** cannot be built before the Castle Age, and can the tick after it (tech 137);
- a **trebuchet** cannot be trained before the Imperial Age (tech 256) - trained, not built, so
  `check_train` carries the same rule as `check_build`;
- a **house**, a **villager** and a **spearman** are unaffected, because no tech gates them.

The refusal is `UnitUnavailable`, checked **before** the price: "you cannot have this yet" outranks "you
are poor". Counted from the data rather than guessed: 109 gated units, 81 bookkeeping techs, exactly one
with no requirements (332, the outpost).

Next, in order: the **`b = 0/-1` kind-2 rows** (the Huns' house and four "enable animal" rows) whose
direction is not established - they are the last thing keeping a civ's *negative* availability out of the
model; then **per-player civs**, which is what would let two players be different civilisations and stop
`[start] civ` being a single name; then the **team bonuses** and the per-civ tuning array.

## Done next: the spear line, and the wrong id that was hiding in it (2026-10-27, v0.65)

Your question - spearmen free with the Feudal Age, pikemen and halberdiers researched - is how the data
works, and checking it found our content naming the **militia** as a spearman (D25: 74 is 40 hp,
60 food + 20 gold, and every infantry upgrade starts from it). The line is **93 -> 358 -> 359**: tech 87
"Pike" makes 93 available (free, instant, requires Feudal), 197 upgrades 93 -> 358 at the Castle Age, 429
upgrades both 93 -> 359 and 358 -> 359 at the Imperial Age.

So content now says `militia = 74`, `spearman = 93`, `pikeman = 358`, `halberdier = 359`, and the barracks
trains **militia and spearmen**. Two engine rules made that honest rather than decorative:

- **An upgrade target is not available, it is an upgrade.** `check_train` refuses a unit whose upgrade
  tech is not researched, so a barracks cannot train halberdiers just because the entry lists the line.
- **The line counts for the entry check.** A pikeman matches a `[train.entry]` that names spearmen,
  through the family root - which is the same machinery the buildings use.

The ten-minute scenario now trains **militia**, which is what a Dark Age barracks can make; it does not
age up, and pretending otherwise would have meant hiding the rule it is meant to exercise.

What is left, in order: the `b = 0/-1` kind-2 rows (the Huns' house and four "enable animal" rows),
per-player civs, team bonuses, and the per-civ tuning array. One more thing this step taught: **the
dat's name codes are not identity** - the spear line is evidence for that, and any future role should be
chosen from stats and the upgrade graph rather than from the name.

## Done next: a civ that takes a unit away (2026-10-27, v0.66)

`[2, unit, 0, -1, 0]` is the disable form, and there is exactly one in the table: the Huns' tree takes away
the **house**, which is true of the game. The bar is a civ-level rule that no tech lifts — `UnitBarred`,
naming the civ — and it is **derived on demand** rather than cached, because a cached set would have been
one more field on `World` and pass seven priced that shape.

The four `b = -1` rows are a different animal and are now named as such (D24a): effects called "Enable
sheep/llama/cow/turkey", carried by techs that cost 40 food + 25 wood and take 15 s, with an attribute and
a value in them. Nothing about that reads as enable/disable, so they stay uninterpreted rather than
guessed at.

**The measurement finding outranks the feature (pass thirteen).** The A/B claimed +6% for a change with no
tick path. Chasing it showed the probe has a between-build sensitivity of roughly ±8% at 126 entities, and
that every "~2% residual" since pass seven was that sensitivity — partly because the base worktree ran with
a `units.toml` stripped of `variant_of`. Future passes: compare against the *previous commit* with the
**same** data files, and treat a sub-10% difference in the idle probe as unresolved unless the change runs
per tick.

Next: per-player civs (which is what would let two players be different civilisations), then team bonuses
and the per-civ tuning array.

## Done next: per-player civilisations (2026-10-27, v0.67)

`[start] civs = ["British", "Huns"]` - entry *n* is player *n+1*, the last entry covers any further
player, and a one-entry list still means "everyone plays this one". The two bar lookups now take a player,
and the ids are hashed **per player**, so a replay cannot lose whose civ was which.

The project's own pack naming two civs is the point rather than a flourish: it makes the bar rules a live
case. Player 1 is refused the Hussar (428) and builds houses; player 2 is allowed the tech and cannot
build a house. One test asserts both halves in one world, and it matches on the error *variant*
(`TechBarred` versus `MissingPrerequisite`) rather than on a message, because 428 has a prerequisite chain
that would otherwise make the assertion about the wrong rule.

Next: team bonuses (`team_bonus_effect`, carried and unapplied), then the per-civ 211-number tuning array,
then the remaining effect kinds (0 and 1 modify a named unit's attributes and costs).

## Done next: teams, and a queue that had to be re-ordered (2026-10-27, v0.68)

`[start] teams = [1, 1]`, empty meaning a free-for-all, and `AlliedTarget` refusing an attack on a
teammate - the general case of `FriendlyFire`, which was always a team rule with one player in it. Teams
are hashed only when content names them, so a world without teams hashes exactly as before.

**Team bonuses were the announced next step and were deliberately not taken (D26).** Every sampled
`team_bonus_effect` targets a mechanic this engine lacks: British and Goths modify a building's *work
rate*, French line of sight, Slavs garrison capacity, Teutons a farm resource. Applying them would produce
a step whose tests cannot fail - invisible behaviour - so the effect ids stay carried and unapplied until
the mechanic exists.

That also retired the plan after it: kinds 0 and 1 are the same story. Kind 0's attributes are 19/20/21
(garrison, projectiles, and four hundred rows of it), kind 1's first field is a *resource* id - the farm
food line (Horse Collar 75, Heavy Plow 125, Crop Rotation 175) - and the sim has **zero** mentions of a
farm. The `[farm]` block in `economy.toml` has been parsed and unused for a long time.

So the next honest step is the one this exposed: **farms**, because `[farm]` is dead content, because the
farm upgrade line is the first kind-1 effect with somewhere to land, and because a gatherable building is
the first entity that is neither a resource node nor a producer.

## Done next: farms (2026-10-27, v0.69)

`[farm]` is read at last: `unit = "farm"` and `resource = "food"` say which building yields what, and that
is the extent of the engine's farming knowledge. `gather_source` is the one authority on "what may be
worked here", used by the validator, the order and the trip home; a farm holds `food_per_farm` from the
moment its plot is marked out and cannot be worked until it is finished.

**Three defects fell out, all about where a worker stands.** Arrival was `position == node.position`
(never true for a blocked building tile); the builder walked *onto* its site and finished the job standing
inside the building, from which no route home can be planned; and `return_task` read the stockpile column
off the node's *kind*, so a farm gave no trip home at all and a villager filled its load and then stood
there for the rest of the match. Now: `work_position` picks the neighbouring tile that is free **and
reachable** (a free tile inside a wall cost a scenario step), both ticks accept adjacency, and the column
comes from `gather_source`.

Because positions changed, the ten-minute scenario's `nth` tree choices needed to become
reachability-aware: `nearest_resource` now filters on `World::can_reach`, so the script picks a tree it can
walk to instead of ordering a trip the engine refuses.

Next: farms make the kind-1 farm food line (Horse Collar 75, Heavy Plow 125, Crop Rotation 175)
implementable, so that is the first thing `[farm] depletion` should learn.

## Done next: the farm food line (2026-10-27, v0.70)

Kind 1 is `[1, 36, 1, -1, 75]`: a change to a *resource*, and the earlier reading of that field as a unit id
was wrong. The importer now decodes kind 1 into `resource_modifiers` (its own namespace, next to kind 4/5's
unit attributes), and content names the id: `[farm] food_resource = 36`. The numbers settle the reading -
75/125/175 are the published Horse Collar/Heavy Plow/Crop Rotation bonuses, all additions to resource 36.

A tech that lands adds its food to every plot the player has that still holds some, and every plot built
afterwards starts at the new total; a spent plot stays spent. Two tests pin it (an existing farm 250 -> 325,
a new farm at 325) and a third pins the spent case.

Next: the remaining kind-1 resources are the same shape and mostly need mechanics we lack (Guilds, Banking,
the market), so the honest next step is **unit availability by age** - the tech tree's kinds 6/101/102 -
which is what `[train.entry]` is missing and what the farm gate (tech 216, "Farms (make avail)") already
demonstrates: a free, instant tech whose only job is to make a building available.

## Done next: the tech tree, and why it was unreachable (2026-10-27, v0.71)

Written as tests first, because the machinery looked complete: availability (`unit_available`, kind-2
`b = 1`), upgrades (kind 3), prerequisites, civ bars. Four of the six tests passed immediately - a Dark Age
barracks is buildable, a Dark Age archery range is not, a barracks cannot train a spearman before Feudal,
and it can afterwards.

The fifth failed, and it was not a test bug: **the archery range was unobtainable even in Feudal.** Tech 147
requires 667, which requires 122 "Shadow Barracks -- Age Two" - an effect with no commands - and 638, which
has no effect at all. The dat spells "a barracks exists" as a tech, and nothing ever granted it. The same
mechanism is why every earlier test granted 659 by hand before researching an age: the Feudal Age needs a
*shadow town centre*.

Fixed by extending `availability_techs` to effects that do nothing, not only effects that enable units. The
Feudal Age is now reachable by playing - 500 food at the town centre, no hand-granting - and the sixth test
proves exactly that. Mutation-proved by turning shadow bookkeeping off: the two Feudal-dependent tests fail
and the four Dark Age ones correctly still pass.

**Known simplification (D29):** the shadows are satisfied up front, so an archery range needs no barracks
standing. The building's name is in a display string, not in data the engine should parse.

Next: `[train.entry]` per age is now redundant with availability, so the better next step is **presence** -
a shadow tech that means "you have a barracks" should be granted when the player builds one - which is also
the hook the known simplification replaces.

## Done next: the count in the prerequisites (2026-10-27, v0.72)

The dat's tech carries `min_required_techs` beside `required_tech_ids`, and `ruage-tools` had been
exporting only the ids - so "at least two of these five" was read as "all five" everywhere. Nineteen techs
have `min < count`, and both copies of the gate (the order path and the availability resolver) now count.

The importer clamps a count of zero, or one longer than the list, to "all of them", which is exactly what a
pack written before the field existed meant - so no old pack changes meaning.

**A correction went into D29 as well.** It claimed that an archery range becoming buildable without a
barracks was wrong against the game. It is not: the archery range has no building prerequisite in AoE2, and
the dat says so with an OR-set that contains an always-true marker (638).

**A test that proved nothing, caught by mutation.** The first version of the counting test used the Hand
Cannon tech and passed even with the "all" rule restored, because 285 is granted for free once its own
prerequisites are met. The real test uses a pack of our own - one tech for the order path, one for the
resolver - since no tech in this dat can reach the order path with a count.

## Done next: housing (2026-10-27, v0.73)

`[population]` was the last parsed-and-unused table, and a house was 25 wood and a shape on the map. Now
population is one per live unit, the ceiling is `min(cap_default, finished houses x 5 + finished town
centres x 5)`, and a train order past it is refused with `PopulationCapped` before it is priced.

`town_center_population = 5` is new content and published fact (a house and a town centre both support
five; a castle's twenty is not modelled). The Huns needed a rule rather than a formula - their tree bars
the house, so a cap that counted houses would pin them at zero - and the exemption is read from the same
bar the engine already checks, giving them `cap_default`.

Three mutations, three precise failures: no check at all fails four tests, counting unfinished houses fails
only the construction test, dropping the Huns exemption fails only the Huns test.

Next: **ranged combat** - `range` is attribute 1 and 177 commands, and every unit is melee today. It is the
largest mechanic left and it touches the tick loop, so it gets the full review budget.

## Done next: ranged combat (2026-10-27, v0.74)

`range` - attribute 1, above zero on 277 of 1,166 units - is real. A unit with reach strikes from anywhere
inside it and walks only until it is inside; a unit with none keeps the adjacency rule it always had
(diagonals included). One comparison on squared distance in engine units, so no square root and no
trigonometry in the tick. Range rides on the task, resolved at order time like damage and reload.

That made the archer upgrade line visible for the first time: Fletching, Bodkin Arrow and Bracer are
`range +1` on class 0, and the test is behavioural - an archer five tiles out walks a tile without
Fletching and stands still with it.

Mutation-proved twice: forcing the melee branch fails all three ranged tests, and dropping the range
modifiers fails exactly the Fletching one.

**Still not modelled, and now named:** projectiles (a shot is instant damage at range, not an entity), line
of sight (a unit shoots anything in reach it can path to, walls included), and buildings that shoot (the
town centre's arrows need garrisoning).

Next, in the order I would take them: **training queue** (one slot per building today, and a crowd stalls
even that), **projectiles as entities** if the visual/gameplay difference matters, then **kind 0** - which
is per-unit attribute changes and needs garrison and LOS to land.

## Done next: the training queue (2026-10-27, v0.75)

`[train] queue_length = 5` is content now, and one slot per building is gone. The head of the queue stays on
`Entity.training` - so the tick loop keeps its fast path and `Entity` does not grow - and the rest waits in
`World.queued`, a side table by building, the same shape as `paths` and `Units::variants`.

An order is paid when it is placed; a finished unit hands the building to the next, which starts counting
from the create time it was priced at; and the population gate counts committed units, so a queue cannot
slip past housing.

**The finding worth carrying forward:** with `command_apply = "next_tick"`, `submit_checked` validates
against state where the previous orders have not landed, so five orders in one tick all saw an empty queue
and all passed. The checks now count pending `Train` commands as well. Immediate-apply mode would have
hidden this completely, which is an argument for keeping the suite on `next_tick`.

Mutation run also caught a weak test of my own: the population-cap-queue test placed two orders that were
still pending, so it passed with the committed count deleted. It steps once now.

Next: **projectiles as entities** (a shot is instant damage at range today), or **kind 0** once garrison and
line of sight exist, or **per-civ tuning** (the 211-number array, still unread).

## Corrected: the ceiling pauses, it does not refuse (2026-10-27, v0.76)

The housing step shipped the ceiling as a refusal. It is not one: a queue may hold more than the player can
support, and when the ceiling is reached **training pauses** - the job keeps its remaining ticks and waits
for a unit to die or a house to finish. `check_train` no longer looks at population, `PopulationCapped` is
gone, and the production pass asks once per player with a building in production whether they are full.

The price of the wrong rule was the tests: five of them asserted the refusal and all five had to be
rewritten. And one of the new ones - "pauses the clock" - still passes with the pause removed, because a
crowd of villagers around a town centre gives a finished unit nowhere to stand; the mutation is caught by
the queue-drain test and by the house and death tests instead.

Next: **projectiles as entities**, which was the instruction this correction interrupted.

## Done next: projectiles as entities (2026-10-27, v0.77)

A shot is an entity now: it flies at `[combat] projectile_speed`, carries the damage resolved when the order
was given, and lands where it was **aimed** rather than on the unit it was aimed at - so an arrow whose
target dies on the way finds nothing and does not retarget. It costs no population and cannot be attacked
(`NotATarget`). Speed zero keeps the old instant behaviour, so old packs are unaffected.

Two shapes of the tick dictated the code, and both were found by breaking them: the entity list is
index-stable while positions are applied (so shots are recorded during the strike pass and created after
movement), and `step` already had a `spent` vector for consumed waypoints (so the arrow list had to be
named something else, or arrows would never be removed).

Mutation-proved: forcing shots back to instant fails both projectile tests.

**Known and recorded:** projectile ids are "one past the highest", an O(n) scan per shot - consistent with
how trained units are spawned, and the first thing to fix if arrow-heavy fights ever matter in a profile.

Next: **garrison and line of sight**, which is what kind 0 needs before any of its 106 commands can mean
anything, or **the per-civ 211-number array**, still unread.

## Done next: garrison, and buildings that fight (2026-10-27, v0.78)

`garrison` and the buildings' own `attack`/`range` were both parsed and unused. Now a unit goes inside
(`Command::Garrison`) with the building's own capacity as the limit, must be standing on it or beside it, and
cannot act or be attacked while inside; ungarrisoning lets it out where it stands; and **a garrison dies with
its shelter** rather than pouring out of a razed town centre. A town centre shoots because a building is an
armed entity that cannot walk: reach from `range`, damage from its attack array, shot as a projectile.

Two details worth keeping: `start_attack` skips pathing for a building (pathing from a blocked tile refused
every shot), and that guard must test the *kind*, not the speed - a stationary unit in a test still wants its
route, which one existing test proved by breaking.

Mutation-proved three ways, and one of them found a hole: removing the capacity check fails the full-building
test, letting a garrison survive its shelter fails the razing test, and letting garrisoned units be attacked
was **not** caught by anything until a test was added for it.

**Line of sight stays out, with the reason written down:** 844 units carry `los`, but nothing asks - no fog,
no scouting, no untargetable-when-unseen rule. It waits for a consumer.

Next: **kind 0** now has garrison capacity to act on (attribute 21), or **the per-civ 211-number array**,
still unread.

## Corrected: ejection at 20%, and volleys that count (2026-10-27, v0.79)

D36 got two rules wrong, and both are now the published ones.

**A garrison is ejected, not killed**, when its building falls to 20% of its hit points or below - unharmed,
where it stands - and the building takes nobody in until repaired above that. Content:
`[combat] garrison_eject_fraction = 0.2`, checked where damage lands rather than in a per-tick scan.

**A building's extra arrows are dps arithmetic:** `floor(sum of garrisoned units' dps / building dps)`, where
dps is the **pierce** attack over the reload, divided once at the end. The qualifier is the damage class, and
our own data settles it - a Throwing Axeman (281) and a Gbeto (1013) are ranged with *melee* damage and add
nothing, while archers, cavalry archers and hand cannoneers add.

**Not modelled, and named:** each extra arrow should carry the garrisoned unit's attack (only the first is the
building's), and a garrisoned villager should count as a fixed 2.5 dps. Today every arrow in a volley carries
the building's damage and villagers contribute nothing.

Mutation-proved: removing the ejection fails the ejection test, forcing one arrow fails the archer test, and
counting melee damage as pierce fails the axeman test. That last one also showed the axeman test was passing
for a different reason than its name suggested.

## Corrected again: production buildings do not take outsiders (2026-10-27, v0.80)

A second correction to D36, from the same source. The barracks, archery range, stable, siege workshop and dock
hold **only what they have just trained** (by rallying to themselves) and never take a unit back in; town
centres, towers and castles take units from outside. The data draws the line itself - those five carry a
garrison capacity of ten and **no attack array**, the three shelters carry both - so the rule is "a building
that can fight takes outsiders in" rather than a hard-coded list of ids.

`CannotGarrisonHere` says so. **Not modelled, and named:** the other half (a trained unit kept inside by
rallying to its own building) needs rally points, which this engine does not have - so today a barracks holds
nobody, which is right for a production building.

**Also in this commit:** the ejection and volley corrections above, and the correction to the Throwing Axeman
test that had been passing off the pierce filter rather than the range check.

Next: **rally points** (which is what makes production-building garrison usable, and what a scenario wants
anyway), then kind 0.

## Done next: rally points (2026-10-27, v0.81)

`Command::Rally` with two behaviours: `Some(tile)` sends the finished unit there (`Task::Move`), `None` clears
it, and `Some` of the *building's own tile* keeps the unit **inside** - which is the half of the
production-building rule D38 had to leave out. `RallyPoint::Inside` is derived from clicking the building, so no
second command is needed, and the state lives in a side table (`World.rallies`) that is hashed.

Off-map rallies are refused (`OffMap`); an *unreachable* rally is not, and the unit simply stands where it
appeared - recorded in D39 as a gap rather than assumed.

Mutation-proved both ways: ignoring the inside arm fails the "stays inside" test, ignoring the tile arm fails the
"walks there" test.

Next: **kind 0's 106 commands** (garrison capacity is now a real attribute to modify), or the **per-civ
211-number array**, still unread.

## Corrected: rallies that gather, and rallies you cannot reach (2026-10-27, v0.82)

Two corrections to D39, both from the user.

**Unreachable is not a refusal:** a rally into a walled pocket now sends the unit as close as the map allows -
the tile if there is a route, otherwise the first reachable tile on each ring around it, ten rings out - instead
of leaving it at home. **A rally onto a resource is a gathering order:** the node is chosen by the tile, through
the ordinary gather path, so a villager rallied onto stone with wood on three sides gathers the stone (proven on
a purpose-built map, including that the stone's amount falls) and never substitutes the nearest tree.

Units that cannot work the node, and nodes that cannot be reached, fall back to walking as close as possible.

Mutation-proved: making the rally never gather fails the stone test, and removing the ring search fails the
pocket test.

## Done next: kind 0, and population (2026-10-27, v0.83)

Kind 0 is the third namespace of the effect command (`set` against kind 4's `add` and kind 5's `multiply`),
106 commands, now decoded. **Population (attribute 21) is consumed**: a building supports what content gives its
role unless a `set` modifier replaces it, and the ceiling sums that per finished building - so the Slavs' team
bonus (military buildings support five) works, proven against a British control that gains nothing.

The other named attributes are decoded and **not** consumed, each because the mechanic does not exist yet:
**accuracy 11** (Thumb Ring), **ballistics 19** (leading a shot), **minimum range 20** (Andean Sling), **extra
projectiles 102** (Yasama - a tech-granted volley, unlike the garrison's). Two attributes stay unnamed
(`other:22`, `other:105`) because nothing in the data pins them.

Next: **ballistics**, the one of those four with a mechanism already in place (projectiles fly to an aim
position, so leading a moving target is a change to where the aim is, not new machinery).

## Done next: ballistics (2026-10-27, v0.84)

Two halves, because either alone is nothing: an arrow **lands where it was aimed** and hurts the target only if
the target is still within half a tile of that point (before this, every shot hit), and **Ballistics** (attribute
19, set by a tech) aims at the intercept - the time of flight iterated three times along the target's motion,
in integer arithmetic including an integer square root.

The target's motion is read from the step its task would take this tick, because `Entity.velocity` is a
spawn-time field that movement never updates.

Mutation-proved: removing the landing check makes a runner take arrows without Ballistics; disabling the lead
makes the Ballistics case miss. The anchors do not move, and **do not cover this path** - nothing in the scripted
scenario fires an arrow.

Next candidates: **accuracy 11** (the missing chance-to-hit roll, which is what a missed arrow's randomness would
need), **minimum range 20** (a siege engine that cannot shoot what is next to it), or the **per-civ 211-number
array**, still the largest unread thing in the dat.

## Done next: accuracy (2026-10-28, v0.85)

Attribute 11 is consumed. **Accuracy is the chance a shot is aimed true**: the base comes from the dat (archer
80, hand cannoneer 65, every building that shoots 100), a tech can set it (Thumb Ring 100), the roll is a hash
of the tick and the shooter's id so replays agree, and a failed roll moves the aim point `accuracy_scatter`
(2 tiles, authored) in one of eight directions. The landing rule from the previous step turns that into a miss,
so accuracy, ballistics and the landing rule compose without any of them knowing about the others.

Mutation-proved: never consulting accuracy fails the test, and making every roll pass fails it too. Two fixtures
pin the scatter off where the archers shoot at immobile targets - a stated fixture choice, not a gap in coverage.

Next candidates: **minimum range 20** (a siege engine that cannot shoot what stands next to it), or the
**per-civ 211-number array**, still the largest unread thing in the dat.

## Done next: minimum range (2026-10-28, v0.86)

`min_range` is imported (it was in the dat all along, beside `max_range`) and consumed: **a unit cannot strike
inside its own dead zone**, and **the attacker backs away** until the shot is legal. 46 of 1,166 units have a
dead zone - mangonel 3, bombard cannon 5, trebuchet 4, dock 2 - so the rule is data, not a siege list; an archer's
is 0 and fires from where a mangonel cannot. Attribute 20 (`set`, Andean Sling) moves it.

The number is resolved at order time like damage, reload and range, and it is hashed.

Mutation-proved: ignoring the dead zone makes the mangonel fire from inside it. Not modelled: blast damage, and
the retreat does not check terrain - a mangonel backed into a wall keeps trying to step into it.

Next: **blast damage** would complete the siege engine (a shot that hurts a group), or the **per-civ 211-number
array**, still the largest unread thing in the dat.

## Corrected: `set` modifiers, the skirmisher's dead zone (2026-10-28, v0.87)

A review found two real defects behind D44's claim, both fixed test-first (D44a):

- **`set` modifiers were dropped** by `scale_fixed`, so Andean Sling (seven `set 0`) and the Koreans' team bonus
  (three `set 1`, mangonel/onager/siege onager 3 -> 1) did nothing. Population and accuracy had handled `Set`
  themselves, which is exactly why the gap went unseen.
- **A one-tile dead zone could never bite** on a tile grid, because the comparison was centre to centre. It is
  now measured to the target's **edge** (`min_range + both radii`), so a skirmisher's neighbour is genuinely too
  close. Radii were already imported.

The dead zone is therefore not siege-only: the skirmisher (7), slinger (185) and genitour line have one tile of
it, taken away by Andean Sling.

Next: **blast damage** would complete the siege engine, or the **per-civ 211-number array**, still the largest
unread thing in the dat.

## Done next: `overrides.toml`, and Murder Holes confirmed (2026-10-28, v0.88)

**The override layer now exists** (D45). `"units.<id>.<field>"` in `config/overrides.toml` replaces a unit's
value after loading; unsupported paths are refused by name rather than ignored; every replacement is logged with
its old and new form; and the file joins the pack's handshake hash when present. Proven end to end by a test that
overrides a tower's `min_range` to zero and watches it shoot what stands against it.

**Murder Holes needed no code.** Towers and castles carry `min_range` 1, and the tech is an `add -1` — which the
modifier path already applies — so the test is the proof rather than the feature: a tower cannot shoot an
adjacent unit before it, and can after.

Next: **`techs`/`effects` overrides** (the other half of the configuration surface the user described), or
**blast damage**, or the **per-civ 211-number array**.

Still not interpreted, and counted: **kind 0 (106 commands), kind 6 (2), kind 11 (2), kind 15 (1),
kind 101 (97, undecidable namespace), kind 103 (25)***, plus the `other:`/`unpacked:`
attributes the importer could not name and the effect kinds beyond 3/4/5. Range waits for a reason to
exist: resolution is melee-only, so a range modifier would be code no test could distinguish from a
comment. After that: `required` gating (a tech that needs its prerequisite), and the tech-tree effects
(kind 6 enables/disables) that decide what a building may train.

## Why this is the right next increment and not a bigger one

- **Half the data, two verbs.** Kinds 4 and 5 are "add" and "multiply", and both were pinned by
  published results, not by intuition: Masonry is `[5, -1, 3, 0, 1.1]` (buildings gain 1.1× hp;
  an *addition* of 1.1 hp would be a no-op), Loom is `[4, -1, 4, 0, 15]` (+15 hp to class 4
  civilians). 270 effects contain nothing else, so they are fully unlocked by this one interpreter.
- **The engine can already receive them.** Attack and armour are resolved **when an order lands**
  (`check_attack`), not per tick, and hp is plain state. A per-player modifier table therefore
  changes what a *future* swing resolves to and what an hp total is - no new per-entity fields, and
  the modifier set belongs in the state hash beside `stock`.
- **It does not need the other 11 kinds.** Kind 102 (942 commands) is civ tech-tree gating: which
  techs a civ may research. That is a *permission* question, and it can stay unimplemented while a
  scenario researches by naming the tech directly.

## What the increment actually is

1. `Command::Research { building, tech }` — refuse in a fixed order (not a building, not finished,
   already researched, cannot afford), charge, and start a tick timer from the tech's own
   `research_time`, mirroring `Command::Train`.
2. A per-player `BTreeSet<u32>` of researched techs, hashed; upgrading a unit is *not* a state
   mutation, it is a resolution-time fact.
3. The interpreter: for each command in the tech's effect, apply add or multiply to (hp, attack,
   armour, range) for a target unit, a target class, or everything.
4. The external check: **Loom gives a villager 40 hp and Masonry gives a house 605** (550 × 1.1),
   then a spearman-vs-villager swing that lands where the published numbers say it does.

## What it explicitly does not settle

- Kind 3 (unit upgrade, 227) — a different mechanism (replace the unit id).
- Kind 2 (117) and kind 0 (106) — unresolved meanings, named in `docs/mechanics/effects.md` rather
  than guessed.
- Kind 101 (97) — U16: 13 of its ids are techs, not units, so it cannot be typed by the id.
- Anything needing a renderer.

## What the refactor before it bought

The two previous steps (resources and unit roles as content) were not preparation for its own sake:
the finish line `a_content_pack_with_our_own_resources_loads_and_spends` is a *green test* that an
original pack can price and build, the tick's cost is unchanged within noise (2.26-2.36 µs/tick at
126 entities, 27-28 ms per ten-minute match), and the four-hour run still reproduces its hash. The
effect interpreter inherits that shape directly: a tech's cost is a `Vec<Cost>` that already names
its resources, and a scenario researches a tech by **role**, both of which would have been dat ids
two commits ago.

## After the prototype list: what the skirmish cannot settle

`config/skirmish.toml` (step 30) scripts both sides and they fight, but **it does not produce a decided match**.
Militia do ~3 damage a hit against a 2,400 hp town centre, so a fifteen-minute script ends with two armies
grinding rather than a result. Two ways out, and the owner should pick one before more time is spent on either:

1. ~~**Siege in the content**~~ - **done for the mangonel** (D62): Castle Age 102, siege workshop 49 and mangonel 280
   are roles, the train edge exists, and a mangonel damages a building in a test. What remains is the *economy*: a
   scripted opening that reaches Castle inside fifteen minutes (villagers from tick 0, farms, ten-plus on food) and
   the acceptance test that breaks a town centre with it. A ram is still unbuilt and would be the cheaper second
   siege unit (attack class 20 against buildings, no dead zone). The boom script now also has to satisfy D65:
   two Dark Age buildings for the Feudal Age, two Feudal buildings (blacksmith + market is cheapest) for Castle,
   and the workshop's blacksmith prerequisite (D63).
2. **A second victory rule** - a time limit, a score, or "destroy all buildings" instead of the seat. Cheaper, but
   it changes what the engine promises rather than what the content contains.

Until one is chosen, the honest statement is: the engine can be *played* to a fight, not yet *won*.

## Other named gaps, in the order they would be felt

- **No gates** (D54): walls block their owner too, because pathing is one grid.
- **No unit pushing** (D52): overlap is possible, and a unit already sharing a tile is never moved aside.
- **Fog without occlusion** (D50): sight is a disc, not a line through terrain.
- **The HUD is data, not pixels, and stays that way** (D57; owner's decision, 2026-10-28): no bitmap font. `Hud`
  remains a struct with tests; if the screen ever needs to show a number, colour bars are the cheap half and a
  font is not on the list.
- **Selection has no control groups or hotkeys** (list step 25, not in this batch).
- **The overbuild race is tested, with one branch statically unreachable** (D61): `foundations.rs` covers first
  builder wins, an untouched foundation losing to it, and a blind placement cancelled on sight. The "already
  started" clause inside `cancel_overbuilt` cannot be reached, because a builder that can work the tile must
  have seen it first and the sight rule cancels the placement before work begins. It stays as the rule reads;
  flipping it changes no test, and D61 says so.
- ~~**A very slow unit walks the wrong way**~~ - **fixed** (D64): a builder that finished left its `velocity` set, so
  an idle body slid at `speed / tick_rate` for ever. Work *is* adjacency-gated (the "work at a distance" reading was
  wrong), and a slow move order arrives correctly; the bug was velocity, not pathing. Three tests now hold it.
- **Overbuilding a town-centre foundation is now a refund, not a loss** (D59 + D61): the seat rule is gone, a
  cancelled foundation is paid back in full, and the town centre is class 3 like any other building, so losing
  one costs time and a walk rather than the match. The old sharp edge (a blind house ending the game) cannot
  happen any more, which is worth knowing before anyone "fixes" it back.

## The defeat rule (D59, applied 2026-10-28 - kept here as the reasoning, not as a gap)

Our D46 says a match ends when a player's **town-centre family** is gone. The original game says something else,
twice, in the manual:

> "To defeat a player, you must destroy all units and buildings but the following: towers, walls, gates, relics,
> Fishing Ships, Fish Traps, Transport Ships, Trade Cogs, Trade Carts, Farms, and sheep."

So, in Age of Kings/Conquerors/DE:

- A player is **eliminated when they have no units and no buildings left** - and the listed things (walls, gates,
  towers, farms, trade and fishing ships, fish traps, sheep, relics) do not have to be destroyed, so a player left
  with only those counts as defeated.
- The **town centre is not special.** Losing it does not lose the match: any villager can rebuild it, and any other
  building keeps you alive. "Military conquest" is the only condition that always applies, whatever victory
  setting the lobby chose (Standard adds Wonder and Relic victories, plus Time/Score/Last Man Standing).

The mismatch is not cosmetic. Under D46 a player whose town centre falls has lost even with twenty villagers and a
barracks across the map, and cannot rebuild. Under the original rule the same player is alive and rebuilding -
which is also what makes a razed base a setback rather than a defeat.

**Applied as D59** - the seat rule is gone, replaced by the original's elimination:
a player is out when they have no living unit and no standing building that *qualifies* - and what qualifies is
a **class**, not a role: `[conquest] surviving_classes = [3]` (D59b). "Walls and farms do not count" turns out to
be the same statement as "towers are class 52 and walls class 27". D46's "refuse orders after the match is over"
and the draw case are kept. **The grace window stays out** (owner's decision, 2026-10-28): it is a DE client
behaviour, not part of the manual's rule.
- **Defeat uses classes, so the D59a gap is closed** (D59b): `surviving_classes = [3]` is observed by five
  tests. What is *not* yet exercised is the manual's unit half - sheep, livestock, ships, trade carts are
  excluded there and the pack has none of them. When it does, add a class list for units and the test "a sheep
  does not keep a player alive".

## From the prior-art digest of 2026-10-29 (triaged, not yet done)

The daily watch found four things worth acting on eventually and one worth acting on now (the fixed-point
gate, which is D67 and is done).

- **Reach is measured between boxes, not centres.** `openaok` compares the dat's boxes for every attack:
  building to its footprint, unit to its size box. We compare centre distance and pad by both radii for
  `min_range` only, so a skirmisher can hit a town centre from further out than the original, and a
  building's "8 tiles" is eight tiles from its middle rather than from its wall. For **units** this is nearly
  free - we already emit `radius`, and a box is `2r` on each axis. For **buildings** it needs a footprint,
  and a building is one tile (D54). Do the unit half when reach is next touched; the building half when
  buildings stop being one tile.
- **Crowd at a work goal.** `openaok` now claims and waits for spots at work as well as at move goals; D52
  says work goals are deliberately *not* exclusive. Theirs is nearer what a player sees and costs exactly
  what D53 declined: a claims table released on every order end. Recorded; not adopted.
- **Routing round still bodies.** Their pather bends round units from a per-tick survey; ours goes through
  everything except terrain and walls. We already keep the unit-tile bitset, so the layer is nearly free;
  the cost is invalidating routes when a body moves. Crowd scenes only.
- **`genie`'s declared schedule.** Twenty steps in `BEFORE_PLAYERS`/`EACH_PLAYER`/`AFTER_PLAYERS`, each
  naming the DE routine it ports, with deferred writes so a tick's result never depends on which object ran
  first. Our order is already fixed and deterministic, so this is fidelity, not correctness: keep the
  checklist, restructure nothing until units start affecting each other. Also: the original's unit AI runs
  for **one player per tick**, rotating - our per-unit aggro stagger is the wrong shape, and it shows only
  in large fights.
- **Foundations block nobody until worked.** Confirms D61's shape from the other direction: when buildings
  start to block paths, a foundation must not block until a villager first works it, or every placing
  villager's route changes the moment the site goes down.

## The animal slice, in three increments (D69, started 2026-10-29)

1. **Importer** - *done*: `food` from `unit.resources` type 0, `dead_unit` from `dead_unit_id`, `decay_rate`
   from `resource_decay_rate`. Pinned against sheep 100 / boar 340 / deer 140 and the three body ids.
2. **Herding** - next: a sheep with no villager near it stands still; a villager within a few tiles makes it
   walk **toward the nearest drop site of the villager's player (mill or town centre)**, not toward the
   villager, and it stops when it arrives. Keyed on the animal classes (9, 10, 58) or a content role - never
   on `food > 0`, which five plants and food piles also satisfy.
3. **Retaliation and the carcass** - a struck boar turns on whoever struck it (it already carries the attack);
   a dead animal leaves its `dead_unit`, and that body can be gathered for food until empty. A body that does
   not exist must be a lookup that finds nothing, not an error: 640 records name 768, which no civ has.

### The herding rule, complete (owner, 2026-10-29)

Two corrections in a row, and the second changes the shape of the increment:

1. *"villager is herding sheep towards closest mill or town centre. So sheep do not close distance to him but
   move in the direction of mill or town centre."* The sheep's destination is the **nearest drop site of the
   villager's player**, not the villager.
2. *"Also the villager follows the sheep herding them towards the gathering point."* The villager **trails the
   sheep**, so the pair travels to the drop site together.

So herding is **not a separate order**: it is what gathering from a live animal *is*. A villager sent to gather
a sheep keeps its gather task; the sheep steps toward the drop site; the villager follows it because gathering
requires adjacency. The loop ends when the sheep reaches the drop site, and gathering continues there until the
sheep is empty - at which point it dies into its `dead_unit` (D69) and the villager gathers from the body if
there is anything left.

Two consequences to get right, both testable:

- **The villager must re-path when the node moves.** Every other gather source is static, so the tick loop
  never had to notice a node moving; a herded sheep moves under the villager's feet.
- **The villager is not idle while following.** It carries on gathering whenever it is adjacent, so the walk
  and the work interleave rather than alternating wholesale.

### Who is allowed to make meat (owner, 2026-10-29)

*"Also, sheep or any other food animal provides no food if killed by anybody else than villager."*

So the meat is a property of **how the animal died**, not only of what it was. The rule, as it will be
implemented: when an animal dies, its `dead_unit` becomes edible **only if the killer was a villager** - a
villager of any player, including an enemy's, because the rule names the role and not the side. Killed by a
soldier, an arrow, a tower or another animal, the body is still a body, and there is nothing in it.

**Not verified against the game, and said plainly:** my recollection is that in AoE2 a carcass can be gathered
whoever killed it, and that the "killed by a non-villager means no food" rule is the *first* game's. The
owner's rule is what gets built - it is his game to specify - but it is exactly the sort of thing to check
against HD when someone has the game open, so it is written here with the doubt attached rather than as
settled fact.

**Where it belongs in the code:** the death path already knows who dealt the last blow, because the outcome
names the killer. The rule is one content switch - `[hunt] meat_requires_villager` - consulted when the corpse
is created, so the engine asks content rather than knowing that a villager is special (D17). Testable in one
assertion: the same sheep, killed by a soldier and by a villager, leaves bodies that differ in food and not in
identity.

### Where increment two actually stands (2026-10-29)

**In place and green:** `[hunt] herdable_roles = ["sheep"]` in `economy.toml`, the `sheep` role (594), and
`Hunt` in the economy schema (raw + runtime + mapping). Four tests in `crates/sim/tests/animals.rs` say what
herding means; they are **`#[ignore]`d** because the engine half is not built, and the ignore note lists why.

**Not built, and the three reasons the tests gave:**

1. **`gather_source` refuses a unit.** Sending a villager to a sheep fails with `NotAResource { id }`: the
   function accepts a resource node or a farm building, and nothing else. The fix is content-shaped - a live
   animal of a `[hunt] herdable_roles` family yields `[hunt] resource` - but it is a change to the one
   function every gather order passes through, so it wants its own slice rather than a patch bolted on.
2. **A placed animal carries no meat.** `Command::Spawn` takes an explicit `amount`, and a sheep placed with
   `amount: 0` is worth nothing to gather. Either the spawn derives `amount` from the unit's `food` (D69), or
   whoever places animals must pass it; the first keeps the pack as the single source of the number.
3. **Nothing walks the animal.** The herd pass is not written: while a villager gathers a herdable animal, the
   animal should be sent toward the nearest drop site of that villager's player, and the villager's cached
   route dropped so it follows the moving node instead of a stale path to where the sheep used to be.

**One unexplained observation, recorded rather than forgotten:** in the "a sheep nobody is gathering stands
still" test the sheep moved **0.025 of a tile** over 600 ticks with no order given - the same *shape* as the
"very slow unit walks the wrong way" bug that turned out to be a leftover velocity (D64). It may be the same
class of defect in a path nothing else exercises yet; it is worth a look before the herd pass is written,
because a standing-still test cannot be written against a body that creeps.

### Conversion: how a herdable changes hands (verified, 2026-10-29)

Asked by the owner: *"sheep captured by another player can be recaptured if no unit is nearby (please verify how
and conditions)"*. Verified against the game's own manual and the wikis; the rule is **line of sight**, not
adjacency, and it is immediate:

- *"Sheep start the game neutral (gray) and unowned. As soon as a sheep enters the line of sight of a unit, the
  sheep is under control of that player. You can steal ownership of another player's sheep if it comes within
  your unit's line of sight and there are no other players' units near the sheep."* (AoE2 manual, sheep entry)
- *"if the herdable animal subsequently gets in contact with another player's unit without a unit of their
  current owner nearby, they will immediately fall under the other player's control."* (Food, Fandom)
- *"If more than one civilization has units near an animal, the animal will be owned by the player who has the
  most units nearby."* (Herdable animal, Fandom)
- **Celts** ignore the "no enemy units near" condition entirely. A civ bonus, out of scope until civs decide
  anything mechanical (D63 is the one place a civ has mattered so far).

**What that means for the engine**, and why it is bigger than it looks: the trigger is our **vision layer**
(D50), not proximity - a tile a player's unit can see is a tile whose herdables that player takes, provided the
current owner has no unit seeing it too. We already have `visible_to`/`unit_sees`, so the test is nearly free;
what is new is that **ownership of an existing entity changes**, which nothing does today: every ownership
change so far is a kill or a spawn. That wants its own decision on what happens to a villager *gathering* a
sheep that changes hands mid-load, and whether the meat already gathered stays with the gatherer.

### Step one of the animal slice: a live animal is a gather source (done, 2026-10-29)

`gather_source` accepted a resource node or a farm building and refused everything else, so a villager sent to
a sheep got `NotAResource` before it walked a single tile. It now also accepts a **living unit whose family is
an animal**, with the resource named by `[hunt] resource` and the amount carried by the animal itself.

Content gained the roles it needed (`sheep` 594, `boar` 48, `deer` 65) and a second list, because herding and
edibility are two questions: `animal_roles` is everything worth meat (both kinds), `herdable_roles` is the
subset that walks to a drop site. `Hunt` carries `animal_roles` and `resource`.

Two tests, both RED first: a villager sent to a sheep is accepted and ends up carrying meat; and a sheep placed
with no amount is worth the pack's **100** rather than zero.

**Adversarial review of this step:**

- `meat_of` is applied to *any* unit the pack gives food to, and five are not animals - the forage bush 59 and
  its twin, a food pile 262, a deer's meat pile 333, an oddity 544. Harmless today (a placed bush carrying an
  amount gathers nothing: `gather_source` gates on `animals`, not on food) but it means "lists `food`" and
  "is an animal" must never be conflated later.
- The check is on the **order**, not the tick, so this costs one set lookup per gather order and nothing per
  frame.
- `animals` is derived at seed from config and is not hashed, like `conquest_classes`: it decides what orders
  are *refused*, and a replay runs with the same pack.
- **Left for the carcass step, deliberately:** when a sheep's meat runs out the villager stands down and the
  sheep stays on the map alive at zero amount. That is wrong in the game and right for now, because killing it
  is the carcass rule's job (step three), not this one's.

### Step two: herding (done, 2026-10-29)

`[hunt] herdable_roles` resolves to families at seed (`World.herdable`). While a villager gathers one, the
animal is walked to that player's nearest drop site and the villager's cached route is dropped so it follows.
All six animal tests pass: an unworked sheep stands still, a gathered one walks toward the town centre, the
villager follows it rather than watching, a sheep stops on arrival, a live animal is a gather source, and a
placed sheep is worth 100.

**Two things the tests corrected in me:** my "villager follows" assertion compared the wrong two distances
(the villager starts *at* the town centre, so "closer than it started" is nearly zero) - it now asserts the
villager moved and stays within six tiles of the sheep; and the first implementation re-found the gather node
per gatherer per tick, an O(units) scan that the arm had already done - the node's unit is read once now.

**Still open:** the sheep is not killed when its meat runs out (carcass rule, step three), a boar never
retaliates (step three), and an animal never changes hands (conversion, step four - needs the owner's ruling on
what a gatherer's load does when its sheep is stolen).

### Step three, re-shaped by the slaughter rule (2026-10-29)

The owner's correction - meat implies death, and the killing blow lands **on arrival at the drop site** - means
step one's shape needs reworking, not just extending: a villager currently gathers from a *live* sheep, which
the rule forbids. The corrected sequence, and what each part costs:

1. **Slaughter on arrival**: when the herded animal reaches its drop site, the villager kills it. Cheapest
   faithful form is a transformation in place - the entity's `unit` becomes its `dead_unit`, its health goes to
   zero, its meat amount stays, and the gatherer's task stays valid because the id did not change.
2. **Gathering the body**: `gather_source` must accept a carcass (no health, meat in `amount`), which is
   content-named - `[hunt] carcass_roles` for 595, 356 and 43 - rather than inferred from "no health".
3. **Retaliation**: a struck animal turns on whoever struck it. Independent of the slaughter work and done
   first.
4. **Only a villager makes meat**: the killer decides whether the body holds food.
5. **Conversion** (step four): line of sight, immediate, and now with a clear window - the animal must be
   **alive**, which means during the walk and before the killing blow.

### Retaliation: attempted, parked, and what the attempt proved (2026-10-29)

The first attempt is reverted and its tests are `#[ignore]`d. Three things it established, all worth more than
the reverted code:

- **A boar parked beside the town centre is shot by the town centre.** The blow that lands is an *arrow*, and an
  arrow lands with its damage but not with its shooter - `Task::Fly` carries no attacker - so an animal cannot
  retaliate against a building at all today. My test was never testing melee.
- **Moving the beast to eight tiles changed nothing**, so either the town centre reaches further than I assumed
  or the blow came from elsewhere again. Either way the test's premise was unproven, which is exactly why it
  should not have been written that way.
- **The melee hook never fired**: `retaliate` was called from the melee strike path, its debug print never
  appeared, and yet the boar lost health - so the damage took a path I had not identified.

**What to do instead, next time:** write the test with two units and no buildings within range, so the attacker
is unambiguous; then find which of the four `hp -=` sites in the tick actually delivers a melee blow, with a
print at each, before writing the reaction. The mechanic is still wanted - a boar that does not fight back is
scenery - but the ordering was wrong: identify the damage path first, write the test second.

### The retaliation test's premise, proved wrong by instrumentation (2026-10-29)

Owner's rule, which the instrumentation then confirmed on screen: *"If Town Centre kills the boar, it doesn't leave
the carcass with food. Villager has to be the one delivering last blow."*

Printing at both damage sites and running the parked test settled it: the boar was struck **only** by arrows -

```
ARROW target=EntityId(127) dmg=3   (six times)
MELEE ...                          (never)
```

so the town centre shot it, exactly as the owner said it would, and the villager **never landed a single blow** in
two thousand ticks despite being ordered to attack at eight tiles' distance. Two consequences:

1. The retaliation test was invalid from the start, for the reason now visible rather than guessed: the only
   attacker was a building, and an arrow carries no attacker.
2. **A villager ordered to attack an animal may not strike at all.** That is either a pathing problem (it never
   arrived) or a targeting rule (player 0 is not a valid victim for a villager) - and it matters, because the
   owner's rule makes the *villager's own blow* the thing that decides whether a carcass holds food. Find out which
   before building the slaughter: if a villager cannot strike an animal, the slaughter has to be an act of its own
   rather than a very small fight.

**The damage path, for the record:** melee blows land in the `strikes` loop (`victim.hp -= landed`, which knows its
attacker); arrows land in the `landings` loop (`victim.hp -= damage`, which does **not** - `Task::Fly` carries the
target, not the shooter). Anything that needs to know who killed something must either take the melee path or the
arrow must start carrying its archer.

### Meat is decided by the last blow; hunters shoot, shepherds cut (owner, 2026-10-29)

Two refinements, both narrowing what the earlier notes guessed at:

- *"If soldiers hurt the boar, but villager makes the last hit, then carcass does give the meat."* So nothing about the
  **history** of the wound matters - soldiers may soften it, arrows may stick in it, and a villager's last blow still
  leaves a carcass worth 340. Only the **last** blow counts, which means the engine has to record who dealt *that* blow
  and nothing else.
- *"Hunter villager also use the bow while shepherd villager uses the knife."* A hunter variant attacks at **range**, a
  shepherd at melee. That is the likely explanation for a villager never landing a blow on a boar at eight tiles: an
  attack order with a ranged variant resolves as a **shot**, and a shot lands without carrying its archer - so the
  victim is hurt by an arrow nobody can name, and the variant that is supposed to make meat is the one whose blow the
  engine cannot attribute. This is now the first thing to check, before any carcass code is written.

### Boar luring: what the video is, and what we can and cannot take from it (2026-10-29)

The owner's link (`DkFD8bhtFkI`) is **"How to lure a boar in AoE2 shown in 45 seconds or less (garrison method)"** by
Hjörleif. Its transcript is **disabled**, so the narration could not be read; the title, the description's companion
links (a "Viper method" and a "basic method"), and general knowledge are what this rests on, and it is labelled as
such rather than presented as the video's content.

The **garrison method**, in the terms the engine will need: a villager is sent to hit the boar once (or simply come
close) so that it takes the villager as its target; the villager then walks back toward the town centre with the boar
following; at the town centre the villager **garrisons**, and the boar - having lost its target - stops in place,
where the remaining villagers finish it. The trick is that the boar keeps chasing only while it has something to
chase, so the garrison ends the chase exactly where you want the killing to happen.

Our engine has every part of that except the last: an animal takes no target of its own (retaliation is parked), and
garrisoning exists. A boar lure is therefore a scenario script that a **retaliation** rule makes work - which is a
second, practical reason it is not optional.

And the owner's remark that the **factory AI never lures the boar in HD**: our acceptance scenario must script the
lure explicitly. Nothing in the AI will do it, and a fast-castle script that ignores the boar is leaving 340 food
standing in the forest.

### The acceptance budget: 30 minutes of game time (owner, 2026-10-29)

*"Can you speed up the simulations instead and then add extra 10 minutes?"* - yes, and the two halves are independent:

- **Speed is already free.** The simulation is headless and deterministic: the Dark Age probe played **20 minutes of
  game time in about ten seconds** of wall clock. Nothing in the acceptance test waits on real time; a longer match
  costs seconds, not minutes. There is no speed knob to turn, and none is needed.
- **The budget is a game-time decision, and it is now 30:00**, up from 20:00. The measured reason: a berries-and-farms
  economy reaches Feudal at about 14:30 and Castle does not fit inside twenty minutes without the sheep and boar the
  researched openings rely on. Ten more minutes of game time is ten more seconds of test runtime and removes the
  pressure that was forcing the economy to be faster than it is.

### The boar's leash, and where it goes back to (owner, 2026-10-29)

*"Boar can also lose interest in chasing the hunter and not follow him to the town centre and returns where he was
before. So another shot with arrow is needed before boar flees too far. I usually wait until 2 arrows are shot and then
try to flee."*

So an animal's chase is not a commitment, and the garrison method is not a trick that works from any distance:

- it follows **as long as it is interested**, and interest expires - by distance from where it started, or by time
  spent chasing, or both;
- when interest expires it **returns to its origin**, which is a place it remembers, so an animal needs a home
  position in its own entity state (it has none today);
- **each new blow renews the interest**, which is why a second arrow lands before the hunter runs: the arrow is not
  aimed at damage, it is aimed at keeping the boar following.

Three states, then, not two: **grazing at home -> chasing a target -> returning home**. Built testably as: a struck
animal walks toward its attacker; break line or walk far enough and it walks back to where it began; a second blow
turns it round again.

### Priorities, weighted towards what a simulation can settle (owner, 2026-10-29)

The owner's aim, in his words: *"I intend to keep system, that allows simulations ... when adding new civilizations, it
would be helpful to use the simulations for game balancing."* That changes what "next" means: work that a headless run
can measure comes before work that only a person can see.

**Simulation-testable, and wanted:**

1. **The slaughter and the carcass** - in flight. Last blow decides the meat; the body is gathered; the animal becomes
   its `dead_unit` in place. Every assertion is arithmetic.
2. **The animal's chase and leash** - the three states above. Timeable, distance-measurable, deterministic.
3. **A scripted opponent and a scenario format** - steps with times, so a match is reproducible. This is the thing the
   balance goal actually needs: *"at 2:10 build the mill, at 4:30 send two villagers to wood"* is a build order that
   can be run for every civ.
4. **A batch runner with metrics** - N scenarios x M seeds, headless, emitting a table: time to Feudal, time to Castle,
   food/wood/gold per minute, villagers lost, first military unit, match outcome. Without this, balance is opinion.
5. **The acceptance match** - fast castle, siege workshop, mangonel, raze the town centre, `Victory { winner: 1 }`
   inside 30:00. It is the one test that says the whole economy-to-army chain works.

**Not yet simulation-testable, and why:** there is no AI (only scripts), civs are data that nothing branches on beyond
one building prerequisite (D63), and the market, trade and full counter triangle are absent. Balance work on those
would be measuring a game we have not built yet.

**The order that follows:** 1 and 2 are small and both close a hole I opened; 3 and 4 are the balance engine and are
the first things that pay back the owner's stated goal; 5 is the acceptance test that uses 3.

### Step three, part one: the slaughter (done, 2026-10-29)

A herded animal that reaches its drop site becomes its body **in place** (unit 595/356/43, no health, meat
intact) and the villager gathers from it. Content names the bodies; the sweep is taught to keep them. Seven animal
tests pass.

**Adversarial review of this change:**

- **A per-tick table clone, found and removed.** The first sweep change cloned the entire unit table once per
  tick to answer "is this a body"; the set now holds ids *and* families (six entries) so it costs a lookup.
- **`hp == 0` no longer means "delete" for these units** - a rule the rest of the engine has to know. Anything
  that treats zero health as gone (target selection, blast victims) already skips them, which is right: a body is
  not a target.
- **Still missing, and named:** an animal killed by anything but this slaughter is swept away with nothing left.
  The last-blow carcass needs the killer, which melee knows and arrows do not - the same fact that parks
  retaliation.
- **Open questions kept:** a body belongs to Gaia, so anyone's villager can work an enemy's carcass (probably
  right, unverified); a body never decays (`decay_rate` is imported and unread); and a body emptied of meat stays
  on the map for ever.

### Corrected: the shepherd kills, then there is a corpse (owner, 2026-10-29)

The transform-on-arrival in increment three has the right body and the wrong trigger. It must become:

1. the herded animal arrives at the drop site;
2. the villager **attacks** it - the shepherd's knife, melee - and the animal **dies of the damage**;
3. **death** leaves the body: `dead_unit`, meat intact if the last blow was a villager's, empty if it was anyone
   else's;
4. the villager goes on gathering from the body, which is why the body should keep the entity's id.

**Order of work, because 2 is blocked:** establish that a villager's melee blow actually lands on an animal
(the instrumentation showed no `MELEE` line at all, only `ARROW dmg=3` - which the owner's hunter/shepherd split
predicts: a hunter variant attacks at range, and a shot lands without its archer). Then the death path learns its
killer, then the body is made at death rather than at arrival.

## Item 1 - the damage path, retaliation, and the boar's leash (2026-10-29)

*This section is the compact for the increment; the review is appended to it when the code lands.*

### Verified before writing anything (and it corrects the previous session's diagnosis)

- **`start_attack` was never called, so "its silent refusal" was a misdiagnosis.** `retaliate` runs only on the
  *instant* melee path (`lib.rs`, the `strikes` loop's `victim.hp -= landed`), and no blow has taken that path:
  `combat.toml` sets `projectile_speed = 10.0`, and the loop turns **any** blow into an arrow whenever
  `flight > 0 && source.0 != aim`. For a non-Ballistics attacker `aim` is the victim's own position, which differs
  from the attacker's for every melee blow - so a villager's knife, a spear and a militiaman's sword all become
  arrows in the air. The data says the same from the other side: villager 83 has `range = 0.0, reload = 2.0`, and
  `check_attack` is happy with it.
- **Consequence, and it is one defect with three faces:** melee damage lands in the `landings` loop, which carries
  no attacker, so (a) `retaliate` never fires, (b) the last blow cannot be attributed, and (c) the previous
  session's "no `MELEE` line" was the whole tick, not one fixture.
- **The knife test that passes is passing through the arrow path.** `a_villagers_knife_lands_on_a_sheep` asserts the
  sheep lost health, and it did - by a projectile.

### The fix, in two halves that are separately testable

1. **Melee lands in place.** A blow is a projectile only when the attacker's *resolved* range is above zero. Melee
   damage applies on the tick it is swung, in the `strikes` loop, which is the path that knows `(attacker, target)`.
2. **An arrow carries its archer.** `Task::Fly` gains the shooter, the landing carries it, and the landing applies
   damage *and* the reaction - so ranged damage is attributable too.

Then retaliation is reachable at all, and the leash has something to hang off.

### The leash: three states, and where the numbers come from

**Verified:** the dat carries **no leash field** for animals (`BOARX` 48 has `los = 4.0`, `speed = 0.8`, no
distance or duration of interest), so the leash cannot be derived from the pack.
**Assumed, and content-authored with provenance:** an animal chases until it is `leash_tiles` from the place it was
first struck, or until `leash_seconds` have passed since the last blow - each blow renews the clock. Both live in
`[hunt]`. These are the knobs balance work will tune; they are judgement calls and are marked as such rather than
presented as game numbers.

### Tests this increment owes

- `a_struck_boar_turns_on_whoever_struck_it` (parked `#[ignore]` today) - now reachable, so un-ignored.
- a melee blow never puts a projectile in the world (the fix's own control).
- an archer's arrow names its archer: a boar shot from range turns on the shooter.
- the leash: struck -> chases -> out of interest -> walks back to where it started; a second blow turns it round.
- the refusal record: `retaliate`'s swallowed error is readable instead of invisible.

### Item 1, review: what the change costs, and what it nearly broke (2026-10-29)

**Landed:** melee lands on the tick it is swung; `Task::Fly` carries its `shooter`; retaliation works on both
damage paths; the animal's leash is built (three states, home recorded at the first blow, each blow renewing the
clock); a swallowed reaction refusal is recorded and readable. Tests: 8 in `retaliation.rs`, five of them written
and watched failing first (`a_melee_blow_never_becomes_an_arrow` reported **3 arrows** during a knife fight).

**Performance.** The per-tick cost is unchanged in shape: one `matches!` on an already-loaded task per blow, and
the leash pass returns immediately when `World.leash` is empty - which is every world where nobody has struck an
animal. The `landings` vector is one struct wider (a shooter id); it was invented here not at all. No new table
walk in the tick: the leash numbers are resolved at seed, and the shooter rides on the arrow rather than being
looked up. **Not measured with the A/B probe yet** - the honest statement is "no new per-entity work", and the
probe is owed before anyone claims a number.

**Stability.** The state hash gained the leash (only when entries exist) and the shooter on an arrow, so a replay
that lost either diverges visibly rather than silently. The anchors move for exactly that reason in any world that
strikes an animal; the scenario/probe hashes must be re-recorded when next they are run.

**Three defects the review found in my own work, all fixed:**

1. **`leash` was left out of the bookkeeping tables.** A dead animal would have left its entry behind for ever, and
   `bookkeeping_entries` - which exists precisely to make that visible - would not have noticed, because it did not
   count the table. Added to both `bookkeeping_entries` and `bookkeeping_touches`, and to the removal sweep.
2. **The no-float gate went red.** The first version converted `10.0` seconds with `f64::from(tick_rate)` inside
   `seed_from`, in `lib.rs` - a float in the sim crate's tick-adjacent code, which is the exact thing the gate
   exists to refuse. Fixed by making both numbers **whole seconds and whole tiles in content** (`u32`), so the
   conversion is integer arithmetic and there is no value a scale could round.
3. **Two fixture bugs, mine.** A `Move` to a hand-picked `(2, 2)` is `NoPath` on this seeded map, and an archer
   spawned ten tiles out stood somewhere unreachable and refused its attack order at apply time. Both now use a
   position the test has already proved reachable (the villager's own start tile), and the archer sits at
   (12, 1) - recorded here so the next fixture does not repeat either.

**What is still not right, named rather than glossed:** blast damage retaliates (the shooter is known now) but a
*body* cannot, and nothing stops a boar chasing into a wall - it plans a route and gives up only if there is none.
The leash numbers are judgement calls, not game numbers; a balance run is what will tune them.

## Item 2 - the last blow leaves the body (2026-10-29)

*Compact before the code.*

**Verified:** every animal names its body in the pack (`dead_unit`: sheep 594 -> 595, boar 48 -> 356, deer 65 ->
43), and the body's own record is a plain unit (`SHEEP_D` class 11, `hp = 0`, `food = 0`, `decay_rate = 1.0`). A
blow now carries its attacker on **both** damage paths (D71), which is the fact this item was waiting for. The
sweep already keeps an entity whose unit id is one of `[hunt] carcass_roles`, so a body that exists is not swept.

**Assumed from the owner, and only from him:** the **last blow** decides the meat - soldiers may soften the animal
and arrows may stick in it, and a villager's final hit still leaves 340 on a boar; anything else's last blow leaves
a body with nothing in it. The dat has no field for this.

**Unknown, and content's to answer:** *which* roles count as "a villager". That is a role list, not a hard-coded
id: `[hunt] butcher_roles = ["villager"]`, resolved to unit families at seed like every other content list.

**What is built:** death by damage promotes the animal **in place** to its body - same id, no health, meat kept
only when the killer is a butcher, task idle, no motion - because the gatherer's task names the id and must stay
valid across the blow (D70). The killer is recorded per tick (`last_blow`, a `Vec` of `(victim, attacker)` filled
by both damage sites), because the blow that kills is always the blow of the tick it dies on; nothing is carried
between ticks and nothing new is hashed.

**Tests this owes:** a villager's last blow leaves the meat; a soldier's last blow leaves an empty body; and a
soldier who *softens* the boar does not take the meat once a villager finishes it.

## Item 3 - the shepherd kills, and meat only comes out of a body (2026-10-29)

*Compact before the code.*

**The owner's correction, verbatim:** *"No, shepherd has to kill the animal. It doesn't change to corpse itself."*
So arriving at the drop site is not the kill. What is built today is the right body with the wrong trigger: the
herding pass promotes the animal the moment it reaches the drop site (a corpse without a blow).

**And a second thing the same rule forbids, which the code does today:** a villager gathering a **live** animal
takes meat out of it (`take_amount` reads the node's `amount`, and a live animal carries its meat). *Meat implies
death* (the owner's rule), so a live animal must yield **nothing** while it is being herded; the work during the
walk is following, not taking.

**The shape:** on arrival at the drop site the villager is **ordered to attack** the animal (the ordinary combat
path, so the blow carries its attacker and the last-blow rule already decides the meat), the animal dies of that
damage, and death promotes it to its body in place (item 2). Then the gather order **resumes on the same id** -
recorded in a side table `slaughter: villager -> animal` - because the villager's original `Gather` task was
replaced by the fight and the body is what it should be working.

**Numbers, verified from the pack:** a sheep is 7 hp and a villager's blow is 3 damage at a 2 s reload, so three
blows (six seconds) kill it. The villager stands adjacent, which is where gathering put it.

**Tests this owes:** a live animal yields no meat (a villager beside a boar carries nothing while it lives); the
shepherd strikes before the body exists (the villager's task is an attack on the sheep, and the sheep's health falls
while it is still a sheep); and the gathering resumes on the body once the animal is dead.

### Item 2, review: what the body cost (2026-10-29)

**Landed:** death by damage promotes an animal in place to its `dead_unit`; the last blow decides the meat
(`[hunt] butcher_roles`); the killer is collected per tick from both damage paths.

**Performance.** The first version walked **every entity every tick** to find the dead - the same shape as the
per-tick unit-table clone the previous session's review caught. It now runs only on a tick where blows landed, and
only over this tick's *victims*: one lookup per corpse, nothing on a quiet tick, and nothing added to `Entity`.

**Stability.** Nothing new is hashed: the killer record is a local `Vec` in `step`, empty in any world where nobody
is fighting. The promoted body keeps its id, so every task that names it stays valid - which is the property the
gatherer depends on.

**The defect the review found in my own change, fixed test-first:** with the body now surviving the sweep, "the
target is gone" stopped being how a fight with an animal ends, so an army would have swung at a corpse for the rest
of the match. `an_attacker_stops_swinging_once_its_quarry_is_a_body` failed before the fix (the attack phase now
requires a target with health) and passes after it.

**The trap in the fixtures, recorded because it will recur:** `Command::Spawn`'s `amount` is in **whole units** and
scales itself, so a fixture passing `340 * scale` builds a 340-food boar worth 340 x scale x scale. Both new
fixtures had it; the assertion caught it at 136,000,000,000 against 6,800,000.

### Item 3, review: the slaughter is a real kill (2026-10-29)

**Landed:** on arrival the villager attacks; the animal dies of the blow; the gather order resumes on the body;
a live animal yields nothing.

**Performance.** No per-tick scan: the herding pass already visits the workers that are gathering an animal, and the
resume pass costs one map walk that is empty unless a slaughter is in flight. `take_amount` gained one set lookup
(the animal id is now in `animals` alongside its family precisely so this costs no table read).

**Stability.** `slaughter` is hashed (it decides what the villager does next), joins the bookkeeping tables, and is
dropped in both directions when either side dies - a dead villager cannot leave its animal behind, and a dead animal
cannot leave a stale resume behind. No new `Entity` field.

**Two of my own defects, both caught by running rather than by reading:**

1. **The animals fixture never loaded `combat.toml`**, so `check_attack` refused every slaughter with `NoTables` and
   the villager stood beside a living sheep for 2,000 ticks. Item 1's refusal record named it on the first run.
2. **A test assertion that was too strict**: it demanded "the villager is swinging *and* the animal is alive" at
   every tick the task was an attack - but the attack task lingers one tick past the killing blow, so it fired on
   the tick the body appeared. The assertion now has to see both facts on the *same* tick, which is what the rule
   means.

**Named as still open:** the villager's first blow lands the moment it is ordered (it is already adjacent), so the
shepherd spends six seconds beside a sheep; a real one would not need to wait, but the reload is the data's, and
three blows is what a 7-hp sheep needs.

## Item 4 - a body rots at the rate the pack gives it (2026-10-29)

*Compact before the code.*

**Verified, and from two directions.** `decay_rate` is imported per unit and unread. The numbers are
**food per second**: `BOARX` 0.4, `SHEEP` 0.25, `DEERX` 0.25 in our own pack, and two independent community accounts
give the same figures for the game - *"Boar decays at 0.4 Food per second, Deer and Sheep at 0.25"*, and *"sheep and
deer decay at a rate of 15 food per minute, while boars decay at 24 food per minute"* (15/60 = 0.25, 24/60 = 0.4).
The pack agreeing with both to the digit is what settles the unit of the field.

**A trap this closes:** the *body's* own record carries `decay_rate = 1.0` for all three bodies (`SHEEP_D`,
`BOARX_D`, `DEERX_D`), which is not a rot rate at all - it is the same 1.0 on every corpse, while the living
animal's value is the one that differs per species and matches the published numbers. So the body inherits the
**living animal's** rate, resolved once at seed.

**What is built:** at promotion the body's rot rate is fixed (`rots: body -> raw per tick`), and a per-tick pass takes
it out of `amount` until the body is empty, at which point it leaves the map - which closes the "an emptied body
stays for ever" hole as a side effect rather than as a separate item.

**Cost:** the pass returns immediately unless a body is rotting, and it walks only bodies. Nothing new is hashed: the
rate is derived from the unit id, which is state already.

**Tests this owes:** ten seconds of rot on a boar's body is exactly four food (0.4 x 10); and a body rotted to
nothing leaves the map.

### Item 4, review: rot, and the tick a body was born hungry (2026-10-29)

**Landed:** a butchered body rots at its species' `decay_rate` (food per second, resolved to raw units per tick at
death); a body that rots to nothing leaves the map and leaves nothing in the side tables.

**Performance.** No new pass: the rot is applied **inside the sweep that already walks every entity once per tick**,
with `rots` taken out with `mem::take` so the closure borrows one field rather than two. `rots` is empty unless a
body is rotting, and its per-entity cost is one `BTreeMap::get` - bounded by bodies, not by the map.

**Stability.** Nothing new is hashed: the rate is derived from the unit id, which is state already. `rots` joins the
bookkeeping tables so a rotted body's cleanup is testable (`bookkeeping_touches` is asserted after the body leaves).

**Three of my own defects, all found by running:**

1. **A body lost exactly one tick of food as it appeared** - `the_last_blow_of_a_villager_leaves_the_meat` failed at
   6,799,600 against 6,800,000, which is precisely one tick of a boar's rot. Promotion happens before the sweep in
   the same tick, so a body made this tick was already rotting in it. Fixed by collecting the new rot entries during
   promotion and inserting them **after** the sweep: bodies rot from the tick after they are made.
2. **The `retain` closure cannot mutate**, so the first rot attempt could not decrement anything without a second
   walk of the world; `retain_mut` plus a taken map is the shape that keeps it to one pass.
3. **Non-butchered bodies still have no lifetime** - a soldier's kill leaves a body with no meat and no rot entry,
   so it stands there for ever. That is item 6's "emptied body" hole, still open, and now named in one place.

## Item 5 - a herdable changes hands by line of sight (2026-10-29)

*Compact before the code.*

**The rule is the manual's, quoted in D70 and re-checked there:**

> *"Sheep start the game neutral and unowned. As soon as a sheep enters the line of sight of a unit, the sheep is
> under control of that player. You can steal ownership of another player's sheep if it comes within your unit's
> line of sight and there are no other players' units near the sheep."*

So: the trigger is **vision** (a *unit's* LOS, not a building's), it is **immediate** (no timer, no adjacency), only
**herdables** convert, and ties go to whoever has **more units near**. The Celts' exemption is a civ bonus and out of
scope while civs decide nothing mechanical.

**Content answers "which animals".** `[hunt] herdable_roles` already names the animals that **walk**; the manual's
sentence and the walking are the same set of animals in this pack, so the list does double duty rather than gaining a
second list that could disagree with it. Recorded as a decision, not a coincidence.

**What the engine must know cheaply.** Each unit's `los` is a pack value, so it is squared and resolved at seed
(`sight`), and the herdable *entities* are tracked in a set filled at seed and on spawn - both so the tick can ask
"who sees this sheep" without walking the unit table.

**What changes when an animal changes hands:** its owner, and nothing else - but the *old* owner's gatherer was
following it, so any task naming it as a gather node is stood down and its own route is dropped: it belongs to
somebody else now and walks to nobody's drop site. That is the "window for stealing is the walk" the chicken-and-egg
earlier slices kept pointing at, and the meat is not in question because meat only exists after death.

**Tests this owes:** an unowned sheep joins the first player whose unit sees it; an owned one is **not** stolen while
its owner still sees it; it **is** stolen when its owner cannot; a tie between two stealers goes to the side with
more units near it; and a boar - food, but not herdable - never changes hands.

### Item 5, review: line of sight, and a sheep that kept itself (2026-10-29)

**Landed:** herdables change hands by a **unit's** line of sight; the owner's own sight protects its animal; a tie
goes to the side with more units near it and then to the lower player number; a boar never converts; and the animal
that changes hands drops its route and stands its old herder down.

**Performance.** The pass is one walk of the units times the herdables alive - and returns immediately when no
herdable stands, which is every world whose pack names none. The sight reach is `los` squared, resolved at seed, so
the tick does no table read and no square root. `sight` is a map over the unit table (~1,389 entries); the
alternative was a table read per unit per tick, and the map is paid once at seed. **Not measured with the A/B probe**
- the honest claim is "O(units x herdables) per tick, zero when there are no herdables".

**Stability.** Ownership is state, and it is hashed through the entity. Nothing new is carried in `Entity`. The
herdable set joins the bookkeeping tables and is pruned with the entity.

**Three defects, all mine, all found by running:**

1. **A sheep is not a witness.** Animals have a line of sight of their own, and the first version counted them as
   seers - so an owned sheep saw itself and could never be stolen. The debug print showed `seen = [1, 0, ...]`, which
   is the signature. Animals are now excluded from the seer set.
2. **The contested test could not test the tie-break.** Its counts were two against one, so the mutation that
   reverses the comparison (`>` to `>=`) passed it untouched. A second test with two watchers on **each** side was
   added, and that mutation now fails it. The first test is kept: it pins the count rule, the second pins the
   ordering rule, and neither implies the other.
3. **A fixture ordering trap worth knowing:** ownership is decided the instant anybody sees the animal, so placing
   the sheep before the thieves lets the first thief own it and then its own sight protects it. The tie-break test
   places the watchers first and the animal last. It cost one failing test to see, and it is recorded in the file.

## Item 6 - three known-wrong answers, paid off (2026-10-29)

*Compact before the code.*

**(a) `settle_blind_sites` runs an O(n) scan every tick.** It has a guard, but the guard *is* the scan ("is anything
under construction?"). A maintained set of foundations makes the guard a set lookup, and a `debug_assert!` that the
set agrees with the world keeps the maintenance honest - drift in a maintained set is exactly the failure a
behaviour test cannot see.

**(b) Two count assertions in the data tests follow the pack.** `pack.rs` re-asserts the dat's 801 techs and 798
effects. That is the *dat* test's business (`techs.rs`, where pinning the census is the point); in a pack test it
fails the day a civ is added, for a reason that has nothing to do with packing. Replaced with a relationship: the
pack's tables are the ones its files hold, compared against a direct load of the same file.

**(c) A body with no meat had no lifetime.** A soldier's or a tower's kill leaves a body with nothing in it and - as
of item 4 - no rot entry either, so it stood on the map for the rest of the match. The lifetime is **derived from the
pack**, not invented: a body lies as long as its species' *full* carcass would take to rot (meat / decay rate - 850 s
for a boar, 400 s for a sheep), whether or not it holds meat.

**Tests this owes:** a soldier's kill still leaves a body that can be seen at a minute, and is gone after the span
the pack implies.

### Item 6, review: the three known-wrong answers (2026-10-29)

**(a) The blind-site guard is a set, not a scan.** `settle_blind_sites` asked "is anything under construction?" by
walking every entity, every tick, and the answer was "no" in every match that was not being built in. `foundations`
is maintained where a construction starts, where it finishes and where the entity dies, and a `debug_assert!`
compares it with the world on every tick it is non-empty - so the drift a maintained set invites fails in debug
instead of silently skipping a rule.

**Measured, interleaved against a `HEAD` worktree** (2,000 finished buildings, no foundations, release, 1,000 ticks
each): **68.5-70.1 µs/tick on this tree against 71.5-72.0 on `HEAD`**, and **the state hash is identical**
(`7a7f41250fdd3ab8`) - so the change is behaviour-neutral and worth 2-4% in a world that never builds anything. It is
an O(n) -> O(1) change, so the payoff grows with the world; at 2,000 entities it is real but small.

**(b) The pack's counts became a relationship.** `pack.rs` asserted the dat's 801 techs and 798 effects; it now
compares the pack's tables against a direct load of the same files, which is what a *pack* test can prove (it loaded
the files it says it loaded) and which survives a pack that grows. The dat's census stays pinned in `techs.rs`, where
the number is the subject rather than a side effect.

**(c) An empty body has a lifetime, derived rather than invented.** A soldier's kill leaves a carcass with nothing in
it and - after item 4 - no rot to work with, so it stood on the map for the rest of the match. Every body now carries
an expiry tick as well as a rot rate: **its species' own meat divided by its species' own rot rate** (850 s for a
boar, 400 s for a sheep), so a pack whose animals change changes the lifetime with them. Nothing was invented, and
the test asserts both halves - present at a minute, gone inside eighteen.

## Items 7 and 8 - the balance engine (2026-10-29)

*Compact before the code. Item 7 is verified as already met; item 8 is new.*

**Item 7, verified rather than rebuilt.** The scenario format exists (`config/scenario.toml`, `crates/sim/src/script.rs`,
schema 2): steps carry `at_seconds` and are **converted to ticks once**, in `Runner::new`, then sorted by
`(tick, player)` - so the same file means the same match at any tick rate (D16). Selectors name roles, not ids, and
an unresolvable step is reported rather than skipped. The pass condition the item states is already a test:
`the_scenario_is_deterministic` runs the project script twice and compares state hashes, with the negative control
that dropping one step changes the hash. What it does **not** have is the vocabulary a build order needs (place a
mill by the berries, attack a boar, move home, garrison) - that will be added as the acceptance script demands it,
in item 9, rather than as speculation here.

**Item 8, new: a batch runner with metrics.** N scripts x M seeds, headless, one row each:
time to Feudal, time to Castle, resources **delivered** per minute, villagers lost, first military unit, outcome,
final hash.

- **Delivered, not held.** Stock rises only on a deposit, so "delivered" is the sum of the *increases* per slot; the
  decreases are spending and are counted separately. Summing final minus initial would confuse a bank that was spent
  with an economy that gathered nothing.
- **Cheap to observe.** Stock is four array reads a tick; the entity walks (villagers lost, first military unit, age
  techs) happen once a second, because a batch run is a measurement tool and not the engine.
- **Content answers "military".** `[metrics] military_roles`, resolved through `[role]` - the engine never learns that
  a militia is 74, and a pack of our own names its own soldiers.
- **The engine gains nothing.** No new state, no new hash field: every number here is derived from what the world
  already reports, which is what keeps the runner from becoming a second simulation.

**Tests this owes:** a world that gathers for a minute reports delivered wood above zero and no ages, no military and
no losses; a unit that dies is counted as a loss exactly once; and the same script on the same seed reports the same
numbers twice.

### Items 7-8, review (2026-10-29)

**Item 7 was already met and is reported as verified, not rebuilt:** steps at `at_seconds`, converted to ticks **once**
in `Runner::new`, roles rather than ids, refusals reported, and `the_scenario_is_deterministic` as the pass condition
with its own negative control (drop one step, the hash must move).

**Item 8 landed:** `Metrics` (a library type, 4 tests) and `ruage-batch` (a binary). The first real table:

```
script                        seed   feudal   castle food/m wood/m gold/m lost    army    outcome
scenario.toml                    1        -        -     11     23      5    0   522.0 open (0 refused)
scenario.toml                    2        -        -     11     23      5    0   522.0 open (0 refused)
```

Two seeds agree to the hash, which is *expected* here and worth saying out loud: the map is authored and the project
script never draws from the RNG streams, so the seed cannot change this match. A seed sweep only becomes a
measurement when a script or a map uses the streams.

**A TDD honest note:** this slice's test file and module were written in one pass, so the RED was not watched. The
mutation evidence stands in for it - folding *spending* into *delivery* fails `buying_something_is_not_a_delivery`,
and counting a loss on every sample fails `a_dead_villager_is_counted_as_a_loss_once`. The first version of the
delivery test could not see the spending mutation at all (nothing was bought in it), which is why the purchase was
added to it.

**Performance.** The metrics cost per tick is four array reads; the entity walk is once a second. The batch runner in
release plays the ten-minute project script (`--release`, one process) in well under a second per seed.

## Item 9 - the acceptance match: what is built, and the two gaps it hit (2026-10-29)

*Compact, measurement and review in one, because the item is not finished and saying so is the honest artefact.*

**Built and tested:** the scenario vocabulary a build order needs - `where = "<resource>"` on a build step (which was
being **parsed and ignored**: `where` was written in `scenario.toml`, no step has `deny_unknown_fields`, and nothing
read the key, so a lumber camp asked for beside the wood landed beside the town centre), `target = "enemy_building"` /
`"enemy_town_center"` on an attack step, and `node = "farm"` on a gather step (a farm is a **building**, and
`nearest_resource` only finds `Resource` entities, so a script could not touch the food engine a long opening needs).
Five tests, three mutations bite: the first `where` test passed with the selector disabled entirely because player 1's
nearest tree is four tiles from its town centre, so it now uses **gold** - eight tiles out, outside the placement ring,
where only the selector can explain where the camp lands.

**The script:** `config/acceptance.toml`, 35 steps, 1800 s: berries -> farms -> mill + lumber camp -> Feudal -> Market +
Blacksmith -> Castle -> siege workshop -> mangonel -> the enemy town centre.

**Measured, not asserted** (`ruage-batch --script config/acceptance.toml --seeds 7`):

```
script                        seed   feudal   castle food/m wood/m gold/m lost    army    outcome
acceptance.toml                  7        -        -     12      6      0    0       - open (15 refused)
```

and the timeline from `ruage-headless --report 200`: food climbs to **328 by 800 s and then stops**, wood to 139, the
population pins at **11**, and 15 steps are refused - the lumber camp for want of 100 wood, three villagers for want
of 50 food, two farms for want of 60 wood each.

**The two gaps, and neither is a scripting mistake:**

1. **A villager idles when its node runs dry.** Each tree holds 100 wood, so three lumberjacks exhaust their three
   trees and stand still for the rest of the match. A thirty-minute boom needs a villager that *seeks the next node of
   the same kind* - an engine rule, small and well scoped, and the thing standing between this script and Feudal.
2. **No scripted repeater.** "Keep four on wood" is a policy, not a list of 35 timestamps; without a repeating step the
   script has to guess every re-tasking time, and every guess goes stale when the economy changes.

**What is therefore *not* claimed:** the acceptance match does not yet win inside 30:00. The infrastructure that
measures it does exist, which is what makes the next attempt honest: `ruage-batch` prints where the run stopped, and
`ruage-headless --report` says which step was refused and why.

## The game's own shape: rules and villager shares (2026-10-29)

*Compact before the code, from `docs/ai-of-the-game.md`.*

**Two things the game's AI does that our scenario format cannot, and both are the reason the acceptance match kept going
stale:**

1. **Rules, not timestamps.** The AI's Dark Age is five rules whose guards are counts, amounts, `can-*` predicates and
   an age - re-evaluated forever. Ours is 35 `at_seconds` entries that were measured once and rot the moment the economy
   changes.
2. **Shares, not assignments.** The AI never picks a tree; it sets a percentage per resource and the engine moves idle
   villagers to meet it (`Low-pop Villager Distribution.per` is 76 `set-strategic-number` lines and no orders).

**What is built:** an additive `[[rule]]` block in the scenario format (`when` = conditions, `do` = one action, a
cooldown so a satisfied rule does not spam orders) and a `[policy]` table of villager shares by villager count, applied
by the engine to idle villagers. The scenario schema stays **2**: both are optional fields, so a v2 file without them
means exactly what it did, and a schema bump is for changes that *break* a file, not for additions.

**Tests this owes:** a rule that does not fire until its condition holds and then fires once per cooldown (not once per
tick); a share table that moves idle villagers to the resource furthest below its target, and moves them again when the
table changes; and the acceptance match rewritten on both, measured rather than asserted.

## Rules and shares - review (2026-10-29)

*Adversarial, against my own work, in the order the damage would show.*

**The freeze is not fixed, and it is older than this change.** With the rules and the share table in place the run still
stops dead: **8 units, 125 wood, 147 food, from 300 s to 2400 s**, with the town centre reporting a full queue and food
in the stockpile. Re-running the *old* timestamp script reproduces it (337 food / 138 wood frozen from 600 s, reported
last session), so the freeze predates the rules and the policy. What this session did establish is the shape of the
answer: 0 A.D. enters `FINDINGNEWTARGET` on depletion and **always resolves to some order**, including "walk to the
dropsite and idle there". Our gather pass has no such fallback, and a worker stuck *inside* a task is invisible to every
policy above it. That is the next thing to build, and it is a `sim` change, not a script one.

**Performance: two measurements, one lesson.** 400 game-seconds cost **319 ms** with the policy pass doing nothing, and
**11,922 ms** with the first version of the pass - because it asked `can_reach` (a full path search) of every candidate
node once a second per player. Reachability is now asked exactly once, by `start_gather`, which needs the answer anyway;
the pass costs noise. **Cost per path search is ~29 ms**, which is the next performance question in its own right: a
4,000-tile map should not cost 29 ms to path, and the acceptance run is ~6x slower than real time because of it.

**Tests: the rules were written after their code, and that is recorded rather than hidden.** The compile failures the
tests were first run against only proved the type names - the discipline's whole point is a *behavioural* failure. Both
rule tests were then shown to bite by mutation: deleting the cooldown check fails `a_rule_respects_its_cooldown` alone,
and deleting the `at_seconds` check fails `a_rule_waits_for_its_time_too` alone. The share tests were written first
(`no method named set_villager_shares`), and the worst-deficit mutation fails all three.

**One refusal the rules produce that a player would not see: "no idle villager to build with".** A rule that builds is
refused when every villager is working, and it retries on its cooldown. That is honest but noisy, and the next version
should distinguish "nothing to do" from "nothing free" so a build-order log is only the second kind. The train rule has
the same problem in a different form: it fires at a full queue, which the game's own AI avoids with `(can-train
villager)` - a `can_train` condition is the missing guard, and it is a small one.

**What is solid:** `453` tests, clippy `-D warnings`, fmt clean; the scenario format's rules parse, fire once per
cooldown, respect `at_seconds`, and are reachable from a rule-only file; the share table resolves content's names to
columns, refuses rows that do not sum to 100, and moves villagers between resources when the row changes.

## Rules and shares - the second pass (2026-10-29)

*Four defects the first version shipped, each found by a test that was already there.*

**The pass picked the same villager every time.** `workers.iter().find(free)` returns the first free villager; the retry
memo was then consulted *after* the pick, so a villager the policy had just moved was chosen again, rejected, and the
pass ended - three idle villagers, one of them considered, forever. The whole condition (free, off the retry window, no
site queued) now lives inside the search. `idle_villagers_follow_the_shares` is the test that caught it, and the fix is
the reason it passes in a minute of game time rather than in ten seconds: one villager is moved per pass, by design.

**Six `gather.rs` tests and one `build.rs` test changed behaviour, and the loader was wrong, not the tests.** Installing
`[policy]` from `load_resources` gave every world a second hand on its villagers, including the worlds that exist to
watch a villager's own behaviour. The share table is content and a **match** is where it applies, so it is now
`World::apply_content_policy()`, called by the headless and batch runners and by the acceptance test - not by the loader.
`build.rs`'s "the builder stops when the house is finished" asserted `Task::Idle`, which is no longer what a freed
villager is; it now asserts that the build is over, which is what it always meant.

**Farms at Feudal starved the run.** A farm is a Dark Age building gated by the mill, and the game's own AI farms
through the Dark Age; gating the farm rule on `age = "feudal_age"` stopped the economy dead at 18 villagers and 46 food.
With the guard moved to "a mill stands and the larder is under 120", the run reaches **22 villagers and 13 buildings by
800 s** instead of freezing at 8 units.

**A rule's refusal is not a step's refusal.** Both used to land in `Runner::outcomes`, so "not enough food at 8 s" - the
train rule asking before anybody has gathered - read like a broken script. `Runner::rule_refusals` now holds them
separately, and the acceptance test asserts what actually matters about the opening: no step refused, the rules firing,
and villagers gathering after a minute.

## Rules and shares - the third pass (2026-10-29)

*Two more walls, both of them the same mistake in different clothes: a mechanism that only works once.*

**Shares that only describe the first assignment are not shares.** With the pass restricted to *idle* villagers, the
table described the opening and then nothing: a **32-villager economy sat on 35 wood for 1,000 game-seconds** while
fourteen villagers fed a granary nothing was spending, because nobody was ever free again. The pass now also
**rebalances** - it takes one villager off the column that is over its share, bounded by the same retry window, so the
cost stays one path search per window and the table keeps meaning what it says for the whole match. 35 units and 16
buildings by 800 s became reachable in place of 22 and 13.

**A worker the policy keeps busy is a worker a rule can never hire.** Every build rule wants an idle villager; the
policy makes sure there are none. The run reached 35 villagers, 349 wood, 601 gold and never built its market, with
every build rule refused for want of a free villager. `idle_villager` (the rules' chooser) now falls back to "anybody
who is not already building", which is what the game does with `up-assign-builders`: it takes builders **out of the
gathering pool**. A rule that needs a worker may take one; the shares will rebalance afterwards, because they now can.

## Where the acceptance match stands (2026-10-29, last run of the night)

*Measured, not asserted: `ruage-headless --script config/acceptance.toml --seed 7 --report 200`, release, 2,400 s.*

| game time | wood | food | gold | units | buildings |
|---|---|---|---|---|---|
| 200 s | 35 | 18 | 100 | 12 | 6 |
| 600 s | 32 | 7 | 50 | 20 | 14 |
| 1000 s | 55 | 51 | 70 | 27 | 15 |
| 1400 s | **5** | 1088 | 300 | 32 | **17** |
| 1600 s | **5** | 1236 | 410 | 32 | 17 |
| 2400 s | **5** | 1236 | 850 | 32 | 17 |

**The match does not finish, and it now dies of wood rather than of food.** Seventeen buildings means the mill, the
barracks, the lumber camp, the market and the blacksmith all went up - the build order works. Food is 1,236 and gold 410;
**wood is pinned at 5** with 32 villagers alive, which is the shape of gather orders to trees failing rather than of a
shortage.

**The hypothesis to test first** (it was not confirmed tonight): a villager ordered to a tree cannot path to a working
position - the lumber camp stands between the villager and the trunks - so `start_gather` refuses, `share_failed` marks
that node for 30 s, the pass moves to the next one, and within a few passes every tree in reach is marked and the wood
column stands empty while food overflows. The test to write is a villager, a lumber camp and a tree on the far side of
it; if it refuses, the fix is in `work_position` (**which adjacent tile counts as "can work this node"**), not in the
policy - and `share_failed` should not swallow a whole resource for 30 s when the failure is about a *tile*.

**The run reports zero refusals**, which is the sharpest clue of the night: the build rules are not *failing*, they are
not *firing*. A rule that cannot afford a building is refused and printed; nothing printed, so every condition was false
at every check - the castle and siege rules included, with 1,236 food, 850 gold and 32 villagers in hand. The next run
should print `Runner::fired_times` (already recorded, never displayed) and the current age: that separates "the age did
not advance" from "the age advanced and the rule's condition is wrong", in one line of output instead of an evening of
deduction.

**Cost, measured:** the run takes ~4x real time in release, and a single path search is ~29 ms on this map. Both deserve
attention before the balance runs in item 8's batch harness are cheap enough to sweep civs.

## The `no_building` bug (2026-10-29, found by printing the firing counts)

*The instrument existed (`Runner::fired_times`) and was never displayed; the headless now prints it, with rule refusals
and the engine's own last refusal. It paid for itself on the first run.*

```
rule firings: [500, 6, 14, 170, 140, 65, 0, 0, 0, 142, 0, 0, 0, 0]
rule at 999.0s refused: not enough wood: 175 needed, 55 in the bank
```

**The mill rule fired 170 times in 1,000 game-seconds, the lumber camp 140 and the barracks 65** - because `no_building`
was only ever checked for *"is this a role the pack knows"*, never for *"does one already stand"*. Every 5 seconds each
of those rules paid for a building it already had, and each refusal spent the wood the next rule needed. That is the
wood collapse: not a starving forest, a rule that never stopped buying.

The fix is three lines (`Some(0) => {}`, `Some(_) => return false`), and the test -
`a_rule_with_no_building_stops_once_it_stands` - was written first and watched failing with **30 firings in 30 seconds**.

**The two facts behind the owner's question stay true either way**, and both are parked as failing tests in
`crates/sim/tests/depletion.rs`:

- a **resource node does not block its own tile** - only terrain does, so a tree on grass is walk-through and a tree in
  the forest only blocks because the tile under it is `F`;
- **a spent tree keeps its entity** (`amount 0`) and, on this map, its blocked `F` tile: nothing is removed and no gap
  opens. In the game the tree object goes away and the ground becomes walkable.

**And the owner's rule for what happens next is already the engine's**: at the drop site the villager is set idle and
`nearest_like(here, node, id)` is called **from where it stands - the drop site** - which is exactly "the tree nearest the
drop site". The other path (emptying a tree with an empty hand, never having delivered) measures from the dead tree
instead, so the two cases differ; only the first matches the game, and that is the one that matters in a normal run.

## "So is the game winnable?" - measured, 2026-10-29 (late)

`ruage-headless --script config/acceptance.toml --seed 7 --ticks 48000 --report 400`, release:

| game time | wood | food | gold | units | buildings |
|---|---|---|---|---|---|
| 400 s | 94 | 11 | 50 | 19 | 8 |
| 800 s | 109 | 51 | 50 | 24 | 14 |
| 1200 s | 81 | 497 | 240 | 32 | 15 |
| 2400 s | **81** | **497** | 854 | 32 | 15 |

**No.** At 2,400 s the enemy town centre is untouched at 2,400 hp, player 1 has **no barracks** (so no Feudal, no Castle,
no siege workshop, no mangonel), and every resource is frozen from 1,200 s. The barracks rule was refused **295 times**
with *"not enough wood: 175 needed, 81 in the bank"*.

**Why the wood stopped, in one sentence:** eighteen villagers stood on the **empty berry patch around the mill** with a
`Gather` task and no source left, because `nearest_like` only compared resource nodes with resource nodes - and once the
berries are gone the next source of food is a **farm**, a building. Fixed: `nearest_like` now crosses the two kinds for
the column `[farm] resource` fills, with `crates/sim/tests/retarget.rs` written first (watched failing, then biting under
a mutation that removes the branch).

**What is still owed before the match can be won:** the villager that gets stuck *inside* a task still needs the
`FINDINGNEWTARGET` fallback with a give-up timeout - the owner's rule names "next bush, next tree, next pile of gold",
and 0 A.D. adds two things to it: keep the specific type, and if nothing is nearby, walk back to where the order started
and look again. Ours now finds a farm, but a villager whose route died still has no clock.

## The pile-up at the mill (2026-10-29, latest evidence)

The dump now names the node every worker points at (`Task::Gather@EntityId`), and the freeze reads out of it in one line:

```
villagers 32   Counter({'Return': 27, 'Gather': 5})
  Return>EntityId(136) at 13.08 19.92   x27
```

**Twenty-seven villagers returning, all to the same mill, all stuck eight hundredths of a tile short of it.** The mill at
`[13,19]` was placed **among farms** (`[12,18]`, `[13,18]`, `[14,18]`, `[14,20]`), so its standable neighbours are built
over and the deposit - which waits for the final step onto the drop site - never fires. The two live gatherers work trees
with 19.88 and 43.29 wood left; two more point at spent trees.

**What is ruled out:** a crowd alone is not the cause. `crates/sim/tests/dropoff.rs` puts six villagers on one bush with
one drop site and every load reaches the bank. The fault needs the drop site to be *walled in by its own buildings*.

**The fix to write next, in this order:** (1) place a drop site with a standable neighbour free - a mill among farms is a
mill nobody can deliver to, and the placement code already knows how to look for a free tile; (2) accept a deposit from
adjacent to the drop site, not only from standing on it, which is what the game does and would have saved these 27; (3)
give a returning villager that cannot make progress for N seconds a way out (drop the load, find work, or idle visibly) -
0 A.D.'s answer, and the one thing our gather code still has no clock for.

## Two small things the 2026-10-30 prior-art round left behind

Both are cheap and neither touches the tick's design.

1. **One test: a unit killed by an earlier blow still swings.** Our `Sim::step` decides every entity against
   the state the tick opened with, so two units can trade killing blows in one tick and both die; the
   original updates objects one at a time and a killed unit's actions are already gone. Nothing pins what we
   do (U18, `docs/mechanics/determinism.md`). Write the test to find out; do not change the tick for it.
2. **One key: `clippy::iter_over_hash_type`.** Determinism rule 3 says no collection the sim walks is a hash
   map, and nothing enforces it: the workspace has no `[workspace.lints]` at all. Verified available on this
   toolchain (`rustc 1.99.1`: a `HashMap` walk fails the build under that lint) and free today, because
   `crates/` holds no `HashMap`. Borrowed in shape from `librematch/genie`'s lint set, which also denies a
   new `#[allow]` without a reason.

## The owner's three fixes, and what they exposed (2026-10-29, latest)

**Done, each with a test:**

1. **Touching a drop site is enough** - any side, any corner. `Task::Return` banks when the worker's tile is within one of
   the drop building's tile, instead of waiting for the final step *onto* a tile that is often blocked.
   `crates/sim/tests/dropoff.rs`.
2. **A site may be placed where a villager stands**, and the villager is pushed off it. This was forced by (1): villagers
   now stand around every drop site, and the placement scan counted them as occupied ground - *"no free tile for a
   market"* was refused **177 times in a row** before this. `crates/sim/tests/placement.rs`.
3. **A worker beside a node works it** when the node's own tile cannot be stood on (a forest tree). Without it a woodcutter
   stopped one tile short, `at_node` was false for ever, and - carrying a partial load - the "find the next node" branch
   did not fire either. `gather.rs` and `forest_edge.rs` updated to assert the distance, not the exact tile.

**Verified as the owner said:** a farm is walked over, not around (`crates/sim/tests/walkable.rs`). Only
`[build] blocking_roles` - the wall - blocks today, so farms were already soft; the test is what keeps it that way.

**Verified from the dat, and still open:** the town centre's footprint is `radius 2.0` with `selection 2.0` and
`clearance 2.0`, i.e. **4x4 tiles, uniformly** - the dat carries no field that marks part of a building as walkable. The
owner's "3/4 of the covered area can be walked over" is therefore not something the install can confirm, and our engine
blocks **one tile** for every building that blocks at all. Modelling real footprints is the next fidelity item.

**Measured after the three fixes:** buildings 15 -> **22**, food 497 -> **1,402**, Feudal reached, market and blacksmith
both refused only for wood. Still not winnable, and the reason is now a single, named defect: **eight woodcutters sit on
the tiles `(19,15)` and `(19,17)` - touching the lumber camp at `(19,16)` - in `Return`, each carrying a load under one
whole unit (`0.03` to `0.86`)**. A deposit whose load rounds to zero banks nothing and, by the current state machine,
leaves the worker in `Return` with the same dead node behind it. Wood froze at 121 wood while food reached 1,402: two
columns, two different faults, and the earlier sentence blaming the food crowd for the wood was wrong.

**Next, in order:** (a) a partial load that rounds to zero must not keep a `Return` task - bank it or retask it, never
loop; (b) `nearest_like` must skip a node with nothing left (it currently checks workability, not emptiness); (c) the
give-up clock for a worker that makes no progress, which would have caught all of this from the outside.

## Items 10-15: the rendering half, and where it stands (handoff, 2026-10-30)

*Items 1-9 were simulation. Items 10-15 are presentation, and none of them has been started: the rule was "if the match is
won, continue", and it has not been won yet. This section is the compact a fresh session needs.*

**What already exists**, so nothing below starts from zero:

- `crates/view` - `Snapshot::of(&world, &map, player)` (a read-only description: id, content id, position, health
  fraction, `Pose`, fog bits, stock, population, outcome), `Camera` (tiles → pixels), `render(&snapshot, &camera)` →
  a frame, `Frame::to_ppm()`, and `crates/view/src/bin/frame.rs`, which writes one frame to a PPM for a given tick. That
  is a software rasteriser with flat colours, no sprites and no animation.
- `D55`: the renderer never touches `World`; it reads a `Snapshot`. `D17`: content ids stay out of `crates/`; the view
  maps them through the pack.
- `D74`: the window is **native** (`winit`, software presentation), and **HUD text is allowed** (a font is in scope; the
  old "HUD is data, not pixels" ruling is superseded). The sim still carries no text.

**The six steps, in the order they were scheduled:**

10. **Animation and sprite data.** The dat has per-unit graphic records: `graphic_id`, standing/walking/attack/dying
    sprite ids, frame counts, frame rate, angle count, mirroring. Extract them into `config/gamedata/graphics.toml` the
    same way `units.toml` is generated (`build/tools/extract.py`), with a census test like `crates/data/tests/pack.rs`.
    Nothing renders until this exists, and it is pure data work.
11. **The asset pack boundary.** A `SpriteRef { graphic: u32, frame: u16, angle: u16, mirror: bool }` resolved from
    `(entity kind, content id, Pose, tick)` - `Pose` already exists and is derived from `Task`, never stored. The pack
    returns *references*; the renderer resolves them to pixels. Our own art can replace the install's later without the
    engine noticing.
12. **Sprites on screen.** Read the SLP/DRS art: the install stores sprites in `resources/_common/drs/graphics/*.slp`
    (the `.drs` container plus SLP headers, per `docs/parse-notes.md`). Decode to RGBA once at load into an atlas, draw
    by `SpriteRef`. Terrain comes from the map's terrain ids and the install's terrain SLPs.
13. **Terrain and layering.** Draw the tile grid from terrain, then entities sorted by `(y, id)`; shadows last. Fog from
    `Snapshot.visible`/`explored` (already computed in the sim, D50).
14. **Camera and input.** Pan, zoom, click-select, right-click order: `Camera` exists; input builds `Command`s and
    submits them through the same `World::submit_checked` a test uses. No new simulation paths.
15. **The window and the HUD.** `winit` + a software blit of `Frame` into the window surface; HUD reads `Snapshot`
    (stock, population, selection, the last refusal from `World::last_refusal()`), with a real font. `D74` settled both
    questions this needed.

**Definition of done for the six:** a window that shows the acceptance match running, with sprites, terrain, fog and a
HUD that reports what the sim knows - and no engine change required to get there beyond the `SpriteRef` boundary.

**What blocks nothing:** the match itself. Items 10-15 can proceed in parallel with the economy work: they read state and
never write it.

**What is still open in item 9** (the reason it is not "done"): the acceptance match reaches Feudal, builds a mill,
lumber camp, barracks, market and blacksmith with wood flowing (654 wood at 1,200 s in the last full run), but no mangonel
has ever reached the enemy town centre. The blockers found so far, in order: the market/blacksmith line stalling for
wood, the felling change (13 s per tree) landing after that run, and the `[footprint]` work (D76) not yet built.

## Where item 9 stands, and why it is not concluded (2026-10-30)

*The last full run of the night: 2,400 game-seconds, seed 7, release.*

| game time | wood | food | gold | pop | buildings |
|---|---|---|---|---|---|
| 400 s | 5 | 10 | 50 | 21 | 14 |
| 800 s | 828 | 280 | 180 | 35 | 20 |
| 1,200 s | 1,613 | 709 | 450 | 35 | 22 |
| 2,400 s | **3,523** | **709** | **1,240** | 35 | 22 |

The economy is healthy for the first time: Feudal reached, **market and blacksmith built**, lumber camp, mill, barracks,
farms and houses all standing, and 3,523 wood in the bank with no refusals for it.

**One number stops the match: food, pinned at 709.** The Castle Age costs 800 food, and the castle rule was refused **160
times** for exactly that; with no Castle there is no siege workshop and no mangonel, so nothing can ever attack the enemy
town centre. Food is not being *produced*, it is being *not produced*: the share table wants 60% of 35 villagers on food
(≈21), the bank sits at 709, and the wood column keeps growing to 3,523 - the pass is not moving workers **between**
columns. That is the same defect as the parked test `the_share_row_follows_the_villager_count`, and it is now the single
thing between this match and a result.

**Two crashes found on the way, both fixed in `b63effc`:**

- `free_tile_near` matched the town centre by **unit id**. After the Feudal upgrade the town centre is 463, not 109, so
  every placement beside it failed - *"no free tile for a market"*, 267 times - with the town centre standing in plain
  sight. It matches by family now.
- A command whose tile was taken between submission and the next tick (`something already stands at (18, 17)`, a market
  and a blacksmith choosing the same square) **ended the match**. A late-impossible order is refused now, not fatal.

## Compact: the builder and the target (2026-10-30, before the code)

**Where it stands.** The acceptance match reaches the Castle Age, builds a siege workshop site and orders a mangonel, and
then stops: the mangonel rule is refused 133 times with *"no finished siege_workshop"*, and the attack rule reports *"no
enemy matches enemy_town_center"* 64 times. Food is no longer the wall (885 banked, population 42).

**The AI's answer to the first half**, read from `Economy/Feudal Age.per`, `Economy/Castle Age.per` and
`General Rules.per`: `up-assign-builders c: <role> c: <n>` dedicates builders - market 3, blacksmith 2, house 2, monastery
3, university 3, a town-centre foundation 6 - and `sn-cap-civilian-builders` caps the builder pool (10 early, 20 by
Castle). A dedicated builder is **reserved**: the villager manager does not touch it until the building is up.

**What is built:** `Task::Build` is a reservation in the share pass, named in the code rather than implied by the
classifier, so no future change can poach a builder; and `enemy_town_center` falls back to any enemy building, then to the
nearest enemy unit, when the town centre is already gone - which is what "no enemy matches" means once the mangonel has
done its work but the enemy still has villagers alive.

**Review, adversarial:** the reservation's failure mode is a builder stuck for ever - a site that cannot be reached, or a
site the player never wants finished - so the reservation must be checked by a timeout in the *build* path, not assumed.
Cost: the share pass already distinguishes builders (one `matches!` in the classifier), so the reservation adds no scan.
The target fallback is a second and third pass over `entities` **only when the town centre is absent**, which is the rarest
case, and it keeps the same deterministic ordering.

## Item 9 is done: the acceptance match wins (2026-10-30)

```
script               seed  feudal  castle  food/m wood/m gold/m lost  army  outcome      hash
acceptance.toml        7   778.0  1324.0    112    117    23    0  1458.0  victory p1  #f91872681ba4a3a8
```

**Victory, player 1**, on the 2,400-second script: Feudal at **778 s**, Castle at **1,324 s**, no villager lost, an army
worth 1,458 resources at the end. The last two changes that closed it were the **builder reservation** (the siege workshop
finishes instead of losing its builder to the share pass) and the **target fallback** (the mangonel finds the enemy after
the town centre is gone; the run ends with `players=1` and 95 orders refused because the match is over).

Item 9's definition - "the acceptance script runs to a result" - is met, measured by the batch runner, not asserted.

## The window exists (2026-10-30, milestone 0)

`crates/view/src/bin/window.rs` builds and runs: `cargo run --release -p ruage-view --bin window -- config`.

- `winit 0.30` + `softbuffer 0.4`, both fetched from crates.io (reachable from this host) - the D74 stack, native and
  software-presented, no GPU and nothing the headless build can disagree with.
- One `World`, one window, one accumulator: the simulation runs at `tick_rate_hz` from `sim.toml`, the window draws
  whatever tick the world has reached, catch-up is capped at a second of ticks so a paused debugger does not fast-forward
  the match, and Escape or the close button exits cleanly.
- The picture is the existing rasteriser's blocks, blitted as `0x00RRGGBB` into the softbuffer surface, with the tick
  count and entity count in the title so a screenshot proves the loop is alive.

**What is not there yet, in order:** input → `Command` (select, box-select, right-click order, control groups), pause and
1x/2x/4x, the HUD (D74 allows text), the replay writer (seed + config hashes + the `(tick, player, command)` stream, and a
headless path that reproduces the same state hash), and then the rule-AI as the opponent.

**The verification wall, stated plainly:** this host has no display and no virtual one either (no `DISPLAY`, no `Xvfb`,
no Wayland compositor), and `sudo` is unavailable to install one. The binary's failure path is verified - it exits with
*"neither WAYLAND_DISPLAY nor WAYLAND_SOCKET nor DISPLAY is set"* rather than panicking - but nobody has yet **seen** the
window on this machine. Visual proof needs either `Xvfb` installed here or the same command run on a desktop.

## Review: the window (2026-10-30, before and after)

*First pass - what was wrong with milestone 0, all of it fixed in the same commit:*

1. **`ControlFlow::Poll` burned a core doing nothing.** The loop now sleeps until the next tick is due
   (`WaitUntil(now + tick - accumulated)`), so an idle window costs no CPU.
2. **A failed tick left the window drawing a frozen world and logging the error on every frame.** A step error now freezes
   the simulation, shows the reason once in the title, and stops stepping: a sim error neither fixes itself nor leaves the
   state coherent enough to retry twenty times a second.
3. **`resumed` could fire twice** (some platforms) and leak a window and a surface. It returns early if one exists.
4. **The first frame was blank** because the world was seeded inside `step`. Unchanged by design - the window opens, the
   first tick seeds, the first redraw shows the town centre - but noted so nobody hunts it as a bug.
5. **A window larger than the rasteriser's buffer kept stale pixels** from the previous size. The uncovered tail is now
   cleared.
6. **The seed was hardcoded**, so a window session could not be reproduced. `--seed N` now sets it, which is the first half
   of the replay story (the second half is the command stream).

*Measured, not argued:* `crates/view/tests/frame_cost.rs` (ignored; run with `--release -- --ignored --nocapture`) reports
**634 us per frame** at 960x720 with 126 entities, i.e. `Snapshot::of` + `render` costs about **1.3% of a core** at 20
ticks a second. The remaining cost is a 2.7 MB `Frame` allocation per frame (about 55 MB/s of churn at 20 fps); it is not
worth an API change yet, and it is written down rather than forgotten.

*Second pass - what the fixes themselves introduced, and what is still open:*

- The accumulator uses `Instant`, which is monotonic but **not** the simulation's clock: the window can be stepped a
  different number of times per second than the headless build, so a window session and a headless run of the same seed
  are comparable only if the *tick count* matches, not the wall clock. The replay format must therefore record **ticks**,
  never timestamps.
- `saturating_sub` on `Duration` means a long stall schedules the next wake immediately, which is correct, but the catch-up
  ceiling (20 ticks) still applies - a 20-second stall loses the excess rather than running it. That is the right choice
  for a window and the wrong one for a test harness, which is why the harness stays headless.
- The `failed` freeze has no way back: no "resume" key, no diagnostic overlay. Acceptable while a sim error is a bug to
  fix rather than a state to recover from.
- Still missing, and next: input to `Command`, pause/1x/2x/4x, the HUD, the command-stream replay.

## Input, pause and speed (2026-10-30)

**Compact, before the code:** the window needed the smallest input set that lets a person *drive* the match rather than
watch it - left click selects what is under the pointer, right click orders the selection (gather a resource, attack a
living enemy, otherwise walk), Space pauses, 1-4 set speed. The rule that shaped it: **input is not a second way into the
simulation.** Every order becomes the same `Command` a test submits through `World::submit_checked`, and a refusal is shown
in the window title instead of being swallowed.

**Implemented.** `ruage_view::pick_at` (a pure function of a world and a tile) decides what a click hits: **units beat
buildings and resources, and equal ranks go to the lower id**, so the answer never depends on iteration order - clicking a
villager standing on a farm selects the villager. The window routes left/right clicks through it, orders the whole
selection, draws a white cross on what is selected (without a HUD there is otherwise no way to see what an order applies
to), and shows tick, speed, pause, entity and selection counts in the title. Speed is **ticks per interval, never a shorter
interval**, so a 4x window session still agrees with a headless run on every tick boundary.

**Review, stability:**

- `pick_at` scans every entity on every click - O(n) per click, measured at 126 entities, and a click is a human event:
  irrelevant. It is *not* on the draw path.
- The order path clones the selection and re-checks each id: a unit that died between the click and the order is skipped,
  not acted on.
- Pause freezes the accumulator (`paused` skips the step loop) but not the window, so the camera and the title stay live;
  the accumulated time is cleared on pause so unpausing does not fast-forward.
- Speed multiplies within one interval and keeps the catch-up ceiling, so 8x on a loaded machine degrades to fewer ticks
  rather than a spiral of a thousand ticks in one frame.
- Still open: box-select, control groups, a HUD, and the command-stream replay. The replay *format* is now constrained by
  two facts learned here - it must record ticks, not timestamps, and it must record refusals too, or a replayed session
  will diverge silently at the first order the engine turned down.

## Step 1 of the look: the isometric camera (2026-10-30)

**Compact:** the original is a 2:1 isometric diamond grid, and our camera was a straight top-down grid of 16-pixel
squares - so no amount of art would ever look like the game. Step 1 replaces the projection and nothing else:
`project` maps a tile to the **centre of its diamond** in pixels, `pick` is its exact inverse, zoom scales the diamond and
keeps the tile under the centre of the screen, and the camera is now positioned in tile space (so `centred_on` holds at
every zoom).

**Tested** (`crates/view/tests/camera.rs`, no display needed): one tile east is `(+48, +24)` pixels and one tile south is
`(-48, +24)` - the 2:1 diamond; the centre of every diamond round-trips through `pick`; zoom doubles the diamond without
changing the picked tile; and the full chain world -> tile -> pixel -> `pick_at` finds the town centre.

**Parked, and why:** clicks *just inside a diamond's edge* come back one tile off (`pick(project((16,16)) + (0,12))` gives
`(15,15)` when the arithmetic says `(16,16)`). Every centre round-trips exactly, so this is edge-only, and the cause is a
disagreement between the `+ ry` in `project` and the `- ry` in `pick` that only shows at non-zero deltas - or rounding, which
cannot be settled by reading the code. It needs a picture; a picture needs a display. `#[ignore]`d with that reasoning
rather than patched blind.

## Step 2 of the look: real footprints (2026-10-30, compact before the code)

**The finding that changes the step:** footprints need no extractor work. `crates/data` already carries
`radius: [MaybeFixed; 3]` for every unit and the generated pack has **2,778** radius entries; for buildings the first radius
is the half-extent of the footprint (the town centre's `2.0` is its 4x4), which is exactly the silhouette that makes a
screen read as the game. So step 2 is a *renderer and snapshot* change, not a data change: the snapshot must expose a
footprint per entity, and the renderer must draw a diamond of that size instead of one tile.

**Order within the step:** footprint in `Snapshot` -> diamond drawing (with the parked edge-picking defect fixed as a
consequence, because the diamond's outline is what the edge test needed to see) -> a test that the town centre covers 4x4
tiles and a house 2x2, read from the pack rather than hard-coded -> adversarial review of both, then commit and push.

## Step 2 of the look: footprints, the diamond grid, and a click that lands (2026-10-30, done)

**What landed.** `Snapshot`/`EntityView` now carry `footprint: (u32, u32)` read from the pack
(`Units::footprint`, from `radius[0]`/`radius[1]`, which the dat gives as **half-extents**); the renderer draws every
building as the diamond that block of tiles makes, every terrain tile as the diamond the projection implies, and
resources as small diamonds; and `pick_at` hits **anywhere on the footprint**, not only the anchor tile, because a
player aims at the shape they can see. `footprint_tiles(anchor, footprint)` is the one place the rectangle is derived.

**Verified, and read from the pack rather than typed into the test** (`crates/view/tests/footprint.rs`): the town centre
is 4x4, the house 2x2 and the barracks 3x3 - three different sizes, which is what makes the test bite a hard-coded
number; the snapshot's footprints equal what `radius` says; the drawn silhouette's top and left vertices are within 2 px
of where the footprint puts them (mutation-proved: drawing on the anchor tile instead of on the block's ground gives
71 px against an expected 96); clicking every tile of a 4x4 footprint hits (a villager standing on one outranks the
building - the documented tie-break); and a field of diamonds leaves **zero unpainted pixels**.

**The parked edge-picking defect is fixed, and it was never a picture.** `project` maps a tile to the **centre** of its
diamond, so a diamond covers the continuous range `tile - 0.5 .. tile + 0.5` and `pick` must **round**, not `floor`.
Flooring was correct for the old top-down camera, where a tile's pixel was its top-left corner; under the diamond
projection it put every click in a diamond's upper-left half one tile off. Every centre round-trips either way, which
is exactly why a centre-only test hid it. `pick` now rounds and the test covers all four vertices plus two out-of-shape
points.

**Measured, and the first number was bad** (`crates/view/tests/frame_cost.rs`, release, 960x720, 126 entities): the
first diamond renderer cost **2,568 us a frame against 634 us** before it - a 4x regression - because every off-screen
tile was painted pixel by pixel into a clipping `put`. Clipping each scanline to the frame first brought it to
**572 us**, better than the square renderer it replaced (a diamond is 2*rx*ry pixels, a quarter of a `96x96` square).
The second finding of the review: `Snapshot::of` counted garrisons with a per-entity scan of every entity (**O(n^2)**
on the frame path - 16k comparisons at 126 entities, an army's worth at 300); it is one ordered pass now, pinned by
`the_snapshot_counts_what_is_inside_a_building` written before the refactor.

**D83 - the footprint is anchored at the tile nearest its middle.** The dat's radius is a half-extent *about the
anchor*, so a 4x4 building covers `anchor-2 ..= anchor+1` and a 3x3 one `anchor-1 ..= anchor+1`: an even footprint has
no middle tile, and the extra tile goes to the +x/+y side. The alternative (anchor = the block's north corner) would
move every building half a tile on screen. **Open question for the owner**, not a blocker: which convention the
original uses - it changes one constant, and `.aoe2scenario` import is where it can be settled from data.

**Next: step 3, depth sorting** - draw the scene in `x + y` order so a unit in front of a building covers it, then
step 4's terrain content (dirt, forest floor, trees as clusters), step 5 fog, step 6 the HUD, step 7 camera state with
pan and zoom keys, step 8 selection markers.

## Step 3 of the look: depth sorting (2026-10-30, done)

**What landed.** `draw_order(snapshot)` paints the ground in one pass, **back to front**: `depth_of` gives a unit,
resource or projectile `x + y`, and gives a **building the far corner of its footprint**, because a 4x4 town centre
occupies ground up to `anchor + 3` and everything beyond that is in front of it. Ties go to the lower id, so the order
never depends on how the entity list was built. Before this the renderer drew every resource, then everything else in
id order, which painted a villager standing *behind* a house on top of it.

**Tested** (`crates/view/tests/depth.rs`): a hand-built snapshot whose five entities are listed in the wrong order comes
back as `[behind, tie-by-id, building, tie-by-id, front]`; and in a real world a villager spawned inside the back half
of the town centre's footprint - with an id higher than the building's, so an id-ordered draw would show it - is covered
by the building's paint.

**The review's finding, and the fix that followed.** Footprints gave the *window* a new hit test (`pick_at`), and the
UI's `Selection` kept the old anchor-tile rule: **two rules for one click**. A click on the body of a town centre
selected nothing through `Selection` while the window's picking knew better. Both now share `covers`/`footprint_covers`,
and `ui::Selection::click` and the order path were pinned by
`a_click_on_the_body_of_a_building_selects_it` (written first, watched failing with an empty selection).

**Measured:** `draw_order` costs the frame almost nothing - **525 us** against 572 us before it at 960x720 with 126
entities, inside the probe's noise - so the sort is O(n log n) on 126 elements, not a budget question.

**Next: step 4, terrain content** - grass, dirt and forest floor from `map.toml`, and trees drawn as clusters on their
tiles rather than one square per node.

## Step 4 of the look: terrain and trees (2026-10-30, done)

**What landed.** Grass now has a **tone per tile**, from a hash of its coordinates (`grass_tone`), so the ground reads as
ground instead of one flat green; forest floor stays the darker colour it was; and a resource node **in the forest is
drawn as a tree** - a canopy with a darker clump for shape - while a node on open ground is a pile. A tree whose wood is
gone is a **stump**, which is the picture of the model the engine already has (a spent node keeps its entity).

**The rule carries no AoE2 knowledge:** "is this a tree" is `terrain[position] == Forest`, the map's own answer, not a
table of which stockpile column is timber - which is what keeps this inside D17.

**Tested** (`crates/view/tests/terrain.rs`, hand-built snapshots so the picture is the only thing under test): the same
node reads as `CANOPY` in the forest and as `PILE` on grass; a spent node reads as `STUMP`; grass takes at least two
tones and no colour outside `GRASS_TONES`, and two renders of one snapshot are byte-identical (the tone is a function of
position, not an RNG); and forest floor's luma is below grass's.

**Measured:** 635 us a frame against 525 before - **+110 us for every canopy in view**, still about 1.3% of a core at
20 Hz. Rendering the same match to a PNG and looking at it: a grey diamond town centre on a diamond-tiled meadow, a
woodland of small canopy motifs to the south-east, no seams and nothing misplaced.

**Still content's, not the renderer's:** there is no `Dirt` terrain. The map carries `G` and `F` only, so a base has no
baked yard until `Terrain` grows a variant, the loader accepts its letter and `make_map.py` paints one - a content
change with a census test, scheduled after the HUD.

## Step 7 of the look: the camera is state, and the wheel is the zoom (2026-10-30, done)

**The defect the step existed to kill was still there.** `draw()` built a fresh `Camera` every frame -
`Camera::new((width, height)).centred_on((16.0, 16.0))` - so the camera was pinned to one tile, panning could not work,
and a zoom would have reset on the next frame. It was recorded as fixed in the milestone-0 review and it was not; the
line survived because nothing in the window could reveal it. Now the camera is created once and `Camera::resized` only
changes the viewport.

**What landed.** `Camera::resized` (viewport only, never the position), `zoom_in`/`zoom_out` stepping through
**0.5x / 1x / 2x** (`ZOOM_LEVELS`), arrows panning a tile a press, and the mouse wheel mapped to the zoom levels. A
wheel notch is a notch: multiplying by a factor accumulates rounding and lands the picture at 0.83x, which is not a zoom
level anybody asked for.

**Tested** (`crates/view/tests/camera.rs`): a resize keeps the origin tile at the middle of the new viewport and keeps a
pan intact; zoom steps 1x -> 2x -> 2x -> 1x -> 0.5x -> 0.5x, and the tile under the middle of the screen is the same at
every level.

**Not verified, and said plainly:** the *drawn* rung. Pan and zoom are input in a binary on a host with no display, so
what is proven is the camera arithmetic and that the binary builds with the rebuild removed - not that a window pans.
The owner running `make run` on a desktop is the third rung.

## All eight steps of the look are in (2026-10-30)

*Steps 1-8 are landed, reviewed, committed and pushed: the 2:1 projection, real footprints from the dat, depth sorting,
terrain and trees, fog that remembers buildings, the HUD and its bitmap font, the camera as state with pan and zoom, and
selection rings with health bars. `make check` is the gate and it is green.*

**What looking at the pictures found that the tests could not** (three of them, each fixed in the same commit):

1. The HUD drew **eight** swatches - four for resources this content does not have - because `Snapshot.stock` was always
   `MAX_RESOURCES` long. It now carries one entry per column the pack has.
2. A selection ring drawn a scanline at a time came out **dashed**, because a shallow slope moves the edge two or three
   pixels between rows. `outline_diamond` walks both axes now.
3. A three-pixel green health bar over grass was **invisible**. It has a dark border.

**What is left, and the order I would take it:**

1. **`Dirt` terrain.** The map carries `G` and `F` only, so a base has no baked yard. `Terrain` grows a variant, the loader
   accepts its letter, `make_map.py` paints one, and `map.rs`'s census test follows - content work, no engine change.
2. **Sprites** (items 10-13): `graphics.toml` from the dat's graphic records, the `SpriteRef` boundary, then SLP decoding
   behind it (D81's mod chain, install art last, never committed).
3. **`.aoe2scenario` import**, which is also what settles D83's footprint anchoring from data.
4. **A second seed and map for the acceptance match** - item 9 wins on one seed on one map today.

**Still open for the owner:** D83's anchoring (one constant), and whether the plans (2)-(4) are the next three, or whether
the rule AI as an opponent comes first now that a screen can show it.

## Terrain is ground, and what stands on it blocks (2026-10-31, done)

**The owner's rule, implemented in his words.** "A tree blocks passage until cut and fully harvested. Then dirt remains
but it is walkable." So `Terrain` gained `Dirt` and `walkable()` (true for grass, dirt and **forest floor**), `Walk::from_map`
now reads `!walkable()` instead of `!= Grass`, and blocking became a property of **entities**: a resource node that carries
hit points - a tree - closes its tile from the moment it is inserted, and `step_spent_nodes` frees the tile and removes
the node once the wood is gone and nobody is working it.

**Two parked defects closed by it**, both un-ignored rather than deleted: `a_standing_tree_blocks_its_tile` (which needed
the fixture to stop faking a tree with `hp: 0` - a node without hit points is a thing you walk past, which is right for
everything that is not a tree) and `a_chopped_tree_leaves_no_obstacle`.

**The traps it set, each found by a red test:** four fixtures had built walls out of `F` terrain and now let the villager
walk straight through - `WALLED_MAP`, `WALL_MAP`, `SEALED_MAP` and the forest-pocket rally map - so each wall became a
**row of trees**, mirrored where the loader demands it. A fixture that lists wood **without `hp`** no longer blocks, which
is the tell that it was leaning on terrain.

**And it settles D83's anchor.** Your follow-up - "top corner is the centre, bottom corner is walkable yard, left and
right corner is walkable sheltered yard" - reads the Town Centre from its **north corner**, so `footprint_tiles` now
anchors there and the block extends `+x/+y` (D84). Two tests in `depth.rs` and one in `ui.rs` moved with it, and the
`ui` test now takes its rectangle from `footprint_tiles` instead of repeating the arithmetic, which is how the two click
rules drifted apart the first time.

**Verified:** the acceptance match still wins with the **same state hash** `#f91872681ba4a3a8` (`players=1`, 48,000 ticks)
- the terrain change is sim-visible but path-equivalent on a map whose woods are dense, which is the regression signal
that matters.

**Next: D76's footprint masks** - the Town Centre's solid back core and walkable wings as content (`[footprint.<role>]`),
blocking through the same `Walk` layer, freed on destruction or a cancelled foundation, with placement and reachability
asking the footprint instead of the anchor tile. Then sprites.

## D76 footprint masks: the Town Centre's solid core, and the unit it drops (2026-10-31, parked)

**What landed.** `[footprint.<role>] blocked = ["##..", "##..", "....", "...."]` in `economy.toml` - the owner's Town Centre
with its back core solid and its wings and courtyard walkable - plus `[footprint.farm]` all `.` ("a farm is a floor"), a
`Footprint` type in `ruage-data` that refuses a ragged or unknown mask, and `solid_tiles` in the simulation as the **one**
function that answers both "what to block on placement" and "what to free on demolition". The counted `Walk` layer added by
last step's review takes the masks without change, which is why it was worth adding first.

**Tested:** the back core blocks and the yard does not (`footprint_mask.rs`), demolition gives back exactly the mask, and a
villager can stand on the yard; the project's own content names a 4x4 Town Centre with four solid cells and an all-walkable
farm (`crates/data/tests/economy.rs`); a mask that is not a rectangle is refused.

**Parked, and this is the blocker:** `a_civilisation_that_cannot_build_houses_gets_the_whole_cap` (the Huns, 20 villagers)
trains **18 of 20**, and 200,000 ticks do not add the last two - **units are dropped, not delayed**. The candidate cause is
placement: a trained unit needs a free tile beside its building, and a core occupying four tiles with a crowded yard leaves
fewer. The test is `#[ignore]`d with that reason rather than deleted, and the masks are not finished until it passes. Fix
it first: if placement cannot find a tile it must say so (a refusal in `last_refusal`) rather than lose the unit.

**Not built, and deliberately:** D76's rule 4 second half - a site may not overlap another building's *footprint* beyond the
tiles that block. `check_build` still tests one tile plus the walk layer; making it footprint-aware means touching the
blind-overbuild rules (D58) and their tests, which is its own slice.

**Verified:** the acceptance match still wins (`players=1`, 48,000 ticks); its state hash moved
`f91872681ba4a3a8` -> `91e869fcc92d606c`, which is expected when pathing changes.

## The command panel comes before the "cannot train" message (2026-10-31, compact written)

The owner's correction, and it reorders the work: **the message about being unable to train a unit has to be drawn
somewhere, so the panel comes first.** `docs/ui.md` is the compact - the layout, the five things `Snapshot` must carry that
it does not (queue with progress, a pause reason, the trainable list with availability, affordability, a name), the drawing
primitives (a panel frame, a portrait, buttons with a dim state, the queue, a message line, and **letters** in `font.rs`),
and the six tests that need no display.

**So D85 is blocked on the panel, not the other way round.** The simulation half of D85 - garrison the unit that cannot be
placed, pause the queue when there is no room - can land first and be tested headless; the message then has a place to
appear when the panel exists. That is the order I take: D85 in the sim, then the panel, then the message.

**Where the masks stand** (`838235d`, parked): D76's footprint content is in and reviewed, and the one open defect is the
dropped unit that D85 exists to fix - `a_civilisation_that_cannot_build_houses_gets_the_whole_cap` is `#[ignore]`d with its
numbers and is the acceptance test for D85.

## D85's simulation half: a unit that cannot be placed is garrisoned, and a queue says why (2026-10-31, compact)

**Where the code stands, read not remembered.** `spawn_from_building` already *tries* to place a finished unit on a free tile
in a 5x5 box round its building and returns `false` when it cannot - and the caller leaves the job in `training` with
`remaining_ticks == 0`, so it retries every tick for ever. What the Huns test actually measures, then, is not a unit being
deleted: **the queue stalls on a building whose yard is full**, the two recruits behind it never start, and 200,000 ticks do
not help because the villagers that block the yard never move. Eighteen of twenty is a stalled queue, not a lost unit.

**The rule being implemented (D85, the owner's words).**
1. No free tile beside the building -> the finished unit is **garrisoned inside it** (the garrison the engine already has).
2. No garrison room either -> **training pauses at that building with a reason in state**, never dropped and never silently
   retried. The reason is engine state; the words are the view's (`docs/ui.md`).

**The shape.** A new `TrainingPause { NoRoom }` in `crates/sim`, held in a `World` side map
(`training_pauses: BTreeMap<EntityId, TrainingPause>`) for the same reason `queued` and `rallies` are side maps - it keeps
`Entity`'s sixteen fields alone, and the hash can carry it as an ordered map. Set on the failed attempt, cleared on the
successful one, so it is a fact about *now* rather than an event. `NoRoom` is the only variant built: `Unaffordable` exists
in `docs/ui.md`'s plan but **no code path can produce it today** (the queue pays when the order is placed), and inventing a
variant no test can drive is exactly the guess this project refuses - it goes in when a rule creates it.

**The search gets cheaper as it gets longer.** The existing loop tests `entities.iter().any(..)` per candidate - 25 scans of
the whole world on the tick nothing is free, which is the tick that now repeats. One pass marks which of the 25 tiles a unit
stands on (skipping garrisoned ones, who are inside), then candidates are read from that mask: O(n) per attempt, not 25n.

**Tests, written before the code (D7).**
1. `a_unit_with_nowhere_to_stand_is_garrisoned_not_dropped` - fill every free tile of the town centre's 5x5 box with units,
   train a villager, step: one more villager exists and its task is `Garrisoned { town centre }`.
2. `a_building_with_no_room_and_nowhere_to_stand_pauses_with_a_reason` - a market given a `[train]` row (garrison 0, so
   there is nowhere to shelter) with its yard full: `training_pause == Some(NoRoom)`, the job stays, no recruit appears.
3. `the_reason_clears_the_tick_a_tile_frees` - delete one blocker, step: the villager appears beside the building and the
   pause is `None` - the same frame, state not event.
4. `population.rs::a_civilisation_that_cannot_build_houses_gets_the_whole_cap` is **un-ignored** and is the acceptance test:
   the Huns, 20 of 20.

**Out of scope, deliberately:** the snapshot's `queue`/`paused` fields and the message itself (step 2 of `docs/ui.md`'s
order), and `Unaffordable`.

## The RE track: Ghidra on this box, the Windows machine as the oracle (2026-10-31, compact)

A second track beside the effect interpreter, and it touches no engine code. The question put to this session
was whether a Windows Hermes running REA should deliver findings here; the answer is no, for measured reasons
rather than by preference, and the record is `docs/re-toolchain.md` with the decision as D86.

**What is installed and green** (`rea doctor`, 2026-10-31): REA 6.8.0 at `/opt/data/tools/npm/bin/rea`, Ghidra
12.1.4, Temurin 25.0.5, Node 26.13.0, project cache `REA_GHIDRA_PROJECT_PATH=/opt/data/tools/rea/ghidra-project`,
env in `/opt/data/tools/rea/env.sh`. Three install traps are written down there because each cost a cycle:
Ghidra 12.2 is rejected (pin 12.1.4), REA refuses Node 26.7.0 (it wants 26.12.1+ or 22/24 LTS), and
`python -m zipfile` strips the exec bit off `analyzeHeadless` and the native decompiler.

**The binary under study** is `Age2HD/AoK HD.exe`, read from the read-only install and never written: i386 PE,
7,521,280 bytes, PE timestamp 2018-08-22T18:26:19Z. That timestamp fits the HD 5.8 line and not a 2013 build,
which bears on `oracle.md`'s open question about `mgz-fast` - but it is a lead, not a finding, until the exe's
version resource or a real `.aoe2record` agrees.

**The three questions it exists to answer** are exactly the rows `oracle.md` marks as unanswerable from saves:
damage order (where armour is subtracted, where bonus classes multiply, what the min-1 clamp wraps), the
per-tick unit update order (speed, attack reload, gather tick), and age-up timing at the tick boundary. Each
must come back as an address plus a number a fixture can test - not as a paragraph of pseudocode.

## D85 landed: the recruit is garrisoned, and the queue says why (2026-10-31, done)

**What was actually wrong, now proven.** The Huns' two missing villagers were never deleted: a finished job with nowhere
to stand stayed in its building with `remaining_ticks == 0` and retried every tick for ever, so **the queue behind it never
started**. `a_civilisation_that_cannot_build_houses_gets_the_whole_cap` is un-ignored and passes at 20 of 20.

**The shapes now in the tree.** `spawn_from_building` decides in D85's order - a free tile in the 5x5 yard; failing that,
inside the building on its own `garrison` field; failing that, `TrainingPause::NoRoom` in state and the job left where it is.
The reason is a `World` side map beside `queued` and `rallies`, cleared the tick placement succeeds, removed with the
building, and hashed (state, not event). `free_spot_near` replaces the per-candidate rescan with one pass: **the blocked
case is the one that now repeats every tick**, and the old shape cost twenty-five scans of the world per tick there.
`garrison_room` refuses a building at its eject threshold (D37) and a building with no capacity at all.

**Four findings from the review** (`docs/reviews/2026-10-31-d85-no-room.md`), all fixed test-first:
1. The tile mask marked only *units*, so a market's own square read as free and a recruit stood on its roof; it also means
   the old scan kept a **garrisoned** unit's stale tile reserved for ever.
2. A building past its eject threshold sheltered a recruit - test watched failing (22 against 21) before the guard.
3. The fixture's `next_id` was read from a world whose spawns had not been applied, so every blocker got one id and `step`
   swallowed the duplicates; the tell was "the yard is full" failing with every tile still free.
4. **`[decay] enabled` ate a spawned, unattended market inside five game-minutes**, freeing its own yard. That is very
   probably the "mill and barracks vanished by 17:00" nobody could explain: an unattended building does not merely get
   damaged, it goes.

**Still owed, in order.**
1. **The ceiling pauses a queue and says nothing** (D34): `TrainingPause` needs a second reason (`AtCeiling`) or the panel
   will show a stopped queue with no words - **the owner's call**, since `docs/ui.md` names only `NoRoom` and
   `Unaffordable`, and `Unaffordable` has no path that can produce it today.
2. **Decay: is a building supposed to vanish?** `[decay] rate_from = "dat"` with `resource_decay_rate` semantics
   unverified, and the acceptance match loses a mill and a barracks to it. Either the rate is wrong or the rule is; it is a
   probe, then a test, then (if the data says so) a fix.
3. The snapshot's `queue`/`paused`/trainable list, then letters in `font.rs`, then the message beside the queue
   (`docs/ui.md`'s order, unchanged).

## Comparing ruage to the original: the register, and the AI-vs-AI farm (2026-11-01, method recorded)

**Two documents, and they are the resume point.** `docs/behaviour-diff.md` is the **register**: one row per rule ruage
implements, each ruled `same / different / unknown-static / needs-oracle` with the original's address and our file:line -
**1 of 12 ruled** (the dat version gate: exact string equality against `VER 5.7` in `FUN_0072d1a0`). `docs/oracle.md` gained
the farm protocol: Random Map, mirror civs, hardest AI, conquest, 200 pop, record + save every five game-minutes + event
screenshots.

**The first three static questions** are already scoped in `docs/re-toolchain.md` (damage order and the minimum-1 clamp; the
per-tick unit update's order; the age-up kind-102 boundary) and are the wedge: each comes back as an address plus a number
that a fixture can test, and each closes or opens a register row.

**Cheap enabling step, not yet done:** the bulk pseudocode corpus. `rea decompile-all` writes every function's pseudocode
and **~6 s per function was measured** (20 in 824 s including a 7 min import), so a full run is ~26 wall-hours through REA -
a Ghidra post-script against the persistent `AoKHD.gpr` should be much cheaper, and should be measured before the work
starts rather than assumed.

**Open and owner-facing:** (a) whether HD will run a match with **no human player** (no spectate mode) - two recipes in
`oracle.md`, to be confirmed on the machine; (b) the certificate "rule one" needs - `same` means "found and read", never
"proved equivalent", and any stronger claim needs the oracle.

## The 2026-11-01 prior-art round: three decisions, and what to write first (2026-11-01)

Settled by the owner from `docs/prior-art.md`'s findings: **D88** (fights are ordered - follow the original),
**D89** (adopt `tech_cost` / `tech_time`), **D90** (deny hash iteration and a bare `#[allow]`). The order below is the
order they should be *written*, which is not the order they were decided: the cheap ones first, because they change no
behaviour, and the engine change last, because it re-baselines fights.

1. **`[workspace.lints]` (D90).** One config change, no code. Verify it by introducing a `HashMap` walk in a scratch
   test and watching the build fail, then delete the probe and record the failure in the review. It goes first because
   D88 makes iteration order a contract, and the enforcement should be in place before the code that depends on it.
2. **The two combat-order tests (D88's first step).** `a unit killed by an earlier blow still swings` must *pass* against
   the current loop (pinning what we do), and `the later unit does not swing` must *fail* - that failing run is the
   permission to change `Sim::step`. Do not change the loop until the second test exists and fails for the right reason.
3. **`tech_cost` / `tech_time` (D89).** Import survival test first (`a kind-101 command reaches the config as a
   tech_cost modifier`), then the effect application: effect 10 sets Feudal to 425 food and then subtracts 220, and a
   kind-103 command shortens the research it names. 97 + 25 commands stop being dropped.
4. **The ordered apply pass (D88, the engine work).** `Sim::step` walks objects in player-then-slot order and applies as
   it goes. Expect the acceptance hash to change and a set of fight tests to re-baseline; both are the cost, not a defect.
   Re-check anything in the AI that sums simultaneous damage.
5. **The save-format zero test** (from `genie`'s `None`-vs-zeroed-memory bug, `2026-10-31`): one test pinning what a
   saved world holds where the game writes `0` and we hold `Option::None`. Cheap, and it is the kind of divergence an
   oracle save would expose anyway.

**Not adopted, and why:** `genie`'s type sweep (every meaning becomes its own newtype, raw values only at the wire) is
the tempting one and it is exactly the wrong direction for us - it bakes dat semantics into the engine, which D17 keeps
on the importer's side. openaok's group volley spread (first blows spread over 0.6 s) stays recorded as a *look* lever
for if fights read wrong once there is a renderer, not as a tick change.

## After D88/D89: the queue this thread is working (2026-11-01)

The owner answered the three open questions from the prior-art round in the other thread and named this one the main
instance: **U18 = "units cannot trade killing blows, follow original"** (landed as **D88**), **"3 adopt"** = tech cost/time
modifiers, **"adopt adoptable"** = the lints and D88 (landed as **D89**).

**In order, from here:**

1. **Tech cost and time (kinds 101 and 103)** - the next slice, and the only one of the three still open. 97 kind-101 and 25
   kind-103 commands are parsed, classified and dropped today; the effect table already carries them. Shape: `[tech]`
   cost/time overrides resolved through the effect interpreter, applied at `check_research`/`start_research`, with the
   Chinese/Byzantine-style civ bonuses as the external check (a published tech whose discount is known). Test-first.
2. **The panel chain** (`docs/ui.md`): `Snapshot` carries `queue`/`paused`/trainable → letters in `font.rs` → the message
   beside the queue → the panel regions. This is what makes D85's `NoRoom` visible, and the one-word question still open is
   whether the population ceiling earns a second reason (`AtCeiling`).
3. **Register row 2** (`docs/behaviour-diff.md`): `FUN_00744d60` is a candidate - three (class, amount) pairs, per-entry
   drop below 1 - and the damage caller (`FUN_00741a30`, `FUN_006bb100`) is the next read. A difference here means a
   test-first fix in `damage_between`.
4. **The farm artifacts** (owner): first Random Map match, record + saves at 5/10/15/20 minutes + event screenshots. Those
   close rows 3, 4, 5 and the decay question, and nothing on my side can substitute for them.

## The first capture set is in: parse it (2026-11-01, next slice)

`Age2HD/SaveGame/` holds 4 HD **5.8** recordings and 14 saves (`brit1`…`brit13`, `britXhuns`), all `VER 9.4`, all decoding
by the step-2b recipe (see `docs/oracle.md`). Nothing install-derived is committed (D8/D22) - the files stay in the install
folder, and what lands here is the reader plus the findings.

**The slice:** a save reader that answers three questions, in this order of value.

1. **The tick counter and the clock.** Without it, two saves cannot be placed on a timeline and the series is just a pile
   of files. This is the first field to find.
2. **Per-player stockpile and population.** With (1), any two consecutive `britN` saves give the gather rate over a known
   interval - register row 6 - and the age-up times give row 4.
3. **The entity list**, or as much of it as the format allows: unit and building counts per player per save, which answers
   the build-order question and, by *absence*, the decay row (row 5: a building that is there in one save and gone in the
   next with no attack).

Test-first in the repo's sense means: a fixture saved from the real game (kept out of git, hash committed), and a test that
parses it and asserts the fields the three questions need - not a parser written against a guess.

**What it unblocks:** register rows 3, 4, 5 and 6, and the acceptance match's second data point (our headless report against
a real 20-minute British opening).


## The save container reads; the world does not yet (2026-11-01, landed v1.47)

**Done:** `tools/savegame.py` + `tools/test_savegame.py` (18 tests; 29 in `tools/`). It inflates a save, reads the
header and walks the named string records, and it refuses the inputs it cannot understand rather than half-reading them
(a `.dat`, a truncated stream, a version the game itself would reject, a body past 64 MiB). `docs/save-format.md` is the
register: verified facts with the address of the code that settled each, the errata for the `HA` magic, the corpus table
(31 files: 23 at save version 12.5, 8 at 12.36, 0 refused, 0.69 s), and the map of the four sections still to walk.

**What that changed in what we know.** The header is `VER 9.x\0` + **float32 save version**, not a magic followed by
`HA`; the 12.5 ceiling is the game's own (`FUN_005e6730`); a string record is `u32 (0x0A60<<16|len)` + bytes; the eight
saves that yielded no name records are exactly the eight at 12.36, which closes the "campaign vs random-map" guess in
`oracle.md`.

**The clock is found, and the series is a timeline (v1.48).** `tools/savegame.py --series` locates the field that
advances in whole seconds (exactly one candidate of 5,466 for `brit1`..`brit13`) and prints the interval between saves:
10, 181, 295, 380, 526, 648, 964, 1,156, 1,171, 1,429, 1,941, 2,226, 2,681 s. Confirmed three ways - the recording
`142146` sums to 2,681,138 ms against brit13's 2,681,000, the AI's first chat is at 852.4 s and no save before 964 s
holds a chat string, and the `Build-town-size` lines from 2,277 s are in brit13 and in nothing earlier. `docs/save-format.md`
carries the whole derivation, including the correction that I had dismissed this exact field one commit earlier.

**So the interval is paid. What is left is the content, in this order:**

1. **Walk section 2 (the scenario, `FUN_00545460` / `FUN_00567630`)** and then section 3 (the map, `FUN_00836830`), which
   hands you `&player_info[i].{Food,Wood,Gold,Stone}` on the way - register row 6 - and ends at section 4's start.
2. **Read `pturn`** (`world+0x100`, read 4 bytes at a time in `FUN_00730600` around line 448) and then the object loop,
   which is what turns the series into build orders and the decay row.
3. **Done since: the recording walker is in `tools/`** (`recordings.py`, v1.49) - 111,494 ops on the long file to exact
   EOF, 128.8 ms - so command streams are readable without any save parsing. **Next there: the action bodies** (type
   byte, unit ids, coordinates, research and queue ids), which is what turns the walker into "what did the AI build,
   train and research, and at what second".

## The scripted AI: decisions made, first piece landed (2026-11-01, D91/D92/D93)

The owner agreed to all three proposals in `docs/ai-scripts.md`. In order:

1. **D91 - the counter report: done** (`tools/counters.py`, v1.51). Damage, dps and damage per resource-second between
   any roles the pack names, using the engine's formula, with the "class the target does not list contributes nothing"
   trap pinned by a test. This is an *authoring* tool: with D93 the engine does not need the table at runtime yet.
2. **D92 - the memory: done** (v1.52). `World::sighting(player, unit)` returns what that player last watched and when;
   filled once a second from the per-player sight masks, hashed, and **off unless the match installs it**
   (`observe_for`), so a world without an AI pays one `any()` a tick and the acceptance hash did not move.
   Measured: +20 ns/entity/tick at 2,126 entities with it on, and the first version was 16x worse (380 ns) until
   observation moved to once a second. Review: `docs/reviews/2026-11-01-memory.md`.
   **Still owed from D92: the gate.** It belongs with the conditions, because there is nothing yet to walk.
3. **D93 - the conditions, next** (`enemy_share`, `enemy_at_least`, `army_at_least`, `health_share_below`) over that
   memory, wired so a scenario's rules call `observe_for` for the players they serve, plus **the gate** (a test that
   walks the condition variants and fails any that reads ground truth) and the scenario test whose negative half is the
   anti-cheat check: the rules switch **after** the enemy composition is seen and **not before**.
4. **Scouting** follows from (2): a `scout` action and a chooser for unexplored ground, or the memory only fills when
   the armies already meet.

**Do not chase `old_time`.** It is `timeGetTime()` - a wall clock for pause accounting - and it is overwritten on load.
That trap is written down in `docs/save-format.md` because it would have cost a session: the value looks like a clock and
is not one.

**A negative result to keep, stated precisely:** scanning for a `u32`/`float32` that increases across `brit1`..`brit13`
at one fixed offset produced **0 hits at byte phase 0 and 20 at phases 1-3**, and every hit is a coincidence of
alignment - the layouts shift between saves, so the same offset holds a different structure in `brit1` and `brit13`.
The three inspected candidates (`0x224c52`, `0x270f52`, `0x22662e`) all fail that neighbourhood check. A value scan
cannot find this clock; the section walk or the differential pair can.

## Queued, not this step: U19 and U20 from the 2026-11-02 prior-art run

Two dat facts arrived that the sim does not use yet, both recorded in `docs/mechanics/README.md` and `docs/prior-art.md`.
Neither is part of the save-reader work in hand; they are the next engine slices after it.

- **U19 — the Spanish construction rate.** Effect 446 "Spanish Technology Tree" sets **player resource 195 to 1.3**
  (`[1, 195, 0, -1, 1.3]`). Resource 195 is the builder work-rate multiplier, `195 x the builder's work_rate`, 0 meaning
  default. Nothing applies it: a civ's `tech_tree_effect` is read only for its kind-2 bars, and `build` progress counts
  builder-ticks without reading 195. Test first: a Spanish barracks finishes in 1.3x fewer working ticks than a British
  one on the same ground.
- **U20 — a farm's food base is 175, not 250.** Resource 36 is 175 and the farm techs *add* to it (Horse Collar +75,
  Heavy Plow +125, Crop Rotation +175, Chinese team bonus +45). `economy.toml`'s `food_per_farm = 250` plus
  `Sim::farm_food_total`'s tech bonus gives 325 after Horse Collar where the game gives 250. Test first: 175 unupgraded,
  250 after Horse Collar, 220 for the Chinese.


## Queued, not this step: U22 from the 2026-11-04 prior-art run

`librematch/genie`'s port of the resource search (`crates/engine/genie-core/src/gathering/gather_search.rs`, lines
386-500) states the rule we do not have: a walk from the unit's tile over explored and standable tiles only, out to the
unit's **line of sight + 5**; at most **30 objects reach-tested** (the exe's `0x1e`) before the best fallback wins; and
from a drop site a first pass seeded from the site's footprint at the site's **LOS + 10**, then the unit's own at LOS + 5.
Ours are unbounded distance sorts (`Sim::nearest_drop_site`, the scripts' `nearest_resource`, which runs a `can_reach` on
every candidate). Three tests, in this order: a search does not answer beyond the unit's LOS + 5; a drop-site search
starts from the site's footprint at the site's LOS + 10; after 30 reach tests the fallback wins. Recorded as **U22** in
`docs/mechanics/README.md`; evidence in `docs/prior-art.md`, 2026-11-04.


## The AI queue, as it stands (2026-11-01)

The owner's decisions are D94 (AI lives in `config/ai/`, apart from the mechanics config), D95 (the AI names **lines**,
and a per-civ delta says what answers a line) and D96 (spatial awareness is in scope, as a query over the visible
mask). What each part costs, measured rather than guessed:

- **Done:** `config/lines.toml` (five lines from the pack's 227 upgrade edges), twelve roles for the line members, and
  `tools/lines.py` + `tools/test_lines.py` (12 tests) checking the table against the effect table. **Not wired**: no
  engine code reads a line, so it is deliberately not in `PACK_FILES` yet.
- **Next, in this order:**
  1. **`config/ai/`** - move `[policy]` out of `economy.toml` and every `[[rule]]` out of the three scenario files into a
     personality file; the scenario keeps its timed steps and names a personality per seat; the loader resolves it, the
     pack identity covers it.
  2. **Line resolution** - `train { line = "knight" }` picks the last member the civ can actually train. Needs the tech
     tree's per-tier availability, which exists (`check_train` refuses an unresearched upgrade target); what is missing
     is expressing "the highest tier I have" as one request.
  3. **Groups** - declared in the AI file, exclusive membership, persistent orders with a completion condition and a
     state (`idle`/`moving`/`engaged`/`retreating`), hashed. The risk is the hash, not the movement.
  4. **Spatial** - `local_contacts(player, point, radius)` over `visible_mask`, cached per group per second.
  5. **Raiding** - the rule that ties 3 and 4 together: retreat when the time-to-wipe ratio is bad, resume when the
     group is home and idle.
- **Cheap and already unblocked:** the AI can switch units on resources today (`when.stock` exists) - only goals are
  missing, and without them the two rules flap on their cooldowns.

### The AI folder, first slice (2026-11-01)

`config/ai/base.toml` now holds the 14 rules that were inside `config/acceptance.toml`, and the scenario says `ai =
"base"` instead. The loader resolves the name beside the scenario, refuses a missing personality by name, refuses a
scenario that has both rules and a personality, and refuses `ai` on a scenario held only as text. `ai/base.toml` is in
`PACK_FILES`, so a peer with a different AI is refused rather than playing a different game.

**Next in this line of work, in order:**

1. **`[policy]` into the personality.** It is the other half of D94 (villager distribution is behaviour) and the last
   piece of AI still living in `economy.toml`. `Economy` loses the field; five call sites of `apply_content_policy`
   change; the share table's validation (rows summing to 100) moves with it.
2. **A second personality, and a sorted directory hash.** `PACK_FILES` is a fixed list today; a folder needs a
   deterministic walk or two peers can hash the same set in a different order.
3. **`config/ai/civ/<name>.toml`** - the per-civ delta (D95): what this civ fields where the line's last tier differs.
4. **Line resolution** - `train { line = "knight" }` takes the last member the civ can train.
5. **Groups, then spatial, then raiding** (D96), as recorded above.

### Readable names (2026-11-02, D97)

`config/names.toml` holds 1,390 identifiers - 1,132 populated units plus corpses - derived from the install's string
table through the dat's `name_str_id`, because the dat's codes are shifted (283 says `PALDN` and is the cavalier).
`tools/names.py` rebuilds it; `tools/test_names.py` (13 tests) checks uniqueness, snake_case and the four shifted cases.

**Next in this line:**

1. **Resolve names in the engine** - `[role]` currently repeats `villager = 83`; with names it can say `villager` and let
   `names.toml` be the vocabulary, so refusals read "no villager" rather than "no unit 83".
2. **Names for the things the effect table references** - the ~30 armour/attack classes (3 pierce, 4 melee, 8 cavalry,
   15 archers...), so an effect command can be written `[add, knight, cavalry, +15]` instead of `[4, 38, 8, 15]`. That is
   the prerequisite for flipping the tables to name keys.
3. **Then the flip** - tables keyed by name with `id` kept as a field, once every reference has a word. This is the step
   that makes the configuration ours, and the owner's plan is to commit it at that point (no reimport).

### Translation, documented for later (2026-11-02)

`docs/i18n.md` records the owner's question and the answer: gettext is possible, but the only text drawn today is digits
(D82's 3x5 font) and every other sentence is a developer diagnostic. The order when someone wants it is message keys,
then a font with letters, then display text per locale, then gettext as an adapter - and the rule that a translated
string never reaches the simulation or the state hash.

### Next slice, planned to the last call site: `[policy]` moves into `config/ai/` (2026-11-02)

D94's other half. The villager share table is behaviour - "how many villagers go where" - and it is the last piece of
AI still living in `economy.toml`. What it touches, enumerated rather than estimated:

- **`crates/data/src/economy.rs`**: `Economy.policy: Policy`, `RawPolicy`/`RawShare`, the "rows sum to 100" validation
  (line ~846), and `Policy`'s own deserialisation. The *type* can stay in `data` (both crates need it); the *parse* moves.
- **`crates/sim/src/script.rs`**: `Personality` gains `policy: Policy` (default empty), so `config/ai/base.toml` carries
  `[policy]` and `[[policy.share]]` beside its rules. `deny_unknown_fields` means the field must land in the same commit
  the content does, or the file stops loading.
- **`crates/sim/src/lib.rs`**: `apply_content_policy()` (~1472) becomes `apply_policy(&Policy)`; it reads
  `economy.policy.share` today (~1483-1487) and would take the table as an argument instead.
- **Five call sites**: `bin/headless.rs:146`, `bin/batch.rs:177`, `view/bin/window.rs:199`,
  `sim/tests/scenario.rs:934`, `view/tests/frame_cost.rs:29`. The three runners already load a scenario, so they have a
  personality to take the policy from; the two tests need one too, which is the honest cost of the move.
- **`config/economy.toml`**: the `[policy]` block (19 share rows) leaves; **`config/ai/base.toml`** gains it.
- **Pack identity**: `ai/base.toml` is already hashed as part of the `ai/` walk, so the table is covered without a new
  entry - and `economy.toml`'s hash changes because the block left it.

**Order:** personality field first (so the content can name it), then the content, then the call sites, then delete the
old path. Test first: a personality carrying a share table reaches `World` and moves a villager, and an `economy.toml`
that still has `[policy]` is refused rather than silently ignored.

**Why it was not done in the same session as D95/D97:** it changes five call sites including two test fixtures, and the
last refactor of this size (the window's loader) shipped a bug that the suite could not see. It deserves its own pass.

### Done: `[policy]` lives in the personality (2026-11-03, v1.57)

D94a, applied. `Economy` lost `policy`; `Personality` gained it; `Scenario` carries the resolved table so a runner has
one thing to install; `apply_policy(&scenario.policy)` is called once per match at its five call sites (headless, batch,
window, `sim/tests/scenario.rs`, `view/tests/frame_cost.rs`). `Policy` deserialises itself in `crates/data`, so the
"rows sum to 100" refusal fires where the table is written and names that file. Three new rules: a personality may be
shares without rules; a scenario may carry its own shares only when it writes its own rules; a stale `[policy]` in an
economy is refused **by name** with the destination. `load_personality_file` is public so a caller without a scenario
(the frame-cost probe) parses the same way the scenario loader does.

**Evidence it changed nothing:** the acceptance match at 36,000 ticks is byte-identical to `57bf7c1` — same 521 report
lines, same rule firings, same `hash=98c1c2a051d044f6`. Load time only: pack load + identity 92.70 ms, personality parse
234.8 µs, `apply_policy` 1.47 µs, no hunk inside `step`. Six mutations, six caught; the review is
`docs/reviews/2026-11-03-policy-in-the-personality.md`.

**Named behaviour change:** a headless run with no `--script` installs no shares any more — there is no AI to take them
from. `make headless` and the window both always load `acceptance.toml`, so only a bare `--ticks` probe is touched.

**Next session starts here:** names for the ~30 attack/armour classes in `config/names.toml`'s sibling (their members
are derivable from the unit table; 15 of the 31 class ids already have effect-modifier words attached, 0 have a name in
the pack), then the table flip to name keys; evasion separately, with the binary map below.

## Evasion, the original's shape - the compact for the slice (D98a, ruled 2026-11-03)

The owner's ruling, verbatim: **"B adopt from the original game."** So the mechanism is the original's, not D98's
"wait and let pass". What the binary says, read from `FUN_008a4360` (`doIntelligentMovementUpdate`, move_obj.cpp
3,290+; the collision block is 4,600-4,760, pseudocode at `FUN_008a4360_008a4360.c` lines 1,964-2,197):

1. **The query is at the new position, not at a tile** - "ObSystem collisions at our new position", and each collided
   unit is logged with `Pos` and `Radius (x, y)`. So collision is **radius-based on continuous positions**, which our
   engine also has (`radius` is already in the pack).
2. **Classification, per collision:** terrain (`record+8 == 0`) is counted separately; a unit is resolved, and then
   - **target collision** (the collided object id equals the path's target id, `path+0x24`): counted, and the decision
     reads "That collision is with our target, so we're done";
   - **non-past collision**: counted, its id pushed onto a list (max 0x100 ids, `DAT_00de1dc8`);
   - **past collision**: *not* counted - the branch that is taken when the collided object is already in the path's own
     avoid list (`FUN_007fe010` is a membership test over a list the path object holds at `+0x20`/`+0x24`), and when a
     class comparison says it is already handled.
3. **The decision:** `if (collisions != 1 || target_count != 1)` - i.e. unless the *only* collision is the target - then
   `if non_past_count < 1` → **"Don't know what to do about my collision(s)"** (nothing to do), else
   **"We have N non-past collisions and M terrain collisions, repathing"**.
4. **The repath takes the non-target collided ids with it** (the `DAT_00de1dc8` list is walked next), i.e. it plans
   *around the units it just met*, not merely again.
5. A **straight-line shortcut** exists above all this: if the line to the goal has no non-target collisions, the unit
   walks the line and never consults the path (move_obj.cpp 4,079-4,245).

**The one predicate still unread:** the final "is it past" comparison inside the class-filtered branch
(`iVar5+0x10 < (virtual +0x2ac)()`, with class ids 3 / 0x1b / 0x27 / 0x34) - it involves two object-system fields we have
not identified. Until it is read, this engine's "past" means **already in this path's avoid set**, which is what the
membership test reads as and is operationally the same statement: a known obstacle means keep the plan, a new one means
replan. Write that reading down in the decision so the next session does not re-derive it as a guess.

### The slice that follows (order, and why each order)

1. **`find_path_avoiding(walk, from, to, extra: &BTreeSet<(u16,u16)>)`** in `crates/sim/src/path.rs` - today's
   `find_path` becomes a wrapper with an empty set. No behaviour change on its own; the repath needs it.
2. **A side table, not a wider `Entity`**: `World.avoiding: BTreeMap<EntityId, BTreeSet<EntityId>>`, the same shape as
   `queued` / `rallies` / `shares`. It holds the units this route is already planned around. **Hashed when non-empty**,
   joined to `bookkeeping_entries`/`bookkeeping_touches`, and cleared by `stand_down`, a new order, and death - a route
   that ends must forget what it was avoiding or the next order starts pre-blind.
3. **The collision test in the movement apply-loop** (`crates/sim/src/lib.rs` ~7,343, where `avoid` is called): the
   `occupied` grid is already built there for exactly this. Compute the tile the step lands in; find a *unit* occupant
   (the grid does not say which - add an `occupants: Vec<EntityId>`-style index or collect `(tile, id)` while building
   the grid). Then, matching the original:
   - occupant is the order's **target** → zero the step and let the existing arrival logic finish the order;
   - occupant not in `avoiding[id]` → **collect a repath** (id, goal, blocked tiles) for after the loop; zero the step
     this tick;
   - occupant already in `avoiding[id]` → zero the step, **no repath** (the plan already routes around it).
4. **Apply the repaths after the movement loop**, the same pattern as `shots`: `plan_path` with the collided tiles as
   extra obstacles, insert the collided ids into `avoiding[id]`, and on failure keep the old route and wait (a refusal is
   already recorded through `last_refusal`, so a deadlock is diagnosable).
5. **Tests (D98's own, now satisfied by B, plus the original's branches):** two villagers walking through each other
   never share a tile and both arrive; a unit whose route is blocked by a standing friend **goes round it and arrives**;
   a unit with no way round **waits, does not enter the tile, and records a refusal**; and a second collision with a unit
   already being avoided causes **no second repath** (count them through a probe or a counter on `World`).
6. **Then the radius form** (slice 2): collision by overlapping circles rather than shared tile, which is what the
   original actually does and what makes a crowd behave. The tile form is the first slice because our pathing is
   tile-based and the test is exact; the radius form changes where units come to rest, so it gets its own review.

**Measured before writing any of it:** a blunt guard ("never step onto a tile another unit stands on", two lines in
`avoid`, nothing else) run over the suite fails **exactly two tests**, both in `build.rs` - *"the villager banked 0
timber, which does not pay for a house"* - because a refusal without a repath strands a villager whose route crosses
another. That is the measurement that says the repath is the load-bearing half, not the guard.

### The movement module is mapped, and the original repaths on collision (2026-11-03)

The cheap step is done and it was cheaper than priced: the original's asserts carry its own source file, line and a log
string, and the pseudocode corpus at `/opt/data/tools/rea/pseudo-aokhd` is one file per function, so `grep` for
`motion.cpp` / `move_obj.cpp` **is** the map. Measured: **240 distinct `(file, line)` assertion sites across 18 functions** (81 in motion.cpp's
three, 159 in move_obj.cpp's fifteen; 265 raw string matches), of the 1,029 `.cpp` string matches in the whole corpus. `docs/movement-in-the-binary.md`
carries the address-to-role table, the reproducible commands, and what the strings settle.

**The finding that matters:** `doIntelligentMovementUpdate` (move_obj.cpp 3,290+) has an explicit collision decision -
"non-past collisions and terrain collisions, **repathing**" (4,747), a collision with the *target* means arrival (4,733),
a "past" collision is ignored (4,709), and a straight line with no non-target collisions skips the path entirely
(4,079-4,245). So **the original does not wait; it repaths.** D98 ruled "wait and let pass" as the cheap deterministic
answer and it is built and tested; this is recorded against it rather than acted on. **Default if the owner says
nothing: keep D98**, and treat a collision-triggered repath as its own slice - the conditions alone are a dozen
comparisons, and unread.

Still unread, and named: the A* cost model (no heuristic/open-list string exists in the corpus, so it needs `008007f0`
in pathsys.cpp read as pseudocode), the tie-break, and the ObSystem occupancy query `008a4360` calls.

### Reading pathfinding and evasion out of the binary (lead, 2026-11-02)

The owner asked whether the original's pathfinding and evasion can be read from the game binaries. **Yes**, and there is
already a map for it rather than a cold start: `docs/re-toolchain.md` records that the original asserts through
`FUN_005404f0(path, line)`, so every assertion names its source file - **939 path references across 84 source files** -
and the movement module is the densest one: **`move_obj.cpp` (159 sites)** and **`motion.cpp` (81)**. `Path` is
`Age2HD/AoK HD.exe`, the REA workspace is at `/opt/data/tools/rea` (`aokhd.snapshot`, `export-pilot/`,
`find_in_corpus.sh`).

**The first step is cheap and worth doing before any evasion code is written:** list the assertion sites whose string is
`move_obj.cpp` or `motion.cpp`, resolve each to its containing function, and that names the movement module's boundary -
which functions are the pather, which are the unit-motion update, and where collision is resolved. Only then is
decompiling worth it, because the interesting questions are narrow:

1. **the A\* cost model** - confirm 10/14 (ours) against the original's integers, and its tie-breaking;
2. **the evasion rule** - whether a blocked unit waits, slides, or repaths, and what decides it (this is exactly the
   "soft evade" question, and getting the original's answer is cheaper than guessing);
3. **determinism** - how the original orders equal-cost nodes and equal-distance units, since our replay depends on it.

Cost: a session for the map, one or two for the reading. It is *not* on the critical path for `[policy]`, names, or
groups, so it belongs alongside the evasion slice rather than before it.


## Session 46 - walls are placed; the view has no placement path (2026-11-08)

**State**: `main` at `2f37d56`. Suite **749/0/19**, clippy `-D warnings` and fmt clean, acceptance hash
**`45596da62c9e3668`** (135 entities, 125 repaths, 116 made-room). D153 (gates researched), D154 (gates: radius 1, an enemy
inside holds one open), D155 (walls by drag) are landed.

**Landed this session**

- **D153** `docs/gates-research.md`: a gate is a **global** open/closed state, not per-player passability (D54 corrected);
  0 A.D.'s `Gate.js` is the closest model; the trigger radius is the one number the data does not give.
- **D154** `World::step_gates` before the movement pass; opens for an owner or team-mate **within one tile**, closes only when
  **no unit of any side** is on its tiles; `Walk` writes only on a transition, `open_gates` keeps it balanced. Gate is content
  (`gate = 64`, `[build] gate_roles`). Four tests in `crates/sim/tests/gates.rs`.
- **D155** `World::wall_line(from, to)` in `world/lines.rs` (one bend: 45 degrees first, then straight) and
  `Command::BuildLine { villager, unit, tiles }` (blocked tiles skipped, the line continues past them). `Command` is no longer
  `Copy`. **A pre-existing build-queue defect was fixed**: with three or more queued sites the advance path re-queued what it
  popped, so nothing ever started - split into `start_build_site` (now) and `start_build` (queue if busy).

**Next, in progress: the UI drag (the owner asked for it explicitly after D155)**

What exists: `crates/view` renders and picks (`pick_at`, `camera::tile_at`, `Snapshot`, `ui::Layout`, a command card that only
offers **train/research on a selected building**). What does **not** exist: any way to place a building, and a villager has no
card at all. So the slice is:

1. `crates/view/src/placement.rs` - a state machine with no window in it: `begin(tile)`, `hover(tile)`, `tiles()`,
   `validity(&World)`, `release() -> Option<Command::BuildLine>`; tests in `crates/view/tests/placement.rs` (test-first).
   `wall_line` is reused, so the preview and the command cannot disagree.
2. Window wiring in `crates/view/src/bin/window.rs`: a key enters placement for a wall (provisional until the villager build
   card exists - say so in the UI notice), **first left click starts**, **second left click commits**, right click or Escape
   cancels, `PointerMoved` updates the hover.
3. The preview: semi-transparent ghost diamonds, **red where the tile cannot be built on** (the owner's rule from D155's
   questions), drawn above the ground and below the HUD.

**Then**: the villager build card (so placement is reachable the way a player expects), the gate lock, "a gate must be built
inside a wall", the other three wall types (`117/119/155`).
