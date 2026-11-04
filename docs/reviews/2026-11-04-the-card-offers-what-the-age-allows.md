# The card offers what the age allows

Owner's note, 2026-11-04: *"techs, upgrades and units should be shown based on age the player is in. Imperial Age upgrades
should not show in Feudal Age."*

Until now the card listed everything the building *could* make or research and refused the impossible in red. That was a
deliberate earlier choice - a button that explains itself teaches - and the owner has overruled it, correctly: on a town
centre in the Dark Age it filled the grid with things the player cannot act on, and the grid is a *choice*, not a list.

## The rule, and why it is not a list of techs

A card item is hidden when its refusal is the **age talking**, and kept (greyed, with the engine's sentence) when the refusal
is something the player can act on in this age:

| refusal | on the card? | why |
| --- | --- | --- |
| `UnitUnavailable`, `UnitNotUpgraded` | hidden | the availability tech is not reachable yet |
| `UnitBarred`, `TechBarred` | hidden | the civ will never have it |
| `AlreadyResearched` | hidden | not a choice |
| `MissingPrerequisite` where the gap **is an age** | hidden | the age is what is missing |
| `MissingPrerequisite` otherwise | shown | e.g. a monastery tech missing a building |
| `AgeNeedsBuildings` | shown | "two Dark Age buildings" is what the player does next |
| `NotEnough`, `QueueFull`, `TechInProgress`, … | shown | the player's own to fix |

The age question is answered by the pack (`[age] order`), not by the dat: `is_age_tech` asks whether the missing prerequisite
is one of the ages, because the numbering (dark 104, feudal 101, castle 102, imperial 103) is alphabetical noise and
`BTreeMap` order is worse.

`World::is_age_gate` rather than `SimError::is_gate`: the first attempt put the policy on the error type and had to classify
`AgeNeedsBuildings` as a gate, which hid the very button that tells the player how to age up - caught because
`a_town_centre_offers_its_ages` went red. The policy needs the world's age table, so it lives on the world.

## Tests, written first

`crates/sim/tests/card_age.rs`, both red before the change:

- `a_dark_age_card_hides_the_spear_and_a_feudal_one_shows_it` - a Dark Age barracks offered `["militia", "spearman"]`; the
  spear must be absent, and must appear after the Feudal tech. (The first run of this file "passed" nothing at all because
  the fixture forgot `load_research`: without the tech tables every item looks available. That is the second time this
  session a missing table made a test speak with confidence about nothing.)
- `an_unaffordable_item_stays_on_the_card` - four villagers' worth of food spent, and the villager is still offered with a
  refusal about money.

Two older tests in `card.rs` encoded the overruled behaviour and were **rewritten, not deleted**:
`a_barracks_card_refuses_the_spear_line…` became `…hides_the_spear_line…`, and
`a_card_slot_does_not_move_when_a_button_opens` now states the invariant that survives: money never moves a slot, and an age
that opens something **appends** it rather than reshuffling what is already there.

## Performance

`card()` costs **7.0 µs** for a five-item town centre card (release build, 1,000 calls, median of one probe run). It asks each
refusal exactly once - the first version asked twice per tech (once to filter, once to render the sentence), which is the
same answer computed twice and a wasted `check_research` per button.

The card is drawn once per frame at most, so 7 µs is 0.006 % of a 117 µs HUD: invisible next to the panel it is drawn into.
Fewer items also means fewer `check_research` calls in the frame than before the change, since gated items never reach the
refusal-to-string step.

## Measured

- **623 passed / 0 failed / 18 ignored**.
- `card()` costs **7.0 µs** for a five-item town centre card (release, 1,000 calls, one probe run). It asks each refusal once:
  the first version asked twice per tech (once to filter, once for the sentence), which is the same answer computed twice.
- Acceptance is **bit-identical** to the previous run - `entities=162 players=1 hash=13f09cf0930da590` - which is the point:
  the card is a query, and a query cannot change the match.

## Stability

Fewer items means fewer refusals computed per frame and fewer buttons drawn, so this cannot be slower than what it replaced.
The cost is bounded by the building's own `[train.entry]` and its techs' `location`; the filter is one `matches!` and a
`[age] order` walk of four entries.

The honest gap: a unit unlocked by a *non-age* tech (e.g. a unique unit behind a Castle) is hidden the moment its
availability tech is missing, even if that tech is researched at the same building in this age. The player then has to
research the tech before the unit appears - correct in the sense that the unit is not yet a choice, but it is a *smaller*
card than the game's, which greys the unit. If that reads wrong in play, the fix is a fifth column of the card: keep
`UnitUnavailable`, hide only when the missing availability tech is itself age-gated. That needs the tech tree walked
transitively, which is why it is not in this change.
