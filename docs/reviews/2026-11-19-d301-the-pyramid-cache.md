# Review: the pyramid cache, and the comparison that was wrong (D301)

*Two findings, one of them a correction of D300's conclusion. All numbers are **release**, `crates/sim/tests/levels_cost.rs -- --nocapture`,
200 runs.*

## 1. The cache works, and D300's premise was wrong

D300 said a cache "would never hit" because `Ground::block`/`unblock` fire on every unit step. **Read against the code, that is false**:
`occupants` is touched only by **objects** - a tree seeded or felled, a building laid or razed, a gate opened - and there are exactly
**eight** call sites, every one of them an object event. A unit's body is not in `occupants` at all; the step pass handles it through
`objects_touched`. So the pyramid is warm across every tick a unit walks.

Landed: `Ground::pyramid()` (built on demand, `Rc`-shared, `#[serde(skip)]`), `Ground::coarse_route`, `Ground::pyramid_builds()` as a
diagnostic, and invalidation inside `block`/`unblock` - the only two mutators. Measured: **201 routes, 1 build**.

## 2. The comparison in D300 measured the wrong case

| walk | coarse | flat |
| --- | --- | --- |
| open ground, straight line available | 4.8 µs | **477 ns** |
| **a wall across the path, so both must search** | **4.6 µs** | **28.9 µs** |

On open ground the flat path never searches - `line_clear` answers with one waypoint in 477 ns - so D300's "~52× slower" compared the
hierarchy against a **line test**, not against a search. Where both actually search, the hierarchy is **6.3× faster** (4.6 vs 28.9 µs),
which is the whole reason the original has one.

**So D300's conclusion is withdrawn.** The correct shape for 11d is the one the binary and this engine together imply: **straight line
first** (477 ns, the cheapest answer there is), then **the levelled search** when the line is blocked, then the flat tile search as the
fallback for what the hierarchy cannot express.

## Performance

- pyramid build **21.1 µs** (once per ground change, not per search), descent **11 ns**, coarse search **5.1 µs** for a 56-tile
  diagonal (15 cells), cached route **4.8 µs**;
- the flat tile search on a walled walk: **28.9 µs**;
- the cache costs one `RefCell` borrow per call (~2 ns uncontended) and one `Rc` clone (~1 ns);
- `crates/sim/tests/pyramid_cache.rs`: **2 passed / 0.00 s**; probe: **1 passed / 0.14 s** debug.

## Stability

**A cache keyed to the wrong mutators is a correctness bug that shows up as a route through a wall**, so the fixture pins both sides: two
searches on unchanged ground rebuild **zero** times, and a single `block` forces exactly one rebuild *and* changes the route (the fixture
asserts the route no longer enters the newly blocked cell). `unblock` forces another.

**The invalidation is inside the mutators, not at the call sites**, so a new caller cannot forget it - which is the failure mode a
"remember to invalidate" rule always has.

**What is still not measured**: behaviour at 480×480 (pyramid ≈ 56× the area, ≈ 1.2 ms once per ground change, which is acceptable but
unmeasured), and a hundred units planning in one tick against the flat search's own cost in the same scene.
