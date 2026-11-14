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

## Register rows still open (kept from `main`'s plan, D208 and D207)

- **An explicit group id on the command.** Until it lands, "same tick" is not "same selection": a caller issuing eight
  separate moves in one tick gets group behaviour it did not ask for, and a selection ordered across several ticks gets
  none. When it lands, `mates` is filled from the command rather than from `ordered_now`, and the three tests in
  `crates/sim/tests/group_overlap.rs` keep their names.
- **The three search fixes extracted from `d205-pause`, and D205's pause on top of them** - both rows are **closed by this
  branch**: the pause is D210, the re-plan's knowledge of bodies and the "past" exemption are D210/D210b, and the fast path
  that answered "walk through him" is D211. What they needed first was a search that can route round a body, which is what
  D211 built.
- `docs/dependency-tree.md` is the map: read it (and re-run its probes) before choosing a slice that is not on this file.

## Regressions this branch carries, parked by name

The collision slices before this one left failures that **pass on `main` (`e0312f9`)** - measured there, and measured again
with D211's search change removed, so they are neither `main`'s nor this slice's. Two are left, both in the *arrival*
family rather than the collision one, and both are `#[ignore]`d with the sentence below in the attribute:

- **`rally::a_unit_rallied_to_its_own_building_stays_inside`** — a villager trained at a town centre whose rally point is the
  town centre's own tile never reaches `Task::Garrisoned` (measured `inside = 0` after 600 ticks). Probe: the trained unit's
  `task` and `position` each tick; it is walking, not standing, so this is the *arrival predicate* or the goal-tile branch of
  `collide` (rule 2), not the pause.
- **`no_pushing::a_man_who_cannot_get_his_square_takes_the_next_tree`** — a villager sent to a second tree because his first
  square was refused ends in `Task::Return { node: 12, drop: 1 }` at `(11.378, 6.947)` and never arrives anywhere for 2,000
  ticks. Probe: the same, plus `world.route_points(id)` - he holds a route, so the question is why he stops following it.
- **`build::a_content_pack_with_our_own_resources_loads_and_spends`** fails on `main` too: a builder with a live route is
  asked for **no step at all** (`step = Idle`) for 1,900 ticks, 0.47 tiles from its own waypoint, `repaths = 0`. Parked with
  the same sentence in its attribute.

## A deadlock found and fixed while landing this (D210 corrected)

The tie-break used to pause **the loser itself** and return, so the loser never asked for a route. Against a man who has no
order and never moves that is a freeze for ever: measured on `vision::an_enemy_is_only_visible_while_somebody_watches`, a
scout stood `Paused` at `(614150, 614150)` from tick 50 to tick 550. The original only ever calls `setPause` on **the other**
unit (`cmp eax,esi / jle`), and the mover that loses the comparison goes on to the rest of the collision block - which is
where its own answer lives. Fixed test-first
(`right_of_way::a_loser_does_not_stand_for_ever_against_a_man_who_never_moves`, red with the walker parked on the stander's
tile for 2,000 ticks), and it closed `vision` and one of the two `rally` fixtures on its own.

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
