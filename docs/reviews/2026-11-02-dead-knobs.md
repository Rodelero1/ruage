# Deleting the dead pathing knobs (2026-11-02)

Scope: `config/movement.toml` (two weights removed, a comment explaining the absence), one test label in
`crates/data/src/lib.rs`, and D98 in `docs/decisions.md`.

**Why delete rather than wire.** `movement.toml` carried `cost_straight = 1.0` and `cost_diagonal = 1.41421356`; `path.rs`
hardcodes `CARDINAL = 10` and `DIAGONAL = 14`. Nothing read the file's numbers - `movement.toml` is not loaded at all -
so the two were advertising a control that did not exist, and they were the first thing a report about diagonal movement
reached for (the owner's own, from `make run`). The ratio is a property of the grid: sqrt(2) on an eight-way grid is not
something a content pack has an opinion about, so it belongs in the pather.

## Performance

No code path changed: the pather still reads its two `const`s. A `cargo test -p ruage-data` run is **16 + 9 tests in
5.1 s**, unchanged in shape; clippy over the workspace finishes clean in **3.7 s** of recheck.

The honest measurement is that there is nothing to measure - and that is the point of the deletion: a knob that costs
nothing because nothing reads it is a knob that will mislead the next reader for free.

## Stability

- **The file still parses**: `movement.toml` has no loader, so removing two keys cannot break a load; the test that used
  `cost_diagonal` as an *example* of a value too precise for the fixed-point domain keeps its purpose and now names a
  neutral key (`pathing.weight`), because the knob it named no longer exists.
- **Nothing can regress quietly**: if someone later wants per-pack pathing costs, they have to add a loader first, and the
  comment says where the constants live.

## Not attacked, and named

- **`movement.toml` is still not loaded.** The file's `[pathing]` and `[collision]` sections are content the engine does
  not read; deleting two keys makes the gap honest but does not close it. When the pather needs tuning, that is the
  commit that adds a loader - and then the weights can come back *with* a consumer.
- **The owner's diagonal observation remains unexplained** by costs, because costs are right. The remaining candidates are
  the step granularity and the route itself; the "wait and let pass" work will touch the same loop and is the natural
  place to look.
