# A place on the ring belongs to the man

**Branch `d186-reserved-places`.** Gathering: fixes the overlap the owner photographed. Building: the regression it caused is
fixed and the ten-minute scenario is green again. It still costs repaths, stated below with the A/B that shows it.

## The defect

A place on the ring was re-chosen **every tick** by `standoff_point`, and it belonged to whoever stood nearest at that instant.
Three failures came from that one design - **0.001**, **0.20** and **0.243** tiles between men who were all supposed to be
working - and the owner's words for the result were *"it feels rather random"*, then, looking at the renderer, *"several
standing over the other"*.

## The change

A man gets a **place**, not a claim: `gather_place: BTreeMap<worker, (node, slot)>`, chosen when the order lands
(`start_gather_with`), read by the settle instead of re-deciding, released when his task stops naming that node
(`place_is_live`) or when something else moves him (`release_gather_place`, called by `step_aside`). A reserved slot is treated
exactly as a body: `standoff_point` will not give it to anyone else.

**A place that is no longer a place is given up** (`place_stands_clear`): walkable ground, and not on any foundation's
footprint, tested against the maintained `foundations` set rather than a scan of every entity, because it runs once per settled
gatherer per tick. `is_free_ground` is deliberately not asked - it answers "may a body stand here" and the body asking is the
man himself; ownership is the reservation's job.

## Performance - measured, with the A/B that makes it honest

36,000 ticks, seed 7, release build, same code both sides, the only difference being whether `gather_place_of` returns the
stored place or `None`:

| | reservations off | reservations on |
|---|---|---|
| acceptance hash | `ea822b6b31a006d8` | `ef7cdba92a83f173` |
| entities at 1,800 s | 137 | 142 |
| players standing | 2 | **1** |
| **repaths** | **226** | **435** |
| **made_room** | **125** | **151** |
| `World::step` | 7.2 us (main: 6.9) | 7.2 us |
| `Snapshot::of` | 98.0 us (main: 101.6) | 98.0 us |

**The repaths go up, not down - 226 -> 435.** A first cut of this branch appeared to show **352 -> 62**, and that number was
wrong: it compared two different code states, one of which also changed the match's course (the same mistake as quoting a hash
from a binary that was not rebuilt). What the reservation actually buys is **correctness and military outcome**: player 2 is
eliminated, and men stop standing on each other. It costs roughly twice the repaths, because a man who holds a place walks back
to it after being shoved off, where the old rule simply drifted to wherever he ended up.

Step cost is unchanged against main (7.2 vs 6.9 us, inside the example's own spread); the first cut's 7.7 us came from a linear
`find` per gatherer per tick in `gather_place_of`, which is gone.

## Stability

**The regression and its fix.** The first cut put the ten-minute scenario's barracks at **`progress_ticks: 0` of 1000**, the
500 s step refused with *"player 1 has no finished barracks"*, and its builder back at a gather task - while `main` at
`9b30e9c` passed. Two causes, both fixed:

1. A place chosen **before a foundation was laid** is on a building site now, so the man kept settling onto the footprint and
   `site_is_clear` could never answer. `place_stands_clear` re-validates the ground each time the place is read.
2. A man **moved off** a footprint kept his place and walked back onto it. `step_aside` now releases it.

Scenario green after both, gather binaries green.

**Retracted**: the earlier note that the scenario's gold step was refused because the economy mined it out. That was a symptom
of the same stalled barracks - a match that never builds a barracks never spends its gold - not a fixture problem, and the
tolerance added for it was removed.

## Tests

- `a_settled_man_is_not_displaced_by_a_later_arrival` - a second man placed beside a settled man's place does not take it.
- `six_settled_men_hold_their_places` - six men round one bush drift under 0.10 tiles in ten seconds, closest pair >= 0.39.
  **Mutation-checked**: with the reservation removed, two of them stand **0.000** apart.
- `a_woodcutter_whose_tree_runs_out_joins_a_patch_that_already_has_men_on_it` no longer counts ticks: a tree that runs dry
  mid-load sends the man home first, so the test waits for the join instead of for tick 2,400.
- `the_ten_minute_scenario_builds_trains_and_fights` - the guard for the regression above.

## The repath cost, isolated

The acceptance run's repaths double (226 -> 435), but that run also changes course: player 2 is eliminated with the change and
survives without it, so its counters mix gathering with a different war. `crates/sim/examples/gather_cost.rs` takes the war out:
eighteen villagers, six bushes, an open map, ten game-minutes, nothing built and nothing fighting.

| | reservations off | reservations on |
|---|---|---|
| repaths | 224 | **229** (+2%) |
| made_room | 124 | **63** (-49%) |
| men working at the end | 13 of 18 | **16 of 18** |

So the gathering cost of a place that belongs to its man is **2%**, not 92%, and it halves the body-shuffling while putting three
more men to work. The acceptance run's repaths are a property of the match it ends up playing, not of this rule.
