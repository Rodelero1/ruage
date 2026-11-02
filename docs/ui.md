# The command panel: where "you cannot train this" is shown

*Compact, written 2026-10-31 before the code. The owner's instruction: the message about being unable to train a unit
belongs in the UI, so **the UI comes first** - there is no message to show until there is a panel to show it in.*

## What the game shows, and what we have

The original's screen is three bands: a **resource bar** across the top, the **map**, and a **command panel** across the
bottom. The panel, left to right, is: the selected object's **portrait** and its stats; a grid of **command buttons**
(train, research, rally, cancel); and the **production queue** with per-item progress. A warning about training - no room,
no garrison space, not enough of a resource - is shown **in that panel**, next to the queue, because that is where the eye
already is when the order was given.

We have, in `crates/view/src/ui.rs`: `Selection` (click, rubber band, double-click - all tested), `Hud` (stock by column,
population), `minimap`, and a 3x5 bitmap font holding **digits only** (`font.rs`, D82). The window draws the world, a HUD
bar and selection marks; it draws **no panel, no portrait, no buttons and no queue**.

> **The screenshot was read on the second attempt** (the owner re-sent it; the first three fetches timed out). What it
> settles is below, and two of the details change the plan.

## What the panel needs from the simulation, which `Snapshot` does not carry yet

Everything below is data the engine already has; none of it needs a new simulation rule.

1. **What a selected building can make.** `economy.train` already says which roles a building may produce; the engine
   resolves it through `[role]`. The snapshot needs, per selected building, the list of `(role, unit id, cost, train
   time)`, plus **whether it is currently available** - unit availability (kind-2 gating), tech prerequisites, and whether
   the unit's upgrade tech is researched - because a button that cannot ever be pressed must be drawn differently from one
   that is merely unaffordable.
2. **The queue, with progress.** `EntityView` carries `pending: usize` today. It needs
   `queue: Vec<QueueItem { unit: u32, progress: f32 }>` - the item in production first, then what waits behind it.
3. **Why a queue is stopped.** D85's rule: a trained unit with nowhere to stand is garrisoned; if the building has no room
   either, **training pauses**. So `EntityView` needs `paused: Option<TrainingPause>`, and the variants are content-free:
   `NoRoom` (nowhere to place and no garrison space) and `Unaffordable` (the next item's cost is gone, e.g. a resource
   spent elsewhere mid-queue).
4. **Affordability and headroom, for dimming a button.** From `stock` and `population`/`population_cap`, both already in
   the snapshot - the panel computes it, the engine does not need to.
5. **A name to draw.** Content's role word (`town_center`, `villager`) is the only vocabulary we have; the dat's name codes
   are a byte array (`RTWC`), not a display string. Role words are lowercase with underscores - fine for now, and a
   content-side `[display]` table can replace it later without touching the engine.

## What has to be drawn, and the primitives it needs

- a **panel frame** (dark rectangle, bottom of the frame, height a constant of the view, not content);
- the **portrait**: a filled diamond in the entity's player colour with its footprint drawn inside it - the same silhouette
  the map draws, so the panel and the map agree by construction;
- **command buttons** in a grid (5 columns x 3 rows in the original; we start with one row of what the building can make),
  each a square with a colour for the unit, its cost as digits, and a **dim state** for unavailable or unaffordable;
- the **queue**: small squares in order, each with a progress bar drawn over it - the same `fill_rect` the health bar uses;
- a **message line** under the queue, which needs the letter set: `NO ROOM - GARRISON FULL`, `NOT ENOUGH WOOD`.
- **letters**: `font.rs` grows A-Z (3x5) and a handful of punctuation (space, dash, digits and `/` already exist), plus
  `Frame::draw_text(x, y, text, scale, colour)` and a `measure_text` so a message can be right-aligned beside the queue.

## The tests, all runnable with no display

The panel is a **pure function of the snapshot plus the selection plus the content**, exactly like `pick_at`:

1. `trainable(&snapshot, content, building)` returns the roles a barracks may make and **not** the ones it may not, and
   marks a Feudal-gated unit unavailable in the Dark Age (the availability rules already have tests in `sim`).
