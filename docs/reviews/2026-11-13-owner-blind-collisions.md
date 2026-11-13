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

## Eighteenth pass: two test rewrites, and the two real defects they exposed

Rewrote the two tests that pinned the withdrawn rules - `gather_continuous::ten_men_work_one_bush_a_body_apart` now asks the
engine's own capacity instead of asserting ten, and `movement::a_walker_whose_goal_tile_holds_a_friend_still_arrives` now
requires him to stand at the pair's reach **beside** the friend rather than through him. Neither went green, and neither is a
test problem:

```
gather_continuous: MEASURED the closest pair 0.107 tiles apart; a villager's circle is 0.40 wide and must fit
gather_standoff_rule: MEASURED the closest pair 0.083 tiles apart after ten seconds; two bodies are 0.40 wide
movement: the walker 1.011 tiles from the tile he was sent to after 600 ticks, task still Move
```

**Defect A - two settled men standing 0.1 tiles apart.** `standoff_point` refuses a place within 0.66 of any body, so the
choice is right; the **settle** is not - it walks a man onto the point it chose, and two men can arrive at the same point from
opposite sides in one tick, or a man can be settled onto a point a body has since occupied. Two bodies 0.083 apart is not a
crowd, it is two men in one shirt.

**Defect B - a move order never completes on an occupied tile.** The walker stops 1.011 tiles out, correctly, and his task stays
`Move` for ever: "arrived" is still judged only against the goal point, so an order onto a tile a friend holds never finishes.
D200 needs the predicate to accept standing at the pair's reach from the goal when the goal itself is taken.

Both are engine, both are test-first next, and neither is a decision.

## Nineteenth pass: two of the three defects fixed, and the third is a settle that does not clamp

**Defect B is fixed.** A `Move` order onto a tile a friend holds used to stay live for ever - the walker stopped at the pair's
reach, `reaches` was never true, and the task never ended. Two changes: `beside` accepts arrival within a quarter tile of the
neighbouring tile centre when the goal itself holds a body, and a new `finish_blocked_orders` pass ends any `Move` that has been
refused for the same forty ticks the nudge waits for. `movement` is 4/4; the order completes at **1.011** tiles and the friend
does not move.

**Two of my own errors in this pass, both caught by tests:**
- the first `beside` limit was `scale + reach`, and `reach_raw` falls back to a *whole tile* when content gives no radius - so a
  walker was declared arrived **1.839** tiles out. The limit is now `scale + scale/4`.
- the whole first attempt at this pass was written while the tree was on `main` (a plan commit had moved it there), so the
  settle fix was measured against `main`'s movement code. Same trap as the 0.038 overlap: **`git branch --show-current` before
  quoting a measurement**.

**Defect A is not fixed.** The settle now asks the engine's own `reach_raw` instead of `2 * body_radius(..).max(1)`, which was
two raw units when a unit has no radius in content - but the closest pair is still **0.083** (`gather_standoff_rule`) and
**0.107** (`gather_continuous`). So the overlap is not the reach arithmetic: two men *converge* on near-identical points, each
sliding along the ring in the opposite direction from the other, and `settle_workers` moves a body **directly** - it does not go
through `collide`, so nothing clamps the last step. The fix that follows from that: the settle must clamp like `give` does, or
refuse the step when the *new* position would be inside the pair's reach.

**Measured now:** `movement` 4/4, `gather_standoff_rule` 1/2, `gather_continuous` 3/4, `evasion` 6/6, `animals` 11/11.

## Twentieth pass: the settle clamps, the order waits for the man who cannot move, and two men still share a place

