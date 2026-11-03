//! Selection, the HUD and the minimap: everything a player needs to *act* rather than watch.
//!
//! Written before the implementation (D7). All three are pure functions of a [`Snapshot`](crate::Snapshot):
//! they read what the player can see, and selection turns a click into `Command`s the engine already has.
//! None of them touches `World` - the UI issues orders, it does not execute them (D57).

use ruage_data::Resources;
use ruage_sim::{Command, EntityKind};

use crate::{Camera, Frame, Snapshot, covers, palette, player_colour};

/// What a player has selected, by entity id, in the order they selected it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    pub ids: Vec<u32>,
}

impl Selection {
    pub fn new() -> Selection {
        Selection::default()
    }

    pub fn clear(&mut self) {
        self.ids.clear();
    }

    pub fn entity<'a>(&self, snapshot: &'a Snapshot, id: u32) -> Option<&'a crate::EntityView> {
        snapshot.entities.iter().find(|entity| entity.id == id)
    }

    /// A click selects what is under the cursor: units before buildings, the nearest first, and only what
    /// the player is allowed to own - clicking the ground clears the selection.
    pub fn click(&mut self, snapshot: &Snapshot, camera: &Camera, pixel: (i32, i32)) {
        let tile = camera.pick(pixel);
        let own: Vec<&crate::EntityView> = snapshot
            .entities
            .iter()
            .filter(|entity| entity.player == snapshot.player)
            .filter(|entity| matches!(entity.kind, EntityKind::Unit | EntityKind::Building))
            // Anywhere on the footprint, the same rule the window's picking uses: clicking the body of a town centre
            // must select it, not only its anchor tile.
            .filter(|entity| covers(entity, tile))
            .collect();
        // A unit standing on a building is what the player meant: units first.
        let chosen = own
            .iter()
            .find(|entity| entity.kind == EntityKind::Unit)
            .or_else(|| own.first());
        self.ids.clear();
        if let Some(entity) = chosen {
            self.ids.push(entity.id);
        }
    }

    /// A rubber band selects every unit of the player's inside it. Buildings are not swept up by a box: the
    /// game does not do it, and a box that picked up a town centre would make every drag a disaster.
    pub fn band(&mut self, snapshot: &Snapshot, camera: &Camera, from: (i32, i32), to: (i32, i32)) {
        let (x0, x1) = (from.0.min(to.0), from.0.max(to.0));
        let (y0, y1) = (from.1.min(to.1), from.1.max(to.1));
        self.ids.clear();
        for entity in &snapshot.entities {
            if entity.player != snapshot.player || entity.kind != EntityKind::Unit {
                continue;
            }
            let (px, py) = camera.project(entity.position);
            if px >= x0 && px <= x1 && py >= y0 && py <= y1 {
                self.ids.push(entity.id);
            }
        }
    }

    /// A double click selects every unit of the same kind on screen, which is the one gesture that makes a
    /// squad out of a crowd.
    pub fn double_click(&mut self, snapshot: &Snapshot, camera: &Camera, pixel: (i32, i32)) {
        self.click(snapshot, camera, pixel);
        // **On the screen, not on the map.** The first version took every entity of that kind that the snapshot held, and a
        // snapshot holds the whole world: a double-click picked up villagers thirty tiles away that the player cannot see.
        let (wide, tall) = camera.viewport;
        let Some(first) = self.ids.first().copied() else {
            return;
        };
        let Some(wanted) = self.entity(snapshot, first).and_then(|entity| entity.unit) else {
            return;
        };
        self.ids = snapshot
            .entities
            .iter()
            .filter(|entity| {
                entity.player == snapshot.player
                    && entity.kind == EntityKind::Unit
                    && entity.unit == Some(wanted)
            })
            .filter(|entity| {
                let (x, y) = camera.project(entity.position);
                x >= 0 && y >= 0 && (x as u32) < wide && (y as u32) < tall
            })
            .map(|entity| entity.id)
            .collect();
    }

    /// What a right click on `pixel` should order, as commands for the current selection.
    ///
    /// The rule is the game's: a resource takes gatherers, an enemy takes attackers, anything else is a place
    /// to walk to. A building in the selection is asked to train or rally, which the engine already models.
    pub fn order(&self, snapshot: &Snapshot, camera: &Camera, pixel: (i32, i32)) -> Vec<Command> {
        let tile = camera.pick(pixel);
        if tile.0 < 0 || tile.1 < 0 {
            return Vec::new();
        }
        let (tile_x, tile_y) = (tile.0 as u16, tile.1 as u16);
        let under = snapshot.entities.iter().find(|entity| covers(entity, tile));

        let mut orders = Vec::new();
        for id in &self.ids {
            let Some(entity) = self.entity(snapshot, *id) else {
                continue;
            };
            let unit = ruage_sim::EntityId(*id);
            match (entity.kind, under) {
                (EntityKind::Unit, Some(target))
                    if matches!(target.kind, EntityKind::Resource(_)) =>
                {
                    orders.push(Command::Gather {
                        villager: unit,
                        node: ruage_sim::EntityId(target.id),
                    });
                }
                (EntityKind::Unit, Some(target))
                    if target.player != snapshot.player
                        && matches!(target.kind, EntityKind::Unit | EntityKind::Building) =>
                {
                    orders.push(Command::Attack {
                        attacker: unit,
                        target: ruage_sim::EntityId(target.id),
                    });
                }
                (EntityKind::Unit, _) => orders.push(Command::Move {
                    unit,
                    to: (tile_x, tile_y),
                }),
                (EntityKind::Building, _) => orders.push(Command::Rally {
                    building: unit,
                    point: Some((tile_x, tile_y)),
                }),
                _ => {}
            }
        }
        orders
    }
}

