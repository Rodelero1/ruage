# Review: a load leaves at the wall, and nothing looser (D313)

*Change: `Task::Return` banks a load only within `radius + one step` of the drop building's footprint. The second clause -
`radius + a quarter tile` whenever the route was exhausted - is gone. One red (the owner's own drop-site invariant) closed; one
fixture had its witness made non-vacuous; the acceptance match is measurably smaller.*

## What was wrong

The owner's rule 3, verbatim (D243): *"If villager drops resource when further from the drop site than 1/2 diameter -> FAIL"*.
The invariant measures it, and it was red:

```
tick 924: villager 9001 delivered 10.00 of a load with his body 0.420 tiles from the drop building -
past the 0.200 half-diameter, even allowing the one step a walker stops short by
```

0.420 is inside the engine's `as_close_as_the_ground` clause (`gap <= radius + scale/4` = **0.45** with the route spent) and
outside the rule. That clause was written before the step allowed **tangency** (D306): touching a wall used to be a collision, so
a carrier whose neighbouring tile was taken could not reach the wall and never banked at all - measured once at 400,000 ticks
(`research::two_players_can_be_different_civilisations`, 890 of 900). Tangency is a passage now, so the carrier walks the last
stretch to `wall_point` instead, and the rescue is no longer needed. **The number never changed: the number of clauses did.**

## Test-first

The witness is the owner's, not mine, and it was **watched failing** at 0.420 tiles before the change and green after - the run
is quoted above. The fixture was not touched.

## One fixture had to give, and it gave something a check was leaning on

With the tighter rule the suite swapped one red for another: `overlap::a_newcomer_does_not_move_the_working_gatherers`. It is
not a rule violation - the man who "moved" had **filled his load and gone home**. Its after-check excluded only men whose task
was no longer a gather, while its before-set was chosen by `progress > 0`; a man who has banked and is walking back is still
`Gather` with `progress 0`, so he was compared against a position he had legitimately left.

Two repairs, both keeping the claim:

- the exclusion is now the one the selection used (`progress > 0`) - a man **at work**, which is the rule;
- a **`judged > 0` guard**, because sharpening the exclusion made the loop able to skip everybody and pass on nothing. It had
  been about to: the scene's only working man filled his load at **tick 13** of the 200-tick window (measured).
- the eight gatherers now stand on the node's own ring instead of three tiles apart, so **four of eight** are taking by
  tick 1,200 and the witnesses outlive the window (measured both ways).

**What is proven and what is not.** The position comparison demonstrably fires - it is what produced the red above, and the
mutation that removes the re-lay's `!at_node` guard fails the fixture at its precondition. A mutation that *displaces* a
working man while leaving others at work was not constructed: one attempt compiled to something the walk step made inert (the
fixture stayed green with the D246 freeze removed, because a man already at his place has nowhere to step). So the fixture is
shown live and non-vacuous, not shown to catch a deliberate shove.

## Performance

- **Acceptance match, release, seed 7, 48,000 ticks**: **9.28 s** and **9.54 s** over two runs, against **8.53 s / 8.81 s** on
  the parent `373902f` - **+9% to +12% wall**.
- The final state **changed**, as it should: hash `89ff37d844cfd11f` against `3f6c802027ff40fb`, **129 entities** against
  **141**, and **10,026 repaths** against **6,031** (+66%).
- **The economy is smaller at 48,000 ticks, and that is the price of the rule.** A carrier now walks the last ~0.21 tiles to the
  wall on every trip instead of banking from 0.45 out; at 0.8 tiles/s that is ~0.26 s a trip, and an 8.5% smaller economy is
  the same order as that per-trip cost compounded over 2,400 s. It is a trade, stated as one: the owner's own invariant is on
  the side of the walk, and the alternative (bank at 0.45) is a rule he wrote down as a failure.
- **What is not measured**: the delivered totals per column on both trees (`--report` prints them), which would turn "+9% wall
  time" into a throughput percentage. Open, and the next measurement for this subject.
- No man is stuck: the 48,000-tick dump has **no carrier in `Return` without a route**, and 8 men still gathering. Repaths
  rising is the search being asked again while a carrier closes the last stretch, not a loop.
- Full workspace suite: **924 passed / 3 failed / 21 ignored** against **923 / 4 / 21** before. The red set: the drop-site
  invariant left and **no new red joined** (`a_walled_off_wood_is_tried_once_and_then_left_alone`,
  `both_sides_field_an_army`, `the_skirmish_only_refuses_what_the_fight_has_taken` remain, all pre-existing). clippy
  `-D warnings` and `fmt --check` clean.

## Stability

- The clause removed is the one that licensed the 0.42-tile bank; nothing else about delivery changed (`footprint_gap`, the
  wall point, the walk to the yard are untouched).
- The risk it carries is the one that motivated the clause: a carrier whose wall is unreachable. The two fixtures that
  measured it - `dropoff::a_crowd_all_banks_its_loads` and `dropoff::a_load_comes_home_past_a_building_beside_the_town_centre`
  - are green, as is the 400,000-tick `research::two_players_can_be_different_civilisations`.
- **The `judged > 0` guard is the load-bearing part of the fixture repair**: without it the sharper exclusion would have turned
  a red into a silent pass, which is the "a fixture that cannot fail" trap. It was one frame from doing so.

Known and accepted: the reach is still `radius + one step` rather than an exact zero-gap test - a step is the quantisation of
a whole-step walker (D286), and the fixture allows it explicitly. The delivered-totals comparison above is owed.