**In:** the settle now clamps its own step (halved until it stays outside the pair's reach) and updates its body snapshot
per move instead of once per tick - a snapshot let two men walk to the same spot in one tick because each check saw the
other's old position. `finish_blocked_orders` now finishes a `Move` **only when the man in the way has no route of his own**:
the first version ended orders whenever the forty ticks ran out, and two villagers crossing head-on blocked each other long
enough to strand one of them **three tiles short** (`a at (8,2)`, goal `(11,2)`) - caught by
`evasion::two_villagers_crossing_pass_through_and_both_arrive`, which now passes.

```
evasion                6 passed    incl. the crossing test and the ground-replan cap
movement               4 passed    the blocked Move finishes at 1.011 tiles, the friend does not move
gather_continuous      3 of 4      the closest pair 0.107
gather_standoff_rule   3 of 4      the closest pair 0.094
whole suite          115 green, 8 red
```

**Still open:** two men on one node end **0.094** tiles apart, both working (progress 4780 each). **My first reading of that
was wrong** - identical progress does not prove a shared place: every worker on one node has the same `rate_per_tick`, so their
progress is equal whenever they started together, which is the ordinary case. What is ruled out is narrower than I wrote: the
ring is fine (its seven places measure **0.592** apart at the closest), the settle's step is now clamped, and the reach
arithmetic is the engine's own. What is not yet excluded is the **nudge**: it moves a body 0.05 at a time and its `crowded`
test only asks about a third body inside the *pair's* reach of the destination.

**The eight reds:** content-pack house, the 0.094 and 0.107 overlaps, the woodcutter joining a patch, the reservation book,
`ten_men_touch...`, and two skirmish rows.

**Eliminated since, by experiment:** with `push_the_standing` **disabled entirely**, `six_settled_men_hold_their_places` still
measures **0.094** - so the nudge is not the cause either. The test does not exist on `main`, so the overlap is **branch-only**:
something introduced with the 0.66 spacing, the even-spread ring or D200 itself puts two men a tenth of a tile apart while both
work, and the suspects that remain are the paths that *place* a body rather than step it (`reserve_places` landing a man on a
point chosen before his neighbour arrived, `settle_workers`' ring-slide when the clamp halves a step it has already taken).

## Twenty-first pass: the overlap is the unplaced man, and the place book holds four for six

The pair, traced every tick (`trace_steps`, both men):

```
t=900..907   0/4  gap 0.094 <-> 0.099   a=Idle  b=Idle      (both working)
PROBE places = [(9700, 9.80, 6.406), (9701, 9.877, 7.158), (9702, 10.594, 7.20), (9703, 10.781, 5.80)]
```

**Four places for six men.** Both men in the overlapping pair are `Idle` - the movement pass is not moving them, they are
working - and the gap oscillates by five thousandths of a tile every tick, which is a man being pulled by something other than
a step. The two men of the pair hold **no place at all**: they are the extra workers a four-place ring cannot seat, standing in
the band anyway because a bush can be worked from anywhere inside the reach.

So the overlap is not a stepping bug and not a place-sharing bug: it is **an unplaced man, working without a place, standing
inside a placed one**. The settle clamps its own step (so placed men keep their distance); the extra workers reach the band by
ordinary walking, and the walking path's at-rest rule takes an **unbounded** step when the body in the way cannot yield (D200
rule 8) - which is exactly how a man ends up a tenth of a tile inside his neighbour.

**Tried and reverted in this pass:** a floor on the ring's minimum spacing (`2 * radius` becomes `scale / 5` when content gives
no radius - the ring was not the cause, the closest pair is unchanged at 0.094), and bounding the at-rest step again (also
unchanged, because the two men alternate being the mover). Neither shipped.

**The rule that follows:** an unplaced worker must stand at the pair's reach from the man on the place he would overlap, or be
turned away from the ring - the same choice the game makes when a resource is full, and the one the AI work will eventually
take over.

**Correction to the paragraph above, before anyone acts on it.** I wrote that the walking path takes an *unbounded* step into a
body at rest. It does not: in the committed code the at-rest branch for a **unit** is bounded (`give`), then give-way, then
`WaitBlocked`; the unbounded `TakeAtRest` is the branch for a **node or building** - something that is not a body and never
yields. So the mechanism I named is not in the tree, and the overlap is still unexplained by anything measured.

