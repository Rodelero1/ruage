# Review: a resource node is clickable, and the panel says what it holds

*Change: the owner's ask - "I would like trees to be clickable showing hit points and volume of wood they have. Same for
farm, can I see amount of food it has left? Same for berries, stone and gold. These resources need to be visible in the
central part of the bottom bar" - and, with his one clarification in this thread, **a left click on a node only shows its
statistics**: it orders nothing, gathering stays on right-click. Three pieces: `Selection::click` accepts a resource node;
`World::held_slot` tells the snapshot **which column an entity holds** (a node from its own kind, a farm from content's
`[farm] resource`); and the panel's selected zone draws `HP n` for a node - which has no maximum on record, so no bar - and
one holdings line, `WOOD 100` / `FOOD 213`, in the pack's own word for the column. D111.*

## Performance

`a_hud_costs_less_than_a_tick` (ignored measurement), interleaved against a `HEAD` worktree with the same
`config/gamedata/`, three pairs alternating, 960x720, 128 entities, five selected:

| | runs (us of HUD) | best | median |
|---|---|---|---|
| `HEAD` (`d8f9751`) | 177 / 280 / 175 | 175 | 177 |
| this tree | 181 / 183 / 221 | 181 | 183 |

**+6 us a frame (+3%)**, which is one more text line and the space advances on it; the 280 in the `HEAD` column is a
contaminated run (the same test on the same tree gives 175 and 177), reported rather than dropped. The honest read is
"a few microseconds, inside the noise of this measurement, and certainly not a tick".

The one per-entity cost this adds is `World::held_slot` inside `Snapshot::of`: a match on the entity's kind, and for a
building one `root` lookup and one table read - it is the same shape as `world.footprint_of`, which the snapshot already
calls per entity. Nothing was added to the tick: the acceptance hash is unchanged from `d8f9751`
(`098fd53e146f0911`), which is what a view-and-selection slice must show.

## Stability

What was attacked, and what the attacks found:

| attack | result |
|---|---|
| click a node, then a unit standing on the same tile | the unit wins the click: nodes are third in the order (unit, building, node), pinned by `a_unit_on_a_nodes_tile_is_what_the_click_means` |
| click another player's unit, then a Gaia node | the unit is still refused, the node selects - `a_click_on_an_enemy_building_selects_nothing_but_a_node_is_gaia`. "Nodes for anyone" is deliberate: every map node is player 0, so "only your own" would mean none of them |
| select a node with `Hud::of_selected` | the title comes from the pack's column, because a node carries no unit id - `the_hud_reads_a_nodes_column_and_holdings` |
| a farm: bar **and** holdings in one row | `a_farm_shows_its_health_and_the_food_left` - the only row with both |
| an empty selection | the holdings band paints nothing (`the_panel_says_what_a_node_holds`'s second half) |
| a house, a villager | `held_slot` is `None`, so the panel invents no column (`a_house_holds_nothing`) |

**The defects this pass found, two of them mine:**

1. **Every word in the interface ran together.** The font has no space glyph and a character it has no shape for "takes no
   room at all", so `format!("HP {}", hp)` painted `HP20` and `WOOD 100` painted `WOOD100` - and the same bug had been in
   the age bar's `FEUDAL AGE` and every other two-word label. Found by **looking at a rendered frame**, which is exactly
   why the frame diagnostic exists. Fixed as a width (`font::SPACE`: half a glyph plus the gap, painting nothing), with
   `a_space_takes_room_and_paints_nothing` comparing the drawn width against `text_width` so the two cannot drift.
2. **My first reading of that frame was wrong.** I said the panel showed `HP 50`; the tree's hit points are 20 (`--dump`
   at the same tick says 20 for every wood node) and a pixel-font 2 at one pixel per dot reads as a 5 in a downscaled
   screenshot. My first decoder then printed `HP??????` because it advanced a fixed six pixels per cell and a space
   advances four. The decoder's second version reads `HP 20`; the vision pass had misread it. Both errors are mine, and the
   second one is the reason to decode the glyphs rather than trust a description of them.
3. **`tile()` was never a "tile centre"**, though four comments said so: it is the tile's origin, which is the corner of
   four cells. That is now named correctly in the one place the fact matters, because a node's panel `HP` is the number
   that made me look.

Not in this slice, and named: **another player's hit points stay unreadable** (the "enemy HP on click" board item needs its
own rules about what the fog lets you know), a node has **no maximum hit points on record** (the number is absolute; a
bar would need a new field in the simulation), and a **band or double-click still sweeps only units** - a box around a
forest selects nothing.

## Known and accepted

- A tree shows `WOOD` twice: once as the title (it has no role name, so the column names it) and once in the holdings line
  (`WOOD 100`). Redundant for a node, and it is what keeps a farm readable as `FARM` / `FOOD 213` in the same shape.
- The holdings line sits in the band at `+34` and the per-selection rows moved from `+40` to `+46` to make room; six rows
  still fit a 142-pixel panel, at 960x720 and at 1280x720 (both rendered and looked at).

---

*This file is what `tools/review_check.py` looks for: `## Performance` with a number, `## Stability` with content, staged
with the change. See `.githooks/pre-commit` and the routine in `AGENTS.md`.*
