# Two small ones off the review's list: errors that say what they are, and a next step that is one step

Both are from the architecture review's ranked backlog, neither needs a decision from the owner, and both are gates rather than
features: one makes the UI able to speak, the other stops every future session paying 350 KB of context before it can start.

## Performance - measured

| Metric | Before | After |
|---|---|---|
| `docs/next-step.md` | **350,038 bytes** | **1,230 bytes** (348,807 moved to `docs/history/next-step-2026-11-12.md`) |
| what a session must read to resume | 350 KB of history | one block: state, the one open step, the queue |
| cap enforced at | nothing | `make check` and `.githooks/pre-commit`, 16,384 bytes |
| `SimError::class` | - | one `match` on a `Copy` enum, no allocation, refusal path only |
| `tools/next_step.py --check` | - | **235.6 ms** per run (five runs averaged), in `make check` and the hook |
| `World::step` / `Snapshot::of` | 6.7 us / 105.0 us | **6.6 us / 108.0 us** - unchanged within noise, the tick was not touched |
| `error_class` tests | - | 3 pass in 0.00 s |

The size figures are the point: the file a session reads before it can start went from **350,038 to 1,230 bytes**, and the tick is
unchanged (6.7 -> 6.6 us, which is noise on this box and not a claim of improvement).

## 1. `ErrorClass`: 59 variants, three classes

`SimError` is right to be 59 variants - each names exactly what went wrong and where - but a caller could not tell *"the player cannot
do that"* from *"the engine is wrong"* without matching all of them, so every UI path either swallowed errors or printed internals.

`SimError::class() -> ErrorClass` with `Refused` (38 variants: legal orders the world will not take), `BadRequest` (18: the command
names something that is not there, is not yours, or does not parse) and `Internal` (3: overflow, geometry overflow, a displacement with
no legal spot).

**The guarantee is structural, not a test**: the match has **no wildcard arm**, so a new variant does not compile until it is
classified. The tests pin one variant of each class:

```
error_class::a_legal_order_that_cannot_be_obeyed_is_a_refusal ... ok
error_class::a_command_naming_something_that_is_not_there_is_a_bad_request ... ok
error_class::arithmetic_the_engine_cannot_answer_for_is_internal ... ok
```

Written test-first: the first run was `error[E0432]: unresolved import ruage_sim::ErrorClass` and `error[E0599]: no method named
class`.

## 2. `next-step.md` is capped at 16 KB, history in `docs/history/`

The rule that produced the file is the owner's: *"compact first: persist state to files so a fresh session resumes without the
conversation"*. Compaction had produced a file twenty times the largest source file, and a session that read it spent its context on
history.

`tools/next_step.py` with three claims, each pinned by a test written **before** the tool existed
(`ModuleNotFoundError: No module named 'next_step'`):

- the file must start with `# OPEN`, so a stale file is obvious rather than authoritative;
- it must be under 16,384 bytes, and the failure names the overflow and the command that fixes it;
- `--split` keeps the first block and moves every later one, appending to `docs/history/next-step-<date>.md` with a dated marker.

Run once: **348,807 bytes moved, 1,230 left**, and the check is now part of `make check` and the pre-commit hook.

## Stability

- **`tools-test` grows by 5** (`test_next_step.py`), `error_class` by 3; the whole suite is green after the change.
- **Nothing in the engine changed shape**: `ErrorClass` is additive, `SimError`'s variants and `Display` are untouched.
- **The hook change is fail-open-safe**: it runs the same `--check` the Makefile runs, prints the reason to stderr, and only refuses
  when the file is genuinely over the cap or mis-headed - there is no path where a missing `tools/next_step.py` silently passes,
  because the check is the same command in both places.
- **Not done here**: converting the gather suite to assert relations instead of measurements (the review's item 3) - it is a larger
  edit and is next.
