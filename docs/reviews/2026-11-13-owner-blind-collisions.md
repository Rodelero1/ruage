# Owner-blind collisions, built and measured — parked on the match outcome

*2026-11-13. The owner's ruling: **"Adopt original game behaviour. Don't make difference of friend or foe as in original
game."** The rule is built on `d189-owner-blind` and it does what he asked at the level of two units. It cannot land yet,
because it changes whether a match resolves, and that is the one number that decides.*

## What changed

1. **`hostile` is gone from `collide`.** Every unit body blocks, whoever owns it: the reach, `bound_to_reach`, the goal-tile
   rule and the at-rest bound all apply to a friend exactly as to an enemy. The original's collision block has no player,
   team or alliance test anywhere in it and its query passes an owner sentinel (`docs/movement-in-the-binary.md`).
2. **A body on the goal tile asks for a route round it**, not for a wait — gated on the mover not already standing beside
   the tile, because without that gate it re-plans every tick (**525 repaths in 600 ticks**, measured).
3. **A body at rest inside the reach no longer leaves a zero step**: the pair resolves it through the same give-way path
   rule 9 uses, which is what stops the D192 freeze coming back from the other side.
4. **Near the goal, a *body* still earns a repath, a wall does not**: D98a suppressed re-plans within a tile of the goal,
   which is right for terrain and wrong for a man — the third villager sent at one bush stands 0.4 from a settled neighbour
   with his own place two steps round it.

## Performance

