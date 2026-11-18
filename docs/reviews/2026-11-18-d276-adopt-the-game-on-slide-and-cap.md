# Review: D127, D135, D253, D255 and D256 reviewed and withdrawn - the slide and the ground cap go

*Change: **D276**. Five decisions re-read against the binary and the decomp, all five withdrawn, and the three slices that
replace them ordered by dependency. `docs/next-step.md` carries the order. **No code changed.***

## Performance

**Nothing executable changed**, so the numbers here are the ones the withdrawals are priced with, re-read from the corpus and
not recalled:

- **D135's price of the cap**: re-plan attempts **11,260 -> 192**, failures **11,070 -> 4**, match **47.5 s -> 11.8 s** - taken
  *before* the sub-tile pencil existed, so deleting the cap re-measures rather than re-quotes it.
- **D253's price of the slide**: **82, 81, 80... raw units a tick** where a step is **800**, for **567 of 1,398** walking ticks
  (10% steps), measured with a body parked half a tile past a corner.
- **D255's price of the order**: slide deleted with the disc step test - **14 red across 8 binaries**; stepping by the tile grid
  alone - **3 red of 200**, one of them the crowd fixture that already fails.

The one claim that would make this note wrong is that the game has no ground-re-plan cap. It rests on D239's read of
`FUN_008a4360` (a terrain collision is a repath, counted nowhere) and on `mConsecutiveSubstituteCount` (an unreachable waypoint is
*faked*, not retried) - both first-party strings in the binary, both recorded in `docs/movement-in-the-binary.md`.

## Stability

**What was attacked.** The three ways this review could have been wrong, and what closed each:

- **"The cap is load-bearing, so it must be the game's."** Refuted by mechanism, not by taste: the loop the cap hides exists
  because two measures of "walkable" disagree (D254), and the game closes that gap with a radius-aware search plus substitution.
  Deleting the cap *before* row 26 lands would reproduce the 47.5 s match - which is why it is slice 3 and not slice 1.
- **"The slide can be repaired."** Already refuted twice by measurement before this review (full-length slide: 16 tests across
  10 binaries; stand instead: 14 across 8). A third repair is what D276 forbids.
- **"Deleting the slide is a one-line change."** It is not, and the order says so: with 3 red left in the animals family, the
  missing piece is a **route that bends inside a tile**, i.e. shoulder points round footprint corners - D261's named piece,
  never built.

**Found and reported rather than smoothed:** D127's own rationale quotes `openempire`'s "re-examine, do not walk into it", which is
the correct principle and is *not* what the cap implements. The decision's stated reason and its mechanism were pointing at
different halves of the same read, and the row has carried that since 2026-11-05.

