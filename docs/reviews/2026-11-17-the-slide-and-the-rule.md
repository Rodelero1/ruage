# The last source of half-steps is the slide, and both obvious fixes are refuted

**Status: measured, not landed.** The fold's own half-tick (a corner landed on whose remainder is refused) has a working
gate; it could not be landed because the fixture that drives it also drives the *creep* below, and the creep needs a decision
this note hands to the owner. No production code changed; `whole_steps` stays green and unskipped.

## What the rule still misses

`whole_steps.rs` says a walking tick is one step or a stand. It passes on a walk through open ground. Put a body half a tile past
the first corner and the same walk breaks the rule **567 times in 1,398 walking ticks**, and the movement is not the fold's:

    tick 161 walked=82  at=(176000,218397) path=4
    tick 162 walked=81  at=(176000,218478) path=4
    tick 163 walked=80  at=(176000,218558) path=4

He creeps forward about **80 raw units a tick** where a step is **800**, slowing by one raw a tick. The cause is
`StepDecision::Slide`, the ground slide: when the straight step is blocked it takes **the step's axis component**, and of a
diagonal aimed mostly along the blocked axis that component is a fraction of a step. A fraction every tick is the stutter the
owner's rule is about, and no fold can repair it - the mover never lands on a route point, so the fold never runs.

## Design A - make the slide a whole step

Spend the same length along the clear axis instead of its component, so the tick is still exactly one step. Measured on the full
suite: **five fixtures regress** -

- `a_trained_unit_walks_to_the_rally_point`
- `a_house_built_across_a_walker_makes_it_go_round`
- `a_villager_walks_round_a_half_built_house`
- `a_chopped_tree_leaves_no_obstacle`
- `the_tile_holds_as_many_woodcutters_as_touch_it`

All five fail for the same reason: a full-length slide walks a man **further than the geometry planned**, so he ends up in places
the route never asked for. Refuted.

## Design B - stand instead of a fractional slide

Refuse any slide shorter than the step, stand, and let the existing repath answer. Measured on the full suite: **twelve fixtures
regress** - `a_live_animal_is_a_gather_source`, `the_villager_follows_the_sheep`, `a_depleted_node_stops_yielding`,
`a_farm_starts_with_a_worker_standing_on_it`, `a_partial_load_comes_home_when_the_node_runs_out`, `a_melee_blow_never_becomes_an_arrow`,
`a_refused_retaliation_is_recorded_rather_than_swallowed`, `a_boar_that_loses_interest_returns_to_where_it_struck`,
`a_second_blow_turns_a_boar_that_had_given_up`, `the_shepherd_kills_the_sheep_and_the_gathering_resumes_on_the_body`,
`the_spear_line_upgrades_the_units_the_player_already_has`, and the crowd fixture that already fails. Refuted: a man who cannot
slide round a wall wedges, which is exactly the finding D188 was written from.

## What the original does, and why this is a fork rather than a bug to fix

`getMaxVelFwd` (`008a2a30`) returns the unit's own speed, or **exactly 0.0 on a pause** - there is no partial arm
(`docs/movement-in-the-binary.md`). The game's own position setter **clamps against the map border alone**; its only
radius-against-radius retraction has its result discarded at the movement site (D210). So **the slide is ours, not the game's**:
D188 added it because standing at a wall wedged a villager for 68 ticks.

That makes the question the owner's, and it is not rhetorical: *does a slide count as walking?*

- **If yes**, the fixture's rule is "one step along the route **or a slide along a clear axis**", and the creep is legal but
  should still be bounded (a fractional slide could count as a stall, so the forty-tick repath reaches it instead of running for
  the whole walk).
- **If no**, the slide has to go, and the twelve fixtures above have to be re-read as evidence that standing is wrong for a
  different reason - which is a bigger slice than this one, and starts with the crowd fixture that already fails.

## The gate that is written but not landed

`move_bodies` can stand a man whose step lands on a corner and whose remainder is refused, using the fold's own leg test
(`fold_leg`) so the two can never disagree. It removes the *fold's* half-tick and leaves the slide's untouched. It is not in the
tree because the only fixture that drives it also drives the creep, so it would have been untested production code - and a test
for it alone needs a corner whose blocking body is out of the approach's reach, which the map's 0.4-tile reach makes fiddly
enough to want the slide question answered first.

## My own errors

The first version of the fixture measured a **re-plan as a giant step** (665,866 raw units in one tick) because it read a route
that had been replaced as if its points had been walked; the guard is now in the fixture and the numbers above are from the
guarded version. And the first "fix" I tried - the full-length slide - I ran only against the new fixture before trying the
suite, which is backwards: the suite is the thing that knows what the slide is load-bearing for.
