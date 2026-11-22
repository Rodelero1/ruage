# 2026-11-22 - the pile census: the owner's "5 in the same spot", measured

A probe change, no engine change. `collision_probe` now prints the **deepest pile** (the largest connected group of
overlapping bodies in any tick) and how many pairs sit within a fifth of a tile, so a report of "villagers stack on each
other" becomes a size.

## Performance

`collision_probe --release -- --ignored --nocapture`, 36,000 ticks, seed 7, HEAD (`1360bb7`):

- **deepest pile: 11 bodies** in one overlapping group; the deepest overlap is the full footprint (0.400 tiles);
- **349,824** pairs within both radii, on **35,336 of 36,000** ticks, max **63** overlapping pairs in one tick;
- **317,041** of those pairs are **closer than a fifth of a tile** - the overlaps are deep, not grazing;
- mean 29.8 bodies a tick.

**Not a regression from today's routing work.** The same probe on `3e3450a` (the tree before D346/D347): 349,413 pair-ticks,
max 61 in one tick, deepest 0.400 - a difference of +411 pair-ticks over 48,000 ticks (~0.1%), i.e. inside the scene's own
variation. The route changes did not create the pile; they also did not reduce it.

The probe costs one union-find per tick over the overlap graph (~150 bodies, ~11k pairs at 30 bodies): the run above is
**11.9 s**, against 11.8 s before the census - inside the band.

## Stability

- The census cannot change the sim: it reads positions after `world.step()` and writes only probe counters.
- `root()` is a path-halving find inside the probe; the parent vector is rebuilt per tick, so no state crosses ticks.
- A tick with no bodies leaves `max_pile` at 0 and the loop empty; the probe prints 0 rather than panicking.

**What it says, and what is not scheduled.** The tree classified these breaches once (D267, 2026-11-18): **mates 0, past
43,315, neither 61,071** - so more than half were pairs the step should have refused. Nothing in the queue targets the pile
today: sequence step 11 is formations (soldiers), step 13 is P3's inset - *touching* collides - which addresses grazing but
not a 0.4-tile overlap. The next step is therefore a read (`FUN_00840830`'s caller: is the body test before or after the
*past* check?) and then a slice, both filed in `docs/next-step.md`.
