# D341 - step 6: a body is an obstacle unless it moved

**Crosscheck first.** `FUN_007f9d80` (`pathfinding.cpp`, HD 5.7) inserts each object of the route's list as a box grown
by the mover's radius **unless** the id is in the mover's ignore list **and** `[vtbl+0x2a0]` (isMoving) is non-zero.
openage (`game_mechanics/pathfinding.md`) says the same from the other side: the high-level search *"ignores moving
objects, e.g. units or wildlife"*. The exemption is **velocity**, not "has a path".

**Prior implementation.** `standing_bodies` (`lib.rs:1991`) skipped a body when `route.points` was non-empty and its
pause was over. A builder whose site is beside it answers *adjacent* from the first tick and therefore never spends its
route: it stands at work with a live route for as long as the site takes, and every plan was certified straight through
it.

**Landed.** `has_route && moved`, where `moved = self.tick - 1 <= other.last_moved`.

- `velocity` is not the field to use: it is a fixture-only drift for idle bodies (`lib.rs:2313`), so the movement is read
  off `last_moved`, which the movement pass writes for every body that took a step.
- The route half is load-bearing. `last_moved` starts at the tick a body appeared, so without it a freshly placed body
  reads as moving for one tick - and that is not a hypothesis: the first cut was `moved` alone and the suite said so.

## Witness, and the error the first cut made

`crates/sim/tests/moving_bodies.rs` stages the stander deterministically: a villager on a tile centre, a house site one
tile east (its footprint beside him), so the Build arm answers *adjacent* at once and the route the order planned is
never spent. A second man is ordered three tiles past him, straight through his tile centre.

> **un-cut (`HEAD`):** `19 ticks inside its box (closest axis gap 0 raw, reach 8000)` - the walker passed exactly over
> him.
> **cut:** `0 ticks, closest axis gap 30018 raw of reach 8000` - he goes 1.5 tiles round.

Both runs are the same scene, one variable, `crates/sim/src/lib.rs` checked out per file and restored.

**My own error, in the same breath:** the first cut dropped the route half and the workspace suite went to **2 failed** -
`evasion::a_blocked_unit_goes_round_a_standing_body_and_arrives` (`came 4506 raw units from the body, standoff 8000`)
and `gather_ring::two_woodcutters_on_one_tree_both_touch_it_and_never_overlap`. Both are the *same* cause: a body that
has just been placed has `last_moved == born`, so `tick - 1 <= born` read as movement for one tick and the plan went
through a body that had existed for a tick. The route half fixes it because a placed body has no route. Fixing it took
the suite back to 0 failed and left the witness green.

## Performance

Same pack, seed 7, 48,000 ticks, release, idle box, three runs each side.

| | un-cut | D341 |
|---|---|---|
| acceptance hash | `36ceb342bd94acc9` | **`1e1fcc3cf2b05a4c`** |
| entities / players alive at 48,000 | 134 / 2 | **133 / 1** |
| re-paths | 40,127 | **36,567** (-9%) |
| wall, median of three | 15.60 s (15.68 / 16.20 / 15.60) | **11.42 s** (11.58 / 11.32 / 11.42) |

**The wall falls 27% and that number is not a cost claim.** The scenario diverges: player 2 no longer survives to
48,000, so the last part of the match is a world with one player's objects instead of two. Re-paths per tick of a live
match is the honest figure (40,127 -> 36,567, -9%), and even that is measured over two different matches. What the slice
is *for* is the rule, and the rule's own number is the witness above.

## Stability

- Suite: **219 result lines, 937 passed, 0 failed, 22 ignored** (`cargo test --workspace --no-fail-fast`, exit line read,
  `suite-step6-final.log`); the two extra lines over D340's 217 are the new `moving_bodies` binary and the ignored
  `substitute_probe`. Conformance **21 / 0 / 0** across 9 binaries.
- Determinism: the cut hash is identical across three runs (`1e1fcc3cf2b05a4c` three times), and so is the un-cut one.
- Attacked: the exemption itself (both halves, with the error the first cut made quoted), the fresh-spawn case (the two
  reds and their cause), a working builder's zero delta (the fixture's own precondition assert), and the *other* readers
  of `route.points` (`is_mate`, `is_past`, `replan_*`) - none re-pointed.
- `cargo fmt --check` and `cargo clippy --all-targets -D warnings` clean. One process note: D340's committed probe was
  not `fmt`-clean (the hook checks the review note, not formatting), so this commit carries its reformatting.
