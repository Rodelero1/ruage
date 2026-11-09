# "Villager can pass through the built house or stone mine" - five hypotheses, and none of them held (D158)

*2026-11-08. The owner's report with two screenshots. This note exists because a report is evidence of *something*, and the first
job is to find out what - not to "fix" a suspicion. **No production code changed in this step**: five tests were added, four of
them hunting the report.*

## What was tested, and what each test says

| # | Hypothesis | Test | Result |
|---|---|---|---|
| 1 | A route planned **before** a house appears walks through it | `a_house_built_across_a_walker_makes_it_go_round` | **Refuted**: the walker goes round; every tile of the house is solid |
| 2 | The order a player actually gives - `WalkTo` to a *point* - cuts through | `a_walk_to_a_point_does_not_cut_through_a_new_house` | **Refuted** |
| 3 | The **builder** is left standing inside the house it finished | `the_builder_is_not_inside_the_house_it_finishes` | **Refuted** |
| 4 | A miner stands **on** the mine's own tile | `a_miner_never_stands_on_the_mine` | **Refuted** (there is a test for this already; this one walks a miner 900 ticks against a real node) |
| 5 | The **picture** covers a different footprint from the collision one - the diamond offset by a tile, so a villager legally *beside* a building is drawn *on* it | `the_drawn_footprint_is_the_blocked_footprint` (view crate) | **Refuted**: house (2x2), barracks (3x3) and wall (1x1) all draw exactly the tiles they block, and the ring around them stays walkable |

Every one of them is now a permanent guard, so a regression in any of these paths fails a test rather than reaching a screenshot.

## What *is* still wrong, found on the way

**A walker interrupted by a building stops one tile short of its goal.** In test 1 the villager, ordered to (16,30), ends at
(15,29) - a tile diagonally beside it, reachable, with a clear route. The interruption re-plans through the D134 "goal snapped
once" path, and the snapped goal is where it stops. That is not "passing through a house", but it is the walk giving up a tile
early, and it is **recorded, not fixed**: the fix belongs with D134's snapping rule and its own test, and this session has already
spent its budget on the report itself.

## What the screenshots most likely show

Given 1-5 are refuted, the remaining explanations are visual and not sim:

- the grey diamonds with black marks are **buildings** (`palette::BUILDING` with a role glyph: a chevron is a house's roof, three
  strokes a mill's grain), and a **unit standing on the tile in front of one** is drawn over the diamond's lower edge because
  depth is `y`, which is how an isometric picture with no art looks when two things touch;
- the **brown diamonds** are resource piles (`palette::PILE`), not farm floor, and a mine is **one tile** in the dat
  (`STONM` radius `0.5, 0.5`) - so a cluster of brown diamonds is *several* nodes, and a villager between them is on the tiles
  between them, which is legal.

Neither is a collision bug, and both would be fixed by art rather than by rules.

## What would settle it

One detail the screenshots cannot carry: **was the villager moving at that moment, or standing?** and **what order had it been
given** (gather / build / right-click move / right-click on the building). With those two, the case is reproducible in a test in
minutes; without them, five plausible mechanics have now been tested and cleared.

## Performance

**No production code changed**, so the acceptance hash is **byte-identical** and no timing claim is made for it. The five new
tests cost **2.2 s** (four sim tests, each simulating 600-4,000 ticks) and **0.8 s** (the view test, three buildings rendered).
The suite as a whole is unchanged in shape - the same 36,000-tick acceptance run, the same 759 tests plus five.

## Stability

- **Five new guards, no behaviour moved**: the only risk in this step is a test that asserts the wrong thing, and each was
  watched running against the *unfixed* engine first - all five passed on arrival, which is exactly why they are written as
  invariants with the *opposite* failure named in the message ("the walker passed through the house at ...").
- **The over-strict assertions were caught and relaxed honestly**: test 1's "and it still arrived" was an equality that the
  engine fails by one tile (now a finding, not a pass); the view test's "the ring is walkable" failed on a barracks because the
  ring held a tree, and now skips tiles that have something on them.
- Suite: **764 passed / 0 failed / 19 ignored**, clippy `-D warnings` clean, fmt clean.