/// One row of the HUD's selected-object panel.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectedRow {
    pub id: u32,
    pub kind: EntityKind,
    /// Zero when the entity has no health bar worth drawing (a resource node).
    pub hp_fraction: f32,
    pub carry: u32,
    pub construction: Option<f32>,
    pub garrison: u32,
    /// Units this building is making, the one in progress included.
    pub pending: usize,
    pub pose: crate::Pose,
}

/// The numbers a player watches: the stockpile, the population, and what is selected.
#[derive(Debug, Clone, PartialEq)]
pub struct Hud {
    /// `(resource name, amount)`, in the pack's column order - the pack names them, so a mod with five
    /// resources shows five rows and nothing in this file learns what food is.
    pub stock: Vec<(String, u32)>,
    pub population: u32,
    pub population_cap: u32,
    pub selected: Vec<SelectedRow>,
    pub outcome: Option<ruage_sim::MatchOutcome>,
    /// **Why the last order was refused**, in the engine's own words, or `None` when the last order was taken. The
    /// window sets it from `submit_checked`'s error; a HUD that stays silent about a refusal makes an ignored order look
    /// like a bug in the game rather than a bug in the order.
    pub notice: Option<String>,
}

impl Hud {
    pub fn of(snapshot: &Snapshot, resources: &Resources, selection: &Selection) -> Hud {
        Hud::of_selected(snapshot, resources, &selection.ids)
    }

    /// The same HUD for a caller that holds a plain list of ids rather than a `Selection` - the window keeps its own
    /// vector today, and a second way to ask "what is selected" would eventually disagree with the first.
    pub fn of_selected(snapshot: &Snapshot, resources: &Resources, ids: &[u32]) -> Hud {
        let stock = (0..snapshot.stock.len())
            .map(|slot| {
                let name = resources.name(slot).unwrap_or("?").to_owned();
                (name, snapshot.stock[slot])
            })
            .collect();
        let selected = ids
            .iter()
            .filter_map(|id| snapshot.entities.iter().find(|entity| entity.id == *id))
            .map(|entity| SelectedRow {
                id: entity.id,
                kind: entity.kind,
                hp_fraction: entity.hp_fraction,
                carry: entity.carry,
                construction: entity.construction,
                garrison: entity.garrison,
                pending: entity.pending,
                pose: entity.pose,
            })
            .collect();
        Hud {
            stock,
            population: snapshot.population,
            population_cap: snapshot.population_cap,
            selected,
            outcome: snapshot.outcome,
            notice: None,
        }
    }
}

