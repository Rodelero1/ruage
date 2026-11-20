# Review: the hierarchy witness now states the right observable (P11c, witness only)

*The `#[ignore]`d conformance row for P11 asserted `route.len() == 1` for a 58-tile open walk - a claim about the **flat** search it was
written against, not about the original. Checked against the binary before writing any coarse search, it is the wrong claim.*

## What the binary actually returns

`FUN_007ee980` walks the coarse route's parent chain (`[0x24]`) one node at a time and, per node, calls `FUN_007edb80` for that node's
tile, `FUN_007eeda0` to step from it toward the next, and `FUN_007fd880` to push each point it reaches. So the route handed to the
movement code is a **chain of cell centres at the level the search started at** - `n * 2^level + 0.5` tiles - not one straight
waypoint and not a per-tile path.

A 58-tile open walk therefore comes back as **several waypoints on the level-3 lattice** (8-tile spacing), because 11b's descent puts
such a walk at level 3.

## The rewrite

The fixture now asserts:

1. **more than one waypoint** for a 58-tile walk, and
2. **every waypoint except the last on the 8-tile lattice** `n * 8 + 0.5`.

Watched failing, with the measurement in the message:

```
a 58-tile walk came back as 1 waypoint(s): the flat search answers one, the levelled one answers the coarse chain
```

## Performance

- the fixture runs in **0.00 s** on a 96×96 open field, and it is still `#[ignore]`d - so the workspace suite is unchanged at
  **907 passed / 6 failed / 21 ignored** and this commit moves no number in it;
- `cargo test -p ruage-conformance --test divergences -- --ignored` now reports **1 passed / 1 failed**: this row fails with the message
  above and the body-radius row still passes.

## Stability

**A witness that asserts the wrong thing is worse than no witness**, because it makes the work look done when the shape is wrong. The
old claim would have been satisfied by any search that short-circuits a straight line, and the levelled search would *fail* it - which
is the trap this rewrite removes: the row now fails for the flat search and will pass only when the coarse chain exists.

Nothing in the workspace suite changed, so no existing route moved.
