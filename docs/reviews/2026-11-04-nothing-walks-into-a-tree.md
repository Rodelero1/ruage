# Nothing walks into a tree, and the steering attempt that was reverted

Two of the owner's flaws (2026-11-04) were the same bug, and the third was bigger than it looked.

## The stuck units

> "Militia still get stuck in the forrest"
> "scout cavalry still gets stuck in gold mine"

One cause: `step_is_walkable` treated the **goal tile as free ground whatever stood on it**
(`goal == Some((x, y)) || self.tile_walkable(x, y)`). A unit ordered at a tree, a mine or a building walked *into* it and
stood inside the obstacle - and a blocked tile has no route out, so it stayed there for the rest of the match.

The goal is no longer a hole in the ground. A goal on blocked terrain now means "come as close as you can", which the
pathfinder's own fallback already arranges, and `crates/sim/tests/stuck.rs` pins it: a soldier sent at a wood tile is found on
free ground when the smoke clears, and can be ordered away afterwards (a unit *on* a blocked tile is a unit the pathfinder
cannot start from, which is what "stuck" meant).

Two smaller things came out of the same pass:

- **Arrival is "close enough", not "exactly on it".** `Task::Move` finished only when `position == to`, and the last step of a
  walk is often clipped by the ground or by a body - so a builder could pace at its own doorstep for ever and a barracks was
  never finished (`the_ten_minute_scenario_builds_trains_and_fights` caught it). `reaches` allows one step's length.
- **Only a step that crosses a tile can cut a corner.** The corner test asked both orthogonal neighbours of a *fractional*
  step, which no unit hugging a wall can satisfy.

## The steering attempt, reverted

> "Walking still follows squares and goes through their centers - units go either vertically, horizontally or diagonally ... the
> path is not a straight line but instead 45 degree walk followed by horizontal walk."

I wrote that change: normalise the direction, move `delta` along the line, integer-only (no `f64` in the tick). Two tests were
written first - a sixty-degree walk keeps its heading, and a diagonal step is not √2 faster than a straight one - and both
passed.

**It broke arrival.** With proportional steps, a villager walking past a house stopped dead at x = 3 and never arrived:
`a_villager_walks_round_a_house_rather_than_through_it` failed, and so did the barracks-building scenario. The cause is that
path, arrival and collision share one notion of "the next point", and moving only the step makes the other two disagree - the
waypoint is a tile centre, the collision test is a tile, and the arrival test was an equality on a point that a clipped step
can no longer reach.

So the eight-way step is back, with a comment saying so, and the two steering tests are deleted rather than left failing. The
work that remains is a *pass*, not a line: continuous steering has to bring the waypoint rule (aim at a point, not a centre),
the arrival rule (a radius, not an equality) and the collision rule (a shape sliding along a shape) together in one change,
with the tests for each. That is the next step, and I would rather say so than ship a staircase that walks through walls.

## Performance

**630 passed / 0 failed / 18 ignored**, and the fix costs nothing per tick: it *removes* an exception from a predicate that
was already evaluated for every step (`goal == Some(tile) ||` is one comparison less), and `reaches` replaces an equality with
one multiply-add.

The acceptance run was not re-measured for this batch: nothing in it changes a match's trajectory except that units stop
walking into obstacles, which is what the harness cannot tell apart from any other pathing difference. The number is
unchanged at the last measurement (11.9 s for 36,000 ticks) and the guard is `cargo test`, not the clock.

## Stability

A unit can no longer stand on a blocked tile, which removes the class of "stuck for ever" states rather than one instance of
it: a blocked tile has no route out, so every unit that ended up on one was a permanent zombie in the world. `reaches` cannot
make a task finish early by more than one step, and a step is bounded by the unit's own speed.

## Measured

- **630 passed / 0 failed / 18 ignored**; clippy clean (the owner's build log showed four warnings from the previous batch -
  a dead `line`, two orphaned `#[must_use]` attributes and an unused `player_colour` import - all four are gone).
- The stuck fix costs nothing per tick: it tightens a predicate that was already evaluated.