/// The minimap: the whole map at one or two pixels a tile, coloured by fog and terrain, with the player's
/// own units on top. It is the vision layer's second customer, and the reason `visible_mask` exists.
pub fn minimap(snapshot: &Snapshot, pixels_per_tile: u32) -> Frame {
    let scale = pixels_per_tile.max(1);
    let mut frame = Frame::new(
        u32::from(snapshot.width) * scale,
        u32::from(snapshot.height) * scale,
    );
    for y in 0..snapshot.height {
        for x in 0..snapshot.width {
            let forest = snapshot
                .terrain
                .get(usize::from(y) * usize::from(snapshot.width) + usize::from(x))
                .copied()
                .unwrap_or(false);
            let colour = match snapshot.tile_vision(x, y) {
                ruage_sim::TileVision::Unseen => [8, 8, 12, 255],
                ruage_sim::TileVision::Explored => [40, 48, 40, 255],
                ruage_sim::TileVision::Visible if forest => [24, 48, 24, 255],
                ruage_sim::TileVision::Visible => [88, 120, 56, 255],
            };
            frame.fill_rect(
                i32::from(x) * scale as i32,
                i32::from(y) * scale as i32,
                scale as i32,
                scale as i32,
                colour,
            );
        }
    }
    for entity in &snapshot.entities {
        let (x, y) = (
            entity.position.0.floor().max(0.0) as u32,
            entity.position.1.floor().max(0.0) as u32,
        );
        let colour = match entity.kind {
            EntityKind::Resource(_) => [180, 150, 90, 255],
            EntityKind::Unit => player_colour(entity.player),
            EntityKind::Building => [220, 220, 220, 255],
            EntityKind::Projectile => continue,
        };
        frame.fill_rect(
            x as i32 * scale as i32,
            y as i32 * scale as i32,
            scale.max(1) as i32,
            scale.max(1) as i32,
            colour,
        );
    }
    frame
}

// --- the HUD ---------------------------------------------------------------------------------

/// The height of the HUD bar: the game is drawn below it, never under it.
pub fn hud_bar_height() -> u32 {
    30
}

/// Where a stock column's **name** starts: under its swatch, one pixel a dot, because a colour is not a name to somebody
/// who has not played this pack. The bar is tall enough for the numbers (three pixels a dot) above it.
#[must_use]
pub fn stock_name_origin(column: usize) -> (i32, i32) {
    (hud_swatch_x(column) as i32, hud_bar_height() as i32 - 7)
}

/// Where a refusal is written: the top line of the bottom panel, where the eye goes after an order.
#[must_use]
pub fn notice_origin(frame_height: u32) -> (i32, i32) {
    (8, frame_height as i32 - hud_panel_height() as i32 + 3)
}

/// Where a stock column's swatch starts. **The pack's order is the display order**, so a pack with three resources
/// shows three swatches and the view never learns their names.
pub fn hud_swatch_x(column: usize) -> u32 {
    8 + column as u32 * 60
}

/// The speed selector's five boxes, at the right end of the HUD bar: `(x, y, w, h)` each, in `Speed::ALL` order.
///
/// A **segmented control**, not a dropdown: the HUD's font has digits and no words (D82), so a label like "1.5x" cannot be
/// written - five boxes with the live one filled say the same thing and are clickable.
#[must_use]
pub fn speed_boxes(frame_width: u32) -> [(u32, u32, u32, u32); 5] {
    let width = 22;
    let gap = 4;
    let total = 5 * width + 4 * gap;
    let left = frame_width.saturating_sub(total + 8);
    let y = 3;
    let height = hud_bar_height().saturating_sub(6);
    std::array::from_fn(|index| {
        let x = left + index as u32 * (width + gap);
        (x, y, width, height)
    })
}

