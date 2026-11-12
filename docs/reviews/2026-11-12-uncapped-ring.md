# No cap: the rule is easy, the crowd is not (parked)

**Branch `d188-uncapped-ring`, not merged.** The owner's rule is implemented and the cap is gone; what stops it landing is that our
settle cannot deliver ten men to ten places when they all arrive from one side.

## The rule, and what was built

> *"villager approaches designated resource ... if there is free place of a full diameter, villager started working. If no, villager
> looks for another one ... If there is none, villager looks for another near source of the same kind."*

- `GATHER_RING_SLOTS` and its table are **gone**. `GATHER_PLACES = 180` samples the 1.156 ring every two degrees and each sample is
  a candidate; a place is the nearest sample that is walkable, not on a foundation, and **one body (0.40) clear** of every other man
  and every reserved place. No count anywhere.
- Capacity is derived: `ring_capacity() = 2 * pi * 1.156 / 0.40 = 18`, used only as a cheap "might there be room" bound by the
  picker; the settle decides sample by sample.
- The reservation keeps a **point**, not an index, so it survives the table's removal.
- **The walk limit** (his answer to the far side, *"take next tree instead"*): a candidate more than `1.5` tiles of *extra* walk over
  walking straight at the source is refused, and the man takes the next source.
- Trig is integer: a 46-entry quarter-sine table, so no float enters a tick (`no_float`).

**Ten men now work one bush** - the first assertion of the new test passes, where nine was the ceiling before.

## Performance

`World::step` on the settled acceptance map: **7.1 us** for 128 entities (`cargo run --release -p ruage-view --example step_cost`),
against 7.2 us on `main` - the 180-sample scan costs nothing measurable, because it runs on arrival rather than per tick. No
acceptance-hash comparison is quoted: the branch does not pass its own test, so a hash would price a half-change.

## Stability - what fails, with numbers

`ten_men_work_one_bush_a_body_apart` reports, in this order across four attempts:

| settle policy | result |
|---|---|
| straight step, no check | closest pair **0.144** tiles (bodies inside each other) |
| blocked men wait | all ten work, but a man stuck **1.962** tiles out, never reaching the ring |
| tangent-only slide | same stranded distance - a tangential step keeps the radius, so he circles for ever |
| 45-degree slide | **0.345**, all ten working |
| 45-degree slide + refuse a step that lands inside a neighbour | spacing holds, but a man stranded at **1.543** |

**Why.** The settle moves a body **directly** - it is not the collision pass - so the two goals, "reach your place" and "never be
inside a neighbour", cannot both be met by choosing a step direction. Ten men arriving from one side form a wall: the ones behind
cannot pass the ones in front, because bodies are solid, and there is no path search around them in this path (the settle has no
route, only a target).

**What would fix it, in order of size:**

1. **Bodies may overlap while walking** - collision enforced at rest and on arrival only, which is what the original does (its own
   villagers pass through each other on the way to a herd; the owner's screenshot shows them overlapping at rest too). This is one
   policy in the movement pass, but it is a **movement** slice with its own tests, not a gather slice.
2. **The settle gets a route** - plan around bodies with the existing `plan_path_avoiding`, once per arrival rather than per tick.
   More machinery, and the cost is a path search per arriving man.

Neither belongs inside "simple logic", which is the owner's actual question.

## Where it stands

Branch `d188-uncapped-ring`: cap gone, places geometry-driven, ten-men-one-bush works, spacing test still red at whichever of the two
failures the policy leaves. `main` is untouched and green.
