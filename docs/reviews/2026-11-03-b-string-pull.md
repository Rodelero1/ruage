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

## Addendum: C was built, measured, and reverted - it needs distance arrival in the same step

Built on top of B and taken out again, because the measurement said so rather than because it was hard.

**What was built.** `collide` now tests the body's **footprint** at the moved position - the sum of the two `radius[0]`
values, falling back to one tile when the content carries no radius, which is the original's rule (`Pos=(...) Radius=`) -
and the tile map now holds a **list** per tile (the map-of-one silently dropped the second body on a tile) of live
entity **indices**, so `blocker_at` reads positions as they are rather than as they were.

**Two defects the tests found, both real:**

- **Stale positions cost 500 raw units of interpenetration.** The tile map was built once per tick with copied positions;
  the first body to move inside that tick left a stale entry behind, and a pair walked 500 raw inside each other before
  the distance was checked properly. Positions are now read through the index.
- **Radius collision deadlocks a head-on pair.** Both villagers stopped dead on tile (4, 1) and stayed there: each refused
  the other's step, a one-tile row gives no sideways component to slide on, and the repath search cannot help because
  both already stand inside the tile it would avoid. Fixed with a **make-room** step - half the way plus a nudge to one
  side, terrain-checked - which opens the gap the next tick follows. Mutation: remove it and the crossing test reports
  `both have to arrive: a at (4, 1), b at (4, 1)`.

**And then the match collapsed**, which is why it did not land:

| | B (landed) | B + C |
|---|---|---|
| wall, 36,000 ticks | 9.97 s | **6.16 s** |
| entities | 159 | 147 |
| bank at 1,600 s | 729 wood / 792 food / 370 gold | **89 / 20 / 50** |
| rule firings | spread across the table | `900, 6, 7, 1, 9, 0, 0, 0, 0, 23, 0, ...` |

Faster and much worse: collision now stops a body **0.4 tiles short** of its target, while the arrival predicates that
let a villager gather, build or deposit are still **tile**-based. Nothing arrives, so nothing fires. C is the right rule
and it cannot land alone - it needs **distance-based arrival** beside it, and that is the next step, with this table as its
acceptance test: the bank at 1,600 s has to come back to at least B's.

**Mutation evidence kept from the run:** put the footprint back to the tile and the walker goes through a standing body at
**closest 500 raw where 8,000 is the footprint** - a sixteen-fold violation, caught.

