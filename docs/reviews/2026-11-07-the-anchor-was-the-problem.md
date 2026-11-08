# +5 was never the problem; the anchor was (D150)

*2026-11-07. The owner's reply to the measurement: "If +5 doesn't work while +8 does, it suggests either (1) issue when
villager started to look for another resource, (2) map issue (forest near town centre depleted). Or is there any other? If
1 is the case, then villager has to return to the place where he gathered the resource the last time and only then to look
for another tree. If 2 is the case, villager was right to become idle."*

**Neither, exactly — and once the real cause was fixed, `+5` works.** This file is the instrumented answer.

## What the failing picks actually were

The reach was instrumented (temporarily) to print, for every pick that found nothing: the worker, its position, the stockpile
column it wanted, and the distance to the nearest **known** node of that column ignoring the reach. At `+5` the acceptance run
produced **586 such failures**, and every single one was the same shape:

```
no-node worker=176 at=(14.0,17.0) slot=2 reach=9 nearest_known=11.2
no-node worker=148 at=(15.0,14.1) slot=2 reach=9 nearest_known=9.2
no-node worker=4   at=(15.2,19.5) slot=2 reach=9 nearest_known=11.5
no-node worker=167 at=(11.0,25.2) slot=2 reach=9 nearest_known=18.5
```

**Slot 2 is gold** (`config/resources.toml`: 0 wood, 1 food, 2 gold, 3 stone). The nearest known gold was **9.2–18.5 tiles
away**, and the map puts player 1's gold at **(24, 11)-(25, 12) against a town centre at (16, 16) — 9.4 tiles**. A reach of
nine tiles misses the base's own gold by 0.2 tiles.

## The owner's two hypotheses, tested

**1. A bug when the villager looks for another resource — half right, and not where it says.** The depletion path is *not*
at fault: a worker whose node runs out chooses while it stands at the node, and that is already what the engine did. It was
proved by writing the owner's rule explicitly (choose at the node, remember it in `pending_gather`, go there after the
deposit) and watching the test pass **with and without** the patch — the patch was reverted, because unprovable code does
not ship. The test is kept as the guard for that behaviour, and it is what caught that a worker stands *beside* a node, not
on it (so a "9 tiles from the tree" case is really 10 from the worker — the first version of the test was wrong about that).

The faulty anchor was somewhere else: **the share table** measured "what is nearby" from the villager's own position. A
villager spawned or standing away from its base therefore found nothing within its reach, and a share row ("eight on wood")
never fired. That is D149's fix: the share pass anchors on the player's **town centre**.

**2. A map issue, forest depleted — no.** Wood and berries sit 4-6 tiles from the town centre and **not one wood or food
pick failed** in the whole run; every failure was gold. Nothing near the base was exhausted. What is true in this hypothesis
is the conclusion — *the villager was right to become idle*: it looked around, found nothing within nine tiles, and stopped.
The map's gold simply sits just outside where it looked.

## The measurement, after the anchor fix

| look-around | reach | entities | players alive | repaths | made-room | hash |
|---|---|---|---|---|---|---|
| **5 (the owner's choice), before the anchor fix** | 9 | 129 | **2** | 83 | 164 | `5d0f8043bfbb7d9f` |
| **5 (the owner's choice), after** | **9** | **135** | **1** | **125** | **116** | **`45596da62c9e3668`** |
| 8 (what shipped before this) | 12 | 152 | 1 | 186 | 159 | `82a5063dcca63108` |

**`+5` now resolves the match** — three consecutive runs, identical hash. It was never the number: it was that a pick from
the drop site, made on behalf of a villager that had walked home, could not see the base's own gold. With the share pass
anchored on the town centre, gold income exists at `+5`.

## Performance

No per-tick path was added. `gather_reach` is one `sight_of` lookup and an `i64` multiply, computed once per pick attempt;
the reach test is a squared-distance comparison that **runs before** the candidate is pushed, so a smaller reach does strictly
less work. The acceptance run at `+5` finishes in the same ~11.5 s bracket as `+8` on this box (load average 3+ throughout),
which is inside the noise floor and is not claimed as a difference.

## Stability

- **The two failures this pass produced were both mine, and both caught by tests**: the reach harvested a *tile* count and
  added a *raw* offset (a 5-tile reach that refused everything — caught by the in-reach control), and a worker stands
  **beside** a node, so a "9 tiles from the tree" fixture was really 10 from the worker (caught by the guard test when the
  constant changed).
- **Unprovable code was reverted rather than shipped**: the node-anchored `pending_gather` patch had no failing test to
  justify it — the engine already behaves that way — so it is not in the tree, and the finding is written here instead.
- **The constant is the owner's number**: 5, with the five tests in `gather_knowledge.rs` written against the geometry it
  implies (15 tiles out, 6 tiles in, 11-tiles-from-the-base guarded by the worker's own position).
