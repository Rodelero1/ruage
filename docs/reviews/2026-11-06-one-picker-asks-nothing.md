# One picker, and it asks nothing (D138)

*2026-11-06. The owner chose **B** on the reachability question: no reachability test in the picker — the order's own plan
is the check, and a failure is remembered. This is that change, and the two pickers it merges.*

## Two pickers, one rule already measured

`World::nodes_of_slot` is the economy pass's picker, and its doc has carried the answer since 2026-10-29:

> **No reachability test here, deliberately.** `can_reach` is a full path search; asking it of six nodes once a second per
> player cost **37x** the whole rest of the run (400 game-seconds: 319 ms without this pass, 11.7 s with it). `start_gather`
> plans the walk and is the one authority on whether a node can be worked, so this returns a few candidates in distance
> order and the caller stops at the first that starts. A node that fails is remembered (`share_failed`) instead of being
> retried every second — 0 A.D.'s `lastFailedGather`.

The **script's** `nearest_resource` kept a second copy that did the expensive thing:

```rust
.filter(|entity| world.can_reach(here, entity.position))   // a path search, per node, per call
```

Two problems, and the second is worse than the cost:

1. **A search per candidate.** The acceptance map has **90** wood nodes, so one `do = "gather"` rule could spend 90 path
   searches to choose between them.
2. **It asked the wrong question.** `can_reach(node.position)` — but a tree's own tile is *blocked ground*, so the honest
   answer is "no" for every tree in the map. `world/gather.rs` records that the first version of exactly this sent every
   lumberjack to Idle.

## The change

`nearest_resource` is now three lines and a delegation:

```rust
fn nearest_resource(world: &World, from: EntityId, slot: usize, nth: usize) -> Option<EntityId> {
    let here = world.position(from)?;
    world.nodes_of_slot(here, slot, nth + 1).get(nth).copied()
}
```

`nodes_of_slot` became `pub(crate)`. One picker, one rule, and the expensive-but-wrong test is **deleted** rather than
duplicated.

The authorities stay where they were — `start_gather` plans, `validate` refuses — and both now **book the failure**:

- `start_gather` inserts `share_failed` when its plan fails;
- `submit_checked` inserts it when `validate` refuses a `Command::Gather` with `NoPath`.

The second one is the one that matters for scripts, and it was the bug this step found: a walled-off node is refused at
**order** time, and `check_order` is `&self` — it cannot book anything. Without that entry the picker walked straight back
into the same tree on the next cooldown.

## The test, and both REDs

`crates/sim/tests/gather_pick.rs`: a wood block **behind a palisade line** (near), a reachable block on the villager's side
(far), and a gather rule with a one-second cooldown. Three assertions, because one would not pin the decision:

1. the picker **takes the walled node** — it asks nothing, so the engine refuses it;
2. the **far block is worked** — a refusal is not a dead end;
3. the refusals are **bounded by the walled nodes, not by the firings** — the memory works.

**RED, watched, first** (`cargo test -p ruage-sim --test gather_pick`):

```
the picker must take the nearest node and let the engine refuse it; it refused nothing, so it pre-filtered
test result: FAILED. 0 passed; 1 failed
```

**RED, watched, second** — after the picker changed but before the bookkeeping existed:

```
the rule was refused 29 times in 30 s for nine walled nodes: something is retrying the same node
test result: FAILED. 0 passed; 1 failed
```

That second one is the useful failure: it says the memory, not the picker, was the missing half.

**Honest note on assertion 2:** it has **no** RED of its own. Its first version compared the far block's wood against a
threshold I had guessed in "wood" when node amounts are the engine's raw units, and it failed at 7,800,000 for a reason that
had nothing to do with the code. It now measures the block **before** the run and asserts a **decrease**, which is the
assertion that was meant, and it passed the moment it was written correctly — so it is a guard, not evidence of a defect.

**GREEN**: `1 passed`.

## Performance

- **Searches in the picker: 90 -> 0** per gather-rule firing on the acceptance map (90 wood nodes; `nodes_of_slot` walks the
  same list and sorts it, which is arithmetic, not a search).
- **The same pattern's whole-run cost, measured 2026-10-29**: **319 ms -> 11,922 ms** over 400 game-seconds — **37x**.
- **Searches actually spent now: one per gather attempt**, inside `start_gather`, which has to plan the walk anyway.
- **The acceptance match is byte-identical** — `f328ebc8d4d34b78`, **141** entities, **188** repaths, **187** made-room,
  **574** refusal lines. `config/acceptance.toml` has no gather rule, so this change is for the packs that do; that the hash
  does not move is the proof that nothing else did.

## Stability

- **The memory cannot grow without bound.** `share_failed` is keyed by node and overwritten on each failure; it is cleared
  when `set_villager_shares` installs a new policy, and entries expire after `share_retry_ticks` (600 ticks, 30 s) by
  comparison in `nodes_of_slot`.
- **A refusal cannot loop.** Bounded by the number of walled nodes per retry window, and the test asserts it: nine walled
  nodes, `<= 12` refusals in 30 s, measured.
- **No new failure mode for `nth`.** `nodes_of_slot(.., nth + 1).get(nth)` is `None` exactly when there are not that many
  candidates, which is the same `Option` the old code returned; `nth + 1` cannot overflow for a `usize` from a scenario,
  and a scenario asking for `nth` beyond the map gets the same "no … node number N" refusal as before.
- **Determinism**: `nodes_of_slot` sorts by `(distance, id)`, so two runs pick the same node; the acceptance hash confirms it
  for the scenario the project ships.

## What this does not do

- It does **not** add a memo of its own: `share_failed` is the book, and a second one would be two answers to one question.
- It does **not** change the order-time refusal itself: a walled node is still refused, which is the rule D134 explicitly
  kept for gather orders ("a tree behind a wall is a refusal, not a walk to the fence").

**720 passed / 0 failed / 19 ignored**; clippy `-D warnings` clean; `cargo fmt --check` clean.
