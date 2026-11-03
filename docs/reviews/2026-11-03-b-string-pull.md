# Review: B - a route carries no slack (and C priced, not yet built)

*Change: `plan_path` now **string-pulls** what the pather returns. From each node the route jumps to the furthest node
`line_clear` reaches, so the corners that remain are the ones an obstacle forces. Runs when a route is planned, never per
tick. Also adds `crates/sim/tests/movement_worth.rs`, the two ignored probes that priced B and C before either was
written.*

## Performance

**What B buys, measured over 200 routes on the project map** (deterministic xorshift seed, greedy pull, same `line_clear`
the engine uses):

- pather polyline: **7,099.2 tiles**; string-pulled: **6,748.6 tiles** → **4.94% less walking**, worst single route
  **8.2%**.

That is the number the slice rests on, and it contradicts my earlier claim that B was worth ~zero: that claim came from one
hand-made case (a straight row, where a route already *is* the line), not from a sample. A sample of 200 says the pather
leaves ~5% of walking distance on the table because A\* minimises 10/14 steps, not the length of the line the unit walks.

**Cost:** one `line_clear` per pair examined, at plan time only, over routes of at most a few dozen nodes. The tick pays
nothing - the route is shorter, so the mover does strictly less work.

**And it compounds in a match.** The acceptance run at 36,000 ticks: **9.97 s** against 12.63 s, hash
`5767526d89d11c35`, `players=2` still - and the economy is not the same game by 1,600 s: **729 wood / 792 food / 370
gold** against **79 / 398 / 110**. Five per cent less walking per trip is a fifth of the villager-ticks spent crossing
ground, and the bank says so. The victory is still not taken (22 buildings at 1,200 s against 27 before), so the gap that
remains is not throughput alone.

**C, priced and not built here:** on the acceptance match, **330,710 touching pair-ticks**, of which **212,620 (64.3%) are
on *different* tiles** - the overlap a tile predicate cannot see - and **3,512 same-tile pair-ticks that do not touch**,
which the tile predicate refuses anyway. The honest reading of the first number is smaller than it looks: with
`radius[0] = 0.2` per body the reach is 0.4 tiles, and the **deepest** penetration measured is **0.004 tiles**. So C is
not worth building for the interpenetration; it is worth building as the foundation for **distance-based arrival**, which
is what the two parked fixtures (eject, retarget) actually need.

## Stability

- **The mutation bites**: removing the pull (`Ok(steps)`) fails `a_route_round_a_tree_carries_no_slack` with *"the stored
  route must be shorter than the pather's own polyline: 6 against 6"*. The test reproduces the engine's obstacle exactly
  in a local `Walk` (`Walk::block` is public), so the comparison is against the same world the plan was made in.
- **The pull cannot route through a wall**: it uses `line_clear`, which applies the same both-corners rule as the search,
  so a chord that clips a corner is rejected and the corner is kept.
- **A pulled route is a subsequence of the pather's**, in the same order, ending on the same goal tile - so everything
  downstream (the exact `end` point, the spent-waypoint test, the collision repath) sees the same shape it did.
- **Determinism**: the greedy choice is a function of the route and the grid, both deterministic; no sets, no hashes.

Known and accepted: pulling is done in **tile** space, so a route's chords are tile-legal but not collision-tested against
bodies - a body on the chord is handled by the movement pass, which repaths (E1a) exactly as it would on an unpulled
route. The sample is 200 routes with a fixed seed; a different seed would move the 4.94% by a little, not by much, and the
probe is kept so anyone can re-run it.

---

*Staged with the change for `tools/review_check.py`. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
