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
suite (`d253-suite.log`, 200 binaries): **16 tests across 10 binaries regress** -

- `a_trained_unit_walks_to_the_rally_point`
- `a_house_built_across_a_walker_makes_it_go_round`
- `a_villager_walks_round_a_half_built_house`
- `a_chopped_tree_leaves_no_obstacle`
- `the_tile_holds_as_many_woodcutters_as_touch_it`
- `a_barracks_trains_the_spear_line_only_as_far_as_it_is_unlocked`
- `a_boar_that_loses_interest_returns_to_where_it_was_struck`
- `a_melee_blow_never_becomes_an_arrow`
- `a_refused_retaliation_is_recorded_rather_than_swallowed`
- `a_second_blow_turns_a_boar_that_had_given_up`
- `a_trebuchet_cannot_be_trained_before_the_imperial_age`
- `the_skirmish_only_refuses_what_the_fight_has_taken`
- `the_spear_line_upgrades_the_units_the_player_already_has`
- `the_ten_minute_scenario_builds_trains_and_fights`
- `two_players_can_be_different_civilisations`
- (plus the crowd fixture that already fails)

They fail for one reason: a full-length slide walks a man **further than the geometry planned**, so he ends up in places the route
never asked for - and because a longer slide changes where men stand, it reaches fights, training and a whole scenario. Refuted.

## Design B - stand instead of a fractional slide

Refuse any slide shorter than the step, stand, and let the existing repath answer. Measured on the full suite
(`d253-suite2.log`): **14 tests across 8 binaries regress**, the crowd fixture among them: `a_live_animal_is_a_gather_source`,
`the_villager_follows_the_sheep`, `a_depleted_node_stops_yielding`, `a_farm_starts_with_a_worker_standing_on_it`,
`a_partial_load_comes_home_when_the_node_runs_out`, `a_melee_blow_never_becomes_an_arrow`,
`a_refused_retaliation_is_recorded_rather_than_swallowed`, `a_boar_that_loses_interest_returns_to_where_it_struck`,
`a_second_blow_turns_a_boar_that_had_given_up`, `the_shepherd_kills_the_sheep_and_the_gathering_resumes_on_the_body`,
`the_skirmish_only_refuses_what_the_fight_has_taken`, `the_spear_line_upgrades_the_units_the_player_already_has`,
`the_ten_minute_scenario_builds_trains_and_fights`. Refuted: a man who cannot slide round a wall wedges, which is exactly the
finding D188 was written from.

## D254 — following the game: the rule holds, the wall is a disagreement

The owner's answer to the fork was *"follow the game"*, so the slide was deleted with his corner fixture written first
(**red**: 82, 81, 80... raw a tick, **567 of 1,398** walking ticks short; **green** after, no creep). The suite then went red:
**14 tests across 8 binaries**, 192 of 200 green - animals and gathering first, then three fight fixtures, a line upgrade and a
whole scenario. Traced to one cause: a gatherer onto a sheep stops at **(316980, 316980)** with
`WaitGround` from tick 40 and `stalled` at **267**, because his aim is refused by `step_is_walkable` (landing `next < 0`, body
clear `now >= 0`) although the pathfinder called the route reachable. Raising the ground re-plan budget from **2 to 40** changed
nothing - the re-plan returns the same route, so the route is right and the step test is the one that disagrees.

**Two measures of "walkable"**: the pathfinder grows every blocked tile by the body's radius, and the mover's step test refuses a
landing that overlaps a grown tile; the original has one collision query at the new position. The slide has been hiding the gap by
letting a man creep along whichever axis the grown tile left free. Unifying the two touches **D176** - the disc rule the owner
asked for after seeing a villager with its disc inside a wall - so it needs fixtures on both sides and is its own slice.

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
