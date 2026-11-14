# OPEN - the *past* test and the stander set cannot fire, because `last_moved` is stamped on bodies that did not move

**Where it stands, 2026-11-14 late, branch `d208-time-not-geometry` (landing on `main`).** The collision story is closed:
D210 (a collision is settled by time and by the clock, never by geometry), D210b (a standing body is an obstacle to the
search that meets it), D211 (the search bends round a body inside a tile - bodies as discs with an octagon of shoulders),
D212 (a search that starts inside a body is re-seeded at `2 x clearance`, and only *away*). All four are in
`docs/decisions.md`; the mechanism and its measurements are in `docs/reviews/2026-11-14-d211-shoulders.md`, and the
narrative of how the guard got there is `docs/history/next-step-2026-11-14-the-guard.md`.

## The one open step

**`move_bodies` writes `entity.last_moved = now` for every delta it is handed, including the `(0,0)` delta of a body that
did not move.** Measured on a stopped villager: its `last_moved` advanced on every tick it stood still. Two rules read that
field, and both are therefore dead:

1. **The past test** (`World::is_past`, D210): `created >= moved` with `moved` always the current tick can only hold for a
   route planned *this* tick, so a collider is never "past". The head-on passes today through the pause and through rule 3
   ("a body already overlapped is not made worse by moving"), not through the rule D210 says ends an encounter.
2. **The stander set** (`stander_tiles`, `stander_bodies`, D210b): only a body that has **never** moved is a stander (`last_moved == 0` passes the filter); a body that moved and then stopped is stamped every tick, so it is never one. The only body a plan treats as an obstacle today is therefore one that has not moved since the world was seeded, or one a re-plan names explicitly.

**So the step is:** a red test that a body which did not move is a stander and a body that did is not (probe
`entity.last_moved` and `stander_bodies`), then the guard in `move_bodies`, then **the whole gate**, then **the timing that
D210b's first attempt measured at 2.3x** - switching the stander set on turns many plans from "straight line" into "graph
with eight more nodes per body", and that is a different engine. Do not land the guard without the number.

## What the step just finished cost, and how it was measured

- the last red guard, `group_overlap::a_mate_who_is_stopped_becomes_solid`, went **4207** raw (a route straight through the
  stopped man) -> **5012** (a re-seed that hopped over him) -> **7728 at the meeting only, 8664 afterwards** (an octagon
  round him, and he arrives). Green, restated to D210's outcome: at most one step of overlap *at* the meeting, nothing
  inside the reach on any later tick.
- two `right_of_way` fixtures were restated for the same reason: orders submitted in one tick are a **group** (D207) and a
  mate never pauses, so the pause fixture had to give each man his own tick; and the "jammed on one tile" fixture starts
  0.45 tiles apart and is inside the 0.40 reach by the second tick, where D52 governs.

## Known red on `main` and parked, not hidden

Two failures were found on `e0312f9` itself (the main checkout, real pack) while landing this step, and one of them is a
real defect:

- **A builder with a live route is asked for no step, for ever.** `build::a_content_pack_with_our_own_resources_loads_and_spends`
  is `#[ignore]`d with that sentence. Measured: the worker is ordered to build at `(3, 4)` on a **6x6** map, its route holds
  one waypoint at tile `(5, 4)`, it stands at `(5.03, 4.37)` - **0.47 tiles away** - and its step is **`Idle`** (nothing was
  asked of it) for 1,900 ticks, `repaths = 0`, `repath_failures = 0`. So the house never leaves its site state. This is the
  silent-stall class: a body with a waypoint in hand and a `Build` task that no rule asks anything of. Probe: `trace_steps()`
  plus `world.route_points(villager)` on that fixture.
- **A repath route round a body used to be asserted as "three waypoints".** `evasion::a_repath_route_is_pulled_like_a_planned_one`
  failed on `main` for a *different* reason - a stander placed before the order is planned around, so no repath ever
  happened (`repaths = 0`, the walker arrived). Restated here: the enemy is placed on the line **after** the route exists,
  and the claim is the shape (every waypoint between the ends sits on the body's clearance circle), not the count. A body is
  an octagon of shoulders, so the honest pulled route round one is five of its eight vertices, not two rectangle corners.

## Hazards a fresh session must not re-learn the hard way

- **A missing pack passes every world test silently.** `config/gamedata/` is gitignored, so a fresh worktree has none, and
  every fixture does `let Some(mut world) = common::fresh() else { return; }`. A whole file reports `ok` without running an
  assertion - measured here: `group_overlap` "passed" and then failed at once with the pack copied in. **Copy**
  `config/gamedata/*.toml` into the worktree; never symlink it (the pack-loader tests write into that directory).
- **One `CARGO_TARGET_DIR` per checkout** - two worktrees sharing one swap test binaries under a running suite.
- **A doc that promises a mechanism is not the mechanism.** "Eight shoulder points round every body" was in a review and in a
  code comment and did not exist; the two fixtures that were supposed to exercise it were satisfied by a whole-tile
  rectangle. When a review claims a mechanism, grep for it.
- **A `#[must_use]` left over a deleted function** lands on whatever is inserted next to it: `put_route` inherited one from
  an orphaned standoff doc block and `-D warnings` refused the tree. Read the diff around a structural edit.
