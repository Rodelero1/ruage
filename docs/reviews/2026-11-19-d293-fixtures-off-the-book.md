# Review: the fixtures stop reading the seat book (first half of the D293 cleanup)

*D293 made the seat book inert: a gatherer walks at the node he was given and presses in, and nothing consults `standoff_point` any
more. This slice takes the **fixtures** off the book, so the deletion of the book itself is a pure code change with no test churn.*

## What changed

- **`gather_ring`** loses its two seat-capacity fixtures and keeps its two press ones. `the_tile_holds_as_many_woodcutters_as_touch_it`
  asserted that one tile seats exactly `touching_place_count` men at 0.66 spacing; `the_owners_own_arrangement_lands_on_the_same_ring`
  asserted that the engine reproduces the 1.156-tile ring six men sat on in the owner's save. **Both describe the rule D293 removed.**
  A note at the foot of the file says so, and names the fixtures that carry the surviving claims (`gather_press_in`,
  `gather_touching`, `gather_contact`).
- **`overlap::a_passer_by_does_not_move_the_seated_gatherers`** (already `#[ignore]`d under D238) now measures a working man's
  **position** instead of a seat, which is the thing a player sees and the thing `working_men_never_move` already pins.
- **`gather_touching`** stops asserting that no place is held - there is no place to hold.

## Performance

- `gather_ring` (2): **0.84 s**; `gather_touching` (3): **1.91 s**; `overlap` (2 + 1 ignored): **2.33 s**;
- the full workspace suite: **895 passed / 11 failed / 21 ignored** against the previous **897 / 11 / 21**. Passed falls by two because
  the two fixtures above are gone; **the red set is byte-identical** (`diff` of the name lists reports no change), so nothing regressed
  and nothing was hidden.

## Stability

Deleting a fixture is the easy way to make a suite green and the worst way to look honest, so the test is whether its claim is still
measured somewhere. For the two that went:

- *"a tile seats exactly N men at a fixed spacing"* - there is no seating rule left to measure. The emergent claim it stood for is
  measured three ways: ten men asked for one bush all stand against the tile (`gather_touching`), two men on one tree press
  (`gather_press_in`, the D293 witness), and a crowd converges on the bush (`gather_contact`).
- *"the engine reproduces the 1.156 ring from the owner's save"* - the ring is gone with the seats, and the owner's own file is the
  evidence that **men fringe a node rather than sitting on a ring**: it recorded six men at 1.105-1.189 from the square's centre, which
  is what pressing bodies produce when a node has neighbours.

**What is left of this cleanup** is now a pure deletion: `standoff_point`, `Standoff`, `gather_place`/`reserve_gather_place`/
`gather_place_of`/`gather_places`/`release_gather_place`/`place_is_live`, and the step machinery that fed them (`Settle`,
`settle_workers`, `reserve_places`, the `places`/`settles` buckets, `owned`). Nothing reads any of it, and no test does either after
this commit.
