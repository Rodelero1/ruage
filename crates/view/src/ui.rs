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
    /// The unit or building the entity is, so the panel can ask the pack for its name. `None` for a resource node.
    pub unit: Option<u32>,
    /// **The pack's word for it** (`"villager"`), filled by the caller that has the content - the view never guesses a name,
    /// and a pack that renames its villagers renames the panel.
    pub name: String,
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
    /// **The age this player has reached**, in the pack's words (`"feudal_age"`), for the bar's middle.
    pub age: Option<String>,
    /// **What the pointer is over**, as one line for the help box: a button's name and price, or a refusal.
    pub help: Option<String>,
    /// Which of the bar's five buttons the pointer is on, for its hover face.
    pub bar_hover: Option<usize>,
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
                unit: entity.unit,
                name: String::new(),
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
            age: snapshot.age.clone(),
            help: None,
            bar_hover: None,
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
#[must_use]
pub fn hud_bar_height() -> u32 {
    BAR_HEIGHT
}

/// Where a stock column's **name** starts: under its swatch, one pixel a dot, because a colour is not a name to somebody
/// who has not played this pack. The bar is tall enough for the numbers (three pixels a dot) above it.
#[must_use]
pub fn stock_name_origin(column: usize) -> (i32, i32) {
    let layout = Layout::of(960, 720);
    let (x, _, _, _) = layout.stock[column.min(layout.stock.len() - 1)];
    (x, BAR_HEIGHT as i32 - 7)
}

/// Where a refusal is written: the top of the panel's selected-object zone, where the eye goes after an order.
#[must_use]
pub fn notice_origin(frame_height: u32) -> (i32, i32) {
    let layout = Layout::of(960, frame_height);
    // The bottom of the selected zone: the name and the health own its top, and a refusal is read after the order.
    (
        layout.selected.0 + 8,
        layout.selected.1 + layout.selected.3 as i32 - 10,
    )
}

/// Where a stock column's swatch starts. **The pack's order is the display order**, so a pack with three resources
/// shows three swatches and the view never learns their names.
#[must_use]
pub fn hud_swatch_x(column: usize) -> u32 {
    stock_name_origin(column).0 as u32
}

/// The speed selector's five boxes, at the right end of the HUD bar: `(x, y, w, h)` each, in `Speed::ALL` order.
///
/// A **segmented control**, not a dropdown: five boxes with the live one filled say "how fast" and are clickable, and they
/// are the bar's five buttons, laid out by `Layout` so they stay against the right edge on any width.
#[must_use]
pub fn speed_boxes(frame_width: u32, frame_height: u32) -> [Rect; 5] {
    Layout::of(frame_width, frame_height).speed
}

/// Which speed a click at a pixel picks, if any.
#[must_use]
pub fn speed_at(pixel: (i32, i32), frame_width: u32, frame_height: u32) -> Option<crate::Speed> {
    if pixel.0 < 0 || pixel.1 < 0 {
        return None;
    }
    speed_boxes(frame_width, frame_height)
        .iter()
        .position(|&(bx, by, w, h)| {
            pixel.0 >= bx && pixel.0 < bx + w as i32 && pixel.1 >= by && pixel.1 < by + h as i32
        })
        .map(|index| crate::Speed::ALL[index])
}

/// Draw the selector: one widget per speed, the current one held down.
pub fn speed_selector(frame: &mut Frame, speed: crate::Speed) {
    let layout = Layout::of(frame.width, frame.height);
    speed_selector_in(frame, &layout, speed);
}

/// The names on the bar's five buttons, in the game's order. Placeholders - the owner's choice - but words, not icons.
pub const BAR_LABELS: [&str; 5] = ["MENU", "STATS", "CHAT", "DIPLO", "TECH"];

/// Which of the bar's five buttons a pixel picks, if any.
#[must_use]
pub fn bar_button_at(pixel: (i32, i32), layout: &Layout) -> Option<usize> {
    layout.bar_buttons.iter().position(|&(x, y, w, h)| {
        pixel.0 >= x && pixel.0 < x + w as i32 && pixel.1 >= y && pixel.1 < y + h as i32
    })
}

