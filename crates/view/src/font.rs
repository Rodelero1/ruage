//! A 3x5 bitmap font: ten digits and a slash. That is everything a HUD has to report - stock, population, the tick -
//! and D82 chose a bundled bitmap font over a TTF because it needs no parser, no dependency and no licence review.
//!
//! Digits only, deliberately: a letter table is five times the data and no more information when the swatches beside the
//! numbers say which resource a number is. When names are wanted, this is the file that grows.

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
