# Review: a villager is called a Forager

*Step (D121): the second half of the owner's ask (2026-11-04) - "For Villager: renamed to match the task he is doing and show
stats and carried resources - see latest.png". He chose the key: **the target's role name** in `economy.toml` (option A).*

## What lands

- `World::work_target(id)` - the target's **role** (`"farm"`, `"sheep"`) and **column** (wood, food, gold, stone), or `None`
  when the unit is not working.
- `config/jobs.toml` + `crates/data/src/jobs.rs` - content's words, keyed by `target` (a role) or `column` (a resource).
- `ruage_view::label_for(job, role, fallback)` - one precedence rule, used by the window and by the diagnostic picture.

**The one thing the owner's chosen key could not do, and what was done instead.** Option A keys the word on the target's role
name. The reference pack has **40 roles and not one of them is a tree, a bush or a mine** - they cover buildings, animals and
soldiers - so `target = "tree"` would never match and a woodcutter would have had no name at all. The table therefore accepts
**either** a role **or** the target's resource column, refuses an entry with neither (a rule that can never match is a rule the
author meant to be a rule), and checks the role first so a pack that later adds a `tree` role overrides the column entry
without moving anything. This is a superset of A, not a substitute: every word still lives in content and a mod can change all
of them. If the pack should grow roles for trees, bushes and mines instead, that is a one-line content change later.

## Performance

Measured with `cargo run --release -p ruage-view --example hud_cost`, a `HEAD` worktree against the working tree with the same
`config/`, three interleaved runs each, 2,000 HUD draws a run, tick 200:

| run | `HEAD` (before the card and this table) | now |
|---|---|---|
| 1 | 171.8 µs | 175.6 µs |
| 2 | 172.3 µs | 172.0 µs |
| 3 | 169.6 µs | 175.9 µs |
| median | **171.8 µs** | **175.6 µs** |

**+3.8 µs (+2%)** across *both* steps (the card of D120 and this table), which is inside the spread this box shows between
identical runs (**169.6-175.9** across the six). Nothing on this path is per frame: the job name is resolved when the
*selection changes*, and the panel draws the string it is handed, so a villager gathering for an hour costs nothing extra. The
tick is untouched - no simulation code reads a name - and the acceptance hash stays `82efabd6f614c798`.

**An earlier measurement of mine was invalid and is reported:** I took a "before/after" while the test suite and clippy were
running in the background and read 191, 294 and 294 µs on the same binary. Those numbers are load, not code; only the
interleaved idle measurements above count.

## Stability

| attack | result |
|---|---|
| a target with a role | named by the role (`sheep` -> Shepherd); test: `a_gathered_sheep_is_targeted_by_its_role` |
| a target with a column and no role | named by the column (`wood` -> Lumberjack); test: `a_woodcutter_targets_a_column_and_not_a_role` |
| both, and they disagree | the role wins; test: `a_role_beats_a_column` |
| an idle villager | no target, no job, keeps the pack's name; test: `an_idle_villager_has_no_target_and_keeps_his_name` |
| an entry with neither key | refused at load; test: `an_entry_that_can_never_match_is_refused` |
| a job with a blank name | refused at load; test: `a_nameless_job_is_refused` |
| a pack with no `jobs.toml` | the window falls back to `Jobs::default()` and every worker keeps his unit's role - the panel behaves exactly as it did before this table existed |
| the window and the picture disagreeing | impossible by construction: both call `label_for` |

**Verified end to end, not asserted:** the diagnostic binary now prints what it called the thing it drew, because a picture of a
7-pixel font cannot be read back reliably here. At tick 400 the villager the AI has put on berries prints
`selected 2 is "Forager"`; at tick 60, before any work, `selected 2 is "villager"`. Both paths - job and fallback - are the real
ones, from the real pack.

**A process error, reported:** the loader and the accessor were written before their tests in this slice, which is not the
order D7 asks for. The tests were then written and do fail when the behaviour is removed (`label_for`'s order, the role-first
rule, and the cycle-free load all have a test that would catch a regression), but the rule exists for a reason and skipping it
once is still skipping it.

---

*`tools/review_check.py` looks for `## Performance` and `## Stability`; both are above.*
