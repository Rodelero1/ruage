# Review: a measurement probe for the E1a/E1b fork (tile versus radius collision)

*Change: `crates/sim/tests/collision_probe.rs`, an ignored probe that plays the acceptance match and counts, per pair of
live bodies per tick, whether a **tile** rule (same tile) and a **radius** rule (centres closer than the sum of
`radius[0]`) would bind - and where the two disagree. It exists to price D98a's staging decision: land the evasion
machinery on a tile predicate (E1a) and swap in radii later (E1b), or write the radius predicate from the start. No
production code changed; the probe asserts nothing and is ignored by default.*

## Performance

36,000 ticks of the acceptance match at seed 7, **mean 29.0 live bodies per tick**, release build, 21 s:

- pairs in the **same tile**: **3,686,676** pair-ticks (a tile rule binds);
- pairs **within both radii**: **3,658,088** pair-ticks (a radius rule binds), on 35,984 of 36,000 ticks, max 328 pairs
  in one tick;
- **disagreements: radius alone 15,329, tile alone 43,917** - about **1.6% of binding pairs**.

So in the world as it stands the two predicates agree on ~98% of what they bind. The radius rule is *sharper at tile
boundaries* (bodies touching across two tiles, 15 k pair-ticks) and *more permissive inside one tile* (stacked centres
that do not actually touch, 44 k pair-ticks). The probe's cost is its own: O(bodies²) per tick, ~0.6 ms a tick at 29
bodies, which is why it is an ignored measurement and not part of the suite.

## Stability

- **It changes nothing.** An ignored test in a new file; no production path, no state, no hash.
- **Its limits are stated in the file**: it measures the world *before* collision exists, where stacking is unconstrained
  (that is why 3.66 M pair-ticks of 14.6 M bind at all). After E1a disperses the clusters the mix will move, so the
  1.6% is a price for the decision, not a constant - re-run it after E1a when the E1b swap is being priced.
- **It cannot answer "how often a step is refused"**, which is the number that would predict deadlocks: that needs the
  candidate position, not the current one. Named here so the next reader does not quote this as a deadlock estimate.
- The filter is explicit: units only, alive, not garrisoned - a building or a corpse in the pair set would make both
  predicates meaningless.

---

*Staged with the change for `tools/review_check.py`. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
