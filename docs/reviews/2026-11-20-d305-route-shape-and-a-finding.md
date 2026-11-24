# Review: route shape, and a finding that changes the record (D305)

*Item 2 of `docs/next-step.md`. The fixture was written to close D302's coverage gap; writing it exposed that D302's headline number cannot
be attributed the way the record says.*

## The finding

`Ground::route_with`'s tier 2 is gated on **`reach == 0`**, and every unit route passes a **non-zero** reach - `plan_path_for` computes it
from `body_radius(entity.unit)`, `plan_path` from `reach_at(from)`. **So the levelled search never runs for a unit.** It runs only for the
callers that plan with no body at the point (scripted moves, probes, tests).

**Therefore D302's "acceptance 33.74 s → 22.96 s, a 32% cut" cannot be attributed to the hierarchy for units**, as the record states. The
number is real - it was measured twice, on the commit and again after - but the cause is not established: it may come from the handful of
reach-0 callers, from the pyramid cache's allocation pattern, or from something else in the same commit. That attribution has to be redone,
and the cheapest way is to remove the gate (see below) and re-measure the acceptance run with and without the hierarchy available to units.

This is the third correction of my own record this session, and the most serious of the three because it was a *number I published twice*.

## Why the gate was there, and why it should go

D302 put `reach == 0` in to keep body awareness out of a body-blind search: a coarse cell knows where **objects** stand and nothing about
bodies, so a route that must keep clear of a friend should not come from the hierarchy. That reasoning is sound for `avoid`, which is why
`avoid.is_empty()` is still right. But `reach` is the **body's own width**, not a body list - it says how wide the walker is, not who is in
the way - and refusing to plan a wide unit's long walk coarsely is not a parity choice, it is a hole.

**The fix is one line**: drop `reach == 0`, keep `avoid.is_empty()`, and certify the chain's first and last legs with `leg_is_clear(...,
reach, ...)` instead of `0`. That is exactly what the radius attempt (D303 step 2) did, and it was reverted **for a different reason** - the
`fits` tangency change that reddened `straight_line`. The two changes were made together and must be separated: the gate is free, the
tangency is the contested one.

## The fixture

`crates/sim/tests/route_shape.rs::cluttered_ground_is_answered_at_tile_level`: a comb of walls so every 8-tile cell holds one, a walk across
it, and an assertion that every waypoint but the last sits on a **tile centre** (`n + 0.5`), not a cell centre. **1 passed / 0.00 s.** It
holds today for the trivial reason (the hierarchy never runs), and it stays valid after the gate is dropped - that is the point of asserting
a *shape*: it distinguishes the tiers whichever way they are wired.

## Performance

- `route_shape`: **1 passed / 0.00 s**; the suite is unchanged at **918 / 6 / 21** plus this fixture;
- nothing in the engine was touched by this commit, so no timing moved.

## Stability

**A shape assertion is the only kind that can catch a tier swap**, and this fixture is deliberately written to pass both before and after the
gate fix - its job is to pin the tiers apart, not to police which one is chosen. What it cannot do is tell whether the hierarchy answered,
because a coarse chain in this scene would be *up and down* rather than straight; that is what the level assertion in
`coarse_route::a_blocked_walk_is_answered_by_the_hierarchy` covers, on open ground.