2. `panel_layout(viewport)` returns the same rectangles every time, and the queue area sits inside the panel (no
   overlapping rects - an assertion a layout bug usually fails).
3. `panel_button_at(layout, pixel)` maps a click to the right button and to nothing outside them - and the command it
   produces is the **same `Command::Train`** a scenario submits, asserted by submitting it to a world and watching the
   queue fill, the way `pick_at`'s tests do.
4. A queue item at 50% draws **half** its progress bar green and half red (pixel counts, as the health bar test does).
5. `draw_text` renders "NO ROOM" with exactly the dots its glyphs say (the same measurement the digit test uses), and a
   message is placed beside the queue, not over it.
6. **D85's own test**: 20 villagers trained at a surrounded town centre end with 20 alive - inside the building or beside
   it - and a queue that can place nothing shows `NoRoom` and stops, instead of losing a recruit.

## Order of work

1. `Snapshot`: `queue` with progress, `paused`, and the trainable list (content-resolved, with availability).
2. `font.rs`: letters + `draw_text` + `measure_text`.
3. `ui::panel`: layout as data (frames, buttons, queue, message line) and `panel_button_at`.
4. Draw it; wire the window's selection to it; clicks on a button become `Command::Train`.
5. Then D85 in the simulation: garrison the unit that cannot be placed, pause the queue when there is no room, and let the
   panel draw the reason it is given.

Each step is its own commit with a review note, as `AGENTS.md` requires. Steps 1-3 are the gate: until the snapshot carries
a queue and a reason, no message can be drawn and no rule can be tested.

## What the screenshot settles (Age of Empires II HD, a Castle screen)

Read off the image rather than remembered:

- **The example's message is a transient flash, and ours is not.** *"Not enough food!"* in that screenshot is the game's
  one-off notification in the world view. The owner's correction (2026-10-31): ours is a **sticky status shown beside the
  queue, while the block lasts** - e.g. *"Building blocked"* - and it clears the moment training can continue. That is a
  different object: not an event line in the world, but state the panel reads every frame.
- **The panel has five regions**, left to right: the **selection block** (portraits of the selected units, here three
  villagers, plus one small button); the **building's own block** (its name in text - *Castle*; its portrait with a green
  health bar and `4800/4800`; `0/20` garrison; `11 (4)` attack with its bonus; `8` line of sight); the **queue**
  (`Creating - 21%`, the unit's name *Teutonic Knight*, a segmented progress bar, then the queued icons in order); the
  **command buttons** (one row of unit buttons at the bottom of the queue area, in blue frames); and the **minimap** in its
  own ornamental frame at the right.
- **The resource bar** carries an icon per resource, the numbers, and population as `449/120` behind a population icon;
  the middle tab shows the **current age** (*Imperial Age*) - worth copying, since the engine knows the age.
- **Progress is a segmented bar** (red where the item is not yet made, green for the part that is), which is the same
  `fill_rect` pair the health bar already uses.
- **The panel is parchment, not dark**: a light background with an ornamental border. Our palette is dark for the HUD bar,
  so the panel gets its own two colours rather than borrowing the HUD's.
- **An instruction line sits at the panel's bottom-left** (*"Click a villager to gather wood from this tree."*) - the
  place for hints and, later, for the engine's own refusals.

`docs/ui.md`'s data list is unchanged (the panel still needs the queue, the pause reason, the trainable list,
affordability and a name); what changed is **where the message goes** (above the panel, red, world-space) and that the
panel needs its own palette and a text line for hints.

## The queue's message is state, not an event

*"Building blocked"* is not a notification that scrolls past; it is the panel's rendering of `paused`, so:

- `EntityView::paused: Option<TrainingPause>` is the **only** source of the words. A message the UI tracks separately from
  the queue would eventually disagree with the queue, and a player reading "Building blocked" beside a unit that is being
  made is worse than no message.
- It is drawn **beside the queue**, in the queue's own block, in the warning colour, for exactly as long as `paused` is
  `Some` - and it disappears the frame the reason goes away (a tile frees up, a garrison empties, the resource returns).
- The wording is per reason, from a small table the view owns: `NoRoom` -> *"Building blocked"*, `Unaffordable` ->
  *"Not enough <resource>"*. The engine says **which** reason; the view says it in words, which is the same split as
  everywhere else (the simulation carries no text, D55/D74).
