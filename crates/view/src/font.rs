//! A **5x7** bitmap font: digits, capitals, and the punctuation a HUD actually needs.
//!
//! D82 chose a bundled bitmap font over a TTF because it needs no parser, no dependency and no licence review. It began at
//! 3x5 - two rows of counting, one of punctuation - and the owner asked for something readable (2026-11-04): "can we have
//! more readable font?" The answer was a proper 5x7: the shapes change, and the code that asks `WIDTH`/`HEIGHT` does not.
//!
//! **Capitals only, and that is enough.** Names come from the content pack, so the pack's own spelling is what a player
//! reads; `glyph` folds lower case onto the capitals rather than carrying a second table, and a character the font has no
//! shape for is skipped rather than drawn as something else.

/// The width of one glyph, in font pixels.
pub const WIDTH: i32 = 5;
/// The height of one glyph, in font pixels.
pub const HEIGHT: i32 = 7;
/// The gap between glyphs, in font pixels.
pub const GAP: i32 = 1;
/// How wide a **space** is, in font pixels: a space is not a shape, so it is a width rather than a bitmap. Half a glyph
/// plus the gap is the smallest break that reads as one - before this the font gave a space no room at all, and every
/// word in the HUD ran together (`WOOD100`, `HP88`, `FEUDALAGE`; found by looking at a rendered frame, 2026-11-04).
pub const SPACE: i32 = WIDTH / 2 + GAP;

/// Digits `0..=9`, seven rows each, the low five bits read left to right (bit 4 is the leftmost column).
pub const DIGITS: [[u8; 7]; 10] = [
    [
        0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
    ], // 0
    [
        0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
    ], // 1
    [
        0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
    ], // 2
    [
        0b11111, 0b00010, 0b00100, 0b00010, 0b00001, 0b10001, 0b01110,
    ], // 3
    [
        0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
    ], // 4
    [
        0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
    ], // 5
    [
        0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
    ], // 6
    [
        0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
    ], // 7
    [
        0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
    ], // 8
    [
        0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
    ], // 9
];

/// The separator in "5/15".
pub const SLASH: [u8; 7] = [
    0b00001, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b10000,
];

/// The capital letters, seven rows each, the high five bits read left to right.
pub const LETTERS: [[u8; 7]; 26] = [
    [
        0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
    ], // A
    [
        0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
    ], // B
    [
        0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
    ], // C
    [
        0b11100, 0b10010, 0b10001, 0b10001, 0b10001, 0b10010, 0b11100,
    ], // D
    [
        0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
    ], // E
    [
        0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
    ], // F
    [
        0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111,
    ], // G
    [
        0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
    ], // H
    [
        0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
    ], // I
    [
        0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100,
    ], // J
    [
        0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
    ], // K
    [
        0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
    ], // L
    [
        0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
    ], // M
    [
        0b10001, 0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001,
    ], // N
    [
        0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
    ], // O
    [
        0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
    ], // P
    [
        0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
    ], // Q
    [
        0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
    ], // R
    [
        0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
    ], // S
    [
        0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
    ], // T
    [
        0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
    ], // U
    [
        0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
    ], // V
    [
        0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001,
    ], // W
    [
        0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
    ], // X
    [
        0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
    ], // Y
    [
        0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
    ], // Z
];

/// The letters the table holds, in table order: what a test walks to prove every glyph fits the box.
pub const ALPHABET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// The digits as characters, for the same walk.
pub const DIGITS_TEXT: &str = "0123456789";

/// Punctuation a HUD actually needs: a separator, a decimal point, a colon, a sign, an underscore, the brackets a price or a
/// refusal uses, and a percent for a health readout.
pub const COLON: [u8; 7] = [
    0b00000, 0b00110, 0b00110, 0b00000, 0b00110, 0b00110, 0b00000,
];
pub const DOT: [u8; 7] = [
    0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00110, 0b00110,
];
pub const DASH: [u8; 7] = [
    0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
];
pub const PLUS: [u8; 7] = [
    0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000,
];
pub const UNDERSCORE: [u8; 7] = [
    0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b11111,
];
pub const LEFT_BRACKET: [u8; 7] = [
    0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0b00100, 0b00010,
];
pub const RIGHT_BRACKET: [u8; 7] = [
    0b01000, 0b00100, 0b00010, 0b00010, 0b00010, 0b00100, 0b01000,
];
pub const PERCENT: [u8; 7] = [
    0b11001, 0b11010, 0b00010, 0b00100, 0b01000, 0b01011, 0b10011,
];

/// The rows for one character, or `None` when this font has no shape for it. **Lower case folds onto the capitals** -
/// the pack's names are the pack's business, and "wood" and "WOOD" have to draw the same thing.
#[must_use]
pub fn glyph(character: char) -> Option<&'static [u8; 7]> {
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
        '(' => Some(&LEFT_BRACKET),
        ')' => Some(&RIGHT_BRACKET),
        '%' => Some(&PERCENT),
        _ => None,
    }
}

/// How wide `text` is when drawn at `scale`: a glyph is `WIDTH` wide with a `GAP` between neighbours, and a character the
/// font has no shape for takes no room at all - it is not drawn, so it must not be spaced.
#[must_use]
pub fn text_width(text: &str, scale: i32) -> i32 {
    let scale = scale.max(1);
    let mut total = 0;
    let mut drawn = 0;
    for character in text.chars() {
        // A space is room without a shape: it advances like a glyph and paints nothing.
        if character == ' ' {
            if drawn > 0 {
                total += GAP * scale;
            }
            total += SPACE * scale;
            drawn += 1;
            continue;
        }
        if glyph(character).is_none() {
            continue;
        }
        if drawn > 0 {
            total += GAP * scale;
        }
        total += WIDTH * scale;
        drawn += 1;
    }
    total
}