**What is established, and nothing more:**
1. six men are ordered onto a bush whose ring seats **four**;
2. the overlapping pair (ids 9700/9704) hold **no place** - they are the extra workers;
3. both are `Idle` (working) with their gap oscillating 0.094 <-> 0.099 every tick, so something moves a body a few thousandths
   per tick without a step being taken;
4. eliminated by experiment: the nudge (disabled, unchanged), the ring's spacing floor (unchanged), the at-rest bound (not in the
   unit path at all), the settle clamp (unchanged).

**Next instrument, and it is the only one left that can answer this:** print, for those two ids only, every writer that can move a
body - `settle_workers`' chosen point, `reserve_places`' reservation, and the position before/after each tick - which turns a
five-thousandths oscillation into a named call site.

## Twenty-second pass: the stale place, and what the writer trace actually said

The writer trace answered it: only `settle_workers` moves either man, and it moves the **place holder** onto a point a body is
already standing on, every tick, in a 0.005-tile loop.

```
WRITER settle id=EntityId(9700) from=(9.8022,6.4090) to=(9.8000,6.4062) d=(11,-99)
WRITER settle id=EntityId(9700) from=(9.8027,6.4040) to=(9.8000,6.4062) d=(-11,99)
```

**The rule that fixed it: a place a body now stands on is stale.** The place is a property of the order (D186), but a neighbour
who walked onto it cannot be walked through - so a held place with a body inside the gather spacing is released and re-chosen
from the ring, exactly as if it had never been taken.

```
gather_standoff_rule   5 passed    both overlap tests (six men, and the four-places-for-six case)
gather_motion          6 passed    the standing friend is walked past/round, not through
gather_continuous      1 of 2      closest pair 0.067 in the ten-men fixture (was 0.111)
```

**What did not work, and was reverted rather than kept:** freezing a place holder whose next step would land inside a neighbour.
It killed the overlap and killed navigation with it - a gatherer idled **1.45** tiles from his bush with a body in the path, and
`a_worker_goes_to_food_it_has_found` failed too. A rule that stops a man moving cannot also let him arrive.

**Still open:** the ten-men fixture, where the ring seats seven and three men are unplaced. Those three are the senior case now:
they hold a `Gather` task, cannot be seated, and crowd the ring - the game's answer is that they stand where they can and the
player's answer is to build another mill. The engine needs the *NoRoom* policy (stand down, or wait outside) rather than
re-choosing forever.

**And the no-float test earned its keep:** it failed on `step.rs:927-929` - my own leftover `WRITER` probe prints, which
convert to `f64` and were still in `settle_workers` after the trace was read. `no_float` scans the source, so an instrument left
behind is caught by the build rather than by review. Removed; the probe now lives only in the review text.

**Suite after the stale-place rule and the probe removal: 118 binaries green, 4 tests red.** The four are one family - *more men
than the ring seats*: the content-pack house, the ten-men bush (closest pair 0.067), the woodcutter joining a patch, and the
reservation book. Every one of them has **ten men ordered onto a ring of seven**, and the engine's current answer is to leave
the three extra men holding `Gather` while standing where they can - which crowds the men who do hold places.

**A refinement tried here:** the staleness window narrowed to the pair's reach (`0.4 + 0.05`) from the gather spacing (0.66),
because places sit exactly 0.66 apart and every place looked stale to its own neighbour. `gather_standoff_rule` stays 5/5; the
ten-men number did not move, so this is a correctness improvement without a measured effect yet.

**What those four need is the *NoRoom* policy,** not another geometry rule: a worker who cannot be seated should stand outside and
say so (idle), which is what the game does and what the owner's AI work will eventually own.

## Twenty-third pass: the ring is not full, and that is why nobody can be seated