- A test, and it is the point of the feature: with a surrounded town centre the message is drawn, and after a tile frees up
  and stepping resumes, **the same frame has no message and the queue advances** - state, not event.

This replaces the earlier placement note: the world-space line in the screenshot was the original's transient notification,
which we do not copy. There is one message line, it lives beside the queue, and it is sticky.

## What the install's own UI data settles (probed 2026-10-31)

The owner's hypothesis - *the UI decorations may be civilisation-specific and already in the game's SLP files* - is half
right, and the half that is wrong changes the plan in our favour.

**What is in the install.** `Age2HD/resources/_common/drs/interface/` holds **354 files, 44 MB**: 170 `.slp`, 134 `.bina`,
50 `.wav`. The `.bina` files are **three formats under one extension** (a trap worth remembering: the extension lies):
- **60 screen definitions**, text, each naming the screen's `background1_files` SLP, `palette_file`, `cursor_file`,
  `button_file` and giving the **six bevel colours** (`243 220 207 169 117 84` - three light, three dark), `text_color1/2`,
  `focus_color1/2`, `state_color1/2` and a `shade_amount`. That block *is* the UI skin as data - the same six-value bevel
  the original's panels and buttons are drawn from.
- **32 JASC-PAL palettes** (`50500`-`50509`, `50530`-`50533`); 50500 is the default drawing palette.
- **42 others**: BMP images under numeric ids (magic `BM`) and small text tables.

**What is not in the install: the panel itself.** Across all **1,938** SLPs in `graphics` and `interface`, **zero frames**
measure 700-1100 x 60-170 - there is no wide panel strip to swap in - and none of the 60 screen definitions is the in-game
HUD (they are `dlg*`, `scr*`, `main*`, `cam*`, `psel`, `credits`, `tml_bck`, `dlg_gam`). So the original **composes** the
panel from small pieces plus the bevel colours, which is why so many definitions read `background1_files none` and still
name a bevel.

**Civ-specific: the icons, not the decorations.** Two readings were falsified by the data, in this order:
- The four-identical-size families (`53161`-`53164`, `53171`-`53174`, `53101`-`53104`, `50235`-`50238`) looked like
  per-icon-set variants. They are **campaigns 1-4**: `53121.bina` names `53101` as `xcam1`, `53141` names `53161` as
  `xcamdlg1`, `53151` names `53171` as `camdlg1`, and `50251`-`50254` name `50235`-`50238` as `cam1`-`cam4`. Pixel-diffing
  frame 0 across each family puts them **98%+ different**: different pictures, not recolours.
- What *is* per-civ is `icon_set` in the dat - **five values across 32 civs** (1: 6 civs, 2: 7, 3: 4, 4: 12, 5: 3). That
  selects **icon art** (the unit and building buttons the panel shows), never the frame. The 36x36 icon sheets exist
  (`50705`-`50708`, 52 frames each) and are pixel-identical on frame 0, so an icon set is not yet a separate sheet family;
  that question is answered when the sprite binding lands, not now.

**What it changes here.**
1. The panel's **frame is drawn, not assembled** - a bevel from six values, which is what the original does. No asset work;
   our `fill_rect` can do it.
2. The panel's **colours are data with provenance, not invented**: a `[ui]` table (bevel light/mid/dark, text, warning)
   carrying the values above tagged `PROVENANCE: install interface .bina`, so "no numeric literals in code" and D74's
   "the simulation carries no text" both hold.
3. The panel's **portraits and button icons are install art** through `SpriteRef` and the mod chain (D81), civ-selected by
   `icon_set` when the sprite binding lands - the boundary already exists, so the panel does not change shape.
4. **A validated SLP decoder now exists as a throwaway probe** (format from openage's `doc/media/slp-files.md`, read not
   copied): it checks every row against its outline budget and the row's end-of-row `0x0F`, and decoded thirty-odd
   interface frames without one mismatch - which is how the pixel claims above are trusted. It is scratch, not committed
   (`build/` and scratch are excluded), and it belongs in `ruage-tools` when the sprite slice lands rather than written
   twice: it is the decode half of `SpriteRef`.
