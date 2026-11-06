# Every click is accepted (D134)

*2026-11-06. The owner, on a scout wedged between a mine and a wood: "clicking into the middle of the forest ends up with
error message. It has to be allowed. Scout cavalry will simply try to close to the point as close as it can without running
into trees." His answers to the nine questions are in D134.*

## What was actually wrong

Two things, and the first was in the planner's own comments:

| | what the code did |
|---|---|
| a click on **blocked** ground | the tile search admits a blocked *goal* only if a **free neighbour** can step onto it: an outer tree works, the middle of a wood has no free neighbour, the search returns `None`, and the order is refused with `NoPath` ("the target is walled off") |
| a click on **unreachable** ground | same refusal - a courtyard behind a wall is a legal tile nobody can reach |
| the refusal's moment | **at `submit`**, in `validate` - so the player sees the error before anything moves |

Both `plan_path` and `step_is_walkable` carry comments saying a blocked goal means "come as close as you can". Nothing
implemented that; the comments described an intention, and the tests never asked.

**The failing run, as written** (`crates/sim/tests/click_into_the_woods.rs`, first attempt):

```
a click in the middle of a wood is a move order, not an error:
    Some(NoPath { from: (1, 4), to: (4, 4) })
```

## The fix

1. **`nearest_reachable`** (`path.rs`): best-first over free tiles, ordered by the distance to the click **alone**, so the
   first tile off the queue is the reachable tile closest to it. Ties break by `(y, x)`, never by iteration order. Bounded by
   the map, and run **once per order** rather than per tick.
2. **`World::walk_goal`**: a walk order's goal becomes that tile **when the order is given**, and it returns the goal it
   planned for. `check_order` (validation) and `start_walk` (apply) both go through it, so the stored goal is always a tile a
   body can occupy - which is what stops the mover from re-planning at the clicked tree every tick afterwards.
3. **Walk orders only.** `Move`, `AttackMove`, `Patrol`, `WalkTo`. A **gather**, **build** or **attack** order names a
   *thing*, and "unreachable" is the answer those must keep.

### The two tests that forced point 3

The first attempt put the fallback in the shared planner, and the suite caught it in one run:

| test | what it said |
|---|---|
| `a_node_with_no_path_is_refused_at_order_time` | a gather order on a node behind a wall must be **refused** - asking for a tree you cannot reach is not "walk to the fence" (item 9's rule) |
| `an_unreachable_site_is_refused_before_the_wood_is_spent` | a build order the builder cannot reach must be refused **before** the wood is spent |

Both pass unchanged with the snap confined to walk orders.

### One old test had to change, and it is a rule change, not a test fix

`walls.rs::a_wall_of_walls_closes_the_way` asserted that a Move into a closed ring of **walls** is refused. That is the same
shape as the wood, and D134 says the click is accepted - so the test now asserts what it is actually about: the unit walks
up to the ring and **stays outside** it. Recorded as a rule change in the test's comment, not quietly relaxed.

### The bug I wrote first

The first version of `nearest_reachable` returned the tile the body already stood on - it is free by definition - so every
order resolved to "stay where you are". The test failed with *"it never left its spawn tile"*; the search now excludes its
own start unless nothing else is reachable.

## The tests

| test | before | after |
|---|---|---|
| a click in the middle of a wood | **FAILED** - `NoPath { from: (1, 4), to: (4, 4) }` | passes: accepted, walks to the wood's edge |
| a click inside a walled court (free ground, unreachable) | refused | passes: accepted, walks to the wall |
| a click on the tile it stands on | - | passes: accepted, stays on its tile |
| a wall of walls closes the way | asserted a `NoPath` refusal | passes: accepted, and the ring is not crossed |

## Performance

**The rule works, and the 17x tick it exposed is D135** - found and fixed before this merged, which is why the branch
`d134-click-accepted` is now `main`:

| | `main` before (D132/D133) | this change, after D135 |
|---|---|---|
| `"the target is walled off"` refusals, 36,000 ticks | **203** | **0** |
| refusals of every kind | 777 | **574** |
| acceptance hash / entities / repaths | `394c4892aac776d2` / 141 / 184 | **`f328ebc8d4d34b78`** / 141 / **188** |
| wall time, release, three runs | 11,199 ms | **11,828 ms** median (11,719 / 12,055 / 11,828) |

The click itself costs nothing measurable: the fallback search ran **twice** in the whole match (16 node expansions), and
the plan count is unchanged. What the click changes is **where the match goes**, and the trajectory it walks into turned out
to be a state where a **failed** ground re-plan cost one whole search **every tick** - `2,559` attempts and `2,533`
failures by tick 12,000, `3.2 ms` each. D127's two-per-route budget counted successes only, so ground that answered "no"
was asked again forever, and a unit with no route entry had no counter to fail to increment.

| | D134 alone | D134 + D135 |
|---|---|---|
| simulation time, 36,000 ticks | **28.9 s** | **~1.7 s** |
| whole match, release | **47,496 ms** | **11,828 ms** |
| state hash | `f328ebc8d4d34b78` | **`f328ebc8d4d34b78`** |

**The hash does not move between D134-alone and D134+D135**: the searches D135 removed decided nothing. The behaviour is
D134's; the cost is the old one.

## Stability

| attack | result |
|---|---|
| a **free but unreachable** tile (a courtyard) | accepted, walks to the wall, does not enter (test) |
| a target **outside the map** | still `OffMap` at validation: a click is a tile on the map, and this is not one |
| the tile a body already stands on | accepted, plan is empty, the body stays on its tile (test) |
| a body standing on **blocked** ground (a villager on its tree) | the search skips its own blocked start and still finds a way out |
| nothing else reachable at all | the search returns the body's own tile, so the order is accepted and does nothing |
| gather / build / attack on unreachable targets | still refused, by two existing tests (`a_node_with_no_path_is_refused_at_order_time`, `an_unreachable_site_is_refused_before_the_wood_is_spent`) |
| determinism | the queue is ordered by `(h, y, x)`; the same command gives the same hash on every run |
| the AI's own orders | refusals fall from 777 to 574 and walled-off ones from 203 to 0 - this is the behaviour the owner asked for, and it is what moved the match into the slow state D135 then fixed |

## What this does not fix

- **The wedged scout's other half.** The owner's second sentence - *"the circle has to stay away from tree square"* - is a
  **clearance** rule, and it is not in this change: `step_is_walkable` still tests only the centre's landing tile, so a body
  may stand with its circle overlapping a tree's square. That is the next slice, and it is the one that touches every
  movement in the game (and D131 with it).
- `D131`'s head-on overlap is still ignored, with its reason on it.
- The refusal message itself still exists in the engine for the two cases above; a player will not see it for a click.

****711 passed / 0 failed / 20 ignored**; clippy and fmt clean.
