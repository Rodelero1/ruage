# The second review's deferred items: a length that is a length, and half a table (D147)

*2026-11-07. The owner: "Proceed", with the queue at 118 of 205 findings verified. This pass takes the deferred items the
triage at `docs/reviews/2026-11-05-second-review-triage.md` left with a named remedy, and lands the two that are real
engine defects. **The review's own JSON is gone** — it lived in `/opt/data/cache/documents/`, which is pruned — so what
follows is worked from the surviving triage table and the code, not from the review's text.*

## [74] [173] [184] — a scenario's length, and the cast that ate it

**Confirmed, and worse than "a missing check".** Both runners computed their tick count with

```rust
u64::try_from((scenario.duration_seconds * f64::from(world.tick_rate_hz())).round() as i64)
```

and `f64 as i64` **saturates**: `NaN` becomes `0` and `1e300` becomes `i64::MAX`. So a scenario with `duration_seconds = nan`
played **zero ticks** and exited **0** — a match that reports success and does nothing — and `1e300` produced a match that
can never end. The `map_err(|_| "the scenario is longer than a u64 of ticks")` beside the cast was **dead code**, exactly
the shape D139 fixed for `at_seconds`.

**Fixed**: `Scenario::ticks(tick_rate_hz)` is the one definition (it calls the existing `tick_at`), `Runner::new` resolves
and stores it, `Runner::duration_ticks()` exposes it, and both binaries use it instead of their own inline cast. The loader
refuses a `duration_seconds` that is not finite or is negative; the product is bounded where the rate is known.

**RED, watched**: `crates/sim/tests/duration.rs`, four tests written before any production change —

```
test a_negative_duration_is_refused_by_the_loader ... FAILED
test a_nan_duration_is_refused_by_the_loader ... FAILED
test a_duration_too_long_for_a_tick_count_is_refused ... FAILED
test an_ordinary_duration_is_still_accepted ... ok

test result: FAILED. 1 passed; 3 failed
```

each failure `expect`ing an error that the code did not produce (the loader accepted `nan` and `-1.0`, `Runner::new`
accepted `1e300`). **GREEN**: 4 passed. The fourth test was written with a weaker assertion first (that a 60 s scenario is
still accepted) because `duration_ticks()` did not exist yet; it was strengthened to assert **1,200 ticks at 20 Hz** after
the API landed — so its RED is the *absence of the API*, not an assertion failure, and that is stated rather than implied.

## [112] — half a research table was skipped in silence

**Confirmed.** `headless.rs` chose its research path with

```rust
if let (Some(techs), Some(effects)) = (parsed.get("--techs"), parsed.get("--effects"))
```

so `--techs` without `--effects` took the `else` branch: **research was skipped entirely and the run reported success**. A
typo in one flag changed the match's behaviour without a word.

**Fixed**: an explicit `match` on the pair — both load, neither is fine, and one alone is an error naming the missing flag.

**RED, watched** (`crates/sim/tests/headless.rs`, both directions):

```
thread 'half_a_research_table_is_an_error_and_names_the_missing_flag' panicked at crates/sim/tests/headless.rs:290:9:
--techs without --effects must be refused, not silently skipped
test result: FAILED. 0 passed; 1 failed
```

**GREEN**: 9 passed in `headless`, the new test included. Its first draft failed for the *wrong* reason (`--seed is
required`, so the run never reached the research block); that is recorded here so the final RED text is not mistaken for
the first attempt's.

## Checked and **not** changed, with the reason

- **[70] `--dump` CSV is unescaped.** True of the code, but the `task` column is `{:?}` cut at its first space or `{`, then a
  single `EntityId(n)` appended — no value it can produce today contains a comma, so no test can be watched failing. A
  defensive quoting would be untested production code, which the project does not accept; it stays queued with that reason
  rather than landing as decoration.
