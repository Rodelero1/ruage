# D343 - step 8: the refined waypoint is the crossing the line makes, not the first from the low end

**Crosscheck first.** `FUN_007eeda0` refines a coarse chain into waypoints, and its **first act is an interpolation**.
Handed the mover's position (`param_6`, `param_7`) and a far point (`param_4`, `param_5`), for a boundary crossed along
`x` it computes

```c
iVar7 = (iVar4 * param_5) / param_4 + param_7;   // iVar4 = distance from the span's edge to the start
```

i.e. where the straight line itself crosses the edge - and only then does it look at a tile. The scan then runs **outward
from that tile**: forward to the span's end, keeping the first crossing and its distance `iVar10`; then backward from the
same point, breaking when `iVar10 < 1` (it has walked back as far as the forward crossing was ahead) or at a crossing,
which wins because it is strictly nearer. The vertical case is the same code on the other axis.

**Online.** The first-party deck (*25+ Years of Pathfinding Problems with C++*, Meeting C++ 2025) puts the three-solver
architecture and the legacy hull algorithm in its text layer, but its levelled-search slides are **images**: the extracted
text contains no "cell", "level" or "refinement" at all. So the interpolation has no independent corroboration - it is the
binary's, and the deck's silence is recorded as silence, not as agreement.

**Prior implementation.** `Levels::refine` walked each span from `first + 0` and took the first crossable tile; there was
no line, no interpolation and no outward scan.

**Landed.** `refine` takes the plan's two endpoints and interpolates the crossing per edge, then searches outward as the
binary does.

## The witness

`crates/sim/tests/refine_crossing.rs`: a level-2 plate whose shared edge (column `x = 4`, rows 0-3) is walled at rows 1
and 3, so the only crossings are rows 0 and 2; the walk is a straight line along row 2.

> **un-cut:** the waypoint came out at `(80000, 10000)` - the boundary at row **0**'s centre, the low end of the span.
> **cut:** `(80000, 50000)` - the boundary at row **2**'s centre, where the line crosses.

One variable, `crates/sim/src/levels.rs` and `ground.rs` checked out per file and restored.

An existing fixture had to be given the endpoints it never had: `coarse_route::the_chain_refines_to_the_crossing_of_each_shared_edge`
called `refine(&route, scale)`; it now walks a straight line along row 4. Its claims are unchanged and still pass.

## Performance

Same tree, one variable, three runs each side, back to back:

| | un-cut | D343 |
|---|---|---|
| acceptance hash | `1e1fcc3cf2b05a4c` | **`1e1fcc3cf2b05a4c`** |
| entities / players alive at 48,000 | 133 / 1 | 133 / 1 |
| re-paths | 36,567 | 36,567 |
| wall, median of three | 12.01 s (12.01 / 11.91 / 12.16) | 12.07 s (12.09 / 12.05 / 12.07) |

**Bit-identical, and that is the measurement**: the refinement only runs inside the levelled tier, and on this map the tier
answers **0 of 61,276** routes (D338), so no match can see this change either. The 0.06 s is the band - the same binary has
measured 11.4 s and 12.0 s on different batches when the box was busier. What the slice is for is the rule, and the rule's
number is the witness above.

## Stability

- Suite: **221 result lines, 939 passed, 0 failed, 23 ignored** (`cargo test --workspace --no-fail-fast`, exit line read).
  The extra line over D342's 220 is the new `refine_crossing` binary.
- The conformance set is in that run: `coarse_route`, `descent`, `levels`, `coarse_cost` and the tier fixtures all pass,
  including `the_chain_refines_to_the_crossing_of_each_shared_edge` after it was given its endpoints.
- Determinism: the hash is identical across three runs on both sides.
- **My own errors, in the same breath:** the first edit shadowed the new `from`/`to` parameters with the loop's cell pair
  (`let (from, to) = (pair[0], pair[1]);`), which would have silently interpolated from the *cells*; it was caught by
  reading the diff and the pair is now `(here, next)`. Two compile-failure rounds followed (the caller, then the fixture),
  which is why the witness is quoted from the pre-change signature and the final run is quoted separately.
- `cargo fmt --check` and `cargo clippy --all-targets -D warnings` clean.
