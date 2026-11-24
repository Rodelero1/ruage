# Review: the radius-aware search, attempted and reverted (D303 step 2)

*The witness is right, the fix as designed is not sufficient, and it is reverted rather than shipped red. Everything below is measured.*

## What was built

- `Ground::fits(x, y, scale, reach)` - a body of this reach centred on the tile: the nearest blocked tile's square, distance minus the
  reach, **strictly** greater than zero because the box ruler collides on `<=` (D286);
- `search_reporting` gained a `reach` and asks `fits` instead of `walkable`, for the node and for both diagonal side tiles;
- `Levels::for_radius(ground, reach)`, `Ground::pyramid_for(reach)`, the cache keyed by reach - four radii, four builds per ground change;
- `route_with` passes its `reach` to all three tiers, and the levelled tier stopped being gated on `reach == 0`.

## What it did

The witness `radius_route::the_gap_is_planned_for_a_villager_and_not_for_a_trebuchet` **passed** - the trebuchet is no longer planned
through a one-tile gap. Two fixtures it must not have touched went red:

```
straight_line::an_obstacle_across_the_line_forces_the_search   "and it goes round the tree"
straight_line::a_route_round_a_tree_carries_no_slack           "one blocked tile is one corner, not a staircase: the route holds 31 points"
```

**The cause is a lattice mismatch, not a bug in the cost.** The **line tier** certifies a leg by walking the tiles the leg crosses and asking
whether the body's box clears the objects in them; the **search** now refuses any tile whose *centre* is tangent to an obstruction. Round a
single tree those two disagree about the corner tiles: the line says a body fits past the tree, the search says it must step out and back,
and a 31-point staircase is the result. Adding a level gate (only answer from level 2 up) did not help, which is how the mismatch was
localised to the flat search rather than the hierarchy.

## Decision, and why it is the right one

**Reverted.** A red suite is not a state to leave behind, and the fix needs the two certifications to agree before it can land: either the
search must ask the same box question the line tier asks (`leg_is_clear`'s test, per node, not a centre-tangency test), or the line tier
must ask the search's question. That is a design choice about which is authoritative, and it is exactly the kind of decision the owner
should make rather than one I pick at the end of a session.

## Performance

- reverted, so the suite is back to **918 passed / 6 failed / 21 ignored** and the two `straight_line` fixtures are green;
- the witness is parked `#[ignore]`d with the whole story in its doc comment: **1 ignored** on top of the 21;
- the cost the fix would add is known and unchanged: one grid pass per radius class, ~21 µs at 64×65 per ground change (D301), and a `fits`
  test per expanded node (3×3 tiles) instead of one `walkable` lookup - measured at a few hundred ns per node in the probe's regime, so
  tens of microseconds per long route. Unmeasured precisely, because the change did not ship.

## Stability

**The value here is the isolation, not the code.** One attempt established: (a) the witness is real and fails for the right reason, (b) the
radius belongs in the search, (c) the *certification* is what breaks, and (d) the fixed-cost plumbing (per-radius pyramid, cache keying,
reach threading) is straightforward and was written once in an afternoon. What is left is one question with two answers, and a fixture pair
that already discriminates between them.
