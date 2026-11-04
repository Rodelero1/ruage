//! The command card on the screen: one button per thing the selection could make or research.
//!
//! The engine answers *what* (`World::card`, tested in `crates/sim/tests/card.rs`); this file is about *where* and *what it
//! looks like* - and, like every other piece of the view, it is a pure function of its arguments so it can be asserted
//! without a display. Written before the implementation (D7, SPEC §11).

use ruage_view::{Frame, palette, ui};

fn button(name: &str, costs: Vec<(usize, u32)>, refused: Option<&str>) -> ui::CardButton {
    ui::CardButton {
        name: name.to_owned(),
        costs,
        refusal: refused.map(str::to_owned),
    }
}

fn count_colour(frame: &Frame, x: u32, y: u32, w: u32, h: u32, colour: [u8; 4]) -> u32 {
    let mut count = 0;
    for py in y..y + h {
        for px in x..x + w {
            if px < frame.width && py < frame.height && frame.pixel(px, py) == colour {
                count += 1;
            }
        }
    }
    count
}

/// **One button per thing, five to a row, in the panel's command grid.** The card used to float above the bar; the game's
/// own arrangement - and the owner's answer - is a five-by-three grid *inside* the bar, so a button is never anywhere but
/// where the game puts it.
#[test]
fn there_is_a_button_per_item() {
    let (width, height) = (960, 720);
    let rows = ui::card_rows(6, width, height);
    assert_eq!(rows.len(), 6, "one rectangle per item");
    let grid = ui::Layout::of(width, height).grid;
    for (x, y, w, h) in &rows {
        assert!(
            *x >= grid.0 && *y >= grid.1,
            "a button starts outside the command grid"
        );
        assert!(
            x + *w as i32 <= grid.0 + grid.2 as i32 && y + *h as i32 <= grid.1 + grid.3 as i32,
            "a button ends outside the command grid"
        );
    }
    // The fifth is still on the first row, the sixth starts the second: five across, three down.
    assert_eq!(rows[4].1, rows[0].1, "five to a row");
    assert!(
        rows[5].1 > rows[0].1,
        "the sixth button starts a second row"
    );
    assert_eq!(rows[5].0, rows[0].0, "and in the first column");
    assert!(
        ui::card_rows(20, width, height).len() == 15,
        "the grid clips at fifteen"
    );
}

/// **A click on a button picks that button, and a click between them picks nothing.** The gap between buttons is a gap.
#[test]
fn a_click_picks_the_button_under_it() {
    let (width, height) = (960, 720);
    let rows = ui::card_rows(3, width, height);
    for (index, (x, y, w, h)) in rows.iter().enumerate() {
        let middle = (x + *w as i32 / 2, y + *h as i32 / 2);
        assert_eq!(
            ui::card_at(middle, 3, width, height),
            Some(index),
            "the middle of button {index} does not pick it"
        );
    }
    assert_eq!(
        ui::card_at((2, 2), 3, width, height),
        None,
        "the corner is not a button"
    );
}

/// **A button says its name, its price, and whether it would be refused.** The refused one is painted in the refusal colour
/// and the affordable one is not: a card where everything looks available is a card that teaches nothing.
#[test]
fn a_button_shows_its_name_price_and_refusal() {
    let (width, height) = (960, 720);
    let buttons = vec![
        button("VILLAGER", vec![(0, 50)], None),
        button("SPEARMAN", vec![(0, 60), (2, 20)], Some("unit unavailable")),
    ];
    let mut frame = Frame::new(width, height);
    ui::draw_card(&mut frame, &buttons, width, height, None);
    let rows = ui::card_rows(buttons.len(), width, height);

    let (x, y, w, h) = rows[0];
    assert!(
        count_colour(&frame, x as u32, y as u32, w, h, palette::HUD_TEXT) > 0,
        "the first button has no name"
    );
    assert_eq!(
        count_colour(&frame, x as u32, y as u32, w, h, palette::REFUSED),
        0,
        "an available button was painted as refused"
    );

    let (x, y, w, h) = rows[1];
    assert!(
        count_colour(&frame, x as u32, y as u32, w, h, palette::REFUSED) > 0,
        "a refused button does not say so"
    );
}
