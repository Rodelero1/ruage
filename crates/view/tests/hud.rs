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
    ui::hud(
        &mut with_hud,
        &panel(&snapshot, Vec::new(), None),
        &snapshot,
        ruage_view::Speed::One,
    );

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
    // The bottom panel and the minimap **are** painted over the game - that is what a HUD is - so this walks the band
    // between them: below the top bar, above the panel, and left of the minimap.
    let below = ui::hud_bar_height() + 4;
    let above_panel = 240 - ui::hud_panel_height() - 4;
    let (map_left, map_top, ..) = ui::minimap_rect(320, 240, snapshot.width, snapshot.height);
    for y in below..above_panel {
        for x in 0..320 {
            // The minimap is drawn in this band too, by design: it is the one part of the HUD that is the world.
            // One pixel of slack for the box the minimap is drawn in.
            if x as i32 >= map_left - 1 && y as i32 >= map_top - 1 {
                continue;
            }
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
    ui::hud(
        &mut frame,
        &panel(&snapshot, Vec::new(), None),
        &snapshot,
        ruage_view::Speed::One,
    );
    for (column, colour) in palette::STOCK.iter().enumerate() {
        let x = ui::hud_swatch_x(column);
        assert_ne!(
            frame.pixel(x + 2, ui::hud_bar_height() / 2),
            *colour,
            "a swatch for column {column} was drawn from nothing"
        );
    }
}

/// The `Hud` the rest of these tests drive: the snapshot's own stock and population, and whatever is selected. The names
/// are placeholders on purpose - the panel's numbers come from the snapshot and its layout from the pack's column order,
/// so a test that hard-coded "food" would be testing the test.
fn panel(
    snapshot: &Snapshot,
    selected: Vec<ui::SelectedRow>,
    outcome: Option<ruage_sim::MatchOutcome>,
) -> ui::Hud {
    ui::Hud {
        stock: snapshot
            .stock
            .iter()
            .enumerate()
            .map(|(column, amount)| (format!("column {column}"), *amount))
            .collect(),
        population: snapshot.population,
        population_cap: snapshot.population_cap,
        selected,
        outcome,
        notice: None,
    }
}

/// How many pixels of an exact colour are in a box: for text, "something is painted" is no claim at all - the bar's own
/// background is painted - so the tests count the glyphs' colour.
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

fn row(hp_fraction: f32) -> ui::SelectedRow {
    ui::SelectedRow {
        id: 83,
        kind: ruage_sim::EntityKind::Unit,
        hp_fraction,
        carry: 0,
        construction: None,
        garrison: 0,
        pending: 0,
        pose: ruage_view::Pose::Idle,
    }
}

/// **What is selected is on the screen.** The panel that reads `Hud::of` was written and tested before the window drew it;
/// the window drew the stock bar and nothing else, so a player could select six villagers and see no sign of it. This is
/// the test that says the panel is painted, in the colour the health bar's own rule gives it.
#[test]
fn the_selection_panel_paints_a_health_bar() {
    let mut frame = Frame::new(960, 720);
    ui::hud(
        &mut frame,
        &panel(&snapshot(), vec![row(0.4)], None),
        &snapshot(),
        ruage_view::Speed::One,
    );
    let (x, y) = ui::selection_row_origin(0, frame.height);
    assert_eq!(
        frame.pixel(x as u32 + 2, y as u32 + 1),
        palette::HP_RED,
        "a selected thing at 40% health has a red bar in the panel"
    );

    let mut bare = Frame::new(960, 720);
    ui::hud(
        &mut bare,
        &panel(&snapshot(), Vec::new(), None),
        &snapshot(),
        ruage_view::Speed::One,
    );
    assert_eq!(
        bare.pixel(x as u32 + 2, y as u32 + 1),
        palette::PANEL,
        "and with nothing selected the row is the panel's own background"
    );
}

/// **The minimap is drawn where `minimap_rect` says, pixel for pixel.** The function existed and was never called by the
/// window; this pins the copy, so a panel that is drawn somewhere else is a failure rather than a surprise.
#[test]
fn the_minimap_is_copied_into_the_corner() {
    let snapshot = snapshot();
    let mut frame = Frame::new(960, 720);
    ui::hud(
        &mut frame,
        &panel(&snapshot, Vec::new(), None),
        &snapshot,
        ruage_view::Speed::One,
    );
    let (x, y, w, h) = ui::minimap_rect(960, 720, snapshot.width, snapshot.height);
    let map = ui::minimap(&snapshot, w / u32::from(snapshot.width).max(1));
    for (px, py) in [(0, 0), (w / 2, h / 2), (w - 1, h - 1)] {
        assert_eq!(
            frame.pixel(x as u32 + px, y as u32 + py),
            map.pixel(px, py),
            "the minimap pixel at ({px}, {py}) was not copied into the panel"
        );
    }
}

/// **A decided match says so in colour**, because the font has digits and no words (D82). A banner in the middle of the
/// screen, green for a win and red for a loss, is the whole vocabulary.
#[test]
fn a_decided_match_paints_a_banner() {
    let mut won = Frame::new(960, 720);
    ui::hud(
        &mut won,
        &panel(
            &snapshot(),
            Vec::new(),
            Some(ruage_sim::MatchOutcome::Victory { winner: 1 }),
        ),
        &snapshot(),
        ruage_view::Speed::One,
    );
    let (x, y) = ui::banner_point(960, 720);
    assert_eq!(won.pixel(x, y), palette::WON, "a win paints the banner");

    let mut lost = Frame::new(960, 720);
    ui::hud(
        &mut lost,
        &panel(&snapshot(), Vec::new(), Some(ruage_sim::MatchOutcome::Draw)),
        &snapshot(),
        ruage_view::Speed::One,
    );
    assert_ne!(lost.pixel(x, y), palette::WON, "a draw is not a win");
    assert_eq!(
        lost.pixel(x, y),
        palette::LOST,
        "and it is not the ground either"
    );

    let mut playing = Frame::new(960, 720);
    ui::hud(
        &mut playing,
        &panel(&snapshot(), Vec::new(), None),
        &snapshot(),
        ruage_view::Speed::One,
    );
    assert_ne!(
        playing.pixel(x, y),
        palette::WON,
        "an undecided match shows no banner"
    );
}

/// **A number says whose number it is.** The pack names its resources, the swatches say which colour, and a colour is not a
/// name to somebody who has not played this pack. The name goes under the swatch at one pixel a dot.
#[test]
fn each_stock_column_shows_its_name() {
    let snapshot = snapshot();
    let mut frame = Frame::new(960, 720);
    let hud = ui::Hud {
        stock: vec![("FOOD".to_owned(), 7), ("WOOD".to_owned(), 42)],
        population: 5,
        population_cap: 15,
        selected: Vec::new(),
        outcome: None,
        notice: None,
    };
    ui::hud(&mut frame, &hud, &snapshot, ruage_view::Speed::One);
    let (x, y) = ui::stock_name_origin(0);
    assert!(
        count_colour(&frame, x as u32, y as u32, 40, 6, palette::HUD_TEXT) > 0,
        "no name under the first stock column"
    );

    let mut unnamed = Frame::new(960, 720);
    let bare = ui::Hud {
        stock: vec![(String::new(), 7)],
        population: 5,
        population_cap: 15,
        selected: Vec::new(),
        outcome: None,
        notice: None,
    };
    ui::hud(&mut unnamed, &bare, &snapshot, ruage_view::Speed::One);
    assert_eq!(
        count_colour(&unnamed, x as u32, y as u32, 40, 6, palette::HUD_TEXT),
        0,
        "an empty name paints nothing"
    );
}

/// **A refused order says so.** The engine keeps the refusal (`World::refusal`) and the window threw it away, so an order
/// that was not accepted looked exactly like one that was. The panel now carries the words.
#[test]
fn a_refusal_is_written_in_the_panel() {
    let snapshot = snapshot();
    let mut showing = Frame::new(960, 720);
    let hud = ui::Hud {
        notice: Some("NO PATH".to_owned()),
        ..panel(&snapshot, Vec::new(), None)
    };
    ui::hud(&mut showing, &hud, &snapshot, ruage_view::Speed::One);
    let (x, y) = ui::notice_origin(720);
    assert!(
        count_colour(&showing, x as u32, y as u32, 80, 6, palette::REFUSED) > 0,
        "the notice is not on the panel"
    );

    let mut quiet = Frame::new(960, 720);
    let hud = panel(&snapshot, Vec::new(), None);
    ui::hud(&mut quiet, &hud, &snapshot, ruage_view::Speed::One);
    assert_eq!(
        count_colour(&quiet, x as u32, y as u32, 80, 6, palette::REFUSED),
        0,
        "and nothing is written when nothing was refused"
    );
}

/// **Escape opens a menu, and the menu is a place to click.** Three rows - resume, restart, quit - each with words
/// (the font has letters now), each inside the frame, and each clickable: a menu you cannot click is a screenshot.
#[test]
fn the_menu_has_three_clickable_rows() {
    let (width, height) = (960, 720);
    let rows = ui::menu_rows(width, height);
    assert_eq!(rows.len(), 3, "resume, restart, quit");
    for (x, y, w, h) in rows {
        assert!(x >= 0 && y >= 0, "a row starts inside the frame");
        assert!(
            x as u32 + w <= width && y as u32 + h <= height,
            "a row ends inside the frame"
        );
        // The middle of each row picks that row, and only that row.
        let middle = (x + w as i32 / 2, y + h as i32 / 2);
        let picked = ui::menu_at(middle, width, height);
        assert_eq!(
            rows.iter().position(|row| *row == (x, y, w, h)),
            picked,
            "the middle of a row does not pick it"
        );
    }
    assert_eq!(
        ui::menu_at((2, 2), width, height),
        None,
        "the corner is not a menu row"
    );
}

/// **The menu is a modal.** It washes the world behind it, draws the rows, and (tested at the window level) stops the clock:
/// a menu that lets the match run under it is a menu a player cannot read.
#[test]
fn the_menu_washes_the_world_and_writes_its_rows() {
    let (width, height) = (960, 720);
    let mut frame = Frame::new(width, height);
    frame.fill_rect(0, 0, width as i32, height as i32, palette::GRASS);
    ui::draw_menu(&mut frame, width, height, Some(0));
    let rows = ui::menu_rows(width, height);
    let (x, y, w, h) = rows[0];
    assert!(
        count_colour(&frame, x as u32, y as u32, w, h, palette::MENU_ROW) > 0,
        "the chosen row is not painted"
    );
    let mut letters = 0;
    for row in rows {
        letters += count_colour(
            &frame,
            row.0 as u32,
            row.1 as u32,
            row.2,
            row.3,
            palette::HUD_TEXT,
        );
    }
    assert!(letters > 0, "the rows carry no words");
    // The wash covers the playfield: the green under the menu is darker than the green that went in.
    let washed = frame.pixel(width / 2, height - 40);
    assert!(
        washed[1] < palette::GRASS[1],
        "the menu did not dim the world: {washed:?} against {:?}",
        palette::GRASS
    );
}