| | `main` (`603e72e`) | `d189-owner-blind` |
|---|---|---|
| acceptance (48,000 ticks, seed 7) | `entities=142 players=1 hash=91572e72c33cee93 repaths=409 made_room=0` | **`entities=147 players=2 hash=d3b087c60013493f repaths=45 made_room=24`** |
| `collision` suite | 3 passed | **4 passed** (the new one is the owner's rule) |
| `evasion` suite | 6 passed | 6 passed |
| `gather_motion` | 3 passed | **1 failed** |

The two numbers that matter: **`made_room` 0 → 24** (men now have to be shoved aside, because a friend is a wall) and
**`players=2` where `main` finishes `players=1`** — the match no longer resolves. That is the same failure the D189 note
recorded the first time the bound was applied to friends (*"164 entities against 141, both players alive at 1,800 s"*), and
it is why the exemption existed.

## Stability — what is verified and what is not

**Verified, in tests:** a walker never ends a tick inside a friend (**the new test**: RED first at
*"the walker came 6149 raw units from its friend at tick 79, inside the 8000 their footprints need"*, green after the
change); two units of one order still never stack; a body already overlapped is still not pushed (D52); the goal-tile
routes no longer spin (525 → 0 repaths in that fixture); a blocked unit still goes round a standing body and arrives.

**Red, and it is the blocker:** `three_gatherers_on_one_bush_do_not_pile_up` — one of three villagers reaches the bush in
1,200 ticks (it was three). **The cause is a number, not a rule: our gather places are spaced one body apart (0.40), which
is exactly the reach that now separates bodies**, so a man walking to his place is stopped by the man already standing on
the next place. The game spaces its working men at **0.661-0.663** (measured on the owner's saves,
`docs/reviews/2026-11-13-the-standoff-is-measured.md`) — a body *and a gap*, which is what makes a crowd deliverable
without overlap. Places must be spaced by more than the reach; the fix is `touching_places`' spacing for gather places,
and it changes capacity, so it is a slice of its own.

**Not measured yet:** the tick cost of the extra passes (`World::step` was not re-timed on the branch), and the 24
`made_room` shoves — whether they are the same 24 units repeatedly or 24 units once.

## Why it is parked rather than merged

The engine is green (`cargo test -p ruage-sim` reaches the parked test and the two rule-pinning ones below, everything
else passes), but **the acceptance match stops resolving**, and a change that turns a won match into a stalemate is not a
collision rule, it is a regression until the crowd machinery underneath it works. The remaining work is bounded and named:

1. Space gather places at `reach + spacing` (~0.66), not one body apart, and re-derive the capacity from the geometry
   (the game shows 7-8 men at 0.66 on the touching envelope).
2. Re-run the acceptance match; `players=1` and `made_room` back near 0 are the acceptance criteria.
3. Update the two tests that pin the withdrawn rule: `step_decision::a_friend_on_the_goal_tile_is_a_take` (a friend on the
   goal tile is now an obstacle and the unit stops beside it) and `movement::a_walker_whose_goal_tile_holds_a_friend_still_arrives`
   (it arrives *beside*, not on, the friend). `evasion::two_villagers_crossing_pass_through_and_both_arrive` needs
   renaming for the same reason - under D200 they go round each other.

**D200 itself is recorded on the branch, not on `main`**: the owner has ruled, and the rule is right, but the decision is
not landed until the engine can deliver a crowd without overlap.

## Second pass: the places were the problem, and the game's own number now falls out

**Gather places are spaced `0.66`, not one body.** `GATHER_SPACING_MILLI = 660`, with the owner's three saves behind it
(0.661-0.663 between working men, D199). One body (0.40) is now *exactly* the reach that separates bodies, so a place one
body from its neighbour is a place nobody can walk into.

**`touching_places` was fitting places greedily, and losing one at every seam.** It walked the grown square's outline and kept
any candidate at least `spacing` from the ones already kept, which under-fills: **11 places where the perimeter arithmetic
allows 13**, and **5 at 0.66 where it allows 7** - and with friends blocking, a missing place is a man who cannot work.
It now emits a place every `perimeter / count` of **arc length** and steps the count down until the shortest *chord* between
neighbours is at least `2 * radius` (the physical rule; `spacing` is the comfort distance). Measured on one tile, radius 0.2:

| spacing | before (greedy) | now (even + chord-limited) |
|---|---|---|
| 0.40 | 11 | **10** |
| 0.60 | 7 | **8** |
| **0.66** | **5** | **7** - the same seven the owner's saves show |

That is the first time the engine's own geometry produces the game's number, and `ten_men_touch_one_bush` now reports
**7 of 10 working the bush** where it reported 5.

**Green on the branch:** `tile_geometry` 4/4, `collision` 4/4 (including the owner's never-inside-a-friend test),
`evasion` 6/6.

**Still red, and it is a new defect rather than the old one.** Three carriers are **frozen at one drop site** - at
`(4.76, 10.02)`, `(4.56, 10.36)` and `(4.97, 10.36)`, positions identical at tick 1,200 and tick 2,400, two in `Return` with
a load and one in `Gather` with none. They are within 0.2 of each other and none steps. Ruled out by experiment, not argument:
the goal-tile rule (a `Return`'s target is a body, and skipping that rule for bodies changed nothing - reverted), and the
overlap escape (rule 3 already returns the step). The next instrument is `step_trace`: which rule refuses, per man, per tick.

**And the two tests that pin the withdrawn rule still need rewriting** - a friend on the goal tile is now an obstacle
(`step_decision`), and a walker arrives *beside* a friend rather than on top of it (`movement`).

## Third pass: what is green, and the eight tests that are not

**Whole `ruage-sim` suite, `--no-fail-fast`, on the branch: 79 test binaries green, 8 tests red.** Everything red is
crowd delivery or a test that pins a capacity. The rest of the engine - combat, research, techs, decay, vision, site
placement, body clearance, the one-tile corridor, the collision and evasion suites - is green.

**Two defects found and fixed in this pass, both real:**

1. **A corner offset was truncated, not ceiled**, so a place could land **one raw unit inside** the tile it was meant to
   touch - and `body_clearance` caught it on the acceptance map. `ceil_million` now rounds outward; one raw unit is
   1/20000 of a tile.
2. **`body_clearance`'s failure message had its arguments swapped** - it read *"unit 0.00005 ... 9 tiles inside"* when the
   truth was *"unit 9 ... 0.00005 tiles inside"*. A test that lies about the measurement is worse than no message; fixed.

**Two experiments run and reverted, with their numbers, so the next session does not repeat them:**

| experiment | result |
|---|---|
| give-way bounded by the reach (as it was) | two villagers **froze on one tile at a town centre's door** for 1,200 ticks: `WaitOwnTile` / `GiveWayOwnTile` every tick, positions identical at tick 1,200 and 2,400 |
| give-way **never** bounded | `three_gatherers` went green, but a head-on pair in a one-tile corridor came **7,372** apart against the **8,000** their footprints need (D131 A broken), and `body_clearance` went red |
| give-way unbounded **only when already inside the reach** | corridor green again, `body_clearance` red from the geometry bug above, and the door jam **unchanged** |

So the door jam is not the bound. The trace says both men are on **one tile** and `pass.moving` holds the other, and the
give-way step - bounded or not - does not move them. The next instrument is the apply-stamp path in `move_bodies`, exactly
where D192's trace ended ("the refusal is downstream of `collide`").

**The eight red tests, by cause:** `build` (a house not finished - the same door jam at a build site), `three_gatherers`
and `a_woodcutter_whose_tree_runs_out...` (the jam), `six_settled_men_hold_their_places` and `ten_men_work_one_bush_a_body_apart`
(capacity/placement expectations written for 0.40 spacing), `the_tile_holds_as_many_woodcutters` and
`ten_men_touch_one_bush` (they compute places with `2 * radius` and expect 10-11 where the engine now has 7 - the game's own
number), and `movement::a_walker_whose_goal_tile_holds_a_friend_still_arrives` (pins the withdrawn rule).

## Fourth pass: there was no deadlock, and two of the reds were tests written for the old geometry

**The "two men on one tile" deadlock was not a deadlock.** The committed test sampled **tick 1,200** and demanded three men
standing at the bush at that instant. Measured over the whole run on this branch, per tick:

```
men standing at the bush: {0: 337, 1: 213, 2: 49, 3: 601}   all_three = 601 of 1,200
closest pair over the whole run: 0.400 tiles  (exactly the reach, never inside it)
```

Three gatherers cycle - chop, carry, walk to the town centre, walk back - so a single sample catches one of them on the road.
**The `WaitOwnTile`/`GiveWayOwnTile` steps were a commuting man squeezing past a working one**, and the three give-way
variants tried earlier were answering a problem that did not exist. `three_gatherers_on_one_bush_do_not_pile_up` now measures
the run: all three on the bush on at least 500 of 1,200 ticks, and no pair closer than 0.39 - it passes at **601** and **0.400**.

**My own error, and it matters because it produced a wrong number twice.** I ran two probes while the working tree was on
`main` rather than on this branch, and read them as D200 results: they reported a **0.038-tile** overlap and a different
histogram. Both were `main`'s behaviour (friends passing through). Re-run on the branch they are 0.400 and 601/1,200. The
lesson is mechanical: `git branch --show-current` before quoting a measurement, because the tree is the instrument.

**Two tests were computing the capacity themselves and had drifted.** `gather_ring::the_tile_holds_as_many_woodcutters_as_touch_it`
and `gather_touching::ten_men_touch_one_bush...` called `touching_places` with `2 * radius` (a 0.40 spacing) and expected
10-11; the engine now spaces places at the measured **0.66** and holds **7**. The first now asks the engine's own
`touching_place_count`, so it cannot drift again; the second expects 7.

**Suite: 81 test binaries green, 6 tests red** (from 8). What is left: `build::a_content_pack...` (a house not finished - the
one that may be a real defect), `ten_men_work_one_bush_a_body_apart` and `six_settled_men_hold_their_places` (written for
0.40 spacing), `a_woodcutter_whose_tree_runs_out...`, `ten_men_touch_one_bush...` (a working man **0.174** from the tile -
inside it, so a place or a settle is wrong somewhere), and `movement::a_walker_whose_goal_tile_holds_a_friend_still_arrives`
(which pins the withdrawn rule).

## Fifth pass: the 0.174 was a walker, and the reservation question is the real one

**A working man 0.174 tiles from the bush tile is not a violation.** A node's tile is not blocked ground (`is_free_ground`
treats a resource's own tile as standable), so a man on his way to his place may cross close to it. The assertion was
checking *every* man tasked to the node, walkers included; it now checks the **reserved place** each man holds, which is
where the settle will put him and therefore what the rule is about.

**And that change found something worth keeping:** one of the ten men tasked to the bush **holds no reserved place at all** -

```
EntityId(8504) works the bush with no reserved place
```

Either a place is taken only at settle time (in which case the test must allow a man to be *between* places) or the
reservation is being dropped somewhere between the order and the settle. That is the next question, and it is a one-test
investigation with a known instrument (`gather_places()`), not a hunt.

**Suite on the branch after this pass:** `gather_motion` 3/3, `gather_ring` 4/4, `gather_touching` 1/2 - the last one red for
the reservation question above. Two tests remain written for 0.40 spacing (`ten_men_work_one_bush_a_body_apart`,
`six_settled_men_hold_their_places`), one pins the withdrawn rule, and the content-pack house is still unfinished.

## Sixth pass: the reservation book is inconsistent, and my fix for it regressed the suite

**First, my error.** I built a `claim_places` pass (a man standing on the ring who holds no place takes the ground under his
feet) plus a live-only filter in `gather_places`. It looked right on one test and **regressed the suite from 5 red to 11**
(killed at 93 green / 11 failed: `collision`, `evasion` and several gather rows). Both engine edits are **reverted**; only an
unrelated warning cleanup in `tiles.rs` is kept. A pass that changes the standoff bookkeeping globally needs to be reasoned
from the book, not iterated against one test.

**What the same probe says depending on how the book is read** - this is the finding, and it is why the last three passes
disagreed with each other:

| read | places counted | men tasked to the bush |
|---|---|---|
| raw map (`gather_places` before) | **7** (some stale) | 7 |
| live only (`place_is_live`) | **4** | 7 |
| with the claim pass, raw | 10, then 8 | 7 |

So the map holds entries for men who are **no longer on that order** (it is never swept - `place_is_live` is asked at read
time by `standoff_point`), and it lacks entries for men who **walked on without a settle** (the settle only reserves places it
chose itself). Both are true at once, which is why "seven men, four places" and "seven men, ten places" were the same seven
men.

**Not re-tried:** claiming a place from a man's position with a band of `reach + half a tile` (a man who had stopped 0.686 out
claimed ground he was not working) and excluding neighbours by `distance_to_tile(held, his tile)` (that finds *every*
neighbour on the same tile "taken", so the pass never fired).

**Next instrument, one script:** for 1,800 ticks on the ten-men fixture, print every `gather_place` entry as
`(worker, node, live?, worker's task, clearance of the place)` and every working man with no entry. That reads the whole book
at once instead of guessing which filter is right.

## Seventh pass: the reds are one defect - men cannot share a drop point

Every remaining red, read from its own message, is the same story: **a crowd cannot deliver.**

```
research: could not bank 400 in column 0; the bank holds 200
research: could not bank 700 in column 2; the bank holds 400
research: could not bank 900 in column 1; the bank holds 790
skirmish: player 1 never fielded infantry: the skirmish is one-sided
skirmish: the skirmish hit a script error: ["player 1 has no unit militia"]
build:    a_content_pack_with_our_own_resources_loads_and_spends - the house is never finished
gather:   a_woodcutter_whose_tree_runs_out_joins_a_patch_that_already_has_men_on_it
```

The banks stall at a fraction of what the tests need, and the skirmish cannot field a unit at all. With D200 friends are walls, so
men queue at the drop: **the economy starves not because gathering is wrong but because delivery throughput is**, and a test
that needs 700 banked holds 400 because the queue is at the town centre, not in the fields.

**Not a parallel-test artefact.** Re-run serially (`-- --test-threads=1`) on `research`, `skirmish`, `step_decision`:
`17 passed; 3 failed`, same messages, 181 s. The suite is not racing on a shared file; the numbers are the numbers.

**So step 1 is not geometry and not the reservation book - it is the drop.** The places on a node are fine (7, 0.66, 0.400
apart, three men working 601 of 1,200 ticks). What the branch lacks is a rule for many men delivering to one building: the
game has them stack at the drop the same way they stack at a node, and we have no standoff ring for a building's drop point.

**My error, again, recorded because it cost a pass:** I read the first six reds as six separate test problems and rewrote
tests for three of them before reading all six messages together. Five of them say the same sentence.

## Eighth pass: the control test - one test, two branches, 8x the time

The same test, run on both trees, same machine, back to back:

```
main              a_trebuchet_cannot_be_trained_before_the_imperial_age   ok      5.86 s
d189-owner-blind  a_trebuchet_cannot_be_trained_before_the_imperial_age   FAILED  46.03 s
                  could not bank 400 in column 0; the bank holds 200
```

**This is the measurement the whole branch needed.** D200 is not merely "some tests now disagree": the simulation is **8x slower**
and the economy **starves** - the same game, same map, same orders, 200 banked instead of 400, in a test that on `main` finishes
in six seconds.

Eight times is not a small constant; it is a loop. The candidates, in the order worth checking:

1. **repathing storms** - `MAX_REPATHS_PER_TICK` bounds each tick, but if every blocked man repaths every tick against every
   neighbour the pass becomes quadratic in a crowd (the branch's acceptance run showed `repaths=45 made_room=24` on `main`'s
   `409/0` - different map, so not directly comparable, but the direction is worth measuring on the same map);
2. **give-way churn** - a granted give-way that is refused next tick returns the pair to `WaitOwnTile` forever, which is CPU
   burned, not progress;
3. **`standoff_point` scanning places against every held place** for every worker every tick (it is called by the settle, the
   picker and the order path).

**Instrument, and it is one run:** the same test with per-tick counters printed every 600 ticks - `passes`, `repaths`,
`made_room`, `moved` - which tells a quadratic pass from a churn loop in a single execution.

**What this pass settles:** the geometry is right (7 places, 0.66, 0.400 apart), the reservation book is inconsistent but not
the cause of the reds, and the branch's real defect is **cost and throughput under a crowd**. Nothing lands on `main` until
that 46 s is back near 6 s.

## Ninth pass: the dead state, named exactly

The trebuchet test, instrumented (per-tick counters on failure, a state dump of the villager):

```
main:             banked 400 in 11,359 ticks, 6.13 s
d189-owner-blind: 400,000 ticks, bank holds 200, repaths 2 (body 1), made_room 0, 45 s

t=20000   task=Gather{node 12} at=(16.23,13.80) carry=0 progress=0 place=(19.80,15.41) node_tiles=[(20,15)]
t=200000  identical
```

**He is frozen 3.9 tiles from his place, with a valid reserved place on the ring, a live gather task, an empty carry, and
`progress = 0`.** Not blocked (2 repaths in the whole run, zero made room), not gathering, not walking. He never moves again.

So the defect is a **dead state**: a worker whose order landed, whose place is reserved, and whose route is gone - consumed,
failed, or never rebuilt - has nothing that gives him a new one. `bank()` in the test only re-tasks a man whose task is `Idle`,
and his task is `Gather`, so the test drives him forever into the same wall.

**Why `main` does not have it:** the same fixture banks 400 in 11,359 ticks there. The difference the branch introduced is the
movement rule (friends are walls) and the place spacing; whichever of the two leaves a route finished-but-not-arrived, the
answer is the same missing rule: **a worker not touching his node, with no route to his place, must ask for one again** - the
same resumption the movement pass already does for a body that has been given the right of way.

**Next step, test-first, on this exact fixture:** a test that walks a worker to a place, removes the route mid-walk, and
requires him to arrive; then the rule in `move_bodies`/`settle_workers` that rebuilds a route for a live gather order whose
place is not reached. The measurement to hold it to is the one above: **11,359 ticks and 6 s on `main`** - the branch must
come back to that, not just pass.

## Tenth pass: he has a route, and he will not walk it

The state dump was one layer too shallow. The frozen villager **holds a live route of three points**:

```
PROBE worker EntityId(2) task node EntityId(12) book Some((EntityId(12), (396000, 308125))) route 3
```

so "no route, nothing rebuilds one" is wrong, and the `resume_workers` pass I wrote for it **never fired** - reverted, with
its reason recorded rather than left as dead code. The route exists, three waypoints long, and the man stands still for
400,000 ticks with **two repaths and zero made room in the whole run**. That is the movement pass refusing every step without
recording why - and `WaitOwnTile` is the only refusal that is silent: it waits for a body that is itself waiting.

**What is established, in order:**
1. the geometry is right (7 places at 0.66, closest pair 0.400, three men on a bush 601 of 1,200 ticks);
2. the reservation book is inconsistent (raw 7, live 4, tasked 7) but is not what stops delivery;
3. a villager with a live order and a valid place can stand **frozen with a route in hand** while `main` completes the same
   fixture in 11,359 ticks and 6 s;
4. the branch is **8x slower** on that fixture (46 s) and the economy starves at 200 of 400.

**The next instrument is the one that worked before:** `trace_steps()` on this fixture, printing the two bodies' `last_step`,
positions and route lengths for the ticks around the freeze. The owner's own save fixture showed the same pair of decisions
(`WaitOwnTile` on one, `GiveWayOwnTile` on the other) - so the shape is known; what is not known is *who* the second body is
on a map with one villager, and whether a route is being walked into a body that never yields.

## Eleventh pass: `WaitBlocked`, forever - the decision that names the bug

Trace on the trebuchet fixture (`trace_steps`, ticks 19,000-19,029), the man with a three-point route:

```
t=19000 WaitBlocked at=(16.235,13.800) carrying=0
t=19001 WaitBlocked at=(16.235,13.800) carrying=0
... identically for 400,000 ticks
```

**Not `WaitOwnTile`. `WaitBlocked`** - the step onto the route's next waypoint is refused because that ground holds a body,
and the man waits instead of re-planning. The source says what should happen next (`lib.rs:676`): *"the step was taken into a
body at rest, whose repath routes round it from the next tick (rule 8)"* - and the repath **never comes**: two repaths in the
whole 400,000-tick run, zero made room.

The mechanism that suppresses it is in the same file, three hundred lines of comment away: the avoid set lives **with** the
route, and *"a collision with a unit in this set is past - the plan already accounts for it, which is exactly what stops a
blocked unit from repathing every tick."* So when the body a route was planned around is **still standing there**, the
collision is classified as already handled, no repath is asked for, and the man waits forever. One villager, one route, one
body that never moves, and the engine calls it "past".

**The rule the branch needs:** a `WaitBlocked` that repeats is a *stale* avoid set. After a bounded number of consecutive
waits on the same waypoint (the `stalls` counter already exists for ground), clear the avoid set and plan again from where the
man stands. Test-first, on this fixture, held to `main`'s 11,359 ticks and 6 s.

**This is the third named cause in this review (jam, reservation book, delivery) and the first one that explains the 8x.**
Each earlier candidate was measured and dismissed with a number; this one is a decision name from the engine itself.

## Twelfth pass: three attempts at the standing friend, all measured, all reverted

The reproduction is now a test (`gather_motion::a_gatherer_walks_round_a_friend_standing_in_his_path`): a gatherer at
`(6,6)`, a friend standing at `(8,6)`, a bush at `(10,6)`, 1,500 ticks. Today it fails at **1.84 tiles, `WaitBlocked`** - the
same decision as the trebuchet fixture.

Three fixes were built and measured; **all three reverted**:

| attempt | what it did | measured result |
|---|---|---|
| drop the `!known` guard for a **resting** body | asks for a repath even when the body is in the avoid set | repaths asked every tick (`budget=0` on entry), route unchanged, still `WaitBlocked` |
| `leave_the_tile` | steps 0.02 sideways when a body shares the mover's tile | rejected at first (0.1 was inside the reach), then taken - mover crept to `(8.10,6.50)` and oscillated |
| alternate start tile | plan from a free neighbour when the obstacle holds the start tile, first leg = that neighbour's centre | no change: `TakeAtRest` at the reach boundary, position unchanged |

**The common cause, and why it is a design question rather than a bug fix:** the mover is *at the reach boundary of a body
that will never move*, and his route points through it. Every mechanism the engine has answers "how do I not overlap" - none
answers "how do I get past". The three answers available are mutually exclusive:

1. **squeeze past** - allow the mover into the resting body's footprint for a tick or two (the at-rest rule already does this
   when the step is non-zero; the zero-step case is the one that jams);
2. **push the standing unit aside** - the sleeping body is displaced by the mover, as an RTS usually does, and the mover
   keeps his line;
3. **route round** - the plan must avoid the body's *point*, not its tile, and the search must be able to start from a tile
   the body is not on. Attempt 3 was the first half of this and did not converge; the search kept returning the straight line
   because the mover starts inside the reach, where no detour is representable as a tile path.

**This is where an owner's ruling is needed**, because (1) and (2) look different in play, and (3) is the most expensive. The
owner's standing rule is *"adopt original game behaviour"* - and the binary is the place to settle which of the three the game
does. The collision block itself carries no owner test (measured earlier); what is not yet read is whether the *movement*
update displaces the body it cannot pass.

## Thirteenth pass: the two rules cannot both hold, and only one of them is ours

Two candidates were measured against each other on the same tree, and they are the whole question:

**A. The bounded step stands (no overlap ever).** `a_gatherer_walks_round_a_friend_standing_in_his_path`: FAILED, `WaitBlocked`
at 1.84 tiles for 1,500 ticks. The trebuchet fixture: **46 s, 400,000 ticks, 200 of 400 banked**. The suite: 5 red.
`an_enemy_body_on_the_line_changes_the_decision` passes (8,000 raw units of separation kept).

**B. An at-rest body is stepped into (squeeze past).** The same friend test: **ok**. The trebuchet fixture: **5.89 s** - `main`
is 5.86 s. The suite: **114 binaries green, 10 red**, and the 46-second stall is gone. But
`an_enemy_body_on_the_line_changes_the_decision` fails at **4,803 raw units where two 0.2 radii need 8,000** - a walker standing
*inside* a body at rest.

**So: no rule inside "never overlap" makes a man pass a body that will not move in a one-tile gap.** Three attempts at routing
round (lateral step, alternate start tile, repath past a known body) all end in the same place - the mover at the reach
boundary with a route through the body - and B fixes the freeze only by allowing the overlap the engine forbids.

**The remaining answer is the one RTS games use and the one the engine has no rule for: the body at rest yields.** A man
walking into a standing friend displaces him by up to a footprint; then no one overlaps at the end of the tick, the mover keeps
his line, and the frozen-with-a-route state cannot arise. That is a rule about a *body without an order*, and it changes what
the player sees, so it is the owner's to call - and the binary is where to check it: `sub_8a4360` (doIntelligentMovementUpdate)
is the function that would do it, and the read is a code walk, not a string search (the corpus's own pseudocode carries no
"displace" literal).

**Tree left at the committed state** (`46e5cc7`): no half-applied fix, the reproduction test red, the numbers above in this file.

## Fourteenth pass: the nudge works, and it breaks three rules we already wrote down

Built the 0 A.D.-shaped nudge (a body with nothing to do, in a walker's path, is displaced a twentieth of a tile along the
dominant axis, onto free ground, never into a third body). Measured:

```
gather_motion::a_gatherer_walks_round_a_friend_standing_in_his_path   ok      (was WaitBlocked at 1.84 tiles)
research::a_trebuchet_cannot_be_trained_before_the_imperial_age       ok      6.24 s   (main 5.86 s; without the nudge 46 s)
full suite                                                            110 green, 16 red
```

**The 8x is gone and the economy delivers.** But four of the sixteen reds are rules the engine already states, not tests written
for old geometry:

| test | what it says | what the nudge did |
|---|---|---|
| `evasion::a_blocked_unit_goes_round_a_standing_body_and_arrives` | *"the body it went round did not move"* | moved it from `(5,2)` to `(6,2)` |
| `evasion::a_body_on_the_goal_is_never_entered_and_costs_no_repath` | *"the body standing there did not move"* | moved it |
| `evasion::a_repath_route_is_pulled_like_a_planned_one` | route pulling invariant | broken by a moved body |
| `animals::a_gathered_sheep_stands_still` | *"nothing gathers a sheep into walking"* | pushed a sheep being sheared |

**So the engine's written rule today is "a body at rest is never displaced; the walker goes round it"**, and that rule is what
freezes a walker when the body shares his tile - the planner has no tile to go round. Scoping the nudge to the mover's own tile
reduced the damage but did not remove it: `a_blocked_unit_goes_round...` still lost its body, because two units standing near a
tile edge share a tile far more often than the phrase "corner case" suggests.

**Reverted, branch clean at `98b3b48`.** The measurements above are the evidence for the one question left: whether a body at
rest may be displaced at all. If yes, those three evasion tests change from "did not move" to "moved no more than a nudge and
was never walked through", and the sheep test gains "unless it is being gathered". If no, the shared-tile freeze needs an
answer that does not involve displacement, and none of the three routing attempts found one.

## Fifteenth pass: the nudge, gated on being stuck - the freeze is gone and the old rules stand

The nudge works only after a body has been refused **40 ticks in a row** (two seconds at 20 Hz), only for a body on the
walker's **own tile**, only for a body with no route and no work place, and never for a body somebody is working (a sheep
being sheared, a mine, an attack target). Everything else keeps the rule the engine already had: *the body it went round did
not move*.

