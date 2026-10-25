# Training

Status: implemented for the step-5 slice (one job per building, one spawn tile).

## Who trains what

The dat carries what a unit costs and how long it takes, and the tech tree disables units per
civilisation — but nothing we parse says *this building makes that unit*. So the edge is content,
in `config/economy.toml`:

```toml
[[train.entry]]
building = 109   # RTWC
units = [83]     # VMBAS

[[train.entry]]
building = 12    # BRKS
units = [74]     # SPRMN (40 hp, 0.9 tiles/s, 60 food + 20 gold, 21 s)
```

The loader refuses an empty unit list, a zero unit id, and the same building listed twice (the
second entry would silently win). A test pins the real file: the town centre trains villagers, the
barracks trains spearmen, and neither trains the other's.

## The timer

Costs and `create_time` come from the unit table, exactly as for building. The timer counts
**ticks remaining**, resolved to `create_time x tick_rate` when the order lands, so a 25 s villager
at 20 Hz appears exactly 500 ticks later. The price is taken when the order lands, not when the
unit appears — same as building.

The tick an order lands is spent *starting* the job, not working on it (`Training::fresh`). Without
that bit the production phase, which runs later in the same tick, would take a free tick and the
unit would appear at 499 ticks instead of 500. It is one bool, it is hashed, and it is the reason a
25 s unit is 25 s.

## Refusals, in the order they are checked

1. the target is a building (`TrainNeedsABuilding`)
2. it is finished, not a construction site (`BuildingNotReady`)
3. it is not already producing (`AlreadyTraining`)
4. it is allowed to make this unit (`NotTrainableHere`)
5. the player can pay every cost entry (`NotEnough`)

Readiness before trainability is deliberate: a house that is still going up should be told it is
not finished, not that it trains nothing.

`AlreadyTraining` exists because the alternative was worse than a refusal. Without it, a second
order overwrote the running job *and* charged for it, so a paid-for villager vanished. The test for
this banks **100** food rather than 50 on purpose: with only 50 the second order would be refused
for being unaffordable and the test would pass without proving anything about the slot. With the
guard removed it fails `expected AlreadyTraining, got Ok(())`.

## Spawning

The finished unit appears on the first free tile within two tiles of its building, scanning in a
fixed order (`dy`, then `dx`, then the four sign combinations) so the result is deterministic and
replayable. If every candidate is blocked or occupied the unit **waits inside the building** — the
timer stays at zero and the spawn is retried each tick — rather than being dropped.

## Deliberate limits in this slice

- **One job per building.** No queue, so the second order is refused rather than queued.
- **No population accounting.** The `[4, n]` cost entry is ignored, so units cost no population and
  houses raise no cap.
- **No prerequisites.** Nothing checks that a barracks needed a mill, or that a spearman needs a
  barracks in the tech tree sense; the `[train]` table is the whole rule.
- **The spawn search is a 5x5 box, not the unit's radius.** A building's footprint is one tile here,
  so this has not yet needed to be right.
