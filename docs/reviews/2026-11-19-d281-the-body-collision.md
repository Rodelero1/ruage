# Review: the body collision, read from the decomp and **not landed** (D281)

*The owner, asked which of three ways a crowd should behave: **"Follow what does the original game and decomp do."** So the
crowd was re-read out of `FUN_00844360` and the path layer, implemented, measured - and **reverted**: with it in place three
of D279's reds went green and **three others went red**, which is a half-fix by this tree's own rule. The readings and the
numbers are the step. **Nothing was kept**: the attempt was reverted whole, because a patch that adds three reds is not a
patch the next session can trust either.*

## What the original does (read, not inferred)

- **A collision against the order's target ends the order** - *"That collision is with our target, so we're done"*.
- **Any other non-past collision is counted, and then the mover re-plans** - *"We have %d non-past collisions and %d terrain
  collisions, repathing"* - and the **higher id of each surviving pair** is told `setPause(rand*1.25/32767 + 0.25)`
  (`0x8a80ff`-`0x8a8141`).
- **The epilogue never writes a position.** The retraction the radius query computes is discarded at the movement site
  (`0x8a7fe7`), so bodies *do* overlap for the ticks they pass each other.
- **A collider is *past* when `path->createdAt >= other->lastMoveTime`** (`move_obj.cpp:2063`).
- **The path-ignore list is the mover's moving group-mates and nothing else** (`isMoving`), so a stopped unit is not passed
  through: it blocks, and the mover's re-plan is what gets round it.

## Performance

Measured on this tree, debug, 48,000 ticks, with the whole attempt in place:

- `the_acceptance_match_trains_more_villagers`: **42.62 s -> 38.96 s**. A mover that bends round a body stops buying
  refused steps against it, so the re-plan paid for itself.
- The re-plan is **one search per collision** (`Ground::route_avoiding` with the colliders' tiles avoided, at most a
  handful), with **no budget** - the original has none, and D277/D279 withdrew ours. What bounds it is the loser's pause
  and the *past* test.

## Stability

**Three reds fixed, three reds created.** That is the whole verdict.

Green with the patch, red without: `a_repath_route_is_pulled_like_a_planned_one`, `both_sides_field_an_army`,
`the_skirmish_only_refuses_what_the_fight_has_taken`.

Red with the patch, green without:

- **`tests::a_spawn_in_the_same_tick_does_not_move_the_wrong_entities`** - *"entity 2 must move by its own delta, not by a
  neighbour's: left 0, right 5"*. A body that should take a five-raw step took none. **This is a regression, not a rule**,
  and it is the first thing to understand before the patch is re-attempted.
- **`the_higher_id_pauses_for_a_quarter_to_one_and_a_half_seconds_and_keeps_its_route`** - *"the higher id stood still for
  44 ticks (2.200 s): a pause is bounded at 1.50 s"*. Under the game's model the loser stands for its own pause **and** is
  blocked by the winner until the winner has moved off, so a 30-tick bound may itself be our invention - but that is a
  reading to make, not an assumption to keep.
- **`the_ten_minute_scenario_builds_trains_and_fights`**.

**Two defects found inside the attempt, both worth keeping when it lands:**

1. **The smoothing pass undid the plan.** The search bent the route round the man and `pull` straightened it back through
   him, because `line_clear` knew only terrain - the re-plan answered a **one-point route straight at the body**, every
   tick. The line test must take the same avoid list the search has.
2. **Re-stamping the route's clock made the avoided man *past*.** `put_route` wrote `created = tick` on every route,
   including a re-plan, and the next tick's past test then called that man *past* and walked through him - **4,207 raw
   inside an 8,000 raw reach**. A re-plan must keep the route's creation time; the original's `path+0x10` is the order's
   clock, not the re-plan's.

**Also worth recording from the attempt:** two bodies that share a tile cannot be planned round at all - the search starts
on the tile they share - so a walker still ends **4,207 raw** inside a stopped man's reach even with the re-plan in place.
That fixture (`a_mate_who_is_stopped_becomes_solid`) asserts "nothing inside the reach after the meeting", which is our old
retraction, not the game's rule (the game's epilogue writes no position). It stays on the known-red list either way.

---

**The shape, so it does not have to be redesigned**: `Ground::search` and `pull`/`line_clear` take an `avoid: &[(u16, u16)]`
slice (blocked tiles, checked beside `walkable`); `route_avoiding` skips the straight-line shortcut when the list is
non-empty; `World::plan_path_avoiding` wraps it; `motion::refusal` fills a `blockers` list with `tile_of(other.position)`
for every collider it counted and the pass re-plans the route's own last point through `plan_path_avoiding`; `put_route`
stamps `created` only when it is zero; and the `going_somewhere`/`pause_remaining` exemption is deleted. The next session
re-applies that, understands the spawn regression **first**, and lands it when the red set is a strict subset of D280's.
