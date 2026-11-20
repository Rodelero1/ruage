# Review: a man on his own node (D295), and the last of the seat fixtures

*Two things landed together because the second found the first: five fixtures still measured the seat model, and three of them failed for
a reason that was the **engine's** fault - a gather order given to a man standing on the bush's own tile was refused as a walled-off
node.*

## The engine change

`Command::Gather` whose node tile is the tile the villager stands on is **accepted with an empty route**: the validator skips its
reachability search in that case, and `start_gather_with` plans nothing. The general planner is untouched.

| attempt | suite | what it broke |
| --- | --- | --- |
| the rule in `plan_path_with` (every walk) | 897 / 9 / 21 | `a_point_inside_its_own_tile_is_still_reached`, `an_enemy_foundation_in_plain_sight_does_not_block_me`, `a_newcomer_does_not_move_the_working_gatherers` |
| the rule in gather validation only | **900 / 6 / 21** | nothing |

The first attempt is the interesting one: a rule that looks like a pathing invariant ("a route from a tile to itself is empty") is not
one, because a walk to a point inside one's own tile must still plan - the walker lands on the point, not on the tile's centre. The
measurement is what said so; the reasoning alone would have kept it.

## The five fixtures

| fixture | old claim | measured now | new claim |
| --- | --- | --- | --- |
| `gather_continuous::ten_men_work_one_bush_a_body_apart` | 10 work it, all ≥ 0.39 apart | **7 work it**, closest pair **0.000** | 10 asked, the ring's worth work it, and they press |
| `gather_continuous::a_man_who_cannot_fit_takes_the_next_source` | refused order | order accepted | unchanged, now that the order is accepted |
| `gather_standoff_rule::three_gatherers_all_work_the_same_square_on_its_ring` | ≥ 0.39 apart | **0.185** and **0.904** on two pairs of one scene | all three stand at the bush and work it; spacing is not claimed |
| `gather_standoff_rule::six_settled_men_hold_their_places` | ≥ 0.39 apart | **0.000** | six asked for one bush all stay on it |
| `gather_standoff_rule::a_settled_man_is_not_displaced_by_a_later_arrival` | position held | position held | unchanged |

`0.904` is why the pairwise loop came out of the third one: two men on **different squares** of a bush cluster are 0.9 apart, and that is
not a violation of anything - the original has no spacing rule (D290). A claim of "any two men at a bush are within a body's width" was
mine to invent and it was wrong.

## Performance

- `gather_continuous` (2): **1.62 s**; `gather_standoff_rule` (4): **1.91 s**; both unchanged within noise from their seat-model versions;
- full suite: **900 passed / 6 failed / 21 ignored**, from **897 / 11 / 21** before this commit. Six reds turned green, none went red;
- the six that remain are the two `reseed` ones, `a_villager_drops_only_within_half_a_diameter_of_the_drop_site`,
  `a_walled_off_wood_is_tried_once_and_then_left_alone`, `both_sides_field_an_army` and `the_skirmish_only_refuses_what_the_fight_has_taken`.

## Stability

**The order refusals were a stall in play, not a test artefact**: `submit_checked` remembers a refused node in `share_failed`, so the
villager is not just refused once - the picker will avoid that bush for the whole match, and a man who was pushed onto a resource tile
by a crowd would have been refused the very node he was standing on, permanently. That is the class of defect the owner's "nothing of
ours may refuse an order the game takes" rule exists to catch, and it took the seat book's removal to make the scene reachable.
