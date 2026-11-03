//! A 3x5 bitmap font: ten digits, a slash, and (since the HUD grew words) the capital letters and a little punctuation.
//! D82 chose a bundled bitmap font over a TTF because it needs no parser, no dependency and no licence review, and this file
//! is where it grows.
//!
//! **Capitals only, and that is enough.** Names come from the content pack, so the pack's own spelling is what a player
//! reads; `glyph` folds lower case onto the capitals rather than carrying a second table, and a character the font has no
//! shape for is skipped rather than drawn as something else.

/// The width of one glyph, in font pixels.
pub const WIDTH: i32 = 3;
/// The height of one glyph, in font pixels.
pub const HEIGHT: i32 = 5;
/// The gap between glyphs, in font pixels.
pub const GAP: i32 = 1;

/// Digits `0..=9`, five rows each, the low three bits read left to right.
pub const DIGITS: [[u8; 5]; 10] = [
    [0b111, 0b101, 0b101, 0b101, 0b111], // 0
    [0b010, 0b110, 0b010, 0b010, 0b111], // 1
    [0b111, 0b001, 0b111, 0b100, 0b111], // 2
    [0b111, 0b001, 0b111, 0b001, 0b111], // 3
    [0b101, 0b101, 0b111, 0b001, 0b001], // 4
    [0b111, 0b100, 0b111, 0b001, 0b111], // 5
    [0b111, 0b100, 0b111, 0b101, 0b111], // 6
    [0b111, 0b001, 0b010, 0b010, 0b010], // 7
    [0b111, 0b101, 0b111, 0b101, 0b111], // 8
    [0b111, 0b101, 0b111, 0b001, 0b111], // 9
];

/// The separator in "5/15".
pub const SLASH: [u8; 5] = [0b001, 0b001, 0b010, 0b100, 0b100];

/// The capital letters, five rows each, the high three bits read left to right (`0b100` is the leftmost column).
pub const LETTERS: [[u8; 5]; 26] = [
    [0b010, 0b101, 0b111, 0b101, 0b101], // A
    [0b110, 0b101, 0b110, 0b101, 0b110], // B
    [0b011, 0b100, 0b100, 0b100, 0b011], // C
    [0b110, 0b101, 0b101, 0b101, 0b110], // D
    [0b111, 0b100, 0b110, 0b100, 0b111], // E
    [0b111, 0b100, 0b110, 0b100, 0b100], // F
    [0b011, 0b100, 0b101, 0b101, 0b011], // G
    [0b101, 0b101, 0b111, 0b101, 0b101], // H
    [0b111, 0b010, 0b010, 0b010, 0b111], // I
    [0b001, 0b001, 0b001, 0b101, 0b010], // J
    [0b101, 0b101, 0b110, 0b101, 0b101], // K
    [0b100, 0b100, 0b100, 0b100, 0b111], // L
    [0b101, 0b111, 0b111, 0b101, 0b101], // M
    [0b110, 0b101, 0b101, 0b101, 0b101], // N
    [0b010, 0b101, 0b101, 0b101, 0b010], // O
    [0b110, 0b101, 0b110, 0b100, 0b100], // P
    [0b010, 0b101, 0b101, 0b111, 0b011], // Q
    [0b110, 0b101, 0b110, 0b101, 0b101], // R
    [0b011, 0b100, 0b010, 0b001, 0b110], // S
    [0b111, 0b010, 0b010, 0b010, 0b010], // T
    [0b101, 0b101, 0b101, 0b101, 0b111], // U
    [0b101, 0b101, 0b101, 0b101, 0b010], // V
    [0b101, 0b101, 0b111, 0b111, 0b101], // W
    [0b101, 0b101, 0b010, 0b101, 0b101], // X
    [0b101, 0b101, 0b010, 0b010, 0b010], // Y
    [0b111, 0b001, 0b010, 0b100, 0b111], // Z
];

/// The letters the table holds, in table order: what a test walks to prove every glyph fits the box.
pub const ALPHABET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// The digits as characters, for the same walk.
pub const DIGITS_TEXT: &str = "0123456789";

/// Punctuation a HUD actually needs: a separator, a decimal point, a colon, a sign and an underscore.
pub const COLON: [u8; 5] = [0b000, 0b010, 0b000, 0b010, 0b000];
pub const DOT: [u8; 5] = [0b000, 0b000, 0b000, 0b000, 0b010];
pub const DASH: [u8; 5] = [0b000, 0b000, 0b111, 0b000, 0b000];
pub const PLUS: [u8; 5] = [0b000, 0b010, 0b111, 0b010, 0b000];
pub const UNDERSCORE: [u8; 5] = [0b000, 0b000, 0b000, 0b000, 0b111];

/// The rows for one character, or `None` when this font has no shape for it. **Lower case folds onto the capitals** -
/// the pack's names are the pack's business, and "wood" and "WOOD" have to draw the same thing.
#[must_use]
pub fn glyph(character: char) -> Option<&'static [u8; 5]> {
    match character {
        '0'..='9' => DIGITS.get(character as usize - '0' as usize),
        'A'..='Z' => LETTERS.get(character as usize - 'A' as usize),
        'a'..='z' => LETTERS.get(character as usize - 'a' as usize),
        '/' => Some(&SLASH),
        ':' => Some(&COLON),
        '.' => Some(&DOT),
        '-' => Some(&DASH),
        '+' => Some(&PLUS),
        '_' => Some(&UNDERSCORE),
        _ => None,
    }
}

/// How wide `text` is when drawn at `scale`: a glyph is `WIDTH` wide and `GAP` follows every one but the last, and a
/// character the font does not have takes no room at all (it is not drawn, so it is not spaced).
#[must_use]
pub fn text_width(text: &str, scale: i32) -> i32 {
    let scale = scale.max(1);
    let mut total = 0;
    for character in text.chars() {
        if glyph(character).is_none() {
            continue;
        }
        total += (WIDTH + GAP) * scale;
    }
    if total > 0 {
        total -= GAP * scale;
    }
    total
}