/// Which speed a click at a pixel picks, if any: the selector is the only part of the HUD that takes input.
#[must_use]
pub fn speed_at(pixel: (i32, i32), frame_width: u32) -> Option<crate::Speed> {
    if pixel.0 < 0 || pixel.1 < 0 {
        return None;
    }
    let (x, y) = (pixel.0 as u32, pixel.1 as u32);
    speed_boxes(frame_width)
        .iter()
        .position(|(bx, by, w, h)| x >= *bx && x < bx + w && y >= *by && y < by + h)
        .map(|index| crate::Speed::ALL[index])
}

/// Draw the selector: one box per speed, the current one filled with the selection colour.
pub fn speed_selector(frame: &mut Frame, speed: crate::Speed) {
    for (index, (x, y, w, h)) in speed_boxes(frame.width).into_iter().enumerate() {
        let chosen = crate::Speed::ALL[index] == speed;
        frame.fill_rect(x as i32, y as i32, w as i32, h as i32, palette::HUD_TEXT);
        let inset = if chosen { 2 } else { 4 };
        frame.fill_rect(
            (x + inset) as i32,
            (y + inset) as i32,
            (w - inset * 2) as i32,
            (h - inset * 2) as i32,
            if chosen {
                palette::SELECTION
            } else {
                palette::HUD_BAR
            },
        );
    }
}

/// Draw the HUD over a frame: one swatch and one number per stockpile column, then population as `have/cap`.
///
/// It reads a `Snapshot` and nothing else (D55): the simulation carries no text, and every number here is one the
/// engine already keeps. D74 allowed this; D82 chose the bitmap font it is drawn with.
/// The bottom panel's height: a selection list on the left, and room above the minimap on the right.
#[must_use]
pub fn hud_panel_height() -> u32 {
    76
}

/// The left edge of a selection row's health bar, and its top. Row 0 is nearest the panel's top edge.
#[must_use]
pub fn selection_row_origin(row: usize, frame_height: u32) -> (i32, i32) {
    let top = frame_height as i32 - hud_panel_height() as i32;
    (8, top + 12 + row as i32 * ROW_HEIGHT)
}

/// The menu's three rows - resume, restart, quit - as `(x, y, w, h)` rectangles, centred in the play area.
///
/// A menu needs words, and the font only grew letters after the HUD needed names; this is the second customer for them.
#[must_use]
pub fn menu_rows(frame_width: u32, frame_height: u32) -> [(i32, i32, u32, u32); 3] {
    const LABELS: [&str; 3] = ["RESUME", "RESTART", "QUIT"];
    let widest = LABELS
        .iter()
        .map(|label| crate::font::text_width(label, 2))
        .max()
        .unwrap_or(0);
    let width = (widest as u32).saturating_add(32);
    let height = 24;
    let gap = 8;
    let total = 3 * height + 2 * gap;
    let top = (frame_height.saturating_sub(total)) / 2;
    let left = (frame_width.saturating_sub(width)) / 2;
    std::array::from_fn(|index| {
        (
            left as i32,
            (top + index as u32 * (height + gap)) as i32,
            width,
            height,
        )
    })
}

/// Which menu row a pixel picks, if any. The same shape as `speed_at`: the menu takes input only where it draws.
#[must_use]
pub fn menu_at(pixel: (i32, i32), frame_width: u32, frame_height: u32) -> Option<usize> {
    menu_rows(frame_width, frame_height)
        .iter()
        .position(|&(x, y, w, h)| {
            pixel.0 >= x && pixel.0 < x + w as i32 && pixel.1 >= y && pixel.1 < y + h as i32
        })
}