/// Draw the bar's five buttons: a labelled widget each, in `WidgetState`, so the bar is the game's bar and not a row of
/// blank boxes.
fn bar_buttons_in(frame: &mut Frame, layout: &Layout, hovered: Option<usize>) {
    for (index, rect) in layout.bar_buttons.into_iter().enumerate() {
        let state = if Some(index) == hovered {
            WidgetState::Hover
        } else {
            WidgetState::Normal
        };
        draw_widget(frame, rect, state);
        let label = BAR_LABELS[index];
        let width = crate::font::text_width(label, 1);
        let (x, y, w, h) = rect;
        frame.draw_text(
            x + (w as i32 - width) / 2,
            y + (h as i32 - crate::font::HEIGHT) / 2,
            label,
            1,
            palette::HUD_TEXT,
        );
    }
}

/// The selector, for a caller that has the layout already: `hud` does, and re-deriving it per part was **316 us a frame**
/// against 151-195 for the whole HUD before the layout existed. The layout is cheap; four of them per frame is not.
fn speed_selector_in(frame: &mut Frame, layout: &Layout, speed: crate::Speed) {
    let boxes = layout.speed;
    for (index, rect) in boxes.into_iter().enumerate() {
        let chosen = crate::Speed::ALL[index] == speed;
        let state = if chosen {
            WidgetState::Pressed
        } else {
            WidgetState::Normal
        };
        let (x, y, w, h) = rect;
        draw_widget(frame, rect, state);
        // The live one says so in the selection colour: the bevel alone is not a readout.
        if chosen {
            frame.fill_rect(x + 4, y + 4, w as i32 - 8, h as i32 - 8, palette::SELECTION);
        }
    }
}

/// Draw the HUD over a frame: one swatch and one number per stockpile column, then population as `have/cap`.
///
/// It reads a `Snapshot` and nothing else (D55): the simulation carries no text, and every number here is one the
/// engine already keeps. D74 allowed this; D82 chose the bitmap font it is drawn with.
/// A rectangle as `(x, y, w, h)`, which is how every part of the layout is handed out.
pub type Rect = (i32, i32, u32, u32);

/// How a widget looks. The states are the interface's grammar: a thing that can be pressed looks one way under the
/// pointer, another while held, and another when it is not available at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetState {
    Normal,
    Hover,
    Pressed,
    Disabled,
}

/// **Where every part of the interface sits.** Anchors, not a table per resolution: what belongs on the right is measured
/// from the frame's right edge, what belongs on the left from its left, and the middle takes what is left over. One layout
/// serves 4:3, 16:9 and 21:9, which is the whole of "it has to run on wide screens".
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    pub width: u32,
    pub height: u32,
    /// The top bar, the map view between the bars, and the bottom bar.
    pub bar: Rect,
    pub view: Rect,
    pub panel: Rect,
    /// One rectangle per stockpile column, then the population, then the age's box, then the five bar buttons.
    pub stock: Vec<Rect>,
    pub population: Rect,
    pub age: Rect,
    /// **The game's five top-bar buttons** - menu, statistics, chat, diplomacy, tech tree - on the bar's right.
    pub bar_buttons: [Rect; 5],
    /// The speed selector, under the minimap: pacing belongs to the window, not to the game's bar (D101), and the bar's
    /// five slots are the game's own buttons.
    pub speed: [Rect; 5],
    /// The panel's three zones: the command grid, the selected object, the minimap - the game's own arrangement.
    pub grid: Rect,
    pub slots: Vec<Rect>,
    pub selected: Rect,
    pub minimap: Rect,
    /// The rollover help's box, above the panel on the left.
    pub help: Rect,
}

/// The top bar's height and the panel's: the panel is three command rows deep, because the grid is five by three.
pub const BAR_HEIGHT: u32 = 30;
/// Three command rows of 40 and the gaps between them, with room above and below: 128 + 14.
pub const PANEL_HEIGHT: u32 = 142;