```
evasion                              11 passed     (including all three "did not move" rules)
animals                               6 passed     (the sheep stands still)
gather_motion                         5 passed     (the standing friend is no longer a dead end)
research::a_trebuchet...              ok, 6.29 s   (main 5.86 s; before the nudge 46 s / 400,000 ticks / 200 of 400 banked)
whole suite                          113 binaries green, 11 tests red
```

**The instrument that made the difference was `stalled`**: a per-entity count of consecutive `WaitBlocked` ticks, kept in the
movement pass. `last_step` could not serve - it is only recorded when `trace_steps` is on, so the first version of this pass
iterated an empty list on every real run and did nothing, which is a trap worth remembering.

**The eleven reds are back to the six that were already there** (the content-pack house, the two tests written for 0.40 spacing,
the woodcutter joining a patch, the reservation book, the withdrawn friend-on-goal rule) **plus five economy/skirmish rows**
(`two_players` 790 of 900, `the_spear_line` 400 of 700, `both_sides_field_an_army`, `the_skirmish_only...`,
`a_friend_on_the_goal_tile_is_a_take`). The economy rows are the next measurement: `repaths 29, made_room 0` at 400,000 ticks
suggests a second stall, and the most likely candidate is the drop - carriers *are* posted (a `Return` task keeps its place),
so the very rule that protects a worker's ground stops two carriers from nudging each other at the town centre.

