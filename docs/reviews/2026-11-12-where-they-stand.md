# Where they stand: reading unit positions out of a save

**No engine change.** A reading tool, a format note, and one measurement that settles a question the owner asked twice.

## What was wrong with the old reading

The tail of the save is an **order queue**: 53 records of `[type][1][0][stamp][n]` plus `n` triples. The first triple is the
position of the unit **when the order was given**, and the last is the destination tile. That is why the tree save measured
cleanly - the owner clicked while his woodcutters were already standing at the tree - and why the bush save did not: its
from-points came out at **1.21 to 2.57** tiles, because the click happened while the men were still walking.

## What the objects look like

A run of **229-byte records** in the world section, first eight bytes `float32 x, float32 y` in tiles, `world = column + 0.5`.
Found by reading every offset in a window around the cluster as a float pair and taking the longest chain at a constant stride;
the bush save gives **21 records at stride 229 from `0x19fee8`**.

## Performance

`tools/savegame_positions.py` walks the 3.3 MB inflated body offset by offset and prints the run and the ring: **998 ms** for one
save on this box (one pass, no indexing; it is a reading tool, not a per-tick cost). Nothing in the engine changed, so the
acceptance run is untouched: `main` at `d499955`, 172 test binaries green, hash `ef7cdba92a83f173`.

## The measurement, and what it settles

Around the bush square at `(64, 78)`, radii from its centre:

```
0.699 0.699 0.699 0.759 | 0.995 0.995 0.995 0.995 | 1.100 1.103 1.103 1.140 1.145 1.159 1.161 1.166 1.174 1.174 | 1.627 1.798 1.975
```

- **Ten men on the ring at 1.100-1.174 (mean 1.143), closest pair 0.400, mean nearest 0.401.** The tree save gives 1.105-1.189
  and 0.404. **A bush and a tree share the ring**, so `GATHER_RING_MILLI = 1.156` is right for both, and the owner's *"quite far
  from the berry bush"* is the game's own distance rather than our error.
- `0.995` x4: neighbouring tile centres, exactly one tile out - men walking, caught at a waypoint.
- `0.699` x3: the tile corner distance `0.5 * sqrt(2) = 0.707` - bodies just off the corners, walking past.
- Ten men, not nine: `2 * pi * 1.143 / 0.400 = 17.9`, so the ring fits eighteen and the game used ten.

## Stability

The tool reads a file and prints; it cannot change a match. Two traps are recorded in `docs/save-format.md` so the next reader does
not fall into them: **the order queue is not a position table** (its from-point is the issuer's position at click time), and **a
fixed offset cannot be trusted between saves** - the record run is found per save by its stride, not by its address.
