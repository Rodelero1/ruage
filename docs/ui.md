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

> The screenshot the owner linked could not be read on this host - the image tool timed out three times on it (1440x900,
> 306 KB, and again halved). The layout above is from the game itself, not from that image; if the panel differs in a way
> that matters, the difference is a layout constant, not a design.

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
5. **A name to draw.** Content's role word (`town_centre`, `villager`) is the only vocabulary we have; the dat's name codes
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
