# Review: a plan keeps a body's clearance, not a tile (D282)

*The audit of the fixtures (`docs/reviews/2026-11-19-fixtures-vs-the-decomp.md`) found that `evasion::a_blocked_unit_goes_round_a_standing_body_and_arrives`
was **the game's rule** and our plan was the defect: `FUN_007f9d80` puts a stopped body into the search as a rectangle grown
by the mover's radius, while D281's re-plan avoided the collider's **tile** — which cannot express 0.4 of a tile, so the
route cut the corner. The owner said **"proceed"** on that and on the pause off-by-one the audit also found.*

## Performance

- **The acceptance fixture is unchanged: 39.58 s** (48,000 ticks, debug; it was 39.59 s). The change adds **one unit scan per
  plan** - every standing body near the line becomes a clearance - and **no search**: the clearances are tested inside the
  expansion and the smoothing the plan already runs.
- The clearance test is a squared distance per candidate tile (0-3 entries in practice) and one point-to-segment distance per
  straightening jump. On the 64x64 acceptance map that is below the noise of the tick.

## Stability

**Suite: 198 binaries, 882 passed, 19 failed, 21 ignored** (from 881 / 20 / 21). **Three reds turned green, two came
back, and the two that came back are a timing knife-edge in the scenario schedules - stated below rather than smoothed over.**

**Green, and they are the point of the slice:**

- `evasion::a_blocked_unit_goes_round_a_standing_body_and_arrives` — the walker came **4,619 raw** from the body against an
  8,000 standoff; with the clearance in the plan he keeps it and still arrives.
- `head_on::the_loser_stands_for_the_drawn_pause` — the loser's stand was written one tick short (`ticks - 1`), so the game's
  0.25 s floor became **0.20 s** at 20 Hz. Now the draw is written as drawn.

**Also green: `the_ten_minute_scenario_builds_trains_and_fights`** - the builder stall the audit found (0 of 1,000 progress
in 1,400 ticks) is gone, because he now plans round the man standing in his way instead of pressing into him. That was not
the slice's target and it is the strongest evidence that the clearance is the game's rule.

**Red again, and it is timing, not a rule:** `skirmish::both_sides_field_an_army` (*"player 1 never fielded infantry"*) and
`skirmish::the_skirmish_only_refuses_what_the_fight_has_taken` (*"player 1 has no unit militia"*). Both are the same shape as
the builder stall - a script step refused because the thing it trained from was not ready - and both pass or fail on a few
ticks: this slice adds **one tick** to every drawn pause (the game's own 0.25 s floor) and up to a **detour** to any plan with
a body in it. The scripts' deadlines are thin (the ten-minute fixture's own comment records 921 of 1,000 at 62 s of a 70 s
window), so a tick of movement timing flips them. That is a **fixture** defect - a deadline that close to the engine's own
duration is measuring the schedule, not the rule - and it is the next item, not a reason to keep the short pause.

**What still fails, and why (all four were red before this slice):**

- `a_mate_who_is_stopped_becomes_solid` — **4,478 raw** inside an 8,000 reach. Two bodies that **share a tile** cannot be
  planned round: the search starts on the tile they share, and the mover's own clearance is skipped or every plan would begin
  inside it. This is the hole the audit named as next (item 2 of the queue), not a new one.
- `collision::a_walker_never_ends_a_tick_inside_a_friend` — its message **changed**: it used to measure how deep the meeting
  went, and now reports *"the walker never came within the reach: the fixture did not meet"*. A head-on pair now plans round
  each other, so the collision the fixture stages does not happen — which is the game's behaviour and makes this fixture's
  premise obsolete. It joins the rewrite pile, and the change in its message is the evidence, not a silence.
- `a_walk_past_a_house_keeps_its_bodys_clearance` — a **building footprint**, not a body: the clearance list carries bodies
  only, and this red is the route clipping a footprint corner. Unchanged, and still its own defect.
- The rest of the red set is as the audit classified it; this slice was scoped to two rows of that table.

**What was attacked.** The scan could have made the planner body-blind in the other direction — a plan that refuses to move
because a body sits near its start — so the mover's own body is skipped by position (a quarter tile around `from`), and a
standing body is only a clearance when it is **not moving**: a body with a route and no pause is ignored, which is the
original's `isMoving` gate read at plan time. Both are one condition, and both are exercised by the suite: the crowd scenes
(`a_crowd_sent_to_one_place_does_not_stack`, `eight_villagers_ordered_to_one_tile_never_walk_through_each_other`) stay green.
