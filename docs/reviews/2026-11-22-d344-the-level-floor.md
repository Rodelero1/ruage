# 2026-11-22 - D344: the level floor moves to 1

The change: `route_with`'s tier gate `descent(start).min(descent(goal)) >= 2` becomes the floor the binary's own loop has -
nothing refused above level 0 - plus the measurement machinery that says whether the tier answers at all (`TierShadow`,
`TierFunnel`, `compare_tier_routes`), the `flat_route` extraction that lets both answers come from the same code, and the
descent re-read that stopped a wrong "correction" from inverting `Levels::descent`.

## Performance

**Acceptance match, seed 7, 48,000 ticks, release, median of three, back to back on an idle box** (`ruage-headless`, wall
clock around the binary):

| tree | wall | band | hash | entities / players | re-paths |
|---|---|---|---|---|---|
| floor 2 (un-cut) | 12.00 s | 11.90 / 11.90 / 12.09 | `1e1fcc3cf2b05a4c` | 133 / 1 | 36,567 |
| floor 1 (cut) | 12.16 s | 11.93 / 12.16 / 12.19 | `1e1fcc3cf2b05a4c` | 133 / 1 | 36,567 |
| floor 0 (priced neighbour) | 11.96 s | 11.85 / 11.96 / 12.14 | `1e1fcc3cf2b05a4c` | 133 / 1 | 36,567 |

The three bands overlap and every counter is identical, so the 0.16 s between the medians is load, not the change. **At no
floor does the match move** - which withdraws D338's 13/108-answer figures for this tree (D341 changed which routes exist;
D338 measured before it).

**The tier's own probe** (`cargo test --release -p ruage-sim --test tier_probe -- --ignored --nocapture`), 48,000 ticks:
`tier answers 0 of 49,088 routes; 36,567 re-paths; hash 1e1fcc3cf2b05a4c`, and the funnel:

- **91** long walks (16+ tiles and past the tier's arms at all), of which
- **60** never reached the search at floor 2 (0 at floor 1 - the change is exactly those 60 candidates being *asked*),
- **13** got no coarse route (`coarse_route` returns `None`: the goal's cell at the search level is occupied),
- **17** had a leg refused by a **body** in the plan's avoid list, **1** by object geometry, 0 by tiles,
- **0** answered, so `TierShadow` has no walks to compare - the tier-vs-flat judgement is *not* made by this slice, and it is
  the next slices that make it possible.

The gate's cost when it refuses is unchanged (one `descent` call per long walk). `compare_tier_routes` is off in production:
one `Cell` read per tier answer, and the shadow's second search only when a probe asks for it.

## Stability

Attacked, and what each attack found:

- **The floor at 0 and at 2**, by edit on the same tree, three acceptance runs each: identical hash, identical counters, wall
  inside the band. The change cannot move the match through the floor alone.
- **`flat_route` extraction**: it is the tail of `route_with` moved, not rewritten - the suite's 940 passed / 0 failed
  includes every route-shape and straight-line fixture, and the acceptance hash is identical to D343's.
- **The funnel is off**: `watched` is read once per long walk, and every counter is `Cell`, never hashed. The suite's
  determinism fixtures (`no_float`, `state`, `reseed`) pass unchanged.
- **A goal in an occupied cell** still falls through to the flat search - the 13 no-coarse walks are the evidence, not an
  argument; nothing panics or loops.
- **An open map** still descends to 3 and answers from the hierarchy at level 3 (`descent.rs`'s three rows unchanged;
  `tier_ignores_bodies` still green), so the floor change did not move the level choice for a start that already passed.
- **The pyramid cache** is still keyed by reach; the floor change reads `descent` on the same pyramid the search uses, so no
  new build path was added.
- **A refused chain leaves no state**: the arm returns nothing and the flat search answers, as before.

**My errors, in the same breath:**

1. The first fixture put the wood resource on a grass tile; the map loader refused it (`"wood" at (11, 43) is not on a
   forest tile`). Fixed by making the tile forest floor - a forest's floor is walkable and the tree is the obstacle.
2. The fixture's descent premise read `3` instead of `1` because `Ground::from_map` carries terrain only: resources are
   stamped by the world's seeding. The premise now stamps every resource tile, the way seeding does.
3. The first cut of `flat_route`/`record_shadow` had eight arguments each and `clippy -D warnings` refused them; bundled into
   `Walk`, which is what they always were.
4. **D338's numbers**, quoted in this tree for a session as the floor's price, do not reproduce after D341 - the row now says
   so instead of leaving them standing.

Not attacked: the two gates that actually refuse the tier (the occupied goal cell, the body certification). They are named
above with their counts and are the next slices.
