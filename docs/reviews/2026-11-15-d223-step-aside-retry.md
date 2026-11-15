# Review: a failed step-aside expires

*Change: `step_aside`'s early return now carries a **rolling twenty-tick deadline** - a step-aside is re-asked only when the man
has not moved in those twenty ticks, tracked in a new `World::aside_retry`. The parked 6×6 build fixture is un-parked and green.
D223.*

## Performance

**One map lookup per tick per man standing on a foundation, one path search per 20 ticks for a man with nowhere to go.** The
expensive shape this must not fall back into is one path search a tick for a stuck body, which is what the early return was
for; a 20-tick expiry caps that at **1/20th** of it. Measured effects: the 6×6 fixture goes from *never* finishing (2,000 ticks
of `progress_ticks = 0`) to finishing, and the build suite runs **12 passed / 0 ignored in 6.96 s**; the gather and movement
suites are unchanged (**22 passed in 17.35 s**, **4 passed in 1.73 s**), which is the check that the expiry did not turn every
crowded foundation into a search storm.

## Stability

- **The expiry is a deadline, not a counter.** `aside_retry` holds `tick + 20`, so a man who keeps failing costs a *constant*
  one search per 20 ticks however long he stands there, and a man who succeeds has the entry removed the moment the search finds
  a tile - there is no state that can only grow.
- **Both failure paths record it.** `exit_tile` answering `None` and `plan_path` failing are the two ways "nowhere to go" is
  reached; both now set the deadline, so the fix cannot be half-applied.
- **A successful walk is untouched.** The early return still fires every tick while `tick < due`, and an in-progress step-aside
  keeps its `resuming` entry, so the "one order, one route" property that the early return existed for is preserved.

- **Three shapes were measured before the working one, and each broke something different.** A `0` default re-planned every
  walk-out (the gatherer test went red); "no deadline means in flight" restored the gatherer and lost the fixture, because the
  stalled man's first attempt had *succeeded*; writing the deadline only on the failure paths never stored it at all, because
  the early return fired before the insert (`in flight, due 2020` printed at t2000, then for two thousand ticks). The rule that
  survives asks about progress, not about which way the attempt ended.

Known and accepted:

- **Both halves of the rule that touch *where* the man goes were measured out.** A `tile_is_wanted` check (a step-aside must not
  aim at the space another body is walking to) was written and removed twice: with the working retry in place, deleting it leaves
  the 6×6 fixture green, so the retry is doing the work and the check is unproven. The claim proper - a builder owning a place,
  and taking the next site in his queue when he has none - is not written at all. `docs/next-step.md` carries both with the
  owner's ruling quoted.
- **The site flip-flops one tick per 20** for a man with nowhere to go: `resuming` is re-written on each retry, which is the same
  write the first attempt made. No test observes a behaviour change from it.
- **The queue fallback is not built.** A builder who finds no place still waits rather than moving to the next site in his own
  queue; `docs/next-step.md` carries it as the head item, with the owner's ruling quoted.

---

*This file is what `tools/review_check.py` looks for: `## Performance` with a number, `## Stability` with content, staged
with the change. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
