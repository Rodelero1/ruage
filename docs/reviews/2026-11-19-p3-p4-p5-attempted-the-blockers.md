# Review: P3, P4, P5 attempted - the two blockers, measured, and what did land

*The owner: **"I want no deliberate differences to original game ... adjusted to meet the original or cut off"**, and, on the two
forks the plan raised: **"the game wins (a): adopt `<=`, a half-tile body stops at a one-tile gap, and D117 is withdrawn"** and
**"cut the book entirely and let collisions and pauses do the spreading."** The plan is `docs/parity-plan.md`. This note is the
attempt at its next three steps and what it cost; the engine changes for P3 and P4 are **not landed** and the reasons are below.*

## What landed

- **`Ground::touches_object`** - `FUN_00840830`'s own test (the box grown by the mover's radius, collided on `<=`), public and
  pinned by `conformance::contact`'s two rows. No call site yet: it is the rule P3 must land on, measured and ready.
- **`conformance::budget`** - one live row (a reachable goal is still reached) and the budget's two rows **ignored with the
  measured reason** below, replacing the stale "the map is small enough that it does not bind".

## Performance

The three steps were each measured on `acceptance_economy::the_acceptance_match_trains_more_villagers` before being judged:

- with P4 + P5 (the throttle and the forty-tick order-finish both cut): **35.79 s**, against 34.65 s with P1/P2 alone and 33.74 s
  at D286. So the price of both cuts is **+1.1 s**, which is affordable and not why P4 was withdrawn.
- nothing was added to the tick in the state that landed: `touches_object` is dead code until P3 wires it.

## Stability - the two blockers, each reproduced

**P3 (touching is a collision) cannot land alone, and the reason is the plan.** Wired into the step as the binary has it (a
landing that touches a tile the body is not already against is refused), a villager ordered at a sheep 1.4 tiles away stood in
`WaitGround` for **3,000 ticks** and never swung: measured, `retaliation::a_melee_blow_never_becomes_an_arrow`. The probe's own
line names it - `here_tiles [] landing_tiles [(28,14),(28,15)] deeper true`: the body touches nothing, and the landing its
**certified route** asks for touches two blocked tiles. So `leg_is_clear` certifies a leg the step refuses; making the plan
as strict as the step (touching crosses a clearance) did not close it either. P3 therefore needs the plan and the step to agree
about touching in one slice, and the first attempt at that is what the next session should read before wiring anything.

**P4 (the throttle goes) cannot land before P8, and the reason is the book.** With the throttle removed the terrain re-plan runs
on every refusal - as the binary does - and that is fine on time (**+1.1 s**, above). What broke is the **goal fallback** that
sat beside the throttle: *"a place the body cannot get to is given up for the tile beside it"*. Every gather place and every
attack stand-point is an **off-grid point**, and no step reaches it; the fallback was what made those routes survivable. It is
part of the seat book's furniture, so it goes when P8 goes - in that order, not this one.

**P5 (no forty-tick order-finish) is unaffected by both.** With `finish_blocked_orders` deleted, the stall, stuck, replan and
repath-budget files are unchanged and the acceptance match is within the +1.1 s above; it is held back only because it was
measured inside a bundle with P4 and the two want separate witnesses. It is a four-line cut and the next slice should land it
first, alone.

**What this changes in the plan.** P1 is blocked on P11 (the budget needs a levelled search: measured, a flat search at
`h = 25 x tiles` truncated a one-tile walk into a fifteen-tile detour). P3 is blocked on the plan/step agreement about
touching. P4 is blocked on P8. P5 is free. The plan file is updated with all four, in this order, so the next session does not
re-derive them.
