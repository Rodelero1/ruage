# Review: the search's own budget and tie-break (P1, P2 of the parity plan)

*The owner: **"I want no deliberate differences to original game ... Please plan the sequence of implementation steps to reach
parity with original game and decomp."** The plan is `docs/parity-plan.md`; this is the first step of it - the two rules inside
`Ground::search` that were ours.*

## The rules, ported

- **The budget is the estimate.** `FUN_008007f0_008007f0.c` (REA corpus), read tonight: `iVar20 = (int)euclid * 2500 / 100`
  (or `* 0x19` when the weighting is on - both 25), then at the top of the expansion loop
  `if (((DAT_00aad420 != -1) && (iVar20 < local_46c)) || (20000 < local_46c)) break;`, with `local_46c` incremented once per
  expansion. So a farther goal is allowed more expansions and the hard cap is **20,000**. The tail pushes the **best node it
  settled** with cost 0 and the caller walks the parents from it, so a budget-exhausted search answers "the closest I got",
  exactly as a walled-in goal does. `Ground::search_reporting` returns the chain and what it spent.
- **The tie-break is the octile distance**, first found wins: `if (iVar21 < local_468)` with `iVar21 = FUN_007ffc80(node, goal)`
  - a strict `<`, so a tie keeps the earlier node. Ours compared `(weighted h, g)` and preferred the *later* cheaper node, which
  is a different route whenever two cells are equally near. `Ground::octile` is `16 x max + 7 x min`.

## Performance

- `acceptance_economy::the_acceptance_match_trains_more_villagers`: **passes in 34.65 s** (33.74 s before this step, 39.58 s
  before the D286 slice). The budget costs one comparison per expansion and gains nothing measurable on a 64x64 map - which is
  what D279's note predicted ("the map is small enough that it does not bind"), now measured rather than assumed.
- The cost this buys is on the *other* side: a hopeless search is now bounded by `h` instead of settling every reachable tile.
  Conformance pins it: on a 64x64 field with a sealed room the search spends at most `h` expansions and strictly fewer than the
  field's 4,096 tiles.

## Stability

**New conformance rows, each seen failing first** (`crates/conformance/tests/budget.rs`, `contact.rs`):

- `a_hopeless_search_is_bounded_by_its_own_estimate` - the count against `Ground::estimate`, and the answer is still the
  closest cell.
- `the_budget_never_exceeds_twenty_thousand` - a corner-to-corner search on a 480x480 field stays inside the cap.
- `a_reachable_goal_is_still_reached` - the budget bounds the work, not the answer, so a walk it can afford still ends on the
  goal. (The one place this rule could bite: a detour longer than 1.56x the straight line is truncated by the budget *by the
  game's own rule*. No fixture in the tree needs one today; if one ever does, that is the game's answer too.)
- `a_point_exactly_at_the_grown_face_touches` and `the_widest_body_in_the_pack_is_the_edge_of_the_scan` - `FUN_00840830`'s
  four `<=`, and the bound at which the three-by-three neighbourhood stops covering the answer.

**One fixture went red and was NOT papered over.** With touching-as-collision wired into the step, a body of radius **half a
tile** can no longer thread a **one-tile gap**: its box exactly fills the lane, so it touches both walls, and `<=` refuses it.
`building_passage::a_big_unit_walks_through_a_one_tile_gap` asserts the opposite, and it is not our invention - it is **D117,
the owner's own rule**: *"a one-tile gap is a gap, whatever the circle"*. That is a fork between his ruling and the binary's
test, so the step keeps its strict-overlap test until he rules, the binary's own test is available as
`Ground::touches_object` and pinned by `conformance::contact`, and the fork is written into `docs/parity-plan.md` as P3.

**Not measured, and named:** the suite's red set for this step (the budget/tie-break touched no fixture in the movement set
when run alone); the 400,000-tick fixtures were not re-run.
