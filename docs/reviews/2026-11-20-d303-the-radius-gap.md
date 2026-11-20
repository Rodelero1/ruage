# Review: where the radius really leaks - the witness for #7 (D303, step 1)

*Crosschecking #7 before building it turned up an important correction to my own register: **the line tier is already radius-aware.** The
gap is narrower than I said in `docs/pathfinding-crosscheck-2026-11-20.md`, and the witness now isolates it.*

## What the crosscheck found

- `plan_path_for` passes `reach = body_radius(entity.unit)` into `route_with`, and the **line tier** hands that to `leg_is_clear`, so a
  one-tile gap *is* correctly refused for a wide body when the straight line would fit through it. My register said "every route in this
  engine is a route a point could walk" - **too strong**: a route that is a straight line is radius-aware.
- The **searches are not**: the flat tile search asks `blocked(x, y)` and the levelled search asks the pyramid, and neither takes the
  reach. So the divergence bites exactly where the line tier fails and a search runs - which is most long walks, and all of them now that
  D302 puts the levelled search in that position.

## The witness, and the failure

`crates/sim/tests/radius_route.rs`: a wall down the map's middle with a **one-tile gap**, plus a stub that blocks the straight line so a
search must run. Watched failing, with the measurement:

```
a radius-0.50 body must NOT be planned through a gap it cannot fit:
its route is [(210000, 190000), (310000, 190000), (310000, 210000), (530000, 210000)]
```

The last point is the goal - so the packed trebuchet **is** planned through a gap it cannot walk. The villager passes the same fixture
correctly, which is what makes it a differential test rather than a claim about one body.

Parked `#[ignore]`d with that reason so the suite stays green and the gap stays visible; it un-ignores when the fix lands.

## Performance

- the fixture is **0.00 s** and the suite is unchanged: **918 passed / 6 failed / 21 parked** plus this one, red set untouched;
- the fix's cost is known from D301/D302 measurements: a radius-grown grid is one pass over the ground per radius class (0.20 / 0.25 /
  0.40 / 0.50 from the dat = four), i.e. ~4 × 21 µs at 64×65 **once per ground change**, and the pyramid then has to be keyed by radius as
  well as by the ground's generation.

## Stability

**A differential fixture is the right shape here**: one scene, two bodies, opposite expectations - it cannot pass by both bodies being
refused (the villager's route must reach the goal) or by both being allowed (the trebuchet's must not).

**The parked red is deliberate and named**, following the conformance register's convention: an `#[ignore]` with the measurement in the
reason, so `cargo test -- --ignored` prints the gap rather than hiding it.

**What this review corrects in the record**: the crosscheck's row #7 claimed the radius "lives in the step, not the planner". Half true -
it lives in the **line test** and in the **step**, and only the searches are blind. That is a narrower gap than the one I wrote down, and
narrower is better news: the fix is one grid, not a new movement model.
