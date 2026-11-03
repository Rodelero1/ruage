# The card is a query, not a list of buttons

**Step:** P4 of the owner's queue (2026-11-03): the command card, and it has to be **moddable** - the owner's D1 answer
was that units, buildings, techs, descriptions, AI and civs must all be changeable by a pack.

---

## 1. What was built

**The engine answers what a building offers** (`World::card(player, entity) -> Vec<CardItem>`):

- the units it makes, from `[train.entry]` - a content edge, so a pack that gives its forge a new unit gets a new button;
- the techs whose `location` is that building, from the generated tech table;
- each item's price as `(stock column, amount)`, resolved through the pack (`slot_of` / `resource_slot`), so a pack that
  names a fifth resource prices its buttons in that column without the engine learning the word;
- each item's name from the pack's own vocabulary - `[role]` and `[tech_role]`, with `"unit 83"` as the fallback, because a
  button with an id is better than a button with nothing;
- and **`refusal`, which is `check_train`/`check_research`'s own error text**. The card and the order go through the same
  function, so a button can never be offered for an order the engine would refuse. That is the lesson this tree paid for
  twice: a HUD that showed what nothing drew (A3), and a selection nobody called (P2).

**The view draws it** (`ui::card_rows`, `ui::card_at`, `ui::draw_card`, `ui::CardButton`): four buttons a row, name and
price at one pixel a dot, a red bar and the engine's words on anything that would be refused, and the pointer's button lit.
`card_rows` is bounded by the frame and the panel, so a nine-button card does not run off the screen.

**The window uses it**: selecting one building builds its card, a click on a button submits `Train` or `Research`, and the
card is rebuilt after every order (a queue may be full now, or the money gone).

## 2. Test-first

`crates/sim/tests/card.rs` (4, watched failing to compile): a town centre offers a villager at the pack's price; a barracks'
card **refuses the spear line in the Dark Age and offers it after the Feudal Age** - the same content, two answers, which is
what proves the refusal is the engine's real one; an enemy building has no card; the town centre offers its ages, named from
`[tech_role]`.

`crates/view/tests/card.rs` (3): a rectangle per item inside the frame and above the panel, four to a row; a click on a
button picks that button and a click between them picks nothing; a refused button is painted in the refusal colour and an
available one is not.

Rendered and looked at (`bin/frame ... card`): the town centre's card shows **VILLAGER 50**, **LOOM 50**, the age techs and
their prices, with red refusal text on the ones the age tree refuses.

## 3. What is *not* in it, and why

- **No build buttons.** A villager's build menu needs placement - choosing a site, a ghost footprint, a legality check - and
  the owner's list named the card, not placement. Training and research need no site, so the card is complete for what it can
  actually do today; a build button that could not place anything would be a lie with a label.
- **No icons.** Names and prices, no pictograms: the marks in `[mark]` are for buildings on the ground, and inventing icon
  vocabulary for nine techs would be a second content pack's worth of decisions.
- **Research is offered even when refused**, with the reason. That is deliberate: a player learns the age tree from the
  card's red lines. The alternative - hiding what is unavailable - hides the game's progression from the person playing it.

## Performance

- **The card is rebuilt on selection change and after an order, never per frame.** That matters because building it walks
  every tech in the pack (~800 in this pack, filtered by `location`) plus `check_research` per candidate; doing that per frame
  would put an A* - `check_train`'s population and upgrade lookups included - into the draw loop. Measured indirectly: the
  frame benchmark is unchanged, and the diagnostic renderer draws the card from one query.
- Drawing is at most 9 buttons x (a box, a name, two numbers, a bar) - a few hundred pixel writes, against the minimap's
  16,384.
- `card_rows` allocates a `Vec` per call. It is called on input and once per frame for drawing; at nine items that is a
  nine-element allocation, well under the noise floor of a 33 ms frame.

## Stability

- **The card cannot disagree with the order**: both call `check_train`/`check_research`. A card built from a second set of
  rules is how a UI starts lying, and this tree has already paid for that twice today.
- **A card belongs to the selection and to the player**: empty for units, for another player's building, and for a
  foundation (a building under construction has no queue yet).
- **`refresh_card` runs after every order**, so a refused order's reason appears both in the panel notice and on the button
  that caused it.
- **Restart clears the card** with the rest of the window state.
- **Known limitation, recorded:** the card is offered for *one* building; a box of twelve buildings shows no card (there is no
  "all of these" card yet), and the villager's build menu waits for placement.
