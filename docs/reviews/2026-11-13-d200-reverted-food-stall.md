# D200 is reverted: the acceptance match banks no food at all

*2026-11-13, second session of the day. The merge of `d189-owner-blind` into `main` (`2c7d8e1`) passed every test and both
gates, and it turned the acceptance match's **food economy off**: from the first report to the last, the bank holds **0
food**. This note is the measurement, the mechanism, and why the tree is back to `50ef391`-equivalent behaviour rather than
forward-fixed in a hurry.*

## What was measured

The same command on both trees, `--seed 7 --ticks 48000 --report 400`, release build, `/opt/data/tools/cargo/bin` first on
`PATH`:

```
50ef391 (before the merge)   400s wood 64 food 56 | 800s food 397 | 1200s food 667 | 2400s wood 2019 food 690 units 43 buildings 20
3f9267f (after the merge)    400s wood 70 food  0 | 800s food   0 | 1200s food   0 | 2400s wood  790 food   0 units 14 buildings  7
```

and the summary line each run prints:

```
before  tick=48000 entities=128 players=1 hash=ace125e60ea99b60 repaths=579 made_room=0  repaths_body=0 repaths_ground=579
after   tick=48000 entities=130 players=2 hash=f4b952433251f435 repaths=31  made_room=6  repaths_body=4 repaths_ground=27
```

Both are reproducible: the "after" line printed identically three times. **The hash in the merged review
(`17c79d51059de985`) is not the hash of the merged tree** - the measured one is `f4b952433251f435`.

**Six observations, from the world dumps at 3,000 and 8,000 ticks.** Four villagers of player 1 sit between the berry bush
`(13.5, 19.5)` and their town centre `(16, 16)`, each holding ten food:

```
id=4   at=(14.33,18.41) carry=10.00 task=Return>EntityId(1)  path=1
id=134 at=(14.44,18.01) carry=10.00 task=Return>EntityId(1)  path=1
id=135 at=(14.16,17.65) carry=10.00 task=Return>EntityId(1)  path=1
id=136 at=(14.86,18.43) carry=10.00 task=Return>EntityId(1)  path=1
```

They do not move again: at 48,000 ticks they hold the same positions and the same loads.

## The mechanism, from the engine's own numbers

An env-gated probe (`RUAGE_PROBE_CARRY`) in the `Task::Return` arm printed every tick for a loaded carrier: its position, the
tile it stands on, the point its route is aiming at, the route's length, and whether it counts as touching the drop site:

```
PROBE carry id=EntityId(135) at=(283254,352965) here=(14, 17) next=(320000,320000) route=1 touching=false    (419 consecutive ticks, identical)
PROBE carry id=EntityId(134) at=(288712,360254) here=(14, 18) next=(320000,320000) route=1 touching=false
PROBE carry id=EntityId(136) at=(297103,368506) here=(14, 18) next=(320000,320000) route=1 touching=false
```

`next=(320000,320000)` is raw scale-20,000 coordinates: **the town centre's own tile, `(16, 16)`** - the building's blocked
anchor. The route is one point long and that point is the anchor. So the carrier walks at a tile he can never stand on, the
collision bound holds him at the pair's reach (friends are solid now, and a building is a friend), and the deposit test
never fires: not `touching` by tile (he is 1-2 tiles from the anchor, not ±1 tile), not `touching` by radius, and not on the
footprint. `next_waypoint` keeps returning the anchor, so `approach_tile` - which the code falls back to **only when there is
no route at all** - never gets asked. Four men in a line, 1.7-2.4 tiles out, all pointing at the same unreachable point, for
48,000 ticks.

Why the tests did not catch it: every fixture in `crates/sim/tests` stages a drop site whose anchor is the tile the man can
approach (a mill beside grass, a town centre with a clear yard), and the `Return` route is planned to a standable tile
there. The acceptance match's town centre sits with its own footprint around the anchor, so the route the AI hands out ends
on blocked ground. `a_crowd_all_banks_its_loads` - the guard written for exactly this - passes both before and after the
change.

**Why `players` moved, and why the merged review's explanation is wrong.** The review said *"`players` differs because the
acceptance script on the branch seeds two sides."* The script is `config/acceptance.toml`, the merge changed no config file,
and the number is computed by the harness as the **highest player id among surviving entities**
(`crates/harness/src/bin/ruage-headless.rs:320`). `players=1` means player 2 has nothing left; `players=2` means player 2 is
still standing - four villagers and a town centre at full 2,400 hp - because player 1 never trained a soldier, because the
bank never held 50 food. It is not a seeding difference; it is the food stall seen from the other end.

## What this note does not claim

It does not claim the D200 rules are wrong. Friends being solid is the owner's ruling and it is what the binary does. It
claims that **the merge changed the acceptance match's food economy from 690 food and 43 units to 0 food and 14 units**, and
that the test suite cannot see it - so the change goes back to its branch with the diagnosis attached, which is the standing
rule for a step that cannot go green.

## Performance

The measurement above is the performance number that matters here: the same binary, the same command, 48,000 ticks, `main`
before the merge `repaths=579 made_room=0`, after it `repaths=31 made_room=6` - and **the 17-fold drop in re-plans is not a
win but a symptom**: men who never move never ask for a route. The food bank at 400 s is **56 before, 0 after**; the unit
count at 2,400 s is **43 before, 14 after**.

## Stability

The reverted tree is the whole workspace gate, run after the revert and quoted from that run: **183 test binaries, 834
passed, 0 failed, 20 ignored, `EXIT 0`** (the merged tree had 839: the branch's five new tests go with it); `cargo fmt --all
-- --check` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean. The acceptance match is back to
`entities=128 players=1 hash=ace125e60ea99b60 repaths=579 made_room=0`, and the timeline is back to 56 / 397 / 667 / 388 /
650 / 690 food at 400..2400 s. Nothing else in `main` changed: this is `git revert -m 1 2c7d8e1`, so the branch
`d189-owner-blind` keeps every commit of the work and `main` keeps `50ef391`'s behaviour.

**Recorded for the next attempt:** a `Return` route that ends on a building's own blocked anchor is a `Return` that never
completes. The fix belongs in the `Return` arm - a waypoint that is not standable ground must be replaced by
`approach_tile`, whether or not a route exists - and it needs a fixture whose drop site is a **town centre with its own
footprint** and a carrier approaching from open ground, watched red first.
