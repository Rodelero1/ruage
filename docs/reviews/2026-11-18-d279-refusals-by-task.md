# Review: the ground refusals are gather approaches - a place route is planned with no radius at all

*Change: none. A probe counted ground refusals **by task** on the branch `d279-substitute` (prefix + substitution + no cap + no
slide), and its answer reframes the fix. The probe is removed.*

## Performance

`cargo test -p ruage-sim --no-fail-fast --test animals --test gather --test gather_motion`, `DBG_REFUSE=1`, 29 refusals counted
in **29 s** of suite time:

```
     29 kind=Gather
```

**29 of 29 are `Gather`.** In the fixtures, every ground refusal belongs to a gather approach - and a gather approach is a **place
route**, which `plan_path(from, to, 0)` plans with **radius 0**.

## Stability

**What that means, and it is a conflation, not a bug in the step test.** `radius == 0` was introduced for one reason (D211): a
place route must ignore the **crowd**, because the ring has already reserved the place against every body, and modelling the
crowd twice made a dense map unroutable. But the same `0` also means "this body has no size" to the **ground** geometry, so the
planner certifies legs for a point that the mover - which knows the body is 0.2 tiles wide - then refuses. Two different questions
share one number, and the answer to one of them is wrong.

**So the next change is one parameter, not one rule:** split them - a place route keeps its **own body's radius for ground
clearance** and **zero for bodies**. Fixture to write first: a gather approach whose place sits beside a footprint must produce a
route the disc step test accepts, watched failing with today's radius-0 route.

**Why this is the game's model and not ours:** the original plans every route for *"the boundaries of where this unit's radii will
go"*, whatever the order is for; it has no notion of a route planned by a point.

Known and accepted: `main` untouched. The animals family is excluded from this slice's gate by the owner's word, with the fixtures
kept.
