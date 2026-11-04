# What openaok knows about a GUI, and what it would cost us

**Question (owner, 2026-11-03):** OpenAoK looks like the most advanced AoE2 implementation - can we learn how to make the
GUI from it? No textures needed yet, placeholders are fine, but a proper top bar and bottom bar would be nice. What steps
are needed?

**Studied:** `github.com/Zaydme/openaok` at `main`, cloned to `/tmp/openaok` (275 `.cpp` / 201 headers, C++20, SDL2, EnTT).
Read in full: `docs/engine/interface.md` (345 lines), `src/rendering/HudLayout.h` (220), `src/rendering/UIRenderer.h` (319),
and the parts of `UIRenderer.cpp` that decide slots, pages and occlusion. **MIT licence**, so the *ideas* are free to take;
nothing below is copied code, and our tree is Rust.

---

## 1. What it actually does, in its own words

- **The interface is art with numbers on it.** `HudLayout.h` is a table per 4:3 resolution (800x600, 1024x768, 1280x1024)
  holding every coordinate: where each stockpile's number ends, where the age is centred, the five top-bar buttons, the
  command grid's origin and pitch, the selected object's portrait/name/hit points, the group-card area, the minimap's
  diamond and its eight buttons, the help box. `Edged()` carries that table out to a widescreen frame by **cutting** each
  part at one column and repeating a plain band of the art across the gap - it does not stretch.
- **One function per surface, used by drawing *and* clicking.** The document says it twice, and the headers enforce it:
  `BuildCommands(sim, w, h, out, 15)` fills a **15-slot (5x3) grid** with kind/slot/unit or tech/rect/affordability and
  returns the count; `BuildQueueSlots` and `BuildSelectionCards` (up to 60 cards) do the same for the queue and for a group;
  `MinimapProject`/`MinimapUnproject` are one projection shared by the picture and the click. *"Drawing and clicking both go
  through this, so a button is always where it looks like it is."*
- **The card has pages.** A villager's build menu is two pages (economic / military) told apart by the dat's interface kind;
  a `SwitchPage` button moves between them; each button carries `hotkey_group`/`hotkey_item` so a key is per page (B opens
  the build menu, and inside it B is the Barracks).
