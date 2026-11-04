# What is selected, named - and a button that stops moving

**Step:** U2 of the interface work (2026-11-03): the selected-object area of the bottom bar, and the command grid's slots
becoming stable.

---

## 1. The selected zone says what is selected

The zone had the room since U1 and showed a list of unlabelled bars. It now carries, at the top:

- **the pack's word** for what is selected, at two pixels a dot (`TOWN_CENTER`, `VILLAGER`), and `NAME x6` when a group is
  selected;
- **a wide health bar** across the zone with its percentage beside it - the same rule as the small rows (green above half,
  red below), so a hurt thing is legible from across the room;
- then up to **six** small rows (health bar, load, queue, construction) instead of five, because the zone is now 130 px
  tall rather than 60.

`SelectedRow` grew `unit: Option<u32>` and `name: String`. **The name is not the view's to invent**: `Hud::of_selected`
leaves it empty and the caller fills it from the content (`World::role_name`, the inverse of `role_unit`). The window does
it, and now `bin/frame` does it too - the diagnostic was showing an empty name, which is exactly the kind of "the picture
is not the window" gap that the previous step's review warned about.

Tests (written first, `hud.rs`): the zone names what is selected (text pixels in `HUD_TEXT` in the header band), its bar is
red at 40%, and with nothing selected the header is blank.

## 2. A slot does not move when a button opens

`World::card` sorted the techs **affordability first**, so a button jumped to another slot the moment the age opened it -
which is how a player clicks the wrong one. The order is now the pack's: what the building makes, then its techs by id,
never by whether they can be afforded.

Test (`crates/sim/tests/card.rs`): the barracks' card before and after `grant_tech(feudal_age)` is the same sequence of
targets. **Mutation-proved**: restoring the old key (`(refusal.is_some(), id)`) makes it fail.

## Performance

- The HUD measures **116 us** a frame at 960x720 with 126 entities and five selected, against 90-112 us at the end of U1.
  The difference is the header (one `draw_text` at two pixels a dot and one 6-px bar across the zone) and the sixth row.
  Inside the run-to-run spread of the benchmark (90-141 us across all U1 runs), and still below the 151-195 us of the HUD
  before the layout existed.
- The sim change costs one `sort_by_key` over at most ~15 items per card build, which happens on selection change and
  after an order - not per frame.

## Stability

- **Names come from content, never from the view.** A pack that renames its villagers renames the panel; a unit the pack
  does not name shows `unit 83` rather than an empty box.
- **Slot stability is a correctness property, not a nicety**: the same card is the same buttons in the same places before
  and after the bank fills, which is what a player's muscle memory depends on.
- **A group of six shows six rows**; a group of sixty shows six and a count. The count is the honest summary - the cards
  openaok draws (up to 60) are a later step, and a row that silently vanished would be worse than a count.
- **Known gap:** more than fifteen commands on one card still clips (no pages yet). No pack in this tree can produce one
  - the largest is the barracks' fifteen - so the page control waits for a content pack that needs it, and for the
  villager's build menu, which is the real multi-page case and needs placement first.
