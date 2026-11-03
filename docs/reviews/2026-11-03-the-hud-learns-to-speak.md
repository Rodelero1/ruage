# The HUD learns to speak, and Escape stops killing the game

**Step:** P1 of the owner's queue (2026-11-03): feedback - what an order was refused for, and a menu instead of a
process that exits.

---

## 1. The font grows letters

`font.rs` said it itself: *"Digits only, deliberately ... When names are wanted, this is the file that grows."* Names are
wanted - by the HUD (which resource is which), by the refusal notice, and next by the command card, whose buttons are named
after units, buildings and techs that come from a content pack. So:

- `font::LETTERS` - 26 capitals, 3x5, the same bitmap as the digits; `font::glyph` **folds lower case onto the capitals**
  (a pack's spelling is the pack's business) and returns `None` for a character it has no shape for;
- punctuation the HUD actually needs: `: . - + _`;
- `Frame::draw_text` returns the width it used and `font::text_width` predicts it, so two labels laid side by side cannot
  overlap by arithmetic;
- **an unknown character is not drawn and takes no room** - the alternative is a box or a question mark standing in for
  somebody's language.

Five tests, written first and watched failing to compile (`glyph`, `text_width`, `ALPHABET` did not exist): a letter lights
exactly the dots its glyph says; a word measures what it draws; lower case equals upper case pixel for pixel; an unknown
character changes nothing; every glyph in the table fits three columns (a row of `0b1111` would draw into the next glyph's
gap and look like a font bug for a week).

## 2. What the HUD says now

- **A name under every stock swatch**, at one pixel a dot, in the pack's own word for the column. The bar grew from 26 to
  30 pixels to hold both the number and its name.
- **A refusal on the panel**, in the engine's words, for three seconds of match time. The window used to put it in the
  window *title* - which is not where a player looks, and on a borderless fullscreen window is not visible at all. The
  window now also clears the order marker when an order is refused, because a marker that stays says the order landed.
- **The Escape menu**: RESUME, RESTART, QUIT. Modal - it washes the world, it takes the clicks, and `about_to_wait` does
  not advance the clock while it is open. Escape used to **exit the process**, which is how a player loses a match they
  were winning. Restart rebuilds the world from the same config and seed, so two restarts of one match are the same match.

Screenshots, rendered headlessly by `bin/frame` (which now takes a fifth argument, `menu`, so the modal can be looked at
without a display): `MEDIA` in the step report. The words are legible at 640x480: FOOD, WOOD, GOLD, STONE under the
swatches; RESUME / RESTART / QUIT on the menu.

## Performance

- **Text is drawn per character, per frame, and there are few characters**: four names of four letters plus a notice of
  under twenty is ~40 glyphs at one pixel a dot - 600 `fill_rect` calls at worst, against the minimap's 16,384 `put`s
  measured in the previous step (151-195 us). No new measurement was needed to see that it cannot matter, and the frame
  cost benchmark is unchanged within its own spread.
- **The menu wash is the one expensive draw**: a full-frame pass (307,200 pixels at 640x480) because a wash has to mix with
  what is behind it. It runs only while the menu is open, and while the menu is open **no simulation ticks happen**, so the
  frame it costs is a frame where nothing else is being paid for. This is the trade recorded rather than optimised.

## Stability

- **Escape no longer destroys the match.** The single most expensive line in the old window was
  `Key::Named(NamedKey::Escape) => event_loop.exit()`.
- **The menu is modal in three places at once** - input (`click` returns after the menu handles it), the clock
  (`about_to_wait`'s while-condition) and drawing (the wash covers everything). A modal that only covers one of the three is
  a menu the match runs under.
- **`Hud.notice` is a `String` the caller sets**, so `ui::hud` stays a pure function of its arguments - the window decides
  how long a notice lives (`notice_ticks`), the view decides where it goes.
- **Restart resets every piece of window state that outlives a world**: world, runner, ticks, accumulator, selection,
  notice, order marker, menu. A field left behind would be a bug that only shows on the second match.
- **A refusal notice is cleared by the next accepted order**, so a player who fixes the problem sees the message go.
- **Known limitation, recorded:** the notice is the engine's `Display` text, so it is developer English ("no free tile for a
  farm"). It is readable, it is not a designed message. A term table belongs with the command card, where the same words
  will be needed.