Tried the NoRoom stand-down: if every live place on a node is held, an unplaced worker's order is finished. **It never fired**, and
the reason it could not is the actual finding:

```
MEASURED [8504, 8505, 8506] working the bush with no place while only 4 of seven are held
```

**Three of the seven places are free and three men hold none**, so the book is not full and the stand-down has nothing to say. The
three cannot be assigned because `standoff_point` refuses *every* candidate point: its guard is "no body within the gather spacing
of this point", and the unplaced men are standing near the ring, so every free place is refused on their account.

**The crowd is self-perpetuating**: men who cannot be seated stand around the ring, and their standing is what stops the book from
seating them. That is why the last four reds all show ten men on a seven-place ring - it is not a capacity problem and not an
overlap problem but a *refusal* problem.

**Reverted rather than kept**: the stand-down, because it measured nothing. The next rule to build is the inverse - an unplaced
worker **steps away from the node** until a place frees, so the ring can seat him - and it is testable on the same fixture that
prints the line above.

## Twenty-fourth pass: the owner chose "back off and wait", and the naive form thrashes

Built it as a **waiting queue**: an unplaced worker inside the ring's reach stands down (`Task::Idle`), his node is remembered in
`waiting_for_place`, and `seat_the_waiting` hands the order back as soon as `standoff_point` can give him a point. Measured on the
ten-men fixture:

```
MEASURED 4 of 10 men working the bush; the ring holds 7 at 0.66 spacing        (was 7 of 10)
MEASURED 4 men working the bush and 7 places held
```

**Worse, and for a visible reason: it thrashes.** A man stands down because no place can be *measured* for him right now (a body is
inside the spacing), is re-seated the moment one can, walks in, refuses again, stands down - so at any sample four are working and
three are in transit. The queue is the right shape; the trigger is not: standing down must be **sticky** (only after the same
forty refused ticks the nudge uses) and re-seating must require a place that is **free in the book**, not merely measurable from
where he happens to stand.

Reverted, so the tree stays at the measured `118 green / 4 red`. The shape to build next is written here rather than in a comment
in the code, because the next attempt should start from the numbers above, not from scratch.

## Twenty-fifth pass: sticky, book-aware, and still four of ten - the capacity is notional

Built the corrected shape: a full second unseated before standing down (`UNSEATED_TICKS = 20`), a booked place that cannot be
reached counting as unseated, the booking released when the man stands down, and `seat_the_waiting` calling a man back only when
the book shows a place free on his node.

```
MEASURED 4 of 10 men working the bush; the ring holds 7 at 0.66 spacing
MEASURED 4 men working the bush and 7 places held
```

**Unchanged, and this is the finding rather than a failure of the plumbing:** the ring's capacity is **notional**. Seven points
exist on the outline, but when bodies crowd the node only four of them can actually be stood on, and a man walking to the other
three is refused by the bodies in the way. Counting ring points therefore over-seats the node, and any policy that trusts the
count - crowd, wait, or idle - inherits the error.

**What the game must be doing (to be confirmed against the binary):** capacity is not a property of the ring alone; it is how many
bodies can stand at the node *given the bodies already there*. That is not a number this engine computes anywhere, and computing it
is the real task - `standoff_point` answers it one man at a time, which is why the book and the crowd disagree.

**Reverted**, so the tree stays at the measured `118 green / 4 red` rather than shipping four-of-ten. The four reds stand as the
record of the gap: content-pack house, the ten-men bush (0.067 overlap), the woodcutter joining a patch, and the reservation book.

## Twenty-sixth pass: the owner's rule, first half - comfortable, then packed

The owner's rule (2026-11-13): *a villager joining others at work, within 1.5 diameters, closes to one diameter; if the space is
full he waits, and then idles.* Built the first half - **placement is two-tier**:

- the ring is searched at the measured **0.66** first (his own saves, and what an uncrowded bush looks like);
- only when the whole ring refuses is it searched at **0.40**, one diameter, the floor;

