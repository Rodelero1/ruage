# A worker beside its node, and a bush that cannot hide it (2026-11-02)

Scope: `crates/sim/tests/neighbours.rs` (new, 2 tests), `crates/view/src/lib.rs` (`draw_order` gains a two-pass layer,
`layer_of`), `crates/view/tests/depth.rs` (one new test).

From `make run`: *"the villagers upon approaching to the berry bush disappeared as if entering a building. They should
stand near the bush while foraging."*

## Which half was wrong

Two candidates, with different fixes: the simulation could be walking the gatherer **onto** the node's own tile (drawing
right, order wrong), or the renderer could be painting the bush **over** a correctly positioned villager.

`a_gatherer_stands_beside_the_node_and_not_on_it` settles it without a window: seed the pack, order the nearest node,
run 600 ticks, assert the villager's tile is Chebyshev **1** from the node's. It passes - so `work_position` is doing its
job and the fault is drawing. `a_gatherer_stays_visible_to_its_owner` covers the other way to disappear (leaving the
visible set) and also passes.

## The drawing fix

`draw_order` sorted by depth alone, so a bush at greater `x + y` painted over a worker behind it. This renderer has no
sprite heights, so a low bush cannot be trusted to occlude anything. Order is now **(layer, depth, id)**: layer 0 is
ground cover (resources), layer 1 is everything else. Buildings keep covering units behind them - the existing
`a_unit_behind_a_building_is_covered_by_it` still passes, and an earlier attempt that put units on top of everything
broke exactly that test, which is why the rule is two passes and not four layers.

## Performance

`draw_order` sorts the visible entities once per frame with a three-key comparator instead of a two-key one: **one extra
`match` per comparison**, over a vector that is already collected and sorted. `layer_of` is a match on a `Copy` enum,
inlined. No allocation was added, and the sort's asymptotics are unchanged (`n log n` over the same `n`).

Measured: the `ruage-view` suite (depth + fog + frame cost) runs **13 tests in ~3 s**, unchanged from before the edit;
the sim suite gained two tests in `neighbours` that run in **0.88 s** for a 600-tick gathering run.

## Stability

- **A mutation proves the new test bites**: replacing `layer_of(a).cmp(&layer_of(b))` with `layer_of(a).cmp(&layer_of(a))`
  (i.e. no layer) makes `a_worker_behind_a_bush_is_not_hidden_by_it` fail with its own message, and restoring it passes.
- **The two existing depth tests are the guard rail**: units still go behind buildings, and back-to-front order within a
  pass is unchanged, so the change cannot quietly reorder the world.
- **The sim is unchanged.** The gathering rules were already right; what this commit adds there is a test that says so,
  which is what the owner's report needed to be traceable.

## Not attacked, and named

- **Overlap and evasion** (villagers passing through each other) is untouched: D52 records that units may share a tile
  and nothing pushes them apart. That is a tick-loop change with its own review, not a drawing fix.
- **`movement.toml`'s `cost_straight` / `cost_diagonal`** are read by nothing - `path.rs` hardcodes 10 and 14. Dead
  config that looks like the control for the diagonal behaviour the owner saw; wiring or deleting it is the next small
  slice.
- **The window path is still untested**: this container has no display, so `crates/view/src/bin/window.rs` is
  compile-verified only.

## My own error

The patch tool's fuzzy matching **duplicated the whole of `crates/view/tests/depth.rs`** when the anchor was a doc
comment at the end of the file: a second copy of the header appeared mid-file and the crate stopped compiling
(`expected outer doc comment`, `repo` defined twice). Rebuilt the file from the original plus the new test, and the
lesson - never anchor an append on a doc comment, and read the file after appending - is the same one already recorded
for derive/use blocks.

## Addendum: the adversarial review he asked about, honestly (2026-11-02)

The question was whether this slice got an adversarial review for **performance and stability**, not whether a review
file existed. Answering it properly found that the first pass did not: it had one mutation and an *estimated* frame
number. What follows is what was measured and attacked.

### Performance, measured

| what | number |
|---|---|
| a frame at 960x720 with 126 entities (debug build) | **6,508 us** and **8,154 us** on two runs - the spread is ~25%, so frame time is the wrong instrument for a per-comparison change |
| sort of 10,000 entries, three keys derived inside the comparator | **472.3 us** |
| the same sort with the keys precomputed into a vector first | **686.8 us - 0.69x, i.e. slower** |

The second row is the finding: **precomputing the sort keys was tried and reverted.** The reasoning for it was sound in
principle (a comparator runs `depth_of` O(n log n) times) but wrong in practice, because a vector of `(layer, depth, id,
&view)` costs an extra allocation and copy that outweighs a `match` on a `Copy` enum. The comparator form shipped, with
the number in a comment so the next session does not "optimise" it back.

A first attempt at this measurement compared a two-key `(f32, u32)` tuple against a three-key `(u8, f32, u32)` one and
reported "3.2x slower" - that measured **tuple size**, not the extra key, and it is recorded here because it was a
plausible wrong answer that nearly became a decision.

### Stability, attacked

- **Mutation**: with `layer_of` neutralised (comparing a tuple with itself), `a_worker_behind_a_bush_is_not_hidden_by_it`
  fails with its own message; restored, it passes. So the new ordering rule is pinned, not merely present.
- **Regression guard**: the two pre-existing depth tests still hold - units are covered by buildings, and back-to-front
  order within a pass is unchanged. An earlier four-layer version (units above buildings) broke
  `a_unit_behind_a_building_is_covered_by_it`, which is why the shipped rule is two passes.

### Not attacked, and named

- **Same-depth ties**: a resource and a unit at exactly equal depth rely on the layer, and the layer decides - but no test
  constructs that pair. The bush test uses `(10,10)` against `(11,11)`, i.e. strictly greater depth.
- **Fog**: a *remembered* resource (drawn as memory) versus a visible unit is not tested; the layer applies to both, which
  is presumably right and is unverified.
- **Projectiles**: they are in the same layer as units, so an arrow at lower depth is drawn *behind* a unit. Probably
  harmless at a few pixels; not decided deliberately.
- **Frame cost with many entities**: the measurement is at 126 entities, which is what a scenario produces. A 1,000-unit
  fight would put the sort at the scale where the comparator's O(n log n) matters, and nothing measures that today.
