# Review: gatherers press in, and the seat book stops being read (D293)

*The owner, 2026-11-19: **"Drop it and let the crowds jam. I want the start as close to the game as sensible"**, and the crosscheck
that followed: the gather arm's taken list is **ours**, not the original's. D290 had already established that the original has no press
floor - it computes a depenetration and discards it.*

## What changed

The gather arm no longer consults a seat:

- **`start_gather`** stops calling `standoff_point` and stops pushing a private place onto the route. The plan's own route - which
  ends beside the node - is the whole approach.
- **the settle** (`settling`) is off. A man walks at the node and takes when `at_node` (the box gap against the step, D285) holds.
- **the crowd is still spread, by the rule that spreads it in the game**: D204's same-kind re-target, kept by D291. Its trigger is now
  the **refusal count itself** - a seat that never existed cannot refuse a man - and only for a man whose **route is spent**, so a
  walker on his way in is never re-tasked. `STALLED_TICKS` still keeps it off the per-frame re-plan path.

The book itself (`standoff_point`, `Standoff`, `gather_place*`) is inert and marked `#[allow(dead_code)]`; deleting it is the next
slice's first task, and it is not deleted here because five fixtures still read `gather_places()`.

## Performance

- `research` (20 tests, including the 400,000-tick banking fixture that the first version of this slice broke): **32.94 s**;
- `gather_touching` (3): **1.88 s**; `gather_contact` (2): **1.33 s**; the new witness `gather_press_in`: **0.97 s**;
- the full workspace suite: **897 passed / 11 failed / 21 ignored**, against D292's 893 / 14 / 21 - four names moved from red to
  green and none the other way. (The commit that landed this slice says 896/12: that number was measured one edit earlier, before
  `gather_contact`'s re-point, and the suite was re-run afterwards to correct it.) `acceptance_economy` is unchanged in shape from D292's 30.34 s.

## Stability - what the slice broke, and how each was caught

**The witness was written first and watched failing**: `gather_press_in::two_gatherers_on_one_tree_press_in` measured **0.960 tiles**
between two men on one tree before the cut (the seat book's one-diameter spacing) and passes after it, pressed inside a body's width.

**A stall the suite caught, and its cause was the cut**: with the seats gone, `research::a_trebuchet_cannot_be_trained_before_the_imperial_age`
could not bank 400 wood in **400,000 ticks** (bank stuck at **210**), because no man was ever re-targeted - the D204 path had been
gated on `Standoff::NoRoom`, which no longer exists, so a jammed crowd stayed jammed for ever. Fixing the trigger (refusal count, route
spent) turned it green and, with it, `gather::a_depleted_node_sends_the_villager_to_the_next_one_of_its_kind` - a red that had been on
the books since before today.

**First attempt at that trigger was too eager**: counting a man **walking in** as refused re-tasked every gatherer forty ticks after he
set off, and three `gather_touching` fixtures went red. The trigger is now "arrived and still not taking".

**Six fixtures re-pointed, each with its measurement in the code**, all of them the seat book's claims rather than the game's:

| fixture | before | now |
| --- | --- | --- |
| `gather_ring::..._both_touch_it_and_stand_a_body_apart` | ≥ 0.39 apart | presses (**0.000**) |
| `gather_spread::..._do_not_share_its_side` | ≥ 0.36 apart | shares the side and presses (**0.00**) |
| `gather_touching::ten_men_touch_one_bush_without_overlapping...` | 7 places held | ten asked, everyone working stands against the tile |
| `gather_touching::every_man_working_a_node_holds_a_place` | one place per man | every man keeps the node and takes; **no place is held** |
| `gather_contact::villagers_on_one_bush_do_not_overlap_each_other` | ≥ 0.36 apart | converges (**0.00**) |
| `invariants::villagers_never_come_closer_than_one_diameter` | never inside a diameter | may overlap to any depth; **measured 0.195** |

`overlap::a_newcomer_does_not_move_the_working_gatherers` keeps its claim but measures it on the thing a player sees - a working man's
**position** - instead of on a seat.

## What is left

Twelve reds: four `P4`/`P3`/search ones, the two `reseed` fixtures, and the gather/seat leftovers that need the book's deletion
(`a_man_who_cannot_fit_takes_the_next_source` and the `overlap`/`gather_ring` fixtures that still read `gather_places()`), plus the four
unexplained combat/wood ones. The book's deletion is a mechanical follow-up: nothing reads it, and five fixtures must stop.
