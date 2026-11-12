# The standoff lands: a friend was a wall on the last step

D192 was parked for one red test. This is the slice that found why, fixed it test-first, and landed the standoff it was holding back.

## Performance - measured on the same box, back to back, one run at a time

| | `main` (`b7d92f2`) | this landing |
|---|---|---|
| acceptance hash | `8999f1baa7e7b897` | `91572e72c33cee93` |
| outcome | `players=1` | `players=1` |
| entities at 1,800 s | 139 | **142** |
| **made_room** | **96** | **0** |
| **repaths** | **240** | **409 (+70%)** |
| `World::step` | 7.2 us | **6.7 us** |
| `Snapshot::of` | 103.4 us | **89.3 us** |

`made_room` at **zero** is the point of the slice: with men standing with their circle on the tile instead of queuing on a ring, nobody
has to be shoved aside. The tick is 7% cheaper and the snapshot 14% cheaper - the crowd does less work because it is not fighting
itself.

**The number that moves the wrong way is repaths, 240 -> 409.** It is reported rather than smoothed over, and it is **not explained**:
the plausible reading is that men whose place is on the far side of a tile now have to re-plan the approach where the ring used to
give them a slot by rank, and the landing has no measurement that separates "more re-planning" from "more men at work" (142 against
139). A profile that counts repaths per working villager-second is the instrument this needs, and it does not exist yet.

## What was wrong, and how it was found

The parked test's failing symptom was a villager frozen on the edge of a fresh foundation while the house sat at **hp 1**. Two hours
of printing in the previous slice established only what it was *not*: the step was walkable (`walkable=true`), it improved his
clearance (0.120 -> 0.129 tiles), his route had one waypoint, his task was still `Move`, and no friendly body stopped him after D189.

This slice instrumented `collide` itself - every call's ask and answer - and the answer was `(0,0)` for an ask of `(759,-253)`, every
tick. That narrowed it to rule 2, and a print at each of rule 2's two clauses named it:

```
RULE2b goal-tile (5, 4) held by EntityId(2)
```

**A friendly body standing on the tile the order asked for was treated as a wall.** The clause exists to stop a unit walking onto
ground another unit claimed (D131, two *players*' units); applied to a friend it means "wait until your neighbour moves", and a
neighbour with no orders never moves.

## The rule now

`collide`'s goal-tile refusal is **hostile-only**, and the player comparison that decides it is computed once, before rule 2, with the
reason written beside it. Two of the player's own units are not walls - they pass while walking (D189) and are separated where they
stand by the settle and the work places (D192) - and that now holds on the **last step** as well, which is where the old text in this
file said the opposite.

## Test-first

The RED is `crates/sim/tests/movement.rs::a_walker_whose_goal_tile_holds_a_friend_still_arrives`, written before the change and
watched failing:

```
MEASURED the walker 1.011 tiles from the tile it was sent to after 600 ticks
(task Some(Move { to: (Fixed(410000), Fixed(410000)) }));
a friend standing there must not stop him - his own units are not walls (D189)
```

The test places the friend at the goal's **centre** (its first version spawned on the tile's corner, where the same bug left the
walker 1.011 tiles out but a loose `< 1.1` assertion would have passed - the tolerance was tightened to `0.75` so the test bites).

The parked content-pack test, `build::a_content_pack_with_our_own_resources_loads_and_spends`, went from **hp 1 after 2,000 ticks**
to green with no change to the test.

## Stability

- **Sim suite: 122 binaries green** (`EXIT 0`), not just the touched files. `cargo fmt --check` clean, `cargo clippy --workspace
  --all-targets -- -D warnings` clean after removing the ring era's dead code (`GATHER_BAND_MILLI`, `node_radius_raw`) and four
  unused locals the deletions left behind.
- **The gate caught my own leftovers twice in this slice**: clippy refused `let asked`, then `GATHER_BAND_MILLI`/`node_radius_raw`,
  then a chain of `let at`/`let tree`/`let tick` that earlier probes had orphaned. Each one was dead weight from the ring, not a
  behaviour change.
- **`site_clear.rs` is a guard, not this slice's RED.** It was written as the deadlock's RED and passed on the branch because its
  squatter's exit tile was free; the RED that bit is the movement test above. Kept, and labelled as a guard in its own docstring.
- **Decision log**: D192 un-parked (193 decisions, `--check` green, area added). `tools/test_decisions.py` 68 tests pass, including
  the shrink guard from the architecture review.
- **Not fixed here**: the +70% repaths above, and `docs/next-step.md` still carries its 348 KB history (the architecture review's
  proposal 2).
