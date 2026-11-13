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
