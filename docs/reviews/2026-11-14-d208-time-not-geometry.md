# D208 - the head-on is settled by time; the search cannot yet carry it

**Verdict: parked.** The rule is right and reproduces the binary, and measured against `main` it is **2.3x slower** and
still ghosts through a body that stood next to the walker, because our search works on tile corners. Nothing here is
merged; `main` stays at D207.

## What the binary says, and what was built

Three reads (2026-11-14) settled the collision rule completely, and this branch implements all of it:

1. **No step is clamped against a body.** The movement site calls the radius retraction `0x8aa990` and the very next
   instruction (`0x8a7fe7`) tests a debug flag - the result is never read, and the position setter `0x8ad300` clamps
   against the **map border** only. `bound_to_reach` has no counterpart in the game and is no longer called from `collide`.
2. **The encounter is resolved by time.** The loser of the id comparison is paused 0.25-1.50 s; the winner re-plans once;
   and a collider that has **not** moved since the route was planned is *past* and dropped
   (`path->createdAt >= other->lastMoveTime`), so the winner walks through. `Route.created`, `Entity::last_moved`,
   one `put_route` choke point and `World::is_past`.
3. **A standing body is an obstacle to the search that meets it** (`FUN_007f9d80`: insert into the per-search rectangle
   list iff not ignored **and velocity is zero**; the list is rebuilt per search and never written into the shared grid).
   `stander_tiles` returns the tiles of units that did not move on the last tick, and both `plan_path` and the re-plan pass
   that set to `short_range` and `find_path_avoiding`.

Green: `crates/sim/tests/head_on.rs` **2/2** - two men meeting head-on now pass each other, and the loser's first stand is
the drawn 0.25-1.50 s rather than a freeze.

## What is red, and why each failure is a different thing

| test | measured | verdict |
|---|---|---|
| `group_overlap::two_units_ordered_apart_stay_solid` | closest **151 raw** against an **8000** reach | **the test is obsolete**, not the code: D208's rule is that two men who meet *do* pass. It must be rewritten to D208's outcome. |
| `group_overlap::a_mate_who_is_stopped_becomes_solid` | closest **4207 raw** | **a real gap**: after a stop the man is solid again, and the walker should route round him - but he is **adjacent**, and a tile-corner graph has no detour that does not begin inside his tile. The fallback straight line ghosts through him. |
| `footprint::a_head_on_pair_in_a_one_tile_corridor_never_overlaps` | 7346 raw on tick 66 | the same gap at its sharpest: a one-tile corridor has no tile-level detour at all. |

## Performance - the number that decides this

Interleaved A/B on the acceptance world, same box, alternating (`build/headless-main` against `build/headless-d208`):

| run | `main` | D208 |
|---|---|---|
| round 1 | 31,001 ms | 68,084 ms |
| round 2 | 30,491 ms | 73,319 ms |

**2.3x slower.** The cause is structural, not a hot loop: with standing bodies in the obstacle set, `short_range`'s
visibility graph fails far more often, and every failure runs the **fallback** `find_path_avoiding` - a second search over
the whole route. Accepting every plan's cost twice is what the 2.3x is.

Acceptance summary also moves: `entities=158 players=2 hash=c2ed74016ee533ea repaths=508 made_room=0 repaths_body=174
repaths_ground=334`, against `main`'s `entities=160 hash=c3b10f4b09e17a43 repaths=324 made_room=147 repaths_body=83
repaths_ground=241` - **+57% repaths**, and `made_room` collapses to zero.

## Stability

- **Deterministic:** `created` and `last_moved` are both hashed (written only when non-zero), `put_route` is the single
  choke point that stamps a route, and the stander set is derived from state, never accumulated.
- **No float, no allocator churn in the movement pass**: the stander set is built where a plan is built, not per body.
- **Mutual and time-bounded:** the mate test (D207) is mutual; the pause is a per-entity tick count; a paused body spends
  no waypoint.
- **One real hazard, not a bug:** `put_route` stamps `created` on **every** re-plan, and a fresh stamp makes every collider
  that has not moved since it "past". That is the game's semantics, but it means a body that repaths every tick is
  permanently "past" everything around it. In our match that is the difference between a pause and a ghost, and it is the
  reason the stopped-mate guard fails rather than passing quietly.

## What would make this land

The search, not the movement rules. The original works on **continuous rectangles** - each body a box of `pos +- radius`,
grown by the mover's radius - so it threads gaps a tile-corner graph cannot describe. Three slices have now failed at this
same wall from the movement side. The next attempt should be `short.rs`: nodes offset from a body's disc rather than tile
corners, with the stander set as its input.
