# Review: a route bends where the ground does

*Change (M2, D113): the route a unit walks is no longer a chain of tile origins. `crates/sim/src/short.rs` is the
short-range pathfinder the original's own maintainers documented - the body shrunk to a point, every obstruction expanded
by its radius, a ray at the goal, and a walk over the corners of the merged rectangles, here as a visibility graph. `Route`
now carries **points** (`Route.steps: Vec<(u16,u16)>` became `Route.points: Vec<(Fixed,Fixed)>`), which is a change at
eight call sites rather than a new movement rule: M1 already made the step walk a segment, so the mover needed no change.
Bounded (window, rectangles, nodes) and it **declines** rather than guessing, falling back to the pulled tile chain at tile
centres.*

The bug it closes, measured rather than asserted: a villager rounding a house **passed through the corner of its footprint**
- distance **1 raw unit** from the wall with a body of 4,000. That is `a_walk_past_a_house_keeps_its_bodys_clearance`,
written to sample every tick, and it reads 1 when the clearance is mutated away and the body's own radius when it is not.

## Performance

`short_probe` (ignored measurement): **40.2 us a solve**, 7.0 corners on average, over a window holding eight buildings and a
wood - the shape a route in a base meets. A match makes a few hundred plans and repaths, so the solver is milliseconds over
36,000 ticks; it is called **once per route, never per tick**.

The acceptance match, 36,000 ticks, seed 7, three runs each, all identical within a variant:

| | wall clock | entities | repaths | made_room | outcome | hash |
|---|---|---|---|---|---|---|
| M1 (`fbfe8c6`) | 8.73-8.91 s | 126 | 194 | 145 | `players=1` | `098fd53e146f0911` |
| this tree | 10.54-10.65 s | **148** | 124 | 171 | `players=1` | `623c3ff8c769eadd` |
| ablation: geometry forced off | 12.42-12.95 s | 156 | 141 | 211 | `players=1` | `f2dff2d47936f6ca` |

**Read that table with care, because it cannot answer "what did the geometry cost".** Each row played a *different match* -
different hash, different base - so the wall-clock difference is mostly the match doing more, not the solver doing work.
The ablation is the interesting one and it points the other way: with the geometry forced off, the *same tree* played a
larger, slower match, which says the tile-centre fallback is what costs time (more collisions, more shoving: 141 repaths and
211 nudges against 124 and 171), not the visibility graph. The number that is a cost measurement is the 40 us solve.

## Stability

| mutation | result |
|---|---|
| pass radius `0` instead of the body's | `a_walk_past_a_house_keeps_its_bodys_clearance` fails: **1 raw unit** from the house, body 4,000 |
| drop the one-raw-unit radius floor | `an_unreachable_site_is_refused_before_the_wood_is_spent` fails: a sealed site becomes reachable *through the pinch between two diagonally touching trees* - verified by reverting only this line |
| drop the window fence | `a_wall_across_the_world_is_no_path` fails: the path walks round the cut end of a wall that continues outside the window |
| keep the start in the returned polyline | `a_gather_order_does_not_teleport_anyone` fails (a tick of standing still) and `an_arrow_misses_a_runner_unless_ballistics_leads_the_shot` fails (a lead-shot reading zero motion) |

Defects this pass found in its own work, in the order they were found - the first integration attempt failed
**eleven** tests across gather, build, dropoff, pathing, rally and ballistics, and each had its own cause:

1. **The start was a waypoint.** Most visible symptom: every runner's predicted motion was zero, so Ballistics stopped
   leading. Fixed by returning the points *after* the start.
2. **A zero-radius pinch.** With no clearance, two blocked tiles that touch at a corner leave a zero-width gap the line can
   pass exactly through - which the tile rule forbids. One raw unit of expansion closes it; that is what the sealed-site test
   was telling me.
3. **An unfenced window.** A wall that runs out of the search window is not a wall that ends there, and the graph happily
   walked round the cut end. Four thin boxes make leaving the window impossible.
4. **Corners outside the map.** A wall at the map's edge has expanded corners beyond it; keeping them let a route run off the
   world.
5. **`q < 0` where `q == 0` was meant.** A segment running *along* an expanded face is the tightest legal line, and the first
   version refused it - which made the graph unwalkable round a plain wall ("no path" for a three-tile block).

**My own process error:** I ran `cargo test -p ruage-view | grep FAILED` after adding a closure, read the silence as green,
and the crate had not compiled at all (a lifetime error). The same mistake as the previous slice's - a pipe hides a compile
error because the error does not contain `FAILED`.

## Known and accepted

- **The fallback is not the geometry.** A goal whose window is over 40 tiles a side, or a window with more than 96
  rectangles, gets the pulled tile chain with its corners at tile centres - straight lines, but a half-tile of clearance at
  best. Named rather than hidden: a long strategic walk is the case that keeps the cheap answer.
- **Bodies are not obstacles in the geometry unless the caller passes the avoid set.** The repath does; a first plan does not,
  so a route may be planned through where a unit is standing - `collide` still handles it, and that is unchanged.
- **Determinism** is by construction (sorted nodes, index-ordered search, integer arithmetic) and `the_path_is_the_same_every_time`
  pins it, but there is no four-hour soak behind this slice; the acceptance hash is stable across three runs.
- Two comments in `lib.rs` still describe waypoints as tile centres (`pull_route`'s docblock); the code is right, the prose
  is one slice behind, and this is the note that says so rather than the reader finding it.

---

*This file is what `tools/review_check.py` looks for: `## Performance` with a number, `## Stability` with content, staged
with the change. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
