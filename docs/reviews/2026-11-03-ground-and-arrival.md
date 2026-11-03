# Review: ground and arrival - two trees at a corner, and a worker beside its node

*Change, two of the owner's observations of 2026-11-03, measured together because they touch the same two lines of the
movement pass:*

- *"Units seem to walk through the forest (each forest tile is blocking passage until tree cut and all wood extracted, and
  there is no passage between trees even if touching by corners of the tile)";*
- *"Villagers should stand around the berry bush or stone/gold mine, not on top of it."*

`World::insert` now closes a node's tile while the node holds **anything** (it keyed on hit points, which only a tree
carries - so a bush or a mine was a floor, and a felled tree with wood still in it stopped blocking). `step_is_walkable`
(both-corners rule for raw-space diagonals, from the buildings slice) is what refuses the corner gap; the tests here are
what prove it. Three tests in `crates/sim/tests/passage.rs`, each watched failing or shown passing for a reason.

## Performance

**The acceptance match is won again, and this is what won it.** At 36,000 ticks, seed 7, against the pre-change branch
(`49b09a3`: `players=2`, hash `ca90bc834457d7a6`, 38 buildings, bank 258/290/170 at 1,200 s):

| | branch before | with nodes blocking |
|---|---|---|
| outcome | `players=2` - not won | **`players=1`** - won |
| hash | `ca90bc834457d7a6` | `51992525a3b1c3fb` |
| bank at 1,600 s | 1,157 / 609 / 280 | **1,294 / 1,209 / 175** |
| entities / buildings | 144 / 38 | 136 / 34 |
| repaths / nudges | 63 / 114 | 160 / 143 |
| wall clock | 47.3 s | 48.4 s |

The change is two lines and the mechanism is visible in the numbers: a miner or forager that stood **on** the node was
also standing in every other worker's arrival, so the second villager at a bush could not reach it and the food column
starved (609 -> 1,209 food at 1,600 s is that queue disappearing). The repaths doubling is the same fact seen from the
other side - workers now walk around nodes instead of over them.

**Cost:** 48.4 s against the landed baseline's 19.9 s at 36,000 ticks. Most of that is not this change (47.3 s before it):
it is the base's size and the building-blocking slice's extra pathing, measured in
`docs/reviews/2026-11-03-buildings-block-their-ground.md`. At 0.55 ms -> 1.34 ms/tick it is 2.7% of the 50 ms budget.

**A claim of mine from the buildings review, corrected:** it said the branch's suite was red in ~25 build fixtures that
raise a house overlapping the town centre, and that each fixture would have to move. They were red at that commit, and the
cause was the **placement search**, not the fixtures: at radius 2 it returned a tile whose neighbour the town centre
already owned, and the layout change (radius 4 first, a lane for every building) made every one of them green without
moving a single fixture. Nothing in `crates/sim/tests/` had to be re-sited.

## Stability

- **Two trees touching at a corner leave no gap** (`two_trees_touching_at_a_corner_leave_no_gap`): trees at (4,4) and
  (5,5), a villager at (4,5) ordered to (5,4) - one diagonal step through the corner. It is asserted **not** to arrive
  inside 60 ticks (three tiles of walking against a five-tile detour), never to stand on either tree, and to arrive in the
  end. The test passes on the branch *because* the both-corners rule landed with the buildings slice; it is the proof of
  the owner's observation, and it fails if that rule is removed.
- **A woodcutter stands beside its tree and still takes** - the tree case, which already worked (`hp > 0` blocked it).
- **A miner stands beside its mine and still takes** (`a_miner_stands_beside_its_mine_and_still_takes`): watched failing
  first with *"a mine with gold in it should close its tile"*, then the ring arrival and the carry.
- **My own test bug, reported rather than hidden:** the first version of both carry assertions read the villager's load on
  the *last* tick, where a worker that has just handed in is carrying nothing - so the test failed on a working engine.
  It now records the highest load seen.
- **A spent node frees its tile**: `step_spent_nodes` removes the node and unblocks its ground, which is the other half of
  the owner's rule ("until tree cut and all wood extracted") and is already covered by `a_chopped_tree_leaves_no_obstacle`.

---

*Staged with the change for `tools/review_check.py`. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
