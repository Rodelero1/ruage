//! The frame's layout: where the two bars are, and what anchors them.
//!
//! The first HUD was a pile of magic numbers (`8`, `60`, `76`, `96x28`) scattered through five functions, and it showed:
//! on a 1280x540 frame the readout sat in the corner, the card floated above the panel with a gap beside it, and the whole
//! right half of the bottom bar was empty. OpenAoK carries a fixed 4:3 table out to the edges for the same reason (its art
//! is a fixed bitmap); with no art to match, **anchors** say it directly: the stockpile from the left edge, the bar's
//! buttons from the right, the command grid from the panel's left, the minimap from its right, and the middle takes what
//! is left over.
//!
//! Written before the implementation (D7, SPEC §11).

use ruage_view::ui::{self, Layout};
use ruage_view::{Frame, palette};

/// A rectangle as `(x, y, w, h)`, and whether two of them overlap at all.
fn overlaps(a: (i32, i32, u32, u32), b: (i32, i32, u32, u32)) -> bool {
    let (ax, ay, aw, ah) = a;
    let (bx, by, bw, bh) = b;
    ax < bx + bw as i32 && bx < ax + aw as i32 && ay < by + bh as i32 && by < ay + ah as i32
}

fn inside(rect: (i32, i32, u32, u32), frame: (u32, u32)) -> bool {
    rect.0 >= 0
        && rect.1 >= 0
        && rect.0 as u32 + rect.2 <= frame.0
        && rect.1 as u32 + rect.3 <= frame.1
}

/// **The bar, the panel and the view between them tile the frame**, with no gap and no overlap: the view is everything
/// that is not a bar, which is what "the map fills the window" means.
#[test]
fn the_bars_and_the_view_tile_the_frame() {
    for frame in [(960, 720), (1280, 540), (1920, 1080), (800, 600)] {
        let layout = Layout::of(frame.0, frame.1);
        assert_eq!(layout.bar.0, 0);
        assert_eq!(layout.bar.1, 0);
        assert_eq!(layout.bar.2, frame.0, "the top bar spans the frame");
        assert_eq!(layout.panel.0, 0);
        assert_eq!(layout.panel.2, frame.0, "the panel spans the frame");
        assert_eq!(
            layout.panel.1 as u32 + layout.panel.3,
            frame.1,
            "the panel ends at the frame's bottom"
        );
        assert_eq!(
            layout.view.1,
            layout.bar.1 + layout.bar.3 as i32,
            "the view starts under the bar"
        );
        assert_eq!(
            layout.view.3 as i32,
            layout.panel.1 - layout.view.1,
            "and ends at the panel's top"
        );
        assert_eq!(layout.view.2, frame.0);
    }
}

/// **The right-hand things are measured from the right edge.** The bar's five buttons and the minimap stay eight pixels
/// from it whatever the frame's width, and the left-hand things stay eight from the left: that is the whole of
/// "it works on a wide screen".
#[test]
fn edges_are_anchored_not_tabled() {
    for width in [800u32, 960, 1280, 1920, 2560] {
        let layout = Layout::of(width, 720);
        let last = layout.bar_buttons[4];
        assert_eq!(
            last.0 as u32 + last.2 + 8,
            width,
            "the last bar button is eight pixels from the right edge at {width}"
        );
        assert_eq!(
            layout.minimap.0 as u32 + layout.minimap.2 + 8,
            width,
            "the minimap is eight pixels from the right edge at {width}"
        );
        assert_eq!(
            layout.stock[0].0, 8,
            "the stockpile starts at the left edge"
        );
        assert_eq!(layout.grid.0, 8, "the command grid starts at the left edge");
    }
}

/// **The bar's buttons march inwards from the right**, in order, without overlapping each other or the minimap's
/// column, and the stockpile keeps its own corner.
#[test]
fn the_bar_has_a_right_hand_cluster() {
    let layout = Layout::of(1280, 720);
    for pair in layout.bar_buttons.windows(2) {
        assert!(!overlaps(pair[0], pair[1]), "two bar buttons overlap");
        assert!(pair[1].0 > pair[0].0, "the buttons are not in order");
    }
    let bar = layout.bar;
    for button in layout.bar_buttons {
        assert!(inside(button, (1280, 720)), "a bar button leaves the frame");
        assert!(
            button.1 >= bar.1 && button.1 + button.3 as i32 <= bar.1 + bar.3 as i32,
            "a bar button is not inside the bar"
        );
    }
    for stock in &layout.stock {
        assert!(inside(*stock, (1280, 720)));
        assert!(
            stock.0 + stock.2 as i32 <= layout.bar_buttons[0].0,
            "the stockpile and the buttons collide"
        );
    }
}

/// **The panel is three zones**: the command grid on the left, the selected object in the middle, the minimap on the
/// right - the game's own arrangement, and the owner's answer. Each zone inside the panel, none overlapping, and the
/// middle is what is left over.
#[test]
fn the_panel_is_grid_selected_and_minimap() {
    for width in [960u32, 1280, 1920] {
        let layout = Layout::of(width, 720);
        assert!(inside(layout.grid, (width, 720)));
        assert!(inside(layout.selected, (width, 720)));
        assert!(inside(layout.minimap, (width, 720)));
        assert!(
            !overlaps(layout.grid, layout.selected),
            "grid and selected overlap at {width}"
        );
        assert!(
            !overlaps(layout.selected, layout.minimap),
            "selected and minimap overlap at {width}"
        );
        assert!(
            !overlaps(layout.grid, layout.minimap),
            "grid and minimap overlap at {width}"
        );
        assert!(
            layout.grid.0 < layout.selected.0 && layout.selected.0 < layout.minimap.0,
            "the zones are not in the game's order at {width}"
        );
    }
}

