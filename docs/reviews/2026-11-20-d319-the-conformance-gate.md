# Review: the conformance gate was dead, and one divergence row was stale (D319)

*Change: the conformance crate **did not compile** - `Ground::search_reporting` gained a `reach` argument in D306 and
`crates/conformance/tests/budget.rs` still called it with four - so none of its rows had run since. Fixed (3 call sites), one
`#[ignore]`d divergence row that D306 had made false is un-ignored and rewritten, and the crate's **build** joins `make check` so this
cannot rot silently again.*

## What was actually wrong

Asked whether conformance tests are still skipped. The answer was worse than "yes":

```
error[E0061]: this method takes 5 arguments but 4 arguments were supplied
   --> tests/budget.rs:63:38, 76:38, 95:37
error: could not compile `ruage-conformance` (test "budget")
```

The crate is its own workspace (D247) so that a **red** conformance rule can never block a commit - the right call - but nothing in
the gate so much as **built** it, so a signature change in `crates/sim` took the whole set out of service and no run said so. Its own
comment ("no test in it is `#[ignore]`d") was also no longer true: `divergences.rs` had one.

## What landed

- `budget.rs`: three call sites pass `reach = 0`, which is the tile search's own value and the subject of those rows.
- `divergences.rs::the_original_plans_for_the_body_radius` -> `the_search_grows_obstacles_by_the_bodys_radius`, **un-ignored**. Its old
  claim ("the radius lives in the step, not in the planner") was made false by D303-D306, and its old body asserted the radius-blind
  route was a straight line through the gap. It now asserts the measured rule on both sides: a body a tile wide is planned **through** a
  one-tile gap (tangency is a passage - the owner's trebuchet measurement), and a body 1.2 tiles wide is **not**.
- `the_original_solves_a_long_walk_in_its_hierarchy`: doc only - it still said "not ported yet, P11 is building the levelled search",
  which landed in D296-D302. The test itself was already passing.
- `Makefile`: a `conformance-build` target (`--no-run`) added to `make check`; the run stays out of the gate, exactly as D247 wants.

## Test-first

The new radius row was **watched failing for the right reason**: with `reach = 0` (the old, radius-blind behaviour) substituted for the
wide body, it prints

```
a body 1.2 tiles wide was planned through a one-tile gap: Some([(Fixed(190000), Fixed(130000))])
```

so both halves discriminate. Not a fresh witness for engine behaviour - the engine's own are `radius_route` and `straight_line` - but
the row is live rather than decoration, which is the thing being fixed here.

## Performance

- **The conformance set: 21 tests, 21 pass, 0 ignored** (was: did not compile; before that, 19 pass / 1 ignored with 2 rows parked).
  The whole set runs in under a second, all in `target/`.
- `make conformance-build` adds one `cargo test --no-run` over a 7-file workspace to `make check`; measured cold it compiles in ~9 s and
  warm in under a second, which is noise beside the workspace suite.
- The **main** workspace suite is untouched by this change (21.1 s of target-compile aside): nothing in `crates/sim` moved, so the
  numbers from D318 stand (**928 / 0 / 21**).
- `fmt --check` clean; the crate builds at `--no-run` with no warnings.

## Stability

- **The root cause was a gate with a hole in it**, not a broken test: a separate workspace that is never compiled. The build step closes
  it without importing D247's hazard (a red rule blocking a commit) - `--no-run` cannot fail on an assertion.
- The un-ignored row is a **rule** row now: if the search ever loses its radius awareness again, this set says so, where before the
  `#[ignore]` said the opposite and in the wrong tense.
- **Still ignored, and named rather than hidden**: the main workspace keeps **21** `#[ignore]`s - **19 instruments** (perf and
  measurement probes that only print, plus the two four-hour determinism runs) and **2 real gaps**:
  `overlap::a_passer_by_does_not_move_the_seated_gatherers` (D238's seat churn) and `head_on::a_stopped_friend_is_not_walked_through`
  (R1b). Both have their causes in `docs/next-step.md`-adjacent decision rows and neither is a fixture the owner asked for.

Known and accepted: the crate is still not *run* by the gate (D247), so a red rule there is still invisible unless `make conformance` is
run by hand - which is the design, and now the only way it can rot is a red row rather than a missing build.