- **[71] `players=` reports the highest player id, not the number of players.** Confirmed by reading
  `headless.rs` (`.max()` over the non-zero ids). It is **unobservable in any fixture this repository can build**: seeding
  gives players `1..=n` contiguously, so the maximum *is* the count until a player loses every entity — which needs a
  sparse-player or a mass-death fixture, and that is a match-level test rather than a report test. Queued with that reason.
- **[177] `line_clear` starts at the `from` tile** — already fixed in D139 (`line_clear` refuses a line whose start tile is
  occupied while `find_path` allows a blocked start), so this finding is stale.
- **[125] [134] [135] path search budget and reachability** — the open design questions D141 refused to guess at; the
  two-stage gather search is queued, not fixed by a bound.

## Performance

**This change cannot affect the tick, and that is the point of the section rather than an excuse for one.** No per-tick code
path changed: `Scenario::ticks` runs **once**, at startup, and replaces a multiplication and a cast that ran once in the
same place. The added work is two comparisons (`is_finite`, `< 0.0`) per scenario load.

**What was measured, and what it is worth**: three acceptance runs after the change gave 11,651 / 11,640 / 11,713 ms, and
three repeats 11,657 / 11,589 / 11,659 ms. Those numbers are **not comparable** to D146's 10.69-11.03 s: `uptime` showed a
load average of **2.75-3.94 on a two-CPU cgroup** during this session, against an idle box for D146's A/B. The honest
statement is that the wall clock moved for reasons that are not this commit, and no claim is made from it in either
direction.

The acceptance **result** is unchanged, which is the measurement that matters here: `f328ebc8d4d34b78`, 141 entities, 188
repaths, 187 made-room - with `--ticks 36000` and, separately, with the script's own length (which is the newly shared
path: `config/acceptance.toml` declares 1,800 s, and 1,800 s at 20 Hz is 36,000 ticks).

## Stability

- **Two real defects, both fixed test-first, both RED watched.** `tests/duration.rs`: **1 passed / 3 failed** (the loader
  accepted `nan` and `-1.0`; `Runner::new` accepted `1e300`). `tests/headless.rs`: the run **exited 0** with half a research
  table. GREEN afterwards: 4 and 9.
- **A cast that cannot fail is the shape to look for.** `u64::try_from(x as i64)` reads like a guard and is not one, because
  `as i64` saturates. This is the second instance (D139 fixed it for `at_seconds`); the pattern is now removed from both
  runners and replaced by one `tick_at`-based definition.
- **The gate caught the new tests, twice.** clippy `-D warnings` failed on `.err().expect(...)` in `tests/duration.rs`
  (`err_expect`); fixed to `expect_err` where `Scenario: Debug` allows it, and left as `.err().expect(...)` only for
  `Runner`, which has no `Debug`. Gate after the fix: 733 passed / 0 failed / 19 ignored, clippy 0, fmt 0.
- **A test that failed for the wrong reason is reported as such**: the CLI test's first draft hit `--seed is required`
  before reaching the research block, so its first failure proved nothing about [112]. The `--seed 7` and the
  supplied/omitted flag pairing were fixed before the RED text above was taken.
- **One latent defect is deliberately left unfixed** ([70], CSV escaping) because no value the column can produce today
  contains a comma, so there is no way to watch a test fail on it - a defensive patch would be the untested production code
  this project forbids. Recorded in D147.4 with the same reasoning applied to [71].

## The review artifact is gone

The 205-finding JSON (`tool_1addba7f8001iwQ2fr4224VYC6`) was written to `/opt/data/cache/documents/`, which is pruned
periodically; it is no longer on disk. **41 (loader) + 63 (sim) findings were written into
`docs/reviews/2026-11-07-second-review-the-rest.md` and `…-the-sim.md` before it went**, and those survive with a verdict
each. The **49 tests / 26 config / 16 other** findings the triage never wrote out are **not recoverable verbatim** — they
can only be worked from the triage's own summaries. That is a process loss, stated plainly: a review's findings should be
committed to `docs/reviews/` as they are triaged, not read from a cache that expires.
