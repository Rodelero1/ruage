# Review: the conformance row was pinning the shape D322 replaced (D326)

*Change: `crates/conformance/tests/divergences.rs`, one row's assertion - from "every waypoint is a level-3 cell centre" to "every waypoint
after the first is a crossing of a level-3 boundary". Found by running the conformance gate, not by a red in the main suite.*

## How it was missed

D322 changed `Levels::refine` from cell centres to edge crossings. I verified the main suite (930 / 0 / 19) and checked the conformance
gate by **counting its `test result` lines** - nine, as on every run before - instead of reading pass/fail. It was **10 passed / 1 failed**,
and the failure had been sitting there since the D322 commit.

The row's RULE paragraph already cited `FUN_007eeda0` ("to step toward the next") as the function that decides where a waypoint lands, so
the **text** described the crossing and the **assertion** pinned the centre. The assertion was the stale half, which is why the fix is an
assertion change and not a rule change.

## The fix

```
for point in &points[1..] {                      // was: &points[..len - 1], on the n*8+4 lattice
    assert!((on_boundary(x) && at_middle(y)) || (at_middle(x) && on_boundary(y)));
}
```

The cell-not-blocked check is unchanged, and the first point is still the start cell's centre (the caller discards it).

## Performance

- No engine code changed; the row is a test. **Conformance: 21 passed / 0 failed / 0 ignored** against 10 / 1 / 0. The main suite is
  unchanged at **931 / 0 / 19** and the acceptance match at `c7191f531bd81b1b`.
- The conformance set runs in 0.02 s, so the cost of running it on every step is nothing; the cost of *not* running it was a gate that
  reported green while one of its rows was red.

## Stability

- The assertion is strictly stronger in one sense (it now constrains *both* coordinates of every point rather than one lattice) and
  weaker in none: a crossing is still required to be a cell the search was allowed to enter, which the cell loop below still checks.
- **The process lesson, which is the real content here**: `crates/conformance` is a separate workspace and is **not** part of the
  pre-commit gate by design (`make conformance` is run by hand, and `make conformance-build` only checks it compiles). A shape change in
  `Levels`, `refine` or `coarse_route` can therefore land with the main suite green. The rule recorded in D326: run `make conformance` and
  **read the totals** after any change in that area.
