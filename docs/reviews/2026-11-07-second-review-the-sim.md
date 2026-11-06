# The second review's sim block: 63 findings, a verdict each

*2026-11-07. The loader block of 41 was the first slice (v1.103 / v1.104); this is the block filed against `crates/sim/src/` -
the engine's own crate, and the one the review's severities were highest on (2 critical, 13 high). Every finding was read
against the **current** tree, and **seven were fixed in this pass** - not the highest-severity ones, the ones that were
cheap, certain, and testable.*

## What the block actually is

| file | findings |
|---|---|
| `script.rs` | 19 |
| `bin/batch.rs` | 14 |
| `bin/headless.rs` | 12 |
| `metrics.rs` | 9 |
| `path.rs` | 9 |

**Severity as filed**: 2 critical, 13 high, 33 medium, 15 low. **Category**: 33 bug, 19 maintainability, 10 performance, 1
documentation.

Note what the block *is not*: it does not touch `lib.rs`, where `step`, the mover and the economy live. It is the script
layer, the two CLI binaries, the metrics watcher and the path module. Two of the nineteen `script.rs` findings were already
closed **before this pass** (`nearest_resource`'s per-node `can_reach`, fixed tonight as D138) and several were fixed by
earlier decisions.

## A class-level correction, again: `f64 as i64` does not wrap

Five findings are built on `as` doing something it does not do. `f64 as i64` and `f64 as u32` **saturate** since Rust 1.45:
`1e300 as i64` is `i64::MAX`, not a wrapped negative. So:

- the claim "an overflow can panic in debug" (`metrics.rs:156`) is **wrong** — nothing overflows in a float-to-int cast;
- the claim "the error path therefore never fires" (`script.rs:287`, `script.rs:307`) is **right**, and for the opposite
  reason: it never fires because the cast *succeeds*, silently clamped — which is still a defect, and it is one of the six
  fixed here;
- the claim "`Infinity as i64` becomes `i64::MIN`" (`headless.rs:181`) is **wrong**: `f64::INFINITY as i64` is `i64::MAX`.

The distinction matters because "it wraps" invites a masking fix (`& 0x7fff…`) while "it saturates" wants a range check.

## The seven fixed in this pass

All seven were cheap, and every one with a behaviour change left the acceptance match **byte-identical**
(`f328ebc8d4d34b78`, 141 entities, 188 repaths, 187 made-room, 11.5 s).

### 1· `path.rs:127` — a body standing on an occupied tile could not see a clear line *(medium)*

`find_path` allows a **blocked start** on purpose (*"a villager stands on the tile of the tree it is working, and must be
able to walk off it"*); `line_clear` refused the same line, so every unit standing on or beside a node paid a full A*
search for a line it could see. The first tile is now exempt.

**RED**: `the start tile is where the body already is: it cannot be an obstruction to its own line`.

### 2· `path.rs:127` (same lines) — a second bounds check per tile *(low, performance)*

`blocked()` already returns `true` off the map, so `contains() && !blocked()` cost two bounds tests for every tile of every
straight-line check. One.

### 3· `path.rs:33` — `Walk::from_map` trusted `terrain.len()` *(medium)*

`Map` is public and constructed directly in tests and fixtures; a terrain array that does not match `width * height`
silently shifted every tile's ground. A `debug_assert_eq!` now names the numbers. Release builds: zero cost.

**RED**: `a map's terrain must be width * height: 29 entries for 10x3` (the `#[should_panic]` test did not panic before).

### 4· `script.rs:1162` — `has_unit` matched one unit id, not the family *(high)*

`has_unit = "militia"` went **false** the moment the militia became a man-at-arms, silently disabling a rule the author
wrote correctly. It asks `is_family` now, which is what `building_count` and the attack rule already did.

**RED**: `a man-at-arms is militia's family: the condition must hold and the house go up (it was 0)`.

### 5· `script.rs:583` — an attack step naming an upgraded role found nothing *(high)*

Same shape: `attacker = "militia"` refused with *"player 1 has no unit militia"* while the man-at-arms stood there.

**RED**: `the step must find the family, not one id: Some("player 1 has no unit militia")`.

### 6· `ruage-batch` was documented and never built under that name *(high)*

`batch.rs` names itself `ruage-batch` in its own USAGE string, and `docs/next-step.md`, `SPEC.md` and
`config/acceptance.toml` all tell a reader to run `ruage-batch`. The file is `src/bin/batch.rs` with one explicit `[[bin]]`
for the headless runner, so cargo built it as **`batch`** — verified: `ls target/release` showed `batch`, and `ruage-batch`
did not exist. Every documented invocation was wrong. Fixed by renaming the file to `src/bin/ruage-batch.rs`, so discovery
names it correctly; `cargo build --bins` now produces `ruage-batch`, and a live run prints the same
`#f328ebc8d4d34b78`.

### 7· `script.rs:271/287/307` — a runner with no clock, and times that saturate *(high)*

`Runner::new` accepted `tick_rate_hz = 0` (every time then rounds to tick 0, and the reporting path divides by the rate), and
accepted `at_seconds = 1e300` as "in 9.2e18 ticks" — a run that can never reach it, with the refusal message that was
supposed to catch exactly that being unreachable. Both refuse now, in the words the message already used.

**RED**: `zero ticks a second is not a clock`, `a step at 1e300 s is not a tick`.

## The other 56, by verdict

**Refuted (6).** The mechanism is wrong or the premise is:

- `path.rs:56-72` "saturating counters silently leave the count wrong" — **deliberate and documented in the comment it
  quotes**; the alternative (wrapping) turns a missed unblock into a tile blocked by 65,535 things.
- `path.rs:93-100` "`octile`'s `as u32` truncates" — the maximum is `(255+255)` tiles of cost ≤ 10, i.e. **~3,610**, four
  orders inside `u32`.
- `path.rs:50-53` "`index` returning `None` hides caller bugs" — that `None` *is* the bounds guard, and `blocked()` treats
  off-map ground as blocked, which is correct for every caller.
- `metrics.rs:156` "the multiply can overflow/panic" — `f64 → i64` saturates; the real bound is `delivered * 60 * rate`
  ≤ 10⁶ × 60 × 20, seven orders inside `i64` for any match this engine runs.
- `headless.rs:181` "`Infinity as i64` becomes `i64::MIN`" — it is `i64::MAX`. (The *finding* under it stands: a huge
  duration is a hang rather than a refusal, and it is on the next-slice list.)
- `script.rs:623-635` "`nth_villager` must use `is_family`" — the pack defines **one** villager role, and 5.7 has no villager
  upgrade; `is_family` here would match by family, which is a wider net than the order means. Left as the exact id, with
  this note.

**Confirmed, deferred with a reason (26).** Real, and each costs a design decision or a measurement rather than a patch:
the `BTreeMap`-per-search and no-node-budget findings in `path.rs` (the proven fix is a flat `Vec` buffer and a nearest-point
fallback, and D134 already gives the fallback); `is_free_ground`'s linear scan and the placement loops in `script.rs`
(measured cost is inside the rule pass's 840 ms worst case that D-script-rules already trimmed, and a spatial index is its
own step); every CSV/escaping/`println!` finding in the two binaries; every "preserve the error type instead of a `String`"
finding (a deliberate choice — the outcome types are for reports, and `describe()` exists to render them).

**Confirmed, cheap, and left for the next slice (24).** These are the ones I would take next, in this order:

1. `script.rs:1533/1541` — **`enemy_building` and `enemy_town_center` can target a Gaia building or a foundation.** Neither
   filters `player != 0` (`enemy_building` does, `enemy_town_center` does not) nor `construction.is_none()`. A rule told to
   attack the enemy town centre can therefore be pointed at a Gaia one, or at a half-built foundation.
2. `script.rs:1103` — `when_holds` returns `false` for a role, tech or resource the pack does not define, disabling a rule
   **forever** with nothing in `rule_refusals`. Same family as D137: a typo should stop the load, not the rule.
3. `batch.rs:192` / `headless.rs:181` — a `duration_seconds` of `1e300` becomes a loop of 9.2e18 iterations: a hang, from a
   number a file can carry. The same `tick_at` guard I added in `script.rs` applies.
4. `headless.rs:300` — `players=` reports the **highest player id**, not the count: a match with players 1 and 3 prints
   `players=3`.
5. `headless.rs:165` — supplying only one of `--techs`/`--effects` silently skips research loading.
6. `batch.rs:141` / `batch.rs:213` — `--player` unvalidated and unknown resource names coalesced to a sentinel, both
   producing all-zero metrics that look like a result.

## Performance

- **The acceptance match is unchanged**: `f328ebc8d4d34b78`, 141 entities, 188 repaths, 187 made-room, 574 refusal lines —
  and **11.47 / 11.51 s** for 36,000 ticks over two idle-box runs (the 16.97 s I reported earlier came from a run whose
  compile overlapped the test suite; that number is withdrawn).
- **The one measured change is a removal**: `line_clear` no longer costs a second bounds check per tile, and a unit standing
  on a node no longer falls back to A*. Neither shows in the shipped match's wall clock, which is the honest way to say it:
  the effect is real and too small to see at 141 entities.
- Everything else in this block is unchanged code.

## Stability

- **Six behaviour changes, all refused-or-allowed at the edges**, none of them in the tick: two validations at load, two
  family lookups, one exemption of the tile a body already occupies, one debug-only assertion.
- **The acceptance hash proving nothing moved** is the strongest statement available here: if any of the six had altered a
  decision the match makes, the hash would say so. It does not.
- **The one with a real risk was `line_clear`** — a straight line accepted where a search used to run can change which route
  a unit takes. Measured: hash unchanged over 36,000 ticks, 141 entities, 188 repaths.
- **Two new test files** (`walk_line.rs`, and four tests in `rules.rs`) fail on the old code for the exact reasons quoted
  above; the `#[should_panic]` one is gated on `debug_assertions` so a release test run does not report a pass it did not
  earn.