- **Refusal is a query with a localisable answer.** `Refusal(sim, command) -> int` is a string id ("Not enough wood", "You
  need to build more houses"), separate from the order, so the UI can say it in the player's language.
- **A rollover help box**, above the panel, filled every frame by whoever knows what the cursor is over
  (`SetWorldHint`): the button's help and its key, or what a right-click would do in the world. Its text is rich
  (`<b>/<i>` and `<cost>/<hp>/<attack>` filled with the player's own figures after technologies).
- **Occlusion is decided by the art's own pixels** - `PointOverHud` asks the interface bitmap's alpha (`atlas->Covers`),
  so a click through a transparent gap in the carving reaches the map. In the stretched band it is a rectangle test instead.
- **The parts scale**: Options sizes the top bar, the panel and the text from 100% to 200%; a part is laid out for a
  narrower frame and copied into the real one at that scale, and every point is mapped into the part's terms before a hit
  test (`HudSpace::ToSpace`).
- **Around the map, not just the bars**: a production-queue overview (icons greened as they progress), message lines and
  minimap alerts, a clock and wonder countdowns, scores, and the minimap's three modes with buttons for idle villager,
  statistics, help and advanced commands.
- **Menus are a separate, simpler layer**: `screens/Widgets` is plain data + free functions (`Button`, `DropDown`,
  `CheckBox`, `Slider`, `ListBox`, `TextField`, `Notice`) with `Draw*`, `Hit`, `Click*`; a screen never touches the
  simulation, it returns a `Request`.

## 2. What we already have (measured in this tree today)

| Piece | Where | State |
|---|---|---|
| Top bar, 30 px | `ui::hud`, `hud_bar_height` | stock: swatch + **pack's name** + number, population, five speed boxes |
| Bottom panel, 76 px | `ui::hud`, `hud_panel_height` | up to 5 selection rows (health bar, load, queue, build progress), a refusal notice |
| Minimap | `ui::minimap_rect` / `minimap_tile` | boxed, 2 px/tile, click centres the camera; **no buttons round it** |
| Command card | `ui::card_rows` / `card_at` / `draw_card`, `World::card` | 4 per row, name + price + red refusal, one building, no pages, **no build buttons** |
| Menu | `ui::menu_rows` / `menu_at` / `draw_menu` | RESUME / RESTART / QUIT, modal |
| Font | `font.rs` | 3x5 bitmap, digits + 26 capitals + punctuation |
| Input routing | `window.rs` | speed → card → minimap → click/box/double-click, each checked in that order |

So the skeleton exists: a bar, a panel, a card whose geometry is one function used by both drawing and clicking (the
openaok invariant, arrived at independently), a minimap with one projection, a font with words. The whole view is
**2,808 lines** (`ui.rs` 699, `lib.rs` 1,262, `window.rs` 737, `font.rs` 110), 29 public functions in `ui.rs`. **What is missing is the
quality**: no hover/pressed/disabled look, no anchors (everything is `8 px` and a fixed height), no age readout, no
buttons in the top bar, no help box, no group cards, no queue strip, and the card *floats above* the panel instead of
living inside a bottom bar.

## 3. The gap, item by item

**A. The two bars become one layout with anchors** - `S` (half a step).
A `HudLayout` of our own, but anchored rather than tabled per resolution: `Bar { height, left, right }` and
`Panel { height, left, middle, right }` where "right" is measured from the frame's right edge. That is openaok's `Edged`
insight without the per-size tables (we have no art to match): the bar's buttons stick to the right edge, the stockpile to
the left, the panel's grid to the left, its minimap to the right, and a wide frame simply gets a longer middle. It also
removes today's magic numbers (`8`, `76`, `96x28`) from five functions.

**B. A widget look with states** - `S`. 
`Widget { rect, label, state }` with `Normal/Hover/Pressed/Disabled/Selected`, and one `draw_widget` that paints a bevel
(or, for us, a lit top edge + shadowed bottom edge) in state colours. Everything we already draw as a bare rect - speed
boxes, card buttons, menu rows - moves onto it, which is what makes the UI *feel* like a UI rather than coloured
rectangles. This is the single highest-value item for "looks like a game".

**C. The command grid inside the bottom bar, 5x3 with pages** - `M`.
Replace the flowing card block with a **fixed 15-slot grid** in the panel, left of the selection area: slot assignment
stable per selection (so a unit's button does not jump when a tech becomes available), page switching for a villager's
economic/military build menus, and `SwitchPage` handled inside the card view without reaching the sim - openaok's exact
split. `World::card` already returns what is needed; it grows a `slot` and a `page`.

**D. The selected-object area and group cards** - `S`.
The panel's middle: the name of what is selected (the font has letters now), its health, and - for a group - a grid of
small cards with the count, instead of today's five rows. openaok caps at 60; ten is plenty for us.

**E. The age readout and five top-bar buttons** - `S`.
Age in the bar's middle (the pack's `[tech_role]` names already exist: `dark_age`, `feudal_age`, ...), and five buttons on
the right: **menu** (already have it), **objectives/stats**, **chat/notices**, **diplomacy**, **tech tree**. With
placeholders the last three can be honest stubs - a button that opens a panel saying "not yet" is better than no button,
and it is where the message and alert layers will hang.

**F. The rollover help box** - `S/M`.
A box above the panel, filled per frame: the hovered button's label, its price and why it is refused (we already have all
three in `CardButton`); or, over the world, what a right-click would do ("attack", "gather wood", "walk"). openaok's
`SetWorldHint` pattern, and it is the cheapest teaching device in the whole interface.

**G. Hotkeys for the grid** - `S/M`.
Nine keys per page (openaok reads the original's defaults from the EXE, we have no such source and will not invent them):
a content table `[hotkey] villager = "b"` style, so a mod can rebind, plus the help box naming the key.

**H. Input occlusion by what was painted** - `S`.
Today the window asks each surface in turn (speed, card, minimap) and the order is a rule somebody has to remember.
openaok asks the *mask*: a click on the HUD never reaches the map, and a click where the HUD is transparent does. For us:
a one-bit-per-pixel mask built while drawing (a `Vec<u64>`), then one `over_hud(pixel)` test - and a test that walks every
pixel of a frame and asserts the mask agrees with what was painted.

**I. Queue strip and messages/alerts** - `M` each, and **not needed for "bars"**. A production overview at the map's top
left (we have queue state in the snapshot), and two message lines over the map with an attack alert on the minimap. Keep
for after the bars.

## 4. Order I would take, and why

1. **A + B** (anchored layout, widget states) - everything after it is cheaper, and it changes nothing the tests cannot
   pin (pure functions returning rects).
2. **C + D** (the 5x3 grid with pages, the selected-object area) - the bottom bar becomes a bar rather than a stack.
3. **E + F** (age, five buttons, help box) - the top bar becomes a bar, and the interface starts explaining itself.
4. **H** (occlusion mask) - small, and it deletes three special cases from the input path.
5. **G** (hotkeys), then **I** (queue strip, messages) if wanted.

Each is a normal step in this tree: compact, test-first on the pure functions, adversarial review with the frame cost
measured, push. Roughly five steps, not one.

## 5. What I would *not* copy, and why

- **The three fixed 4:3 tables and the art-cut stretch.** They exist because the original's art is a fixed bitmap; we draw
  procedurally, so anchors give the same behaviour with a tenth of the numbers.
- **The GDI/FreeType text pipeline and LANGUAGE.DLL.** Beautiful fidelity, and a month of work. Our 3x5 bitmap already says
  words; a 5x7 or 6x8 font later, still ours, keeps it licence-clean and pixel-exact.
- **The per-part scaling (bar 100-200%).** A nice option, and it doubles the layout work (points mapped into part space).
  Not worth it before the parts are right.
- **The 28 command kinds.** We have seven real ones (train, research, build, gather, attack, walk, stance). The grid should
  be built for a *growing* list, but only the kinds the engine can actually execute are drawn - a Stop button with nothing
  behind it is a lie.

## 6. Two things worth stealing outright, today

1. **"Drawing and clicking both go through this."** We do it for the card and the minimap; the rule should become explicit
   in `ui.rs` - every interactive surface returns its rects from one function, and the hit test is that function.
2. **A refusal id, not a sentence.** `World::card`'s `refusal` is the engine's `Display` text today (flagged in the P1
   review). openaok's `Refusal -> string id` is the right shape: the engine answers *which* refusal, the view owns the
   words, and a pack can translate them. One `RefusalKind` enum, and the help box and the notice both use it.

## 7. Decisions I need before starting

- **The bottom bar's shape**: the game's own layout (grid left, selected object middle, minimap right, in one bar) or keep
  today's stacked panel (rows + card above)? I recommend the game's - it is what makes it read as "a bottom bar".
- **Placeholders**: flat rectangles in our palette with a bevel (no icons at all), or one-letter/one-glyph marks per
  command (a letter inside the button, which the font can do)? I recommend the letter: a button that says "VIL" is
  readable now and replaceable by an icon later without a layout change.
- **Hotkeys**: invent a default set now (documented as ours, not the game's), or wait until the grid exists? I recommend
  waiting one step, then a `[hotkey]` content table.
