# D337 - P13: the frontier's key is the game's - and it exposed a rounding bug in the mover

**Crosscheck first.** `FUN_008007f0` read to the end: line 177 stores `FUN_007ffd10(start, goal)` (the octile) as the start's
frontier value; lines 312-316 compute `stored[current] - octile(current,goal) + octile(neighbour,goal) + step`, which is
`g + octile`; and `iVar20 = (int)euclid * 25` (186-191) is read **once**, at line 198, against the expansion counter. So the key
is the octile, weight 1, and the 25 is the budget - the reading D334 recorded and six documents had wrong. openage's pathfinding
note agrees on the structure (three solvers; the high-level one subdivided by passability), and the first-party talk's "no
pushing, bump and repath" is unchanged either way.

**Landed:** the flat search's heap key is `Self::octile(tile, goal)`; the budget still uses `estimate`. Witness, watched failing
first - `crates/sim/tests/search_key.rs`, open ground, (2,2) -> (14,14), **one tree at (8,8)**:

> *"the route round one tree paid 312 instead of the octile key's 294 (the bound is 276, a greedy key pays 312):
> [(3,3),(4,3),(5,4),(6,4),(7,5),(8,5),(9,6),(10,6),(11,7),(11,8),(11,9),(12,10),(12,11),(12,12),(13,13),(14,14)]"*

The fixture asserts the **cost**, not the waypoints. A probe over 400 sampled 16 x 16 maps showed the two keys disagree on the
chain in **3,973 of 4,001** scenes, so the hard part was finding a *small* one, not a differing one.

## The second defect, found by the first

The key change reddened `whole_steps::a_walking_villager_takes_whole_steps` - the owner's rule, *"man stands or walks"* - at
**801.295 raw where a step is 800**. It is not a consequence of the key: it is a **pre-existing rounding hole** in
`walk_the_step` that the old routes never hit.

- `integer_sqrt` **floors**, so a 45-degree step vector of (566,566) - 800.44 raw - read as "exactly the step" (`length == take`)
  and was never rescaled. Measured: `take 800 gap 27594 mx 566 my 566` -> 800.4 raw in one tick.
- The "never longer than the step" trim used the same floored root, so a 1-raw leg of (1,1) - 1.41 raw - passed as 1.
- And the budget was charged `take` rather than what was walked, so two legs could total more than the step.

**Fixed** in the same slice, because a red fixture of the owner's is not landable: the over-step test compares **squares**
(`sq > take^2`), the trim and the charge use `ceil_sqrt`/`round_sqrt` helpers (a leg is tested with a ceiling root and charged
with a nearest one), and the leg that ends on a waypoint lands **on** it exactly (`if take >= gap { (gx, gy) }`) instead of
`step_towards`, which rounded 1 raw past. `whole_steps` is green, and the cheap-vs-expensive helper version of the same change
moved the acceptance wall by **0.16 s** (15.86 -> 15.70), so the helpers are not the cost.

## A fixture re-pointed, with its reason

`gather_ring::two_woodcutters_on_one_tree_both_touch_it_and_press_together` asserted `apart < 0.39` - both men *pressed on one
face*, measured 0.000 while the seat book routed them to a shared standoff point. With the seat book gone (D293) and each man's
route now his own, they take opposite faces: **1.018** apart, both still touching the tree. The rule is not "press together", it
is "each touches the node and no body overlaps another", so the fixture is renamed
`..._and_never_overlap` and asserts `apart >= 0.39` (two radii = 0.400). The old `< 0.39` was the seat book's last trace.

## Performance

Same tree, same data, seed 7, 48,000 ticks, release, idle box.

| | before (greedy key) | key only | key + exact steps (landed) |
|---|---|---|---|
| acceptance hash | `e248486298910e28` | `9d5fea513f3f8efb` | `527bf4ebb9520fa0` |
| entities / players | 127 / 1 | 133 / 2 | 132 / 1 |
| re-paths | 1,727 | 41,557 | 41,473 |
| searches / expansions each (D334 probe) | 5,989 / 28.9, 54% exhausting | 45,088 / 11.8, 5.8% | same key |
| wall, median | 10.04 s | **9.72-9.88 s** | **15.70 s** (15.67-15.93) |
| stockpile at 2,400 s | 1,961 / 960 / 130 | not measured | 1,446 / 852 / 655 |

**The wall rise is the match, not the code, and it is named rather than explained away:** key-only runs the longer match (both
players alive) at 9.8 s; the exact-step build plays a *different* match (132 entities, the match decided) at 15.7 s with the same
re-path count, and rebuilding the identical change with cheaper sqrt helpers moved it 0.16 s. Two runs with the same entity and
re-path counts differ by 60% in wall, so the cost is somewhere in what the units do, not in the search - **unattributed, and the
next perf item**: profile the 15.7 s run per pass before touching anything, and price the per-search allocations (49 KB a call,
~2.2 GB a match at 45k calls) at the same time.

## Stability

- Determinism: three runs each, bit-identical hashes within each variant (`9d5fea…` three times, `527bf4…` three times).
- Conformance 21 / 0 / 0; the suite: **214 result lines, 934 passed, 0 failed, 20 ignored** (one more test than D334's run: the
  new `search_key` row).
- Attacked: the key (fixture), the mover's budget (the owner's fixture, three separate overshoot mechanisms), the seat-book
  fixture (re-pointed, reason in the file), the greedy key's numbers (re-measured), and the wall (A/B with the helpers cheap).
- **My own error, in this slice:** my first mover patch charged each leg its `take` and only tested `gap > take`, which left the
  801.295; the second used a ceiling charge and walked **798.77** (under by 1.2, which the fixture also refuses); the third
  (ceiling test, nearest charge) is the one that holds. The two failed attempts are why the fix has three named roots rather than
  one.
