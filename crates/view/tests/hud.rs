//! The HUD: a player has to see what the simulation already knows - stock by column, population, and what was selected.
//! D82 settled the font: a **bundled bitmap font**, no parser and no dependency. Written before the implementation
//! (D7, SPEC §11).
//!
//! Text is drawn here and nowhere else: the simulation carries no strings, and the HUD reads a `Snapshot` (D74, D55).

use ruage_view::ui;
use ruage_view::{Camera, Frame, Snapshot, font, palette, render};

fn snapshot() -> Snapshot {
    Snapshot {
        tick: 40,
        player: 1,
        width: 32,
        height: 32,
        terrain: vec![false; 32 * 32],
        stock: vec![7, 42, 3],
        population: 5,
        population_cap: 15,
        outcome: None,
        visible: vec![u64::MAX; 16],
        explored: vec![u64::MAX; 16],
        entities: Vec::new(),
    }
}

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

/// The placer puts the glyphs where the table says, one pixel per lit dot at scale 1: this is the measurement that makes
/// \"the number is drawn\" mean something other than \"something is on the screen\".
#[test]
fn a_number_lights_exactly_the_dots_its_glyphs_say() {
    let mut frame = Frame::new(64, 16);
    let used = frame.draw_number(2, 2, 412, 1, palette::HUD_TEXT);

    let expected: u32 = [4usize, 1, 2]
        .iter()
        .map(|digit| {
            font::DIGITS[*digit]
                .iter()
                .map(|row| row.count_ones())
                .sum::<u32>()
        })
        .sum();
    assert_eq!(
        lit(&frame, 0, 0, 64, 16),
        expected,
        "the number of lit pixels is not the number of dots in 4, 1 and 2"
    );
    assert!(used > 0, "drawing a number used no width");

    // And the picture is of the number asked for: 412 is not 123, and neither is blank.
    let mut other = Frame::new(64, 16);
    other.draw_number(2, 2, 123, 1, palette::HUD_TEXT);
    assert_ne!(frame.to_ppm(), other.to_ppm());
}

/// Scale multiplies the glyph: at scale 2 a digit covers four times the pixels, which is what makes the HUD readable at
/// 1x zoom without a second font.
#[test]
fn scale_multiplies_the_glyph() {
    let mut small = Frame::new(64, 32);
    small.draw_number(2, 2, 8, 1, palette::HUD_TEXT);
    let mut large = Frame::new(64, 32);
    large.draw_number(2, 2, 8, 2, palette::HUD_TEXT);
    assert_eq!(
        lit(&large, 0, 0, 64, 32),
        lit(&small, 0, 0, 64, 32) * 4,
        "scale 2 is not four times the pixels"
    );
}

/// The HUD reports the stock in the pack's column order, one swatch per column, plus population - and it stays in its
/// bar: a HUD that painted over the whole frame would hide the game it is reporting on.
#[test]
fn the_hud_shows_one_swatch_per_column_and_the_population() {
    let snapshot = snapshot();
    let camera = Camera::new((320, 240)).centred_on((16.0, 16.0));
    let plain = render(&snapshot, &camera);
    let mut with_hud = render(&snapshot, &camera);
    ui::hud(&mut with_hud, &snapshot);

    // A swatch per stock column, in order, each with digits beside it.
    for (column, colour) in palette::STOCK.iter().take(snapshot.stock.len()).enumerate() {
        let x = ui::hud_swatch_x(column);
        let y = ui::hud_bar_height() / 2;
        assert_eq!(
            with_hud.pixel(x + 2, y),
            *colour,
            "column {column}'s swatch is not its colour"
        );
        assert!(
            lit(&with_hud, x + 14, 2, 40, 20) > 0,
            "column {column} shows no number beside its swatch"
        );
    }

    // Population and the game itself are both still there.
    assert!(
        lit(&with_hud, ui::hud_swatch_x(snapshot.stock.len()), 2, 60, 20) > 0,
        "no population reported"
    );
    let below = ui::hud_bar_height() + 4;
    for y in below..240 {
        for x in 0..320 {
            if plain.pixel(x, y) != [0, 0, 0, 0] {
                assert_eq!(
                    with_hud.pixel(x, y),
                    plain.pixel(x, y),
                    "the HUD painted over the game at ({x},{y})"
                );
            }
        }
    }
}

/// An empty stock list draws no swatches: the HUD follows the pack rather than assuming four resources exist.
#[test]
fn a_pack_with_no_stock_draws_no_swatches() {
    let mut snapshot = snapshot();
    snapshot.stock.clear();
    let camera = Camera::new((320, 240)).centred_on((16.0, 16.0));
    let mut frame = render(&snapshot, &camera);
    ui::hud(&mut frame, &snapshot);
    for (column, colour) in palette::STOCK.iter().enumerate() {
        let x = ui::hud_swatch_x(column);
        assert_ne!(
            frame.pixel(x + 2, ui::hud_bar_height() / 2),
            *colour,
            "a swatch for column {column} was drawn from nothing"
        );
    }
}
