//! **The minimap fills its diamond, with no seams.** The owner, 2026-11-05: *"Minimap has 2 crossing lines artifact"* - thin
//! lines of the panel's own background running across the map, because the minimap was painted tile by tile as a diamond of
//! `step` pixels and the rounding of neighbouring tile centres left one-pixel gaps between them. Two such seams run along the
//! two diagonal directions of the tile grid, which is exactly the pair of crossing lines he saw.
//!
//! Written before the fix (D7, SPEC §11).

use ruage_view::{Frame, Snapshot, ui};

fn resources() -> ruage_data::Resources {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("config/resources.toml");
    ruage_data::load_resources_file(&path).expect("resources")
}

fn fully_explored(width: u16, height: u16) -> Snapshot {
    let tiles = usize::from(width) * usize::from(height);
    Snapshot {
        tick: 0,
        player: 1,
        scale: 20_000,
        width,
        height,
        // All forest: one colour, so a gap is the **panel's** colour and nothing else.
        terrain: vec![true; tiles],
        stock: vec![0; 4],
        population: 1,
        population_cap: 5,
        outcome: None,
        age: None,
        visible: vec![u64::MAX; tiles.div_ceil(64)],
        explored: vec![u64::MAX; tiles.div_ceil(64)],
        ring_scale: 1.5,
        entities: Vec::new(),
    }
}

/// A smaller box - an 860x660 frame gives a **260-pixel** minimap - where the step between neighbouring tile centres is still
/// under three pixels. Same rule, smaller box: this one fails on the old code with **182** gaps, exactly as the 960x720 case does.
#[test]
fn a_small_minimap_box_has_no_gaps_either() {
    let snapshot = fully_explored(64, 64);
    let mut frame = Frame::new(860, 660);
    let hud = ui::Hud::of(&snapshot, &resources(), &ui::Selection::new());
    ui::hud(&mut frame, &hud, &snapshot, ruage_view::Speed::One);
    let (x, y, w, h) = ui::minimap_rect(860, 660, snapshot.width, snapshot.height);
    let (cx, cy) = (x as f32 + w as f32 / 2.0, y as f32 + h as f32 / 2.0);
    let (half_w, half_h) = (w as f32 / 2.0, h as f32 / 4.0);
    let mut gaps = 0;
    for py in y + 1..y + h as i32 - 1 {
        for px in x + 1..x + w as i32 - 1 {
            let nx = (px as f32 - cx) / half_w;
            let ny = (py as f32 - cy) / half_h;
            if nx.abs() + ny.abs() > 0.92 {
                continue;
            }
            let pixel = frame.pixel(px as u32, py as u32);
            if pixel == ruage_view::palette::PANEL
                || pixel == ruage_view::palette::HUD_BAR
                || pixel == ruage_view::palette::MARK
            {
                gaps += 1;
            }
        }
    }
    assert_eq!(
        gaps, 0,
        "{gaps} panel-coloured pixels inside a {w}-pixel minimap: the tiles do not meet"
    );
}

/// **No seam inside the map.** Every pixel of the diamond must be a tile's colour; a pixel of the panel's background inside the
/// diamond is a gap between two tiles, which is the artifact the owner reported.
#[test]
fn the_minimap_has_no_gaps_between_its_tiles() {
    let snapshot = fully_explored(64, 64);
    let mut frame = Frame::new(960, 720);
    let hud = ui::Hud::of(&snapshot, &resources(), &ui::Selection::new());
    ui::hud(&mut frame, &hud, &snapshot, ruage_view::Speed::One);

    let (x, y, w, h) = ui::minimap_rect(960, 720, snapshot.width, snapshot.height);
    let (cx, cy) = (x as f32 + w as f32 / 2.0, y as f32 + h as f32 / 2.0);
    let (half_w, half_h) = (w as f32 / 2.0, h as f32 / 4.0);
    let mut gaps = 0;
    for py in y + 1..y + h as i32 - 1 {
        for px in x + 1..x + w as i32 - 1 {
            // Inside the diamond: |dx|/half_w + |dy|/half_h <= 1, with a pixel of margin so the edge itself is not counted.
            let nx = (px as f32 - cx) / half_w;
            let ny = (py as f32 - cy) / half_h;
            if nx.abs() + ny.abs() > 0.92 {
                continue;
            }
            let pixel = frame.pixel(px as u32, py as u32);
            // The panel's ground and the box's own lines are not the map's colours.
            if pixel == ruage_view::palette::PANEL
                || pixel == ruage_view::palette::HUD_BAR
                || pixel == ruage_view::palette::MARK
            {
                gaps += 1;
            }
        }
    }
    assert_eq!(
        gaps, 0,
        "{gaps} panel-coloured pixels inside the map diamond: the tiles do not meet"
    );
}
