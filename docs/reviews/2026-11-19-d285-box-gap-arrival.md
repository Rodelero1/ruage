# Review: "at the node" is the game's box gap, and four of our rules go (D285)

*The owner: **"remove/adjust all rules on pathfinding in this branch that are not aligned with original game and decomp."**
This is the arrival half of that: four of our own distance rules are deleted and replaced by the one function the game uses.*

## The rule, ported

`func_002690E0` (`compute_distance_sq`) subtracts **both bodies' sizes from each axis** before anything else and clamps at
zero, so the number is border to border and any overlap is exactly **0**. The motion layer compares it with the order's
`rangeDistance` - *"We're in range of our target, so we'll increment our waypoint"* - and a job with range 0 arrives when the
boxes **touch**. `World::box_gap_sq` is that function; `World::node_radius` gives a node its box (a resource or building is
one tile wide, an animal is its own body).

## Performance

- `acceptance_economy`: **passes** in **52.52 s** (55.25 s before this change, 39.58 s at D282, and a *failure* on the
  never-deeper branch). The crew grows.
- Nothing was added to the tick: the arrival test is four integer subtractions where it was a squared-distance compare plus
  four other predicates.

## Stability

**Deleted, because the game has one distance and these were four approximations of it:**

- `touching` and its `touch_tolerance`;
- `own_place` and its eighth of a tile (`scale / 20`);
- `ring_open` as an arrival input;
- `beside_adjacent`.

**Green after the change, and they are the ones that measure this:** `gather_touching` **3 of 3** -
`a_woodcutter_stands_with_his_circle_on_the_tree`, `ten_men_touch_one_bush_without_overlapping_it_or_each_other`,
`every_man_working_a_node_holds_a_place`. The owner's saves agree with the numbers the function produces: **0.70** from a tile
centre at an edge (`0.5 + 0.2`) and **0.935** at a corner (`0.707 + 0.2`).

**No new reds in the four gather files** (`gather_pick` 4/5, `gather_standoff_rule` 1/4, `gather_continuous` 0/2 - every red
there was red before this change and is on the rewrite list).

**Not done in this commit, and it is the other half of the owner's report:** the **step** still tests the landing *tile*
rather than the mover's radius-grown box, so a unit ordered to walk *beside* a building can still put its centre on the
border (`FUN_00840830` grows the query box by `+0x268` and collides on `<=`). That is the next commit on this branch; the
"click the middle, stop half a body short" half is what this one fixes.
