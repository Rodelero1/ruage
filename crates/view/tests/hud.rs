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
        age: None,
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

    // A swatch per stock column, in order, each with digits beside it - at the x the HUD itself computed, so this test
    // cannot drift from the layout the player sees.
    let layout = panel(&snapshot, Vec::new(), None);
    let (columns, population_x) = ui::stock_columns(&layout);
    for (column, colour) in palette::STOCK.iter().take(snapshot.stock.len()).enumerate() {
        let x = u32::try_from(columns[column]).expect("a column on screen");
        let (_, _, swatch_y) = ui::stock_rows(720);
        let y = u32::try_from(swatch_y + 2).expect("a swatch on screen");
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

    // Population and the game itself are both still there, and the population is **past the last column**: it used to be
    // drawn at the last column's own x, which is how stone and population ended up on top of each other (2026-11-04).
    assert!(
        lit(
            &with_hud,
            u32::try_from(population_x).expect("on screen"),
            2,
            60,
            20
        ) > 0,
        "no population reported"
    );
    // The bottom panel and the minimap **are** painted over the game - that is what a HUD is - so this walks the band
    // between them: below the top bar, above the panel, and left of the minimap.
    // The band between the bars, exactly: `Layout` says where the panel starts, and a frame too short for the full
    // panel gets a shorter one, so the band is read from the layout rather than from the panel's nominal height.
    let layout = ui::Layout::of(320, 240);
    let below = layout.bar.1 as u32 + layout.bar.3 + 4;
    let above_panel = (layout.panel.1 - 4).max(below as i32) as u32;
    let (map_left, map_top, ..) = ui::minimap_rect(320, 240, snapshot.width, snapshot.height);
    for y in below..above_panel {
        for x in 0..320 {
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
    let (columns, _) = ui::stock_columns(&panel(&snapshot, Vec::new(), None));
    let (_, _, swatch_y) = ui::stock_rows(720);
    for (column, colour) in palette::STOCK.iter().enumerate() {
        let x =
            u32::try_from(columns.get(column).copied().unwrap_or(8)).expect("a column on screen");
        assert_ne!(
            frame.pixel(
                x + 2,
                u32::try_from(swatch_y + 2).expect("a swatch on screen")
            ),
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
        age: None,
        help: None,
        bar_hover: None,
        notice: None,
        view: None,
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
        unit: Some(83),
        name: "VILLAGER".to_owned(),
        kind: ruage_sim::EntityKind::Unit,
        hp_fraction,
        hp: 25,
        amount: 0,
        resource: None,
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
        // The panel's own ground: the zones are drawn as outlines, not fills (a fill of all three was 130,000 pixels a
        // frame, measured at 316-355 us against 151-195).
        palette::PANEL,
        "and with nothing selected the row is the panel's own background"
    );
}

/// **The minimap is drawn where `minimap_rect` says, and it is a diamond.** The owner asked for the interface to be turned
/// like the map (2026-11-04), so the middle of the box is map and its four corners are not: the panel shows through where the
/// diamond does not reach.
#[test]
fn the_minimap_is_a_diamond_in_the_corner() {
    let snapshot = snapshot();
    let mut frame = Frame::new(960, 720);
    ui::hud(
        &mut frame,
        &panel(&snapshot, Vec::new(), None),
        &snapshot,
        ruage_view::Speed::One,
    );
    let (x, y, w, h) = ui::minimap_rect(960, 720, snapshot.width, snapshot.height);
    // The middle of the box is a tile of the map: its colour is not the panel's.
    let middle = frame.pixel(x as u32 + w / 2, y as u32 + h / 2);
    assert_ne!(
        middle,
        palette::PANEL,
        "the middle of the minimap is bare panel, so no map was drawn"
    );
    // The top-left pixel of the box is outside the diamond, so it is not a tile's colour either - and the map's own top tile
    // is exactly at the top of the box.
    let (top_x, top_y) = ui::minimap_point((0, 0), (x, y, w, h), snapshot.width, snapshot.height);
    let top = frame.pixel(top_x as u32, top_y as u32);
    assert_ne!(top, palette::PANEL, "the map's first tile was not drawn");
    for (px, py) in [
        (x as u32 + 1, y as u32 + 1),
        (x as u32 + w - 2, y as u32 + 1),
        (x as u32 + 1, y as u32 + h - 2),
        (x as u32 + w - 2, y as u32 + h - 2),
    ] {
        assert_eq!(
            frame.pixel(px, py),
            palette::PANEL,
            "the box's corner at ({px}, {py}) is painted, but it is outside the diamond"
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
    // **A banner box, not a stripe**: the owner saw the old full-width band as "a strange green stripe" (2026-11-04), and a
    // box with a word in it is the fix. The centre is the word's ink, so the colour is counted around it.
    let (x, y) = ui::banner_point(960, 720);
    let painted = count_colour(&won, x - 60, y - 14, 120, 28, palette::WON);
    assert!(painted > 400, "a win paints the banner: {painted} pixels");

    let mut lost = Frame::new(960, 720);
    ui::hud(
        &mut lost,
        &panel(&snapshot(), Vec::new(), Some(ruage_sim::MatchOutcome::Draw)),
        &snapshot(),
        ruage_view::Speed::One,
    );
    assert_eq!(
        count_colour(&lost, x - 60, y - 14, 120, 28, palette::WON),
        0,
        "a draw is not a win"
    );
    let painted = count_colour(&lost, x - 60, y - 14, 120, 28, palette::LOST);
    assert!(
        painted > 400,
        "and a draw paints the banner: {painted} pixels"
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
        age: None,
        help: None,
        bar_hover: None,
        notice: None,
        view: None,
    };
    ui::hud(&mut frame, &hud, &snapshot, ruage_view::Speed::One);
    let (columns, _) = ui::stock_columns(&hud);
    let x = u32::try_from(columns[0]).expect("on screen");
    assert!(
        count_colour(
            &frame,
            x,
            2,
            40,
            ruage_view::font::HEIGHT as u32,
            palette::HUD_TEXT
        ) > 0,
        "no name over the first stock column"
    );

    let mut unnamed = Frame::new(960, 720);
    let bare = ui::Hud {
        stock: vec![(String::new(), 7)],
        population: 5,
        population_cap: 15,
        selected: Vec::new(),
        outcome: None,
        age: None,
        help: None,
        bar_hover: None,
        notice: None,
        view: None,
    };
    ui::hud(&mut unnamed, &bare, &snapshot, ruage_view::Speed::One);
    assert_eq!(
        count_colour(
            &unnamed,
            x,
            2,
            40,
            ruage_view::font::HEIGHT as u32,
            palette::HUD_TEXT
        ),
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

/// **The selected zone says what is selected, and how hurt it is.** A player who clicks a building wants its name and its
/// health; the zone has had the room since U1 and showed a list of unlabelled bars.
#[test]
fn the_selected_zone_names_and_measures() {
    let snapshot = snapshot();
    let mut frame = Frame::new(960, 720);
    let hud = panel(&snapshot, vec![row(0.4)], None);
    ui::hud(&mut frame, &hud, &snapshot, ruage_view::Speed::One);
    let zone = ui::Layout::of(960, 720).selected;

    assert!(
        count_colour(
            &frame,
            (zone.0 + 8) as u32,
            (zone.1 + 10) as u32,
            120,
            12,
            palette::HUD_TEXT
        ) > 0,
        "the zone does not name what is selected"
    );
    // The wide health bar: 40% of the zone's width, in the red a hurt thing gets.
    let bar_w = zone.2 as i32 - 16;
    assert_eq!(
        frame.pixel((zone.0 + 8 + bar_w / 4) as u32, (zone.1 + 28) as u32),
        palette::HP_RED,
        "the zone's health bar is not red at 40%"
    );

    let mut bare = Frame::new(960, 720);
    ui::hud(
        &mut bare,
        &panel(&snapshot, Vec::new(), None),
        &snapshot,
        ruage_view::Speed::One,
    );
    assert_eq!(
        count_colour(
            &bare,
            (zone.0 + 8) as u32,
            (zone.1 + 10) as u32,
            120,
            12,
            palette::HUD_TEXT
        ),
        0,
        "nothing is selected and the zone still names something"
    );
}

/// A node's row, as the HUD builds it: no role name (the pack has no word for a tree), the column's name instead, its hit
/// points as a number because a node has no maximum on record, and what is left in it.
fn node_row() -> ui::SelectedRow {
    ui::SelectedRow {
        id: 11,
        unit: None,
        name: "wood".to_owned(),
        kind: ruage_sim::EntityKind::Resource(1),
        hp_fraction: 1.0,
        hp: 20,
        amount: 87,
        resource: Some("wood".to_owned()),
        carry: 0,
        construction: None,
        garrison: 0,
        pending: 0,
        pose: ruage_view::Pose::Idle,
    }
}

/// **The panel says what a node holds.** The owner asked for a tree's hit points and wood, a farm's food, and the same for
/// berries, stone and gold - in the middle of the bottom bar, which is the selected zone.
///
/// A node has no maximum health on record (its `hp` *is* its health), so it cannot have a fraction bar: it gets the number.
#[test]
fn the_panel_says_what_a_node_holds() {
    let snapshot = snapshot();
    let mut frame = Frame::new(960, 720);
    ui::hud(
        &mut frame,
        &panel(&snapshot, vec![node_row()], None),
        &snapshot,
        ruage_view::Speed::One,
    );
    let zone = ui::Layout::of(960, 720).selected;
    // Where the health bar would be: this row has no bar, it has "HP 20".
    assert!(
        count_colour(
            &frame,
            (zone.0 + 8) as u32,
            (zone.1 + 25) as u32,
            120,
            9,
            palette::HUD_TEXT
        ) > 0,
        "the panel does not write a node's hit points"
    );
    // The line under it: the column's name and what is left.
    assert!(
        count_colour(
            &frame,
            (zone.0 + 8) as u32,
            (zone.1 + 34) as u32,
            120,
            9,
            palette::HUD_TEXT
        ) > 0,
        "the panel does not write what the node holds"
    );

    // An empty band proves the two above are about the node and not about the panel's own furniture.
    let mut bare = Frame::new(960, 720);
    ui::hud(
        &mut bare,
        &panel(&snapshot, Vec::new(), None),
        &snapshot,
        ruage_view::Speed::One,
    );
    assert_eq!(
        count_colour(
            &bare,
            (zone.0 + 8) as u32,
            (zone.1 + 34) as u32,
            120,
            9,
            palette::HUD_TEXT
        ),
        0,
        "with nothing selected the holdings line is still written"
    );
}

/// **The HUD names a node from the pack and carries its numbers.** `Hud::of_selected` is what the window calls; a node has
/// no unit id, so the name has to come from its resource column - and a farm is a building, whose column is content's
/// (`[farm] resource`), never the view's guess.
#[test]
fn the_hud_reads_a_nodes_column_and_holdings() {
    let resources = ruage_data::load_resources_file(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/resources.toml"),
    )
    .expect("resources.toml");
    let wood = resources.slot_of("wood").expect("a wood column");
    let snapshot = snapshot();
    let mut node = node_row();
    node.kind = ruage_sim::EntityKind::Resource(wood);
    node.resource = None;
    node.name = String::new();
    let mut world = snapshot.clone();
    world.entities.push(ruage_view::EntityView {
        id: 11,
        player: 0,
        kind: ruage_sim::EntityKind::Resource(wood),
        unit: None,
        position: (10.0, 10.0),
        hp_fraction: 1.0,
        pose: ruage_view::Pose::Idle,
        carry: 0,
        construction: None,
        garrison: 0,
        amount: 87,
        pending: 0,
        remembered: false,
        footprint: (1, 1),
        shape: ruage_view::Silhouette::Dot,
        mark: ruage_view::Mark::None,
        hp: 20,
        resource: Some(wood),
    });
    let hud = ui::Hud::of_selected(&world, &resources, &[11]);
    let row = hud.selected.first().expect("the node is selected");
    assert_eq!(row.name, "wood", "the panel has no word for the node");
    assert_eq!(row.amount, 87, "the panel lost the amount");
    assert_eq!(row.hp, 20, "the panel lost the hit points");
    assert_eq!(
        row.resource.as_deref(),
        Some("wood"),
        "the row does not say whose number 87 is"
    );
    let _ = &node;
}

/// **A farm shows both**: its health as a bar (a farm has a maximum, so a fraction means something) **and** the food left
/// in it. This is the row the owner asked about by name ("same for farm, can I see amount of food it has left?"), and it
/// is the one case with a bar and a holdings line in the same panel.
#[test]
fn a_farm_shows_its_health_and_the_food_left() {
    let snapshot = snapshot();
    let mut frame = Frame::new(960, 720);
    let mut farm = row(1.0);
    farm.unit = Some(259);
    farm.name = "FARM".to_owned();
    farm.kind = ruage_sim::EntityKind::Building;
    farm.amount = 213;
    farm.resource = Some("food".to_owned());
    ui::hud(
        &mut frame,
        &panel(&snapshot, vec![farm], None),
        &snapshot,
        ruage_view::Speed::One,
    );
    let zone = ui::Layout::of(960, 720).selected;
    assert_eq!(
        frame.pixel((zone.0 + 8 + 4) as u32, (zone.1 + 28) as u32),
        palette::HP_GREEN,
        "a farm at full health has no green bar"
    );
    assert!(
        count_colour(
            &frame,
            (zone.0 + 8) as u32,
            (zone.1 + 34) as u32,
            160,
            9,
            palette::HUD_TEXT
        ) > 0,
        "the panel does not say how much food the farm has left"
    );
}
