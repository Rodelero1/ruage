# Review: one widening when the geometry declines - the fixtures are clean and the match is not

*Change (branch `d279-substitute`): `short_range_clear` gains a `widen` flag, and both planning sites - `plan_path_mode` and the
re-plan in `step.rs` - retry **once** with the rectangle and node limits four times wider before any tile fallback. This is the
original's rule: "expand the search area once, otherwise give the closest point path".*

## Performance

`DBG_ORIGIN=1 cargo test -p ruage-sim --no-fail-fast --test animals --test gather --test gather_motion -- --nocapture`, before and
after:

| | before | after |
|---|---|---|
| animals | 10 / 1 | **11 / 0** |
| gather | 22 / 0 | **22 / 0** |
| gather_motion | 6 / 0 | **6 / 0** |
| ground refusals printed | **22**, all `tile-chain` | **0** |

The acceptance (`make headless`, seed 7, 48,000 ticks):

| | `main` | this branch (widen, no cap, no slide) |
|---|---|---|
| entities | 131 | 153 |
| players | **1** | **2** |
| ground re-paths | 135 | **156,433** |

Suite run times, instrumented and after: animals **5.7 s**, gather **17.7 s**, gather_motion **2.7 s**; acceptance **322 s** for
48,000 ticks. Against the branch's previous build the ground re-paths rose **+4.8%** (149,254 -> 156,433).

**The widening is a fixture win and a match failure**, and slightly worse than the branch's own previous run (149,254 -> 156,433).

## Stability

**What was attacked.** (1) A declined search was silently answered by a tile chain whose legs nothing validated - measured, 22 of 22
refusals had a tile-chain first point. The widening removes those **entirely** in the suites, so the mechanism is confirmed and
fixed at the fixture level. (2) The second attack is the one that failed: removing the ground cap on the hypothesis that the
widening would make it unnecessary. The match says otherwise - the refusals there do **not** come from a declined geometry search,
so each one buys a search four times wider than before, and without a cap that is a treadmill.

**Found and reported, in order:** my "the place radius is the fix" message was wrong (measured neutral); my "22 of 22 refusals are
tile-chain" finding was measured on a suite mix I then misread (the probe had been discarded by a `git checkout` before it ran);
and this change's own hypothesis - that widening alone would clear the storm - is refuted by the gate. Three wrong readings, each
corrected by a measurement rather than an argument.

**Known and accepted:** the cap should come back on this branch until D280's slice measures what the match is actually refusing;
the widening stays because it is the game's rule and it removes 22 real refusals. `main` untouched and green.