impl Layout {
    /// The interface for a frame. A frame smaller than the interface gets a smaller interface rather than one drawn off
    /// the edge: every zone is clamped to what is left, and the widths never go negative.
    #[must_use]
    pub fn of(width: u32, height: u32) -> Layout {
        // On a short frame the bars shrink first, so the map keeps some room to be a map.
        let bar_h = BAR_HEIGHT.min(height / 3);
        let panel_h = PANEL_HEIGHT.min(height.saturating_sub(bar_h) / 2);
        let bar = (0, 0, width, bar_h);
        let panel = (0, height.saturating_sub(panel_h) as i32, width, panel_h);
        let view = (
            0,
            bar_h as i32,
            width,
            height.saturating_sub(bar_h + panel_h),
        );

        // The bar: the stockpile from the left, the buttons and the speed from the right, the age between them.
        let stock: Vec<Rect> = (0..4)
            .map(|column| (8 + column * 60, (bar_h / 2) as i32 - 7, 56, 14))
            .collect();
        let population = (8 + 4 * 60, (bar_h / 2) as i32 - 7, 48, 14);
        let buttons_w = 5 * 26 + 4 * 4;
        let buttons_x = width.saturating_sub(8 + buttons_w) as i32;
        let bar_buttons: [Rect; 5] = std::array::from_fn(|index| {
            (
                buttons_x + (index as i32) * 30,
                (bar_h / 2) as i32 - 9,
                26,
                18,
            )
        });
        let age_x = population.0 + population.2 as i32 + 12;
        let age_w = (buttons_x - 12 - age_x).max(0) as u32;
        let age = (age_x, (bar_h / 2) as i32 - 7, age_w, 14);

        // The panel: the grid on the left, the minimap on the right, the selected object in what is left.
        let minimap_w = 160.min(width / 4);
        let minimap = (
            width.saturating_sub(8 + minimap_w) as i32,
            panel.1 + 6,
            minimap_w,
            panel_h.saturating_sub(12),
        );
        // A command button is forty pixels square, as the game's are: a name and a price need the room.
        let slot: u32 = 40;
        let pitch: i32 = slot as i32 + 4;
        let grid_w = 5 * slot + 4 * 4;
        let grid_h = 3 * slot + 2 * 4;
        let grid = (
            8,
            panel.1 + 7,
            grid_w,
            grid_h.min(panel_h.saturating_sub(14)),
        );
        let slots: Vec<Rect> = (0..15)
            .map(|index| {
                let column = index % 5;
                let row = index / 5;
                (grid.0 + column * pitch, grid.1 + row * pitch, slot, slot)
            })
            .collect();
        let selected_x = grid.0 + grid.2 as i32 + 8;
        let selected = (
            selected_x,
            panel.1 + 6,
            (minimap.0 - 8 - selected_x).max(0) as u32,
            panel_h.saturating_sub(12),
        );
        // The speed selector sits at the **bottom right of the selected zone**: pacing is the window's, not the game's bar
        // (D101), and the bar's five slots are the game's own buttons. The notice takes the same row's left.
        let speed_w = 26u32;
        let speed_gap = 4u32;
        let speed_total = 5 * speed_w + 4 * speed_gap;
        let speed_right = (minimap.0 - 8).max(speed_total as i32);
        let speed: [Rect; 5] = std::array::from_fn(|index| {
            let column = u32::try_from(index).unwrap_or(0);
            (
                speed_right - speed_total as i32 + (column * (speed_w + speed_gap)) as i32,
                panel.1 + panel_h as i32 - 26,
                speed_w,
                18,
            )
        });
        let help = (
            8,
            (panel.1 - 28).max(bar_h as i32),
            420.min(width.saturating_sub(16)),
            22,
        );

        Layout {
            width,
            height,
            bar,
            view,
            panel,
            stock,
            population,
            age,
            bar_buttons,
            speed,
            grid,
            slots,
            selected,
            minimap,
            help,
        }
    }
}

