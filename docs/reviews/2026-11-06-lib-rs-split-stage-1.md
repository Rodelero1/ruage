# `lib.rs` split, stage 1: the file becomes a directory (D136)

*2026-11-06. The owner: "crates/sim/src/lib.rs is massive and should be broken down to smaller. Is it doable?" It is, and this
is the first slice - a pure move, proven by the acceptance hash rather than by inspection.*

## The state before

| | |
|---|---|
| `crates/sim/src/lib.rs` | **11,073 lines**: one `pub struct World` with **87 private fields**, one `impl World` with **244 methods** (91 `pub`, 153 private), and 267 lines of inline unit tests at the bottom |
| the rest of the crate | `script.rs` 1,592 · `short.rs` 368 · `path.rs` 331 · `metrics.rs` 183 |
| largest single function | **`step`, 1,559 lines** |
| next largest | `seed_from` 399, `state_hash` 345, `apply` 228, `step_shares` 217, `start_gather` 181, `collide` 178 |

The file had no section markers, so this split also creates the map it was missing.

## What moved in this slice

`crates/sim/src/world/movement.rs` - **collision**: `bodies_at`, `body_at`, `blocker_at`, `step_is_walkable`, `collide`,
`wait_or_give_way`, `order_target` (**354 lines**). `collide` is the only entry point the tick uses; the rest answer one
question about a candidate step.

To make the move compile, and only that:

- **4 fields** became `pub(crate)`: `entities`, `paths`, `scale`, `walk`
- **5 private helpers** that the moved code calls became `pub(crate)`: `find`, `tile_of`, `goal_of`, `tile_of_raw`,
  `body_radius`
- **2 constants and 3 structs** (`MAX_GROUND_REPLANS`, `MAX_REPATHS_PER_TICK`, `Repath`, `Route`, `Passing`) and their fields
  became `pub(crate)`. They were only crate-private already; the modifier says so out loud.

Nothing else changed. No logic, no ordering, no constants.

## Slice 2: `world/memory.rs`

`sighting`, `observe_for`, `observes`, `observe`, `vision`, `nearest_unseen`, `explored_mask`, `visible_to`,
`visible_mask`, `sight_of`, `covers`, `explored_bit` - **267 lines**, moved unchanged. `observe`, `sight_of` and `vision` are `pub(crate)`
because the tick and the tests call them; the rest are internal.

This slice needed **7 more fields** `pub(crate)`: `explored`, `map_size`, `memory`, `observers`, `tick`, `tick_rate_hz`,
`units` - and one doc comment had to follow its function: the `sighting` doc was left behind over `tile_of_raw` by the cut,
which the compiler would not catch and a reader would.

## Slice 3: `world/gather.rs`

`nearest_like`, `gather_source`, `start_gather` - **345 lines**, moved unchanged. All three are `pub(crate)` (the retarget
path and `apply` call them). This slice needed:

- **7 fields**: `animals`, `carcasses`, `economy`, `herdable`, `pending_gather`, `units` (`tick_rate_hz` was already exposed)
- **5 helpers** `pub(crate)`: `is_farm_role`, `modifiers_for`, `nearest_free_farm`, `plan_path`, `work_position`
- **1 free function** `pub(crate)`: `scale_fixed`
- and `MaybeFixed` stopped being used in `lib.rs`, which the build said out loud as an unused import - the kind of tidying a
  move does for free

## Performance

A move cannot change behaviour, and the number that proves it is the acceptance hash:

| | before the move | after |
|---|---|---|
| acceptance, 36,000 ticks, seed 7 | `f328ebc8d4d34b78` | **`f328ebc8d4d34b78`** |
| entities / repaths / made_room | 141 / 188 / 187 | 141 / 188 / 187 |
| wall time, release | 11,828 ms median of three | **10,540 ms** (one run, same machine, warm) |

The hash is the claim; the timing is a single run and is not evidence of improvement.

`crates/sim/src/lib.rs`: **11,073 → 10,719 → 10,449 → 10,103 lines**; `world/movement.rs` **369**, `world/memory.rs`
**280**, `world/gather.rs` **357**. Every slice leaves the hash at `f328ebc8d4d34b78`.

**711 passed / 0 failed / 20 ignored**; clippy and fmt clean.

## Stability

| attack | result |
|---|---|
| behaviour changed by the move | impossible in principle; the acceptance hash is identical, so the same match to the tick |
| a field made `pub(crate)` that should not be | the modifier is crate-internal: no downstream crate can see it, and `ruage-sim`'s public API is unchanged |
| a moved method now private by accident | `collide` is `pub(crate)`; the compiler refuses the tick otherwise, and the build is clean |
| a method left behind that the moved code needs | the build fails: this slice argued with the compiler three times (`Entity`, `EntityKind`, then an unused import) before it was clean |
| the inline unit tests at the bottom of `lib.rs` | untouched, still there, still passing |
| the next slice | `world/memory.rs`: `sighting`, `observe_for`, `observes`, `observe`, `vision`, `nearest_unseen`, `explored_*`, `visible_*`, `sight_of`, `covers` - which needs 7 more fields `pub(crate)` (`explored`, `map_size`, `memory`, `observers`, `tick`, `tick_rate_hz`, `units`) |

## Not done, and deliberately

- **`step` is still 1,559 lines.** Splitting files does not shrink a function. Cutting `step` into named phases is stage 2,
  and it is the change that actually helps a reader - with the same tripwire (the hash must not move).
- `script.rs` (1,592 lines) is the next file with a claim on this treatment.
