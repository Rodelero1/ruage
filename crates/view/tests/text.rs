//! Words: the font grows letters, and the HUD can say what a number is.
//!
//! `font.rs` said it plainly - "Digits only, deliberately ... When names are wanted, this is the file that grows" - and the
//! owner's queue wants names: resources, what is selected, why an order was refused, and (next) a command card whose buttons
//! have to be readable. So the table grows here, in the same 3x5 bitmap, and everything that already draws numbers keeps
//! drawing them.
//!
//! Written before the implementation (D7, SPEC §11).

use ruage_view::{Frame, font, palette};

fn lit(frame: &Frame, x: u32, y: u32, w: u32, h: u32) -> u32 {
    let mut count = 0;
    for py in y..y + h {
        for px in x..x + w {
            if px < frame.width && py < frame.height && frame.pixel(px, py) != [0, 0, 0, 0] {
                count += 1;
            }
        }
    }
    count
}

/// **A letter is exactly the dots its glyph says**, the same measurement the digits have: "something is on the screen" is
/// not the claim, "these dots and no others" is.
#[test]
fn a_letter_lights_exactly_the_dots_its_glyph_says() {
    let mut frame = Frame::new(16, 16);
    let used = frame.draw_text(1, 1, "A", 1, palette::HUD_TEXT);
    let rows = font::glyph('A').expect("a glyph for A");
    let dots: u32 = rows.iter().map(|row| row.count_ones()).sum();
    assert_eq!(
        lit(&frame, 0, 0, 16, 16),
        dots,
        "A's lit pixels are not its dots"
    );
    assert_eq!(used, font::WIDTH, "one glyph's width");
}

/// **What a string measures is what it draws**, because a panel that lays out text it cannot measure overlaps its own
/// labels. Four letters and three gaps for "WOOD".
#[test]
fn a_word_measures_what_it_draws() {
    let mut frame = Frame::new(64, 16);
    let expected = 4 * font::WIDTH + 3 * font::GAP;
    assert_eq!(font::text_width("WOOD", 1), expected);
    assert_eq!(
        frame.draw_text(0, 0, "WOOD", 1, palette::HUD_TEXT),
        expected
    );
}

/// **Case is not information.** The pack's names are whatever the person who wrote the pack typed; a HUD that dropped
/// "wood" because it only has capitals would be a HUD that shows nothing for half the content in the world.
#[test]
fn lowercase_draws_as_uppercase() {
    let mut lower = Frame::new(32, 16);
    let mut upper = Frame::new(32, 16);
    lower.draw_text(1, 1, "wood", 1, palette::HUD_TEXT);
    upper.draw_text(1, 1, "WOOD", 1, palette::HUD_TEXT);
    assert_eq!(lower.to_ppm(), upper.to_ppm());
}

/// **A character the font does not have is skipped, not a panic and not a wrong glyph.** A pack may name a resource in a
/// script this font has no shapes for; the HUD must still draw the rest of the line.
#[test]
fn an_unknown_character_is_skipped() {
    let mut with = Frame::new(64, 16);
    let mut without = Frame::new(64, 16);
    let used = with.draw_text(0, 0, "A?B", 1, palette::HUD_TEXT);
    without.draw_text(0, 0, "AB", 1, palette::HUD_TEXT);
    assert_eq!(
        with.to_ppm(),
        without.to_ppm(),
        "an unknown character changed what was drawn"
    );
    assert_eq!(used, font::text_width("AB", 1), "and took no room");
}

/// Every glyph in the table is 3x5 rows of three bits - a table with a row of `0b1111` would draw a fourth column into the
/// next glyph's gap, which is the kind of thing that looks like a font bug for a week.
#[test]
fn every_glyph_fits_the_box() {
    for character in font::ALPHABET.chars().chain(font::DIGITS_TEXT.chars()) {
        let rows = font::glyph(character).unwrap_or_else(|| panic!("{character} has no glyph"));
        assert_eq!(
            rows.len(),
            font::HEIGHT as usize,
            "{character} has the wrong height"
        );
        for row in rows {
            assert_eq!(
                *row & 0b1111_1000,
                0,
                "{character} has a pixel outside three columns"
            );
        }
    }
}