```
gather_motion 6/6   gather_standoff_rule 4/4   gather_ring 4/4   gather_continuous: still 7 of 10
```

**Why the count did not move:** the seven seated men are already spread evenly at 0.66, and 0.40 clearance is impossible *between*
them - the gap is only 0.33 either side. Ten men fit round the tile only if the **seated men shuffle**, i.e. the node re-lays its
whole ring at the finer spacing when a tenth man is refused. That is the second half of his rule, and it is a node-level operation
(collect the live workers, sort by angle, re-assign from the packed ring), not another per-man search. The two-tier placement is
the groundwork for it: the packed ring is already generated and already preferred last.

**The second half is specified and not built.** A node that refuses a man must **re-lay its whole ring** at the packed spacing:
take every live worker on the node, assign the packed places nearest-first, and let the settle walk them there. It is a
node-level operation, so it needs the same rate limit the nudge uses (once per twenty ticks per node) or the ring will shuffle
every tick. The trigger is the unseated counter of the previous pass - kept in this note rather than in the code, because the code
that carried it was reverted when the count alone did not help.

`gather_continuous::ten_men_work_one_bush_a_body_apart` now asserts **ten** working men with a **0.39** floor, so the harness for
that work is already red and waiting.

## Twenty-seventh pass: built the re-lay, and the geometry says no

Built the second half: a node that refuses a worker queues itself in `work.respreads`, and `maybe_respread` re-lays its whole
ring at the packed spacing, assigning the nearest place to each worker, rate-limited to once per twenty ticks per node.

```
PROBE respread node=EntityId(11) places=1 assigned=7
```

**It fires and it cannot seat ten, because ten do not fit.** `touching_places` at the touching radius (0.2) with a 0.40 chord
floor returns **seven** points: chords round a corner are shorter than arcs, and the ring's own closest pair measures 0.592. A
tenth man could only stand further out - and further out is outside the gather reach, so he would be standing, not gathering.

**So the owner's rule and the owner's saves agree, by a road neither of us took deliberately:** seven men per node is what the
touching ring holds at a body's width, and seven is what his three saves show. Ten men "working one bush" in the real game must
include men standing at a second resource or at the far side of a wide one - not ten on one tile.

**What shipped:** the two-tier placement (comfortable 0.66 first, packed 0.40 second), the node re-lay (reachable and harmless,
and it is what makes the packed spacing real), and the ten-men test now asserts the ring's **measured** capacity rather than a
hoped-for ten.

```
gather_continuous 2/2   gather_motion 6/6   gather_standoff_rule 4/4   gather_touching 3/3
```

## Twenty-eighth pass: 121 green, 1 red - and the last one is a two-man jam

```
whole suite   121 binaries green, 1 test red     (only a_content_pack_with_our_own_resources_loads_and_spends)
```

The house never leaves hp 1, and the trace says why: **the builder and a walker stand on the same tile, each waiting for the
other.**

```
PROBE owntile id=EntityId(2) other=EntityId(4) other_task=Move{to (5.5,4.5)}   here=(4,4)
PROBE owntile id=EntityId(4) other=EntityId(2) other_task=Build{site 11}       here=(4,4)
```

Neither is "a body with nothing to do", so `push_the_standing` skipped both, and the bounded give-way step is zero at nought
distance. Three fixes went in and are kept (all suites still green): `WaitOwnTile` now counts toward the stall total, a jammed
man **with an order** may be nudged after the same forty ticks, and the nudge gets its direction from the man's task goal when his
route is empty (the builder had no waypoint at all - which is why he was never a candidate).

**It still fails**, so the pair must be re-jamming after the nudge: the push moves the body, the builder steps back onto the tile,
and the loop repeats. The next instrument is the one that worked here - the pair's `last_step` and positions every tick for the
first 200 ticks - and the likely rule is that the *walker* should route round rather than the builder being shoved off his own site.
