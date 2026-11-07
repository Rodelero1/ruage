# A worker only goes where it has looked (D148)

*2026-11-07. The owner: "I noticed that villagers go across the map to gather berries even when they were not found before…
Villagers are supposed to look for resources which are nearby when their current supply is depleted. If nothing is found,
they should be idle."*

## The bug, and why the earlier attempt missed it

The picker answered *"which node is this worker's?"* by **distance over every node in the world**. Three callers did it:

- `World::nodes_of_slot` — the script's `do = "gather"` action and the villager-share pass;
- `World::nearest_like` — the "my node ran dry, what next?" path, which sorts all candidates by distance and asks
  reachability of the nearest eight.

None of them asked whether the player had ever **seen** the node. So a berry patch thirty tiles away, in fog nobody had
removed, was a perfectly good candidate — the behaviour the owner watched.

**D141 tried a radius instead** (`los + 5`) and was reverted by measurement: 141 entities became 129, both players alive
became one. That was the wrong axis. The engine already keeps a per-player fog, and a **resource node never moves**, so
"found" is exactly "its tile has been explored" — a fact the engine has and the picker ignored.

## The fix

`World::knows(player, entity)` — one predicate, on the fog: `explored_bit(player, tile_of(position))`, with the observer
(player 0) knowing everything. Used at every automatic pick:

- `nodes_of_slot(from, player, slot, limit)` — the player is now a parameter, because the question is *whose* knowledge.
- `nearest_like` — the worker's own player.

A scripted `gather` step goes through the same picker, so **a scenario cannot order work at a node its player has not
scouted** either. That is deliberate and it is what the owner's sentence asks for; the cost is that a scenario has to
scout first, and `a_scenario_researches_a_tech_named_by_a_role` was rewritten to do exactly that (walk a unit over the
gold, then gather) rather than to be exempted.

**RED, watched** (`crates/sim/tests/gather_knowledge.rs`, written first):

```
test a_worker_is_never_sent_to_food_it_has_not_found ... FAILED
thread '...' panicked at crates/sim/tests/gather_knowledge.rs:168:9:
entity 2 walked to x=31.3, off across the map for a node it has never seen
(task Gather { node: EntityId(12), rate_per_tick: Fixed(310), carry_cap: 10, progress: Fixed(0) })
test a_worker_goes_to_food_it_has_found ... ok
```

The control test passed **before** the change, which is what makes the first one meaningful: the map works, food in sight
is gathered, and only the unseen patch is refused afterwards. **GREEN**: 2 of 2.

## Performance

**The picker got cheaper, not dearer**: an unexplored node is now rejected by one array lookup and a bit test *before* the
distance is computed, so on a map whose fog covers most of it there are strictly fewer candidates to sort. The check is
`explored_bit` — index arithmetic and a `u64` mask, the cheapest thing in the engine's memory module.

**Measured wall clock, with its caveat**: three acceptance runs after the change gave 11,552 / 11,785 / 11,527 ms. `uptime`
read **3.23-3.92 load average on a two-CPU cgroup** during them, against an idle box for the D146 A/B that measured 10.7 s,
so these are **not comparable** to that number and no claim is made from them. What *is* comparable is the run before and
the run after this change on the same box in the same session: both were ~11.6 s, and the diff of their dumps is one line.

## What it did to the acceptance match — measured

The acceptance run's hash **moved**, and this is a behaviour change so it must:

```
before  tick=36000 entities=141 players=1 hash=f328ebc8d4d34b78 repaths=188 made_room=187
after   tick=36000 entities=141 players=1 hash=dfc835be5d880a47 repaths=188 made_room=187
```

**Every count is identical** — 141 entities, 1 player alive, 188 repaths, 187 made-room — so the change did not starve
or unbalance the match. To see *what* moved, both runs were dumped (`--dump`) and diffed on `kind,player,id,unit,amount`:

```
64c64
< resource,0,117,0,310.00
---
> resource,0,117,0,400.00
```

**One line, one node, 90 raw units (0.09 of a unit).** Node 117 keeps 90 more in the new run: a villager that the old
picker sent to an unseen node now works a node it knows instead, and the difference at the end of a 30-minute match is
0.09 of one resource. Positions, tasks, hit points and every other node are byte-identical. That is the smallest change a
filter of this kind can produce on a mirrored map, and it is what "the acceptance run does not depend on the bug" looks
like.

## Stability

- **Test-first, RED watched**: `a_worker_is_never_sent_to_food_it_has_not_found` failed with *"entity 2 walked to x=31.3,
  off across the map for a node it has never seen"*, and the control `a_worker_goes_to_food_it_has_found` passed **before**
  the change — so the fixture provably worked and only the unseen patch was refused afterwards.
- **A real regression, found by the gate and fixed at its cause**: `a_scenario_researches_a_tech_named_by_a_role` failed
  with three refusals, *"player 1 has no gold node number 0 to gather"*. The gold on `config/map.toml` sits **9.4 tiles**
  from player 1's town centre — just outside the starting sight — so the test had been relying on the unfogged picker. It
  now walks a unit over the gold first, which is how a player finds it, and the rule is **not** exempted for scripted
  steps.
- **The suite is green at 735 passed / 0 failed / 19 ignored** (two new tests), clippy `-D warnings` 0, fmt 0.
- **The behaviour change is bounded and explained**: acceptance counts identical (141 entities, 1 player, 188 repaths, 187
  made-room), one node differs by 90 raw units out of a 30-minute match, everything else byte-identical.
- **What is deliberately not done**: a radius. "Nearby" is the other half of the owner's sentence and the number is his;
  the options and the plan to measure them are at the end of this file.

## The new baseline

`dfc835be5d880a47` replaces `f328ebc8d4d34b78` as the acceptance hash. It is the first **deliberate** change to it since
the match was set up: every earlier slice was proved by keeping it, and this one is proved by explaining what moved and
why (one node, 90 raw units, identical counts).

## What is still open — and it is the second half of the owner's rule

**"Nearby" is not bounded yet.** Among the nodes a player *knows*, the picker still takes the nearest, however far that is:
on a well-scouted map a villager can still walk forty tiles. The rule he wrote has two parts — *nearby* and *idle if
nothing is found* — and only the second is implemented (nothing known → the worker stands down, which it now does).

The number is a gameplay rule, so it is his:

- **A. no bound beyond knowledge** (what ships now): nearest known node, anywhere;
- **B. the worker's line of sight** (4 tiles for a villager in this pack): very tight, and D141's measurement is a warning
  about tight bounds, though that bound was applied to every node rather than to known ones;
- **C. `los + 5`** (9 tiles): D141's number, applied to known nodes only this time.

C is measurable cheaply: a constant in the picker, the same acceptance run, and the two new tests. It was **not** done in
this commit because the measurement is only worth making once the owner says which number he wants to see.