## Sixteenth pass: a loaded carrier, frozen at the drop - and one wrong field of mine

The stall behind the economy reds, measured (`two_players_can_be_different_civilisations`, probe every 40,000 ticks):

```
t= 40000 stock=710  Gather{node 102} at=(10.80,20.41) carry=20000   node left 1,780,000
t= 80000 stock=790  Return{node 104, drop 1} at=(11.85,21.14) carry=200000   node left 3,000,000
t=120000..400000    identical to the line above
```

**He is carrying a full load - 200,000 raw, ten units, the whole `carry_cap` - and stands in `Return` for 400,000 ticks without
reaching the town centre.** It is not a "returning nothing" state and not resource exhaustion (three million left in the node);
it is a **carrier that cannot complete the approach to the drop**. The bank stops at 790 of 900 and the match starves.

**My error, and it cost a build:** the probe read `entity.amount` and called it the carry. A villager's load is `carry`; `amount`
is a *node's* remaining resource, so every reading was `0` and I "found" men walking home empty-handed. The fix I wrote for it
flipped every `Return` - including loaded ones - back to `Gather`, which made the trebuchet fixture hang instead of run (a man
would fill his hands, be told to gather again, and never deliver). Reverted. The test that "proved" the bug asserted on the same
wrong field; it now reads `carry` and passes, and it stays as a guard, not as evidence.