/// Draw the menu over whatever is on the screen: a dim wash over the playfield, then the three rows, the chosen one lit.
pub fn draw_menu(frame: &mut Frame, frame_width: u32, frame_height: u32, chosen: Option<usize>) {
    // The wash: a dark screen, translucent enough that the match can be glimpsed behind it. `put` rather than `fill_rect`
    // because a wash has to *mix* with what is there; the frame is ours to darken in place.
    for y in 0..frame.height {
        for x in 0..frame.width {
            let pixel = frame.pixel(x, y);
            let dim = [
                (u16::from(pixel[0]) * 3 / 10) as u8,
                (u16::from(pixel[1]) * 3 / 10) as u8,
                (u16::from(pixel[2]) * 3 / 10) as u8,
                pixel[3].max(palette::MENU_WASH[3]),
            ];
            frame.put(x as i32, y as i32, dim);
        }
    }
    let labels = ["RESUME", "RESTART", "QUIT"];
    for (index, (x, y, w, h)) in menu_rows(frame_width, frame_height).iter().enumerate() {
        let colour = if Some(index) == chosen {
            palette::MENU_ROW
        } else {
            palette::PANEL
        };
        frame.fill_rect(*x, *y, *w as i32, *h as i32, colour);
        let label = labels[index];
        let text_width = crate::font::text_width(label, 2);
        let text_x = *x + (*w as i32 - text_width) / 2;
        let text_y = *y + (*h as i32 - crate::font::HEIGHT * 2) / 2;
        frame.draw_text(text_x, text_y, label, 2, palette::HUD_TEXT);
    }
}

/// **Which tile a pixel on the minimap means.** The box is drawn at two pixels a tile (`minimap_rect`), so the inverse is
/// a division - but a division done in the window would be a second copy of the layout, and the day the box moves, one of
/// the two copies would be wrong.
#[must_use]
pub fn minimap_tile(
    pixel: (i32, i32),
    rect: (i32, i32, u32, u32),
    map_width: u16,
    map_height: u16,
) -> Option<(u16, u16)> {
    let (x, y, w, h) = rect;
    if w == 0 || h == 0 || u32::from(map_width) == 0 || u32::from(map_height) == 0 {
        return None;
    }
    if pixel.0 < x || pixel.1 < y || pixel.0 >= x + w as i32 || pixel.1 >= y + h as i32 {
        return None;
    }
    let across = u32::try_from(pixel.0 - x).ok()? * u32::from(map_width) / w;
    let down = u32::try_from(pixel.1 - y).ok()? * u32::from(map_height) / h;
    Some((
        u16::try_from(across.min(u32::from(map_width) - 1)).ok()?,
        u16::try_from(down.min(u32::from(map_height) - 1)).ok()?,
    ))
}

/// The minimap's place in the frame: bottom-right, **above** the panel, two pixels a tile.
#[must_use]
pub fn minimap_rect(
    frame_width: u32,
    frame_height: u32,
    map_width: u16,
    map_height: u16,
) -> (i32, i32, u32, u32) {
    let scale = 2;
    let width = u32::from(map_width) * scale;
    let height = u32::from(map_height) * scale;
    (
        frame_width as i32 - width as i32 - 8,
        frame_height as i32 - hud_panel_height() as i32 - height as i32 - 8,
        width,
        height,
    )
}

/// The middle of the outcome banner: a strip across the play area, in colour because the font has no words (D82).
#[must_use]
pub fn banner_point(frame_width: u32, frame_height: u32) -> (u32, u32) {
    (
        frame_width / 2,
        (frame_height.saturating_sub(hud_panel_height())) / 2,
    )
}

/// How tall one selection row is, in pixels.
const ROW_HEIGHT: i32 = 12;
/// How wide a selection row's health bar is, in pixels.
const ROW_BAR: i32 = 60;