/// **The grid is fifteen slots, five across and three down**, all inside the panel: a command grid whose last row is
/// under the panel's edge is a command grid with five missing buttons.
#[test]
fn the_grid_is_fifteen_slots() {
    let layout = Layout::of(1280, 720);
    assert_eq!(layout.slots.len(), 15);
    let grid = layout.grid;
    for slot in &layout.slots {
        assert!(inside(*slot, (1280, 720)), "a slot leaves the frame");
        assert!(
            slot.0 >= grid.0
                && slot.1 >= grid.1
                && slot.0 + slot.2 as i32 <= grid.0 + grid.2 as i32
                && slot.1 + slot.3 as i32 <= grid.1 + grid.3 as i32,
            "slot {slot:?} is outside the grid {grid:?}"
        );
    }
    // Five across: the sixth slot starts a new row, under the first.
    assert_eq!(
        layout.slots[5].0, layout.slots[0].0,
        "the sixth slot is not under the first"
    );
    assert!(
        layout.slots[5].1 > layout.slots[0].1,
        "the second row is not below the first"
    );
    assert_eq!(layout.slots[4].1, layout.slots[0].1, "five to a row");
}

/// **A frame too small for the interface keeps the interface inside it** rather than drawing off the edge: the layout
/// clamps instead of trusting the frame.
#[test]
fn a_tiny_frame_is_clamped() {
    let layout = Layout::of(320, 200);
    assert!(inside(layout.bar, (320, 200)));
    assert!(inside(layout.panel, (320, 200)));
    assert!(layout.view.3 > 0, "the view has no room at all");
    for slot in &layout.slots {
        assert!(
            slot.0 >= 0 && slot.1 >= 0,
            "a slot starts outside a tiny frame"
        );
    }
}

/// **A widget has states and they look different.** A button that looks the same under the pointer as away from it is a
/// button nobody knows is a button; a pressed one that looks normal is worse. Measured in pixels, in the state colours.
#[test]
fn a_widget_shows_its_state() {
    let rect = (10, 10, 60, 24);
    let mut normal = Frame::new(120, 60);
    let mut hover = Frame::new(120, 60);
    let mut pressed = Frame::new(120, 60);
    let mut disabled = Frame::new(120, 60);
    ui::draw_widget(&mut normal, rect, ui::WidgetState::Normal);
    ui::draw_widget(&mut hover, rect, ui::WidgetState::Hover);
    ui::draw_widget(&mut pressed, rect, ui::WidgetState::Pressed);
    ui::draw_widget(&mut disabled, rect, ui::WidgetState::Disabled);
    let mid = (rect.0 as u32 + 2, rect.1 as u32 + 2);
    assert_ne!(
        normal.pixel(mid.0, mid.1),
        hover.pixel(mid.0, mid.1),
        "hover looks like normal"
    );
    assert_ne!(
        normal.pixel(mid.0, mid.1),
        pressed.pixel(mid.0, mid.1),
        "pressed looks like normal"
    );
    assert_ne!(
        normal.pixel(mid.0, mid.1),
        disabled.pixel(mid.0, mid.1),
        "disabled looks like normal"
    );
    // And the bevel: the top edge is lighter than the bottom edge, which is what makes a placeholder read as a button.
    let top = normal.pixel(rect.0 as u32 + 30, rect.1 as u32);
    let bottom = normal.pixel(rect.0 as u32 + 30, rect.1 as u32 + rect.3 - 1);
    assert_eq!(top, palette::BEVEL_LIGHT, "no lit top edge");
    assert_eq!(bottom, palette::BEVEL_DARK, "no shadowed bottom edge");
}

/// **A click on the interface never reaches the world.** OpenAoK decides this from the interface art's own pixels; we draw
/// procedurally, so the layout *is* the mask: the bars and the help box are the interface, and everything between them is
/// the map. One function answers it, instead of the window remembering to ask the speed box, then the card, then the
/// minimap, in the right order.
#[test]
fn the_interface_owns_its_pixels() {
    for frame in [(960u32, 720u32), (1280, 540), (800, 600)] {
        let layout = Layout::of(frame.0, frame.1);
        // Inside the bars: the interface.
        assert!(ui::over_hud((4, 4), &layout), "the top bar is not the interface");
        assert!(
            ui::over_hud((4, frame.1 as i32 - 4), &layout),
            "the panel is not the interface"
        );
        // Inside the view: the world.
        let middle = (frame.0 as i32 / 2, (layout.view.1 + layout.view.3 as i32) / 2);
        assert!(!ui::over_hud(middle, &layout), "the map is not the world");
        // Just under the bar and just above the panel are still the world (the one-pixel edges belong to the bars).
        assert!(
            !ui::over_hud((middle.0, layout.bar.1 + layout.bar.3 as i32 + 2), &layout),
            "the pixel under the bar is not the world"
        );
        assert!(
            !ui::over_hud((middle.0, layout.panel.1 - 2), &layout),
            "the pixel above the panel is not the world"
        );
    }
}

/// **A pixel outside the frame is not the interface**, and neither is one on the menu when the menu is closed: a mask that
/// says "yes" everywhere is a game that cannot be clicked.
#[test]
fn outside_the_frame_is_the_world() {
    let layout = Layout::of(960, 720);
    assert!(!ui::over_hud((-1, 4), &layout));
    assert!(!ui::over_hud((4, -1), &layout));
    assert!(!ui::over_hud((960, 4), &layout));
    assert!(!ui::over_hud((4, 720), &layout));
}
