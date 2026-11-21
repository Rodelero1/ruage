# Review: the re-plan takes a fresh stamp, so the past rule can finish its chain (D320)

*Change: `replan_around_bodies` stamps the new route with `self.tick` instead of keeping the old route's creation time
(`crates/sim/src/world/motion.rs`). One line, and the last link in the original's collision chain. The ignored fixture
`replan::a_stopped_friend_is_not_walked_through` comes off ignore **re-specified** to the game's chain, and the acceptance match changes
because villagers no longer sink into each other.*

## What was wrong

The original's chain, read earlier and now completed: a non-past collider means **re-path and pause the loser**; the new path carries a
**new `createdAt`**, and a body that has stood still since *that* moment is **past** (`path->createdAt >= other->lastMoveTime`) and is
walked through. We re-planned but kept the old stamp, so a man who had stopped stayed non-past **for ever**: the walker re-planned on every
tick it touched him, got the same route straight through him, and sank in until their centres coincided. Measured on the fixture before the
change:

```
the walker went 0 raw from a friend standing still at tick 176, deeper than the 7200 he first touched at
```

Instrumented (`DBG replan`), the mechanism was one line of output repeated per tick: `blockers [(230000, 90000, 4000, 4000)] -> [(250000,
90000), (610000, 90000)]` — a re-plan, from a point already inside the friend's grown box, that answers with the straight line through him.
Zero repaths would have been right; 600 is what the old stamp produced.

## Test-first

The witness is the fixture that was ignored **as the specification for this slice** since D242:

```
a_stopped_friend_is_only_walked_through_after_the_replan   (was a_stopped_friend_is_not_walked_through)
```

It is **re-specified**, not deleted, and the rename is deliberate and paired: the old name claimed the game never walks through a stopped
man, and D239's own reading of row 29 says it does - *after* the re-path. The new body asserts the chain with three measurable facts:

- the friend **becomes past** after the walker's re-plan (red before this change: *"the friend never became past"*);
- the walker spends **at most 60 ticks** (the 1.5 s pause) inside a friend who is **not** past, where it spent 600 before;
- the walker re-plans **at most 4 times** in 600 ticks against a man who never moved, where it re-planned on **every** tick.

**Mutation-proved**: with `created = route.created` restored the fixture fails on the first assertion; with `self.tick` it passes.

## Performance

**Acceptance match, release, seed 7, machine idle, same generated `config/gamedata/` on both trees** (parent `25cb1a8` vs this change):

| ticks | before: entities / repaths / wall | after: entities / repaths / wall |
|---|---|---|
| 12,000 | 153 / 2,229 / 1.60 s | 154 / **732** / **1.53 s** |
| 24,000 | 158 / 5,481 / 5.18 s | 159 / **2,645** / **4.84 s** |
| 48,000 | 129 / 10,026 / 9.30 s | 141 / **7,125** / **13.01 s** |

- **Re-paths fall by 67% at 12,000 and 52% at 24,000 ticks**, and the first 24,000 ticks are **~7% faster**: the tick loop is not paying
  for this, it is being paid back.
- **The whole 48,000-tick match is 40% slower (9.30 → 13.01 s)** and the state changes (`89ff37d844cfd11f` → `c7191f531bd81b1b`,
  **141 entities against 129**, and **`players=1`** - the match now ends with player 2 eliminated). The extra time is **in the second
  half only** (no gap at 12,000 or 24,000), i.e. it is the **fight** the script could not previously reach, not the villagers.
- Full suite (idle, clean run): **929 passed / 0 failed / 20 ignored** against **928 / 0 / 21** - one more pass, one fewer ignore and
  **nothing joined**; `clippy -D warnings` and `fmt --check` clean.

## Stability

- **What this cannot break**: it changes no position, no route and no pause; it changes *when a collider stops counting*, which is the
  stamp's only use (`is_past`, one call site).
- **The four body invariants are re-run and green** (`invariants`: pair distance, gather half-diameter, drop half-diameter, a working man
  does not move), as are `overlap`, `group_overlap`, `head_on`, `collision` and `crowd_piles_up` - the suites that a collision change
  would redden first.
- **The risk it carries**: a man who pauses for a moment can now be walked through by a re-planning friend, which is the game's rule but
  is also the pile D238 measured (0.204 tiles). It did not appear here - `invariants::villagers_never_close_into_one_body` is green - but a
  crowd scene with more pausing is exactly where it would.
- **My own error, stated**: I first ran the timing A/B while the full suite was running on the same two cores and read 19.5 s against 13.5 s;
  both figures were contention, not the change. The table above is idle and sequential.

Known and accepted: the seat shuffle (`overlap::a_passer_by_does_not_move_the_seated_gatherers`) is **untouched and still ignored** - and it
now fails its own precondition (*"no gatherer was at work beside the node"*), so it measures nothing today. D238's verdict that the shuffle is
*load-bearing* was measured before this chain existed; re-measuring it is the next step, and this change is what makes that re-measurement
meaningful.
