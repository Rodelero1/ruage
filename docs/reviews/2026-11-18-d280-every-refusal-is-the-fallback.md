# Review: every refused step belongs to a tile-chain route - the fallback is the defect

*Change: the committed `DBG_ORIGIN` probe, run over animals + gather + gather_motion. It answers the register's step-1 question.*

## Performance

`DBG_ORIGIN=1 cargo test -p ruage-sim --no-fail-fast --test animals --test gather --test gather_motion -- --nocapture`, 30 s:

```
     22 kind=tile-chain
```

**22 of 22 ground refusals belong to a route whose first point is a tile point** - the tile-chain fallback - and **none** to a
geometry corner. Combined with the earlier count (29 of 29 were `Gather` tasks), the picture is exact: a gather approach whose
`short_range_clear` answer was declined falls back to a tile chain, and the disc step test refuses that chain's legs.

## Stability

**What this settles.** The register's "nothing extra" entry - *the tile-chain fallback, which the game does not have* - is not a
style observation: it is the mechanism behind every refusal in the movement suites. The original has no point-route fallback at
all; its rule is **"expand the search area once, otherwise give the closest point path"**, with a budget of `h` expansions capped
at 20,000.

**What it rules out.** The step test (the landings it refuses are legal given the route it was handed), the place radius (measured
neutral), the arrival measure (that is the animals red, and it is a different fixture), and the seat reservation (not a route).

**The next change is therefore one substitution, not a repair:** when `short_range_clear` declines, **widen it once** (window,
rectangle and node limits) and answer the closest point if it still fails - never hand back a tile chain. Fixture: the same gather
approach that refuses today, watched failing with `DBG_ORIGIN` reporting `tile-chain` and passing with `geometry`.

Known and accepted: the probe stays until that change lands, then goes.
