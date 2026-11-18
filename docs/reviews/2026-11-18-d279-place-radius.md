# Review: a place route now plans for the body's width - and it moved nothing measurable

*Change (branch `d279-substitute`): `plan_path_mode` gains `ignore_bodies`, and `plan_place(from, to, radius)` plans a place
route for the body's **own radius against the ground** while ignoring bodies. `start_gather` uses it. The conflation it removes
is real; the effect on the fixtures is **zero**.*

## Performance

`cargo test -p ruage-sim --no-fail-fast --test animals --test gather --test gather_motion --test gather_standoff`, before and
after:

| suite | before | after |
|---|---|---|
| animals | 10 / 1 | **10 / 1** (5.74 s) |
| gather | 22 / 0 | **22 / 0** (17.67 s) |
| gather_motion | 6 / 0 | **6 / 0** (2.69 s) |
| gather_standoff | 1 / 0 | **1 / 0** (0.81 s) |

**Identical.** So the 29 `Gather` refusals counted by task are **not** caused by the place route's radius: they survive the split.

## Stability

**What was attacked.** The two jobs one number was doing - "ignore the crowd" (D211) and "this body has no size" - are now separate
parameters, and nothing changed. That is a **negative result and it is worth the commit**: it eliminates the radius conflation as
the source, so the refusal must come from the other route a gather approach plans (the walk to the neighbouring tile) or from the
fallback's geometry rather than from the radius handed to the planner.

**Found and reported:** my previous message proposed this as "the fix". It is not. It is a correctness improvement with no measured
effect, and the 149,254 ground re-paths in the acceptance are still unexplained. The next probe must print, per refusal, **which
kind of route** the body is following (place / walk / fallback) rather than only its task - the task was already unanimous, so the
missing axis is the route's origin.

Known and accepted: parked. `main` untouched and green.