/// **One widget, in a state.** The bevel is what makes a placeholder read as a button: a lit edge above and to the left,
/// a shadowed one below and to the right, and a face that changes under the pointer and while pressed.
pub fn draw_widget(frame: &mut Frame, rect: Rect, state: WidgetState) {
    let (x, y, w, h) = rect;
    if w < 2 || h < 2 {
        return;
    }
    let face = match state {
        WidgetState::Normal => palette::WIDGET,
        WidgetState::Hover => palette::WIDGET_HOVER,
        WidgetState::Pressed => palette::WIDGET_PRESSED,
        WidgetState::Disabled => palette::WIDGET_DISABLED,
    };
    frame.fill_rect(x, y, w as i32, h as i32, face);
    // A pressed widget's bevel is the other way about, which is how a physical button reads.
    let (light, dark) = match state {
        WidgetState::Pressed => (palette::BEVEL_DARK, palette::BEVEL_LIGHT),
        _ => (palette::BEVEL_LIGHT, palette::BEVEL_DARK),
    };
    frame.fill_rect(x, y, w as i32, 1, light);
    frame.fill_rect(x, y, 1, h as i32, light);
    frame.fill_rect(x, y + h as i32 - 1, w as i32, 1, dark);
    frame.fill_rect(x + w as i32 - 1, y, 1, h as i32, dark);
}

/// The bottom panel's height: a selection list on the left, and room above the minimap on the right.
#[must_use]
pub fn hud_panel_height() -> u32 {
    PANEL_HEIGHT
}

/// The left edge of a selection row's health bar, and its top. Row 0 is nearest the panel's top edge.
#[must_use]
pub fn selection_row_origin(row: usize, frame_height: u32) -> (i32, i32) {
    let layout = Layout::of(960, frame_height);
    // Below the name and the health bar, which own the top of the zone.
    (
        layout.selected.0 + 8,
        layout.selected.1 + 40 + row as i32 * ROW_HEIGHT,
    )
}

/// **Whether a pixel belongs to the interface rather than the map.** One question, one answer, computed from the same
/// layout that drew it - OpenAoK asks its interface art's own alpha, because its interface is a bitmap; ours is drawn, so
/// the layout is the mask. The window asks this *before* it decides to select or to box, so a click on the panel can never
/// also reach the ground behind it, and the special-case order (speed, then card, then minimap) stops being load-bearing.
#[must_use]
pub fn over_hud(pixel: (i32, i32), layout: &Layout) -> bool {
    let inside = |rect: Rect| {
        let (x, y, w, h) = rect;
        w > 0
            && h > 0
            && pixel.0 >= x
            && pixel.0 < x + w as i32
            && pixel.1 >= y
            && pixel.1 < y + h as i32
    };
    inside(layout.bar) || inside(layout.panel)
}

/// One button on the command card: a name, its price in stock columns, and why it would be refused (if it would).
#[derive(Debug, Clone, PartialEq)]
pub struct CardButton {
    pub name: String,
    /// `(stock column, amount)` - the columns are the pack's order, the numbers are the pack's prices.
    pub costs: Vec<(usize, u32)>,
    pub refusal: Option<String>,
}

/// **The command grid's slots**, five across and three down, in the panel's left zone - the game's own arrangement, and
/// the reason a card no longer floats above the panel. A card with more than fifteen things is a card whose last button
/// does not exist; callers clip.
#[must_use]
pub fn card_rows(count: usize, frame_width: u32, frame_height: u32) -> Vec<Rect> {
    let layout = Layout::of(frame_width, frame_height);
    layout.slots.iter().copied().take(count.min(15)).collect()
}

/// Which button a pixel picks, if any.
#[must_use]
pub fn card_at(
    pixel: (i32, i32),
    count: usize,
    frame_width: u32,
    frame_height: u32,
) -> Option<usize> {
    card_rows(count, frame_width, frame_height)
        .iter()
        .position(|&(x, y, w, h)| {
            pixel.0 >= x && pixel.0 < x + w as i32 && pixel.1 >= y && pixel.1 < y + h as i32
        })
}

