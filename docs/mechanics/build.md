# Building

Status: implemented for the step-5 slice (one builder, one tile, no footprint).

## Where the numbers come from

A building's price and its duration are both in the unit table, and one of them is under a name
that does not say what it does.

- **Cost** — `costs = [[resource_id, quantity]]`. The ids are the dat's: 0 food, 1 wood, 2 stone,
  3 gold, 4 population. A house is `[[1, 25]]`, a barracks `[[1, 175]]`.
- **Duration** — `train_time`, which is the dat's **`create_time`**. For a unit that is training
  time; for a building it is how long one villager takes to build it. House 25 s, barracks 50 s,
  farm 15 s, mill 35 s, castle 200 s, town centre 100 s — all matching published build times.

This is the correction recorded as U15 in `README.md`: the field was never missing, it was called
something else, and a search for a field *named* `build_time` in two parsers produced a false
conclusion about the data.

## The progress rule

Progress is counted in **builder-ticks**, not seconds and not HP:

```
total_ticks    = create_time seconds x tick_rate_hz
progress_ticks += 1 per tick while a builder stands on the site
hp             = max_hp x progress_ticks / total_ticks      (and exactly max_hp at the end)
```

HP is therefore derived from progress rather than accumulated from it, which matters where HP per
second is not a whole number: a mill is 600 HP over 35 s, 17.142857 HP/s, 0.857 HP/tick at 20 Hz.
Adding a rounded rate every tick would finish at the wrong tick; dividing at read time cannot
drift, and the build lands exactly on `create_time x tick_rate` (500 ticks for a house at 20 Hz —
asserted in `crates/sim/tests/build.rs`).

The clock is working time, not wall time. A villager who walks 25 ticks to the site does not
finish 25 ticks late: progress only advances while the builder is standing on it, and the tick of
arrival is not a working tick because the work phase reads positions before movement.

## What the order checks, and in what order

1. the unit exists, does not move (`speed == 0`) and has a positive `create_time`
2. the player can pay **all** cost entries
3. the tile is inside the map and not blocked terrain
4. nothing already stands on the tile
5. a route to the tile exists

Every check runs before any money moves, so a refused order costs nothing and leaves no site
behind. That includes the path check, which was the one hole found in review: initially the route
was planned *after* payment, which would have left a paid-for construction nobody could reach and
that never progressed. `crates/sim/tests/build.rs` proves it with a map split by a forest wall —
the test fails with `expected NoPath, got Ok(())` if the check is removed.

Commands are queued and applied on the next tick (`CommandApply::NextTick`), so the price is paid
when the order lands, not when it is submitted.

## Deliberate limits in this slice

- **A building does not block movement.** The walk grid is static, built from the map at seed
  time, so a house walls off nothing until the grid is updated as entities are placed.
- **One tile per building.** Real footprints are 2x2 to 5x5; the site is a single tile here.
- **Population cost is ignored.** `[4, n]` entries are not charged, so a house does not raise a
  population cap and a unit does not consume one.
- **No repair, no cancellation, no refund.** A finished building is simply a building with HP.
- **Several villagers on one site each add one builder-tick**, so two villagers build a house in
  250 ticks. That matches the game closely enough for the slice but has not been measured against
  it.
