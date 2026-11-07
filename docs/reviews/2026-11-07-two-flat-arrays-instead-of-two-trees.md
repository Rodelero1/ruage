# Two flat arrays instead of two trees (D143)

*2026-11-07. Step 4 of four from the prior-art watch. Of the four pathing techniques on the list — landmark A* estimate,
search budgeted by route length, per-tick search budget, re-sort last tick's order — **one applies to this tree today and
three need a decision this step should not smuggle in**. This is the one.*

## Why only one

- **Landmark A\* estimate**: an ALT heuristic needs distances computed *to* a landmark set from every tile, i.e. a
  precomputation per map and a memory budget — a design piece, and its only payoff is on long searches our maps barely
  have.
- **Budget by route length / per-tick search budget**: both truncate a search, which means a route that used to exist
  sometimes does not. That is a behaviour change and a decision (the same shape as D141, where the last "cheap" bound cost
  12 entities), not an implementation detail.
- **Re-sorting last tick's order**: we do not sort an order per tick; the pass reads the entity list in id order.

What was left is the one the second review also filed against `path.rs`: **the search allocated two `BTreeMap`s and paid a
tree comparison per lookup**.

## The change

```rust
let mut best: Vec<u32> = vec![u32::MAX; cells];   // g-cost per tile, indexed by y * width + x
let mut came: Vec<u32> = vec![u32::MAX; cells];   // the parent's index, not its coordinates
```

`open` is untouched — same tuples, same comparison order — so **the routes are identical**, which is the property that makes
this an optimisation rather than a new algorithm pretending to be one.

## The measurement, and the instrument

`crates/sim/examples/path_cost.rs` (new): it builds the acceptance map's `Walk`, times `find_path` 2,000 times on three
different lengths, and prints an **order-sensitive checksum of every route**. The checksum is the point: a search that got
faster by finding different routes has not been optimised.

```
cargo run --release --offline -p ruage-sim --example path_cost
```

| search | before (`BTreeMap`) | after (flat `Vec`) | speed-up | checksum |
|---|---|---|---|---|
| corner to corner, 61 waypoints | **50.5 µs** | **8.9 µs** | **5.7×** | `5229cf3a75f30d00` both |
| mid, ~10 tiles, 10 waypoints | **5.2 µs** | **2.0 µs** | **2.6×** | `4382e97fe9580fd0` both |
| short, ~3 tiles, 2 waypoints | **1.2 µs** | **0.9 µs** | **1.3×** | `f627a98f64d14910` both |

Three lengths on purpose: a flat array costs `O(cells)` to zero, so a change priced only on the long case would be priced
wrong — and it does lose that cost on short searches, just not enough to lose the race (1.2 → 0.9 µs).

## Performance

- **The search is 5.7× faster where it matters least and 1.3× where it happens most.** Long searches dominate the cost, so
  the aggregate is the 5.7× figure for a repath storm and the 1.3× figure for a busy crowd of short ones.
- **The acceptance match does not move**: `f328ebc8d4d34b78`, 141 entities, 188 repaths, 187 made-room, 574 refusals,
  **11.50 / 11.48 s** — the same as before, and that is the honest result: 188 repaths in 36,000 ticks is **0.5%** of a
  tick budget at 50 µs each, so a 5.7× cut is worth ~0.04 s of an 11.5 s match. The win is for the scale the project says it
  is aiming at (thousands of units), not for this scenario.
- **Memory**: two `Vec<u32>` the size of the map per search — **32 KB** on a 64×64 map, allocated and dropped per search.
  The `BTreeMap` version allocated a node per *visited* tile (48 bytes plus two pointers each) and kept them until the
  search ended; on a corner-to-corner search that is ~4,000 nodes ≈ 300 KB. So this is also a **memory** win, and it is the
  one that makes the allocation bounded rather than proportional to the search's work.

## Stability

- **Routes are identical, and that is proved rather than argued**: three checksums, before and after, equal. The A* ordering
  (`Reverse<(f, g, y, x)>`) is untouched, so ties break the same way; a change to `open` would have moved the checksums.
- **The acceptance hash is unchanged**, which is the same statement at match scale.
- **No index can overflow**: `cells = width * height` as `usize`, and `came` stores a `u32` index. A map would need
  65,536 × 65,536 tiles to reach `u32::MAX`, which the loader refuses long before (D128's geometry guard).
- **A parent index is always a visited tile** or `u32::MAX`; the walk-back stops on `u32::MAX` *and* on the start index, so a
  corrupt entry ends the path rather than looping.
- **The one failure mode that is new**: `came` and `best` are sized from `walk.size()`, so a `Walk` whose `terrain` array is
  longer than `width * height` (a hand-built one — the loader refuses it) would index past the end and panic rather than
  misread. `Walk::from_map` asserts the length in debug builds (D139), so the panic is in the constructor, not here.