/// Draw the card: each button a dark box, its name in the pack's word, its price in digits, and a refusal in red.
/// `chosen` is the button the pointer is on, which is drawn lit - the same shape as the menu's rows and the speed boxes.
pub fn draw_card(
    frame: &mut Frame,
    buttons: &[CardButton],
    frame_width: u32,
    frame_height: u32,
    chosen: Option<usize>,
) {
    for (index, (x, y, w, h)) in card_rows(buttons.len(), frame_width, frame_height)
        .into_iter()
        .enumerate()
    {
        let button = &buttons[index];
        // Every button is a **widget**: a labelled placeholder with a bevel and a state, which is what the owner asked for
        // instead of bare rectangles drawn to say "icon goes here".
        let state = match (Some(index) == chosen, button.refusal.is_some()) {
            (true, _) => WidgetState::Hover,
            (false, true) => WidgetState::Disabled,
            (false, false) => WidgetState::Normal,
        };
        draw_widget(frame, (x, y, w, h), state);
        // The name, trimmed to the button: a button is not a paragraph, and a name that overflows its box makes two buttons
        // look like one. **Words, not icons** - the owner's placeholder choice - are readable now and replaceable later.
        let room =
            usize::try_from((w as i32 - 6) / (crate::font::WIDTH + crate::font::GAP)).unwrap_or(0);
        let label: String = button.name.to_uppercase().chars().take(room).collect();
        frame.draw_text(x + 3, y + 4, &label, 1, palette::HUD_TEXT);
        // The price: one number per cost, in the pack's column order.
        let mut at = x + 3;
        for (_, amount) in &button.costs {
            at = frame.draw_number(at, y + 12, *amount, 1, palette::HUD_TEXT) + 3;
        }
        // And why not, if it would not be taken: a red bar and the engine's own words, at the bottom of the button.
        if let Some(refusal) = &button.refusal {
            frame.fill_rect(x, y + h as i32 - 5, w as i32, 2, palette::REFUSED);
            let short: String = refusal.to_uppercase().chars().take(room).collect();
            frame.draw_text(x + 3, y + 22, &short, 1, palette::REFUSED);
        }
    }
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

/// The minimap's place in the frame: **inside the panel's right zone**, two pixels a tile where that fits, and centred in
/// the zone - which is what keeps it against the right edge of a wide frame without a second anchoring rule.
#[must_use]
pub fn minimap_rect(
    frame_width: u32,
    frame_height: u32,
    map_width: u16,
    map_height: u16,
) -> (i32, i32, u32, u32) {
    fit_minimap(
        &Layout::of(frame_width, frame_height),
        map_width,
        map_height,
    )
}

/// The minimap fitted inside a layout's right zone, for a caller that has the layout already.
fn fit_minimap(layout: &Layout, map_width: u16, map_height: u16) -> (i32, i32, u32, u32) {
    let zone = layout.minimap;
    let (map_w, map_h) = (u32::from(map_width).max(1), u32::from(map_height).max(1));
    // Two pixels a tile, and never larger than the zone: a 64-tile map at two pixels is 128, and the zone is tall enough
    // for it at any sane panel; a bigger map shrinks to fit rather than spilling over the selected object.
    let scale = 2u32.min(zone.2 / map_w).min(zone.3 / map_h).max(1);
    let (width, height) = (map_w * scale, map_h * scale);
    (
        zone.0 + (zone.2.saturating_sub(width) / 2) as i32,
        zone.1 + (zone.3.saturating_sub(height) / 2) as i32,
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
    // **One layout per frame**, used by every part: deriving it again in each helper was 316 us against 151-195.
    let layout = Layout::of(frame.width, frame.height);
    let bar = layout.bar.3.min(frame.height);
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
        // **The bar's middle says what age the match is in**, and its right is the game's five buttons.
        if let Some(age) = &hud.age {
            let (ax, ay, aw, ah) = layout.age;
            let label = age.to_uppercase().replace('_', " ");
            let width = crate::font::text_width(&label, 1);
            frame.draw_text(
                ax + (aw as i32 - width) / 2,
                ay + (ah as i32 - crate::font::HEIGHT) / 2,
                &label,
                1,
                palette::HUD_TEXT,
            );
        }
        bar_buttons_in(frame, &layout, hud.bar_hover);
    }

    // **From the layout, not from a second height.** The panel used to be filled from `hud_panel_height()` while the zones
    // came from `Layout`: two numbers that had already drifted (76 against 142), so the zones were drawn on ground that was
    // never painted. The probe caught it; this is the fix.
    let top = layout.panel.1;
    if top >= bar as i32 && top < frame.height as i32 {
        frame.fill_rect(
            layout.panel.0,
            layout.panel.1,
            layout.panel.2 as i32,
            layout.panel.3 as i32,
            palette::PANEL,
        );
        // A one-pixel edge along the panel's top: on darkness the panel's own colour is nearly the void's, and a HUD that
        // is drawn but unreadable is the same to a player as one that is not drawn.
        frame.fill_rect(0, top, frame.width as i32, 1, palette::MARK);
        // **The game's three zones**: the command grid on the left, the selected object in the middle, the minimap on the
        // right. Each gets its own ground and a line above and below, so the bar reads as three places.
        for zone in [layout.grid, layout.selected, layout.minimap] {
            if zone.2 == 0 || zone.3 == 0 {
                continue;
            }
            // **An outline, not a fill.** Filling the three zones was 130,000 pixels a frame - measured at 316-355 us for
            // the whole HUD against 151-195 before the layout existed - and the zones read by their edges, not their tone.
            let (x, y, w, h) = (zone.0, zone.1, zone.2 as i32, zone.3 as i32);
            frame.fill_rect(x, y, w, 1, palette::MARK);
            frame.fill_rect(x, y + h - 1, w, 1, palette::MARK);
            frame.fill_rect(x, y, 1, h, palette::MARK);
            frame.fill_rect(x + w - 1, y, 1, h, palette::MARK);
        }
        if let Some(notice) = &hud.notice {
            let (nx, ny) = notice_origin(frame.height);
            frame.draw_text(nx, ny, notice, 1, palette::REFUSED);
        }
        // **What is selected, by name**, at the top of its zone: the pack's word, its health as a bar and a number, and
        // how many are selected when more than one is.
        if let Some(first) = hud.selected.first() {
            let (x, y, w, _) = layout.selected;
            let title = if hud.selected.len() > 1 {
                format!("{} x{}", first.name.to_uppercase(), hud.selected.len())
            } else {
                first.name.to_uppercase()
            };
            frame.draw_text(x + 8, y + 10, &title, 2, palette::HUD_TEXT);
            // The health of the first selected thing, across the zone: wide enough to read at a glance.
            let bar_w = w.saturating_sub(16) as i32;
            let (bx, by) = (x + 8, y + 26);
            frame.fill_rect(bx, by, bar_w, 6, palette::HP_TRACK);
            let lit = (bar_w as f32 * first.hp_fraction.clamp(0.0, 1.0)) as i32;
            let colour = if first.hp_fraction > 0.5 {
                palette::HP_GREEN
            } else {
                palette::HP_RED
            };
            frame.fill_rect(bx, by, lit, 6, colour);
            let percent = (first.hp_fraction.clamp(0.0, 1.0) * 100.0) as u32;
            let used = frame.draw_number(bx + bar_w + 4, by - 1, percent, 1, palette::HUD_TEXT);
            let _ = used;
        }
        // **A row per selected thing, ten at most.** The panel is a list, not a summary: a player who boxed six villagers
        // sees six bars and can see which one is hurt.
        for (index, row) in hud.selected.iter().take(6).enumerate() {
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

    // The speed selector sits under the minimap, where pacing belongs (D101): the bar's five slots are the game's.
    speed_selector_in(frame, &layout, speed);

    let (mx, my, mw, mh) = fit_minimap(&layout, snapshot.width, snapshot.height);
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

    // **The help box**, above the panel on the left: what the pointer is over, in one line. It is the cheapest teaching
    // device in the interface - openaok's `SetWorldHint`, and the reason a player learns what a button does without a
    // manual.
    if let Some(help) = &hud.help {
        let (hx, hy, hw, hh) = layout.help;
        if hw > 0 {
            frame.fill_rect(hx, hy, hw as i32, hh as i32, palette::PANEL);
            let (x, y, w, h) = (hx, hy, hw as i32, hh as i32);
            frame.fill_rect(x, y, w, 1, palette::MARK);
            frame.fill_rect(x, y + h - 1, w, 1, palette::MARK);
            let room = usize::try_from((hw as i32 - 8) / (crate::font::WIDTH + crate::font::GAP))
                .unwrap_or(0);
            let line: String = help.to_uppercase().chars().take(room).collect();
            frame.draw_text(
                hx + 4,
                hy + (hh as i32 - crate::font::HEIGHT) / 2,
                &line,
                1,
                palette::HUD_TEXT,
            );
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