**Next instrument:** `trace_steps` on this fixture, printing the carrier's `last_step` and his distance to the drop's footprint -
the same sentence that named the first stall (`WaitBlocked`) should name this one.

## Seventeenth pass: the carrier was aiming at a wall, and the goal itself was the wall

The trace that named it:

```
t=20000  Gather{node 103}                          at=(12.781,18.800)
t=40000  Gather{node 102}  carry=20000             at=(10.800,20.406)
t=60000  Return{node 104, drop 1} carry=200000     at=(11.850,21.139)  last=WaitGround   <- forever
```

`WaitGround`, not `WaitBlocked` - and **29 ground re-plans, 0 body re-plans**. The cause is in `goal_of`: for a `Return` it
returned `self.find(drop)?.position`, the **town centre's own anchor**, which is blocked ground by definition. Every search
that ended there failed, the route's two stall re-plans were spent on searches that could not succeed, and the man waited
for the match with ten units in his hands while the bank sat at 790 of 900.

**The fix is one goal, not a patrol of the movement pass:** a `Return` now aims at the **walkable yard**
(`approach_tile` - the tile beside the footprint that already existed for this purpose), and the follow-up fallback in the
`Return` arm does the same when the route has no waypoint left. Two smaller things came with it: the nudge now also accepts a
body standing on the walker's **next waypoint** tile (the same dead end one tile out), and the stall counters saturate - a
`u8` that overflows panics the match in a debug build, which this fixture did once the re-plans resumed.

```
research (all twenty)    20 passed, 0 failed   (two_players 53.6 s, the spear line ok)
evasion                   6 passed             including ground_it_cannot_cross_is_re_planned_at_most_twice
whole suite             114 binaries green, 9 red
```

**The nine reds:** the six that were always there (content-pack house, two tests written for 0.40 spacing, the woodcutter
joining a patch, the reservation book, the withdrawn friend-on-goal rule) plus three skirmish rows
(`both_sides_field_an_army`, `the_skirmish_only_refuses_what_the_fight_has_taken`, `a_friend_on_the_goal_tile_is_a_take`).
**Every economy row now passes**, which is the whole of the five that were starving.

**One rule was tried and withdrawn:** allowing a third ground re-plan after forty stalled ticks fixed nothing on its own (the
goal was the problem, not the budget) and broke `ground_it_cannot_cross_is_re_planned_at_most_twice`, which is a written rule.
Reverted; the cap stands.
