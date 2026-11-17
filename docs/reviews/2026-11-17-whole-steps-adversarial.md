# Adversarial review: the corner fold (D252), performance and stability

Reviewed at `3c147b0` on the owner's request, **adversarially**: the question is not whether the fold works (its fixture says it
does) but what it costs, and what it does that nobody asked for. Every number here is from a run of this commit's code; where a
number is missing, it says so.

## Performance

**The standard scenario prices nothing, and that is the first finding.** The acceptance match (release, seed 7, 48,000 ticks,
two runs each from one binary) never fires the fold - its walks are single-leg, so no body ever lands on a corner:

| | wall | entities | players | repaths | hash |
|---|---|---|---|---|---|
| fold off | 39.5 s, 40.3 s | 138 | 1 | 761 | `01a0fb771ee2e75b` |
| fold on | 40.0 s, 39.7 s | 138 | 1 | 761 | `01a0fb771ee2e75b` |

So "bit-identical on the acceptance match" is true and says nothing about cost. A corner-heavy probe has to supply the number.

**And the box is noisy enough to swamp anything the fold could cost here.** A second A/B run of the same four settings read
**44.4 s and 39.6 s** fold-off against **40.3 s and 40.3 s** fold-on - an 11% spread on the *same* setting, which is ±4.5 s of
machine noise against a fold that fires zero times. The first run's table stays as measured; this one is here so nobody reads
either table as a measurement of the fold.

**The corner-heavy probe.** 120 villagers, each ordered across the pack's map (routes that bend round the forest), 6,000 ticks,
release, the same binary with the fold gated by an env return. Three runs each, medians:

| | wall | body-ticks walking | repaths | still walking at tick 6,000 |
|---|---|---|---|---|
| fold off | **4,301 ms** (4,219 / 4,301 / 4,389) | 57,501 | 61 | 0 |
| fold on | **4,235 ms** (4,397 / 4,235 / 4,168) | **57,841** | **71** | 0 |

**CPU cost: none measurable.** The fold-on median is *lower*, the spread is wider than the difference, and every run finishes
inside 4.2-4.4 s. That is what the code predicts: folding is rare (see the census below), bounded to two legs, and each firing
costs one `route_points` clone, one scan of the 9 tiles round the landing and one `stamp_entity`.

**The cost that is real is crowd churn, and it is deterministic.** With the fold on, the same 120 men spend **340 more body-ticks
walking (+0.6%)** and take **10 more repaths (+16%, 61 -> 71)**, and all of them still arrive (`still_walking=0` both ways). The
mechanism is the rule itself: a whole-step tick ends a body up to 0.04 tiles further than a landing would have, which changes who
is in whose way next tick, and a crowd re-plans on that. Nobody walks slower *per tick* - the extra ticks are re-routes, not
partial steps - but "the fold is free" would be the wrong claim to carry forward.

**Frequency, so the O() can be judged.** A census over the whole `ruage-sim` test suite (env-gated counter in the fold, 97 test
binaries) counted **116 fold firings in total**. The `asked` list is scanned linearly per firing and `route_points` is cloned per
leg; at 116 firings across a suite that is noise, and at 10^4 firings a tick it would not be. No action taken, and the number is
here so the next reader does not have to guess which side of that line we are on.

## Stability

**The one hole worth naming is closed by evidence, not by code.** A step that is *slid* rather than let through can, in principle,
land within one raw unit of the front point and then be folded - walking a body past a corner the mover only partly won. The
census says it never happens in this tree: **116 of 116 folds were taken under `Take`**, none under `Slide`, `TakeAtRest`,
`WaitGround`, `WaitBody` or `Paused`. I did **not** add a guard for it, because no test shows the case and a guard without a red
test is a slice that slips past test-first (D7); the census is the evidence, and the guard is the next step if a fixture ever
produces one.

**Determinism.** The acceptance hash is identical across two runs each way, and the fixture is deterministic across repeats. The
fold mutates `self.paths` while iterating `reached`, which is entity order, so the mutation order is fixed.

**Arrival.** The last point is never folded past, so `position == goal` still lands exactly; `a_short_walk_lands_on_its_goal` is
green. The gather/build/deposit approaches are out of scope by task, which is what keeps `settle_workers` (it runs *after* the
mover, line 52 against line 47) from undoing a fold - that interference is what starved the trebuchet fixture before the narrowing.

**Sight.** `stamp_entity` runs on the pre-fold position, so the fold re-stamps; the bitset is an OR and cannot double-count.
`vision`, 7 tests, green.

**The regression this slice exists for** is pinned by a live fixture, not a note:
`research::a_trebuchet_cannot_be_trained_before_the_imperial_age` - the one that froze at **200 of 400** food for 400,000 ticks
when the fold covered `Gather` - is green at **8.9 s**.

**What is still not covered, said plainly:**

1. **A refused fold leaves a short tick.** If the ground or a body refuses the remainder, the body ends the tick part-way through
   its step, which is a *third* state next to "walks a whole step" and "stands". `whole_steps.rs` cannot see it, because its route
   is never refused. The rule as enforced today is "a whole step, or a stand, **unless the rest of the step is refused**", and the
   fixture should grow a case for it (a corner with a body standing just past it) before someone claims the strict form.
2. **A `Move` body that is also in `settles` on the same tick** would be moved by the fold and then by the settle, which runs
   after. Not observed (a settled body is a gatherer, and gatherers do not fold), but it is one task change away.
3. **`asked` is a linear scan per firing** - see the census above; fine now, not fine at scale.

## What I got wrong, in this same slice

- I first reported the fold as safe because the acceptance match came out bit-identical. Bit-identity on one script says nothing
  about a 400,000-tick economy fixture, and the fixture caught what the match could not.
- I then turned the owner's fixture off (`#[ignore]`) to park the problem. That is not mine to do, it is now a rule in the skill,
  and the correct move - diagnose the freeze - took one trace to find.
