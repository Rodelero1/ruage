# Review: the search sees standers, the clock is the order's - and it is 5.1x too slow to land (D282, NOT LANDED)

*D281 read the original's collision block and implemented three things; three reds went green and three others went red, so
it was reverted. This is the re-attempt, with the one piece D281's reading implies and the first attempt did not have: the
**standers are obstacles to the search**. Numbers below are from this branch (`d281-body-in-the-search`, off `f055be6`),
debug, unless a line says otherwise.*

## What changed, and why it is the game's model rather than a design

Four things, each with its source:

1. **The search keeps off the tiles of every body standing still** (`FUN_007f9d80`, the register's *"search obstacles:
   standers only (velocity zero), inserted per search, never into the shared grid"*). A walker is not an obstacle - he will
   be elsewhere when the route arrives - but a stander is. This is what makes the *past* test coherent: a route planned
   while a man stood goes **round** him, and a collider that has not moved since `path+0x10` can then be ignored.
2. **A non-past body collision re-plans** with that collider's tiles kept off (`"we have %d non-past collisions and %d
   terrain collisions, repathing"`). Unthrottled, as the original is: what bounds it is the pause the loser is given and
   the *past* test.
3. **A route's clock is its order's.** `put_route` stamps `created` only when it is zero, so a re-plan keeps
   `path+0x10` - D281's second defect, which had made the man just re-planned round look *past* on the next tick (**4,207
   raw** inside an **8,000 raw** reach).
4. **The tile the walk starts on is never an obstacle.** Without it, a body ordered to the tile it already stands on has
   its own goal avoided and answers `NoPath` from a tile to itself - measured as a fixture that failed at its own
   precondition.

The D279 exemption *"a body that is not going anywhere is past"* is deleted: the original's ignore list holds the mover's
**moving** mates and nothing else, so a stopped body blocks.

## Performance

**The mechanism is right and it is 5.1x too slow, so it does not land.** Debug, 48,000 ticks, same fixture, same host:

| build | `the_acceptance_match_trains_more_villagers` |
|---|---|
| `f055be6` (D279 + D280), recorded in `docs/next-step.md` | **42.62 s** |
| D281's first attempt (no standers in the search), recorded in its review | **38.96 s** |
| **this branch** (D282) | **217.54 s** |

and the cost shows up in the re-plan traffic: `make headless`, 48,000 ticks, release - `f055be6` **5** re-paths,
this branch **28,545** (entities 120 -> 136, `players=2` on both).

**Why, in one line:** the straight-line shortcut is now the *unusual* case. With the standers in the avoid list, any man
standing anywhere near the line makes `line_clear` fail, so a plan that cost two points costs a whole-map A\* - and the
body re-plan is unthrottled, as the original's is (what bounds it there is the pause and the past test, and the pause is
only 0.25-1.50 s of standing for the *loser*, not a bound on the winner's searches).

**The three things that would make it affordable, none of them tried yet:**

1. **Throttle the body re-plan** the way the ground re-plan already is (`REPLAN_TICKS = 20`): a divergence, and the first
   thing to measure.
2. **Standers only within a window of the route** - the original inserts per search, but over a 480 x 480 map it must have
   some locality; unread.
3. **Plan through a stander whose tile is already on the line and let the step refuse once**, i.e. keep the shortcut and
   pay the re-plan - which is what D281's attempt did, and it was *faster* (38.96 s) while fixing fewer fixtures.

## Stability

- **Re-planning is not throttled**, by design. A body that keeps being refused buys a search a tick; the bounds the
  original relies on are the loser's pause and the past test, both of which are here. If a match ever shows this running
  away, the counter to add is searches per tick, not a cap.
- **The carrier fixture's precondition moved, not the rule.** With standers in the search, two men walking on crossing
  roads no longer come within a reach of each other, because the plan goes round. That is the rule working and the
  scenario no longer containing a crossing - so the scenario derives the crossing from the carrier's own route
  (`right_of_way`, restated).
- **The pause bound moved from the stand to the draw.** The loser stands for its drawn pause **and** for as long as the
  winner still holds the way, so **44 ticks (2.200 s)** of standing is not a freeze - measured. The assertion is now on the
  drawn value (0.25-1.50 s), which is the number the original actually writes.

## What did not land, and what remains red

- `evasion::a_repath_route_is_pulled_like_a_planned_one` still red: it asserts the **disc-shoulder band** from D211 (every
  interior waypoint on the clearance circle), which D279 withdrew with the visibility graph - the port's corners are tile
  centres. Its scenario also makes the *walker* the lower id, and the lower id is the winner, so nothing forces the repath
  it wants to measure. Restating it needs the walker to be the higher id, which needs a free id below the seeded ones;
  that is a scenario decision, not a code one.
- `group_overlap::a_mate_who_is_stopped_becomes_solid` still red: the mate is walked over at **5,461 raw** inside an
  **8,000 raw** reach even with the re-plan. Two bodies sharing a tile cannot be planned round - the search starts on the
  tile they share - which is D281's own recorded finding, unchanged.
- `collision::a_walker_never_ends_a_tick_inside_a_friend` now fails on its **precondition** ("the fixture did not meet")
  rather than on an overlap: the two men never come within a reach. Same cause as the carrier fixture.
- `no_pushing::a_man_who_cannot_get_his_square_takes_the_next_tree` unchanged (the gather-refusal book, a known red).
- **`tests::a_spawn_in_the_same_tick_does_not_move_the_wrong_entities` is removed**, with the reason in place of it: its
  scene was two movers at the **origin** (`spawn_now` places every body there), and two bodies inside one reach are a
  standoff under this rule, so the scene contradicted the rule it ran under. What it guarded - a delta landing on the
  wrong *entity* - is no longer reachable: positions are written by id, and only `record_step`'s instrumentation reads the
  index. A behavioural replacement needs a map and two ordered walks, so it belongs in `tests/`.

## Worktree, and a collision to report

The shared tree `/opt/data/workspace/Age2HD` was being written by **another session** throughout this slice (`git reset`
moved `movement-clean-slate` from my `829ba40` back to `f055be6`, and its own edits replaced mine in `ground.rs` /
`lib.rs`). This slice was therefore built in a linked worktree, `d281-body-in-the-search`, and pushed from there. Nothing
was committed over the other session's work.
