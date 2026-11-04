# Review: enemy hit points on click

*Change: another player's units and buildings are now selectable, so the panel shows an enemy's name and health - the item
D111 deliberately left out, asked for in one line: "Enemy hit points on click". Selection order is own unit, enemy unit, own
building, enemy building, node (units before buildings, as the picture is drawn; yours before theirs among equals). The fog
remains the filter: an enemy is in the snapshot only while the player can see them. D112.*

The change is four lines of rule and one real bug:

- `Selection::click` gained a `theirs` set; `World::card` already refuses a building that is not the player's, and a test
  now pins it, because with enemy buildings selectable that guard is load-bearing rather than unreachable. Orders were never
  at risk: `order_at` builds them from the entity under the pointer and only from the player's own units.
- **The snapshot divided an enemy's hit points by the viewer's maximum.** `hp_fraction` used `max_hp_of(player, unit)` -
  the *viewing* player - so once player 1 had researched Loom, a full-health enemy villager (25 hp) read as **0.625**, and
  the same wrong bar was already being drawn over the enemy's head on the map. The maximum is the owner's: it is their
  upgrade that raised it.

## Performance

No new work per tick: this is a selection filter and a different index into a table the snapshot already read. Measured as
the HUD frame costs it: `a_hud_costs_less_than_a_tick` gave **177 us at `HEAD` (`ef2e336`) against 183 us here** in the
previous slice (three interleaved pairs), and this slice adds no drawing at all - no band, no line, the same bar the panel
already drew for your own units. The acceptance match is **unchanged**, hash `098fd53e146f0911`, which is the check that
matters for a view-and-selection change: nothing in the simulation moved. Suite: **652 passed / 0 failed / 18 ignored**
(647 at `ef2e336`; five new tests), fmt and clippy clean.

## Stability

| attack | result |
|---|---|
| an enemy at full health, while the *viewer* has a health upgrade | reads 1.0 - `an_enemy_at_full_health_reads_as_full`; it read **0.625** before the fix |
| an enemy across the map, never seen | absent from the snapshot - `an_enemy_across_the_map_is_not_in_the_snapshot` |
| click an enemy unit | selects it - `a_click_on_an_enemy_selects_it_and_a_node_beside_it_still_selects` |
| click an enemy building, on its footprint not its anchor | selects it - `a_click_on_an_enemy_building_selects_it` |
| your unit and an enemy in one tile | yours - `an_own_unit_beats_an_enemy_on_the_same_tile` |
| the card for an enemy barracks | empty; the player's own town centre still offers one, so the test cannot pass by accident - `a_card_is_only_ever_about_your_own_building` |

**Two of my own mistakes, both caught before the commit:**

1. **A piped `grep` hid a compile error.** I ran `cargo test -p ruage-view | grep FAILED` after adding a closure and read the
   silence as green; the crate had not compiled at all (a lifetime error in that closure). The project's own rule - "do not
   let a piped command hide a red test" - is in the skill and I broke it; the closure became a free function.
2. **The first version of the health test proved nothing.** It stepped 40 ticks after ordering Loom, which takes 25 seconds,
   so both players still had the same maximum, the fraction was 1.0 and the test passed **with the bug in place**. It now
   waits until `max_hp_of(1, _) != max_hp_of(2, _)` and fails loudly if that never happens - and only then does it read
   `0.625` and demand 1.0.

Known and accepted:

- **A remembered enemy building shows live hit points.** Seen before, not watched now, it stays in the snapshot (the map
  intentionally keeps a building the player has seen) and carries its current hp, which the panel will now show. Freezing a
  memory is a change in `Snapshot::of` - capture the hp at the moment of last sight - and its own question about what a
  player may be told; named rather than silently equated.
- The panel's enemy row has no extra marking: it looks like your own row. A player who forgets which is which has the map
  colour and the ring to go by, and adding "ENEMY" to the title is a wording change nobody has asked for.
- Not looked at in a rendered frame: the enemy row is the *same* drawing path as an own unit's row (name, bar, percent),
  which has been looked at, and this slice adds no new band or line. If a screenshot disagrees, this is the claim that was
  not checked by eye.

---

*This file is what `tools/review_check.py` looks for: `## Performance` with a number, `## Stability` with content, staged
with the change. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