/// **Everything a player watches, in one place.** The top bar is the stock, the population and the speed; the bottom panel
/// is what is selected; the corner is the minimap; a decided match paints a banner across the middle. `Hud::of` builds the
/// numbers and this draws them - the window's job is to call it, not to decide what a HUD is.
pub fn hud(frame: &mut Frame, hud: &Hud, snapshot: &Snapshot, speed: crate::Speed) {
    let bar = hud_bar_height().min(frame.height);
    if bar > 0 {
        frame.fill_rect(0, 0, frame.width as i32, bar as i32, palette::HUD_BAR);
        let scale = 3;
        let text_y = (bar as i32 - crate::font::HEIGHT * scale) / 2;
        let swatch = 12;
        for (column, (name, amount)) in hud.stock.iter().enumerate() {
            let x = hud_swatch_x(column) as i32;
            let colour = palette::STOCK[column.min(palette::STOCK.len() - 1)];
            frame.fill_rect(x, (bar as i32 - swatch) / 2, swatch, swatch, colour);
            frame.draw_number(x + swatch + 6, text_y, *amount, scale, palette::HUD_TEXT);
            // The pack's own word for this column, at one pixel a dot: the swatch says which colour, and a colour is not
            // a name to somebody who has not played this pack.
            let (nx, ny) = stock_name_origin(column);
            frame.draw_text(nx, ny, name, 1, palette::HUD_TEXT);
        }
        let x = hud_swatch_x(hud.stock.len()) as i32;
        frame.draw_ratio(
            x,
            text_y,
            hud.population,
            hud.population_cap,
            scale,
            palette::HUD_TEXT,
        );
        speed_selector(frame, speed);
    }

    let top = frame.height as i32 - hud_panel_height() as i32;
    if top >= bar as i32 && top < frame.height as i32 {
        frame.fill_rect(
            0,
            top,
            frame.width as i32,
            hud_panel_height() as i32,
            palette::PANEL,
        );
        // A one-pixel edge along the panel's top: on darkness the panel's own colour is nearly the void's, and a HUD that
        // is drawn but unreadable is the same to a player as one that is not drawn.
        frame.fill_rect(0, top, frame.width as i32, 1, palette::MARK);
        if let Some(notice) = &hud.notice {
            let (nx, ny) = notice_origin(frame.height);
            frame.draw_text(nx, ny, notice, 1, palette::REFUSED);
        }
        // **A row per selected thing, five at most.** The panel is a list, not a summary: a player who boxed six villagers
        // sees six bars and can see which one is hurt.
        for (index, row) in hud.selected.iter().take(5).enumerate() {
            let (x, y) = selection_row_origin(index, frame.height);
            frame.fill_rect(x, y, ROW_BAR, 4, palette::HP_TRACK);
            let lit = (ROW_BAR as f32 * row.hp_fraction.clamp(0.0, 1.0)) as i32;
            let colour = if row.hp_fraction > 0.5 {
                palette::HP_GREEN
            } else {
                palette::HP_RED
            };
            frame.fill_rect(x, y, lit, 4, colour);
            // A load in hand, then a production queue: two numbers, no words.
            let mut next = x + ROW_BAR + 6;
            if row.carry > 0 {
                next = frame.draw_number(next, y - 1, row.carry, 1, palette::HUD_TEXT) + 4;
            }
            if row.pending > 0 {
                frame.draw_number(next, y - 1, row.pending as u32, 1, palette::SELECTION);
            }
            // A building going up shows how far it has got, in the order marker's colour.
            if let Some(progress) = row.construction {
                let lit = (ROW_BAR as f32 * progress.clamp(0.0, 1.0)) as i32;
                frame.fill_rect(x, y + 5, lit, 2, palette::ORDER);
            }
        }
    }

    let (mx, my, mw, mh) = minimap_rect(frame.width, frame.height, snapshot.width, snapshot.height);
    if mw > 0 && mh > 0 && mx >= 0 && my >= 0 {
        // The box, then the map: unexplored ground is nearly black, so without an edge the minimap is a hole.
        frame.fill_rect(mx - 1, my - 1, mw as i32 + 2, 1, palette::MARK);
        frame.fill_rect(mx - 1, my + mh as i32, mw as i32 + 2, 1, palette::MARK);
        frame.fill_rect(mx - 1, my, 1, mh as i32, palette::MARK);
        frame.fill_rect(mx + mw as i32, my, 1, mh as i32, palette::MARK);
        let map = minimap(snapshot, mw / u32::from(snapshot.width).max(1));
        for py in 0..map.height {
            for px in 0..map.width {
                frame.put(mx + px as i32, my + py as i32, map.pixel(px, py));
            }
        }
    }

    if let Some(outcome) = hud.outcome {
        let colour = match outcome {
            ruage_sim::MatchOutcome::Victory { winner } if winner == snapshot.player => {
                palette::WON
            }
            _ => palette::LOST,
        };
        let (bx, by) = banner_point(frame.width, frame.height);
        let _ = bx;
        frame.fill_rect(0, by as i32 - 4, frame.width as i32, 8, colour);
    }
}
