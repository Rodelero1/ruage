//! Selection, the HUD and the minimap: everything a player needs to *act* rather than watch.
//!
//! Written before the implementation (D7). All three are pure functions of a [`Snapshot`](crate::Snapshot):
//! they read what the player can see, and selection turns a click into `Command`s the engine already has.
//! None of them touches `World` - the UI issues orders, it does not execute them (D57).

use ruage_data::Resources;
use ruage_sim::{Command, EntityKind};

use crate::{Camera, Frame, Snapshot, covers, drawn_at, palette};

/// The first **unit** in a list of candidates under the pointer, or `None`: a click means the body it landed on before the
/// ground that body is standing on (a farm is a floor, a node's tile can be walked).
fn first_unit<'a>(entities: &[&'a crate::EntityView]) -> Option<&'a crate::EntityView> {
    entities
        .iter()
        .copied()
        .find(|entity| entity.kind == EntityKind::Unit)
}

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

    /// A click selects what is under the cursor: units before buildings, your own before somebody else's, and a resource
    /// node last - clicking the ground clears the selection.
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
        // **Somebody else's things are inspectable**: the owner asked for enemy hit points on click (2026-11-04), and a
        // resource node is nobody's - every node a map seeds belongs to Gaia (player 0), so "only your own" would mean none
        // of them. What keeps this honest is the snapshot, not this filter: an enemy is in it only while the player can see
        // them, so a click can never reach past the fog.
        let theirs: Vec<&crate::EntityView> = snapshot
            .entities
            .iter()
            .filter(|entity| entity.player != snapshot.player)
            .filter(|entity| matches!(entity.kind, EntityKind::Unit | EntityKind::Building))
            .filter(|entity| covers(entity, tile))
            .collect();
        let node = snapshot
            .entities
            .iter()
            .find(|entity| matches!(entity.kind, EntityKind::Resource(_)) && covers(entity, tile));
        // **Units before buildings, ours before theirs** - the order the picture is drawn in, and the order that makes a
        // click mean what it looks like: a body standing on a farm is the thing a player clicked, and among bodies the
        // player's own is the one whose orders are theirs to give.
        let chosen = first_unit(&own)
            .or_else(|| first_unit(&theirs))
            .or_else(|| own.first().copied())
            .or_else(|| theirs.first().copied())
            .or(node);
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
            // The band follows the **drawn** marker (D168), or a unit is swept up half a tile from where it looks.
            let (px, py) = camera.project(drawn_at(entity.position));
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
                let (x, y) = camera.project(drawn_at(entity.position));
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
                // **A husk is a plot, not a building** (the owner, 2026-11-04: *"only serves the player who can left click
                // villager, right click on the husk and villager starts planting new farm"*). So a villager ordered onto a
                // spent farm of his own side is ordered to **plant a new one there** - the unit the husk was is the unit the
                // new farm is, and the pack's price (60 wood) is paid by the build order as it always is.
                (EntityKind::Unit, Some(target))
                    if target.is_husk && target.player == snapshot.player =>
                {
                    if let Some(unit) = target.unit {
                        orders.push(Command::Build {
                            villager: ruage_sim::EntityId(*id),
                            unit,
                            at: (
                                u16::try_from(tile.0).unwrap_or(0),
                                u16::try_from(tile.1).unwrap_or(0),
                            ),
                        });
                    }
                }
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
                // **A point, not the tile's middle** (the owner, 2026-11-04: *"I don't want units to walk to the centre of
                // the square. That can walk wherever not blocked."*). The click was projected to a continuous position and
                // then rounded to a tile, so every order - however precisely aimed - finished at the centre of a square. The
                // engine already carries this form (`Command::WalkTo`); only the view was throwing the point away.
                (EntityKind::Unit, _) => {
                    let point = camera.pick_point(pixel);
                    let scale = snapshot.scale as f32;
                    orders.push(Command::WalkTo {
                        unit,
                        to: (
                            ruage_fixed::Fixed::from_raw((point.0 * scale) as i64),
                            ruage_fixed::Fixed::from_raw((point.1 * scale) as i64),
                        ),
                    });
                }
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
    /// Hit points as they stand - the number a **node** shows instead of a bar, because nothing records what it started
    /// with (`0.0` as a fraction of nothing tells a player nothing).
    pub hp: u32,
    /// What the entity still holds, in whole resources: a tree's wood, a farm's food, a bush's berries.
    pub amount: u32,
    /// **Which resource that is, in the pack's own word** (`"wood"`), resolved by whoever holds the content. `None` for
    /// everything that holds nothing, so the panel never prints a number without saying whose number it is.
    pub resource: Option<String>,
    pub carry: u32,
    pub construction: Option<f32>,
    pub garrison: u32,
    /// Units this building is making, the one in progress included.
    pub pending: usize,
    /// **What this building is making** (D157): drawn as a second bar under the health bar, because research and training share
    /// one queue and a player watching a barracks wants to see both.
    pub production: ruage_sim::Production,
    pub pose: crate::Pose,
    /// **The numbers the card prints**: attack, both armours, range, speed, sight, work rate and carry capacity, effective
    /// for the owner. `None` for a resource node, which has no unit behind it.
    pub stats: Option<ruage_sim::UnitStats>,
    /// **A spent farm**, drawn as its building with a cross over the pictogram (the owner, 2026-11-05: *"husk can get the same
    /// pictogram as farm but crossed over"*). D122 decided when a farm becomes a husk; this decides how it looks until the
    /// husk unit (357) has art of its own.
    pub is_husk: bool,
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
    /// **What the camera can see**, as the four corner tiles of its view: the box drawn on the minimap. The window sets it
    /// from its own camera (`Camera::visible_corners`), because a picture of a match does not know where anybody is looking
    /// (the owner's note, 2026-11-04).
    pub view: Option<[(f32, f32); 4]>,
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
            .map(|entity| {
                let resource = entity
                    .resource
                    .and_then(|slot| resources.name(slot))
                    .map(str::to_owned);
                SelectedRow {
                    id: entity.id,
                    unit: entity.unit,
                    // **A node has no role**, so the pack has no word for it: its column gives the title instead ("wood"),
                    // which is the same word the stock bar uses. A unit or a building is named by the caller that holds the
                    // content, and left blank here.
                    name: if entity.unit.is_none() {
                        resource.clone().unwrap_or_default()
                    } else {
                        String::new()
                    },
                    kind: entity.kind,
                    hp_fraction: entity.hp_fraction,
                    hp: entity.hp,
                    amount: entity.amount,
                    resource,
                    carry: entity.carry,
                    construction: entity.construction,
                    garrison: entity.garrison,
                    pending: entity.pending,
                    production: entity.production.clone(),
                    pose: entity.pose,
                    stats: entity.stats,
                    is_husk: entity.is_husk,
                }
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
            view: None,
        }
    }
}

// --- the HUD ---------------------------------------------------------------------------------

/// The height of the HUD bar: the game is drawn below it, never under it.
#[must_use]
pub fn hud_bar_height() -> u32 {
    BAR_HEIGHT
}

/// **Where each stock column starts, and where the population readout goes** - laid out from the numbers themselves rather
/// than from fixed slots. The bar used to place the population at the *last stock column's* x, which is why the owner saw
/// stone and population written on top of each other (2026-11-04): a four-digit stonepile ran into a four-digit population
/// and both became noise. Widths come from `font::text_width`, so a five-digit stockpile moves its neighbours instead of
/// overprinting them.
#[must_use]
pub fn stock_columns(hud: &Hud) -> (Vec<i32>, i32) {
    let text_scale = 2;
    let swatch = 10;
    let gap = 16;
    let mut x = 8i32;
    let mut columns = Vec::with_capacity(hud.stock.len());
    for (_, amount) in &hud.stock {
        columns.push(x);
        let number = crate::font::text_width(&amount.to_string(), text_scale);
        x += swatch + 6 + number + gap;
    }
    (columns, x)
}

/// **The three baselines of a stock column**, from the bar's height: the pack's name on top, the number under it, and the
/// colour swatch level with the number. One function, so a test that looks for a swatch and the code that draws one cannot
/// disagree about where it is.
#[must_use]
pub fn stock_rows(frame_height: u32) -> (i32, i32, i32) {
    let bar = Layout::of(960, frame_height).bar.3.min(frame_height) as i32;
    let scale = 2;
    let swatch = 10;
    let name_y = 2;
    let number_y = bar - crate::font::HEIGHT * scale - 2;
    (
        name_y,
        number_y,
        number_y + (crate::font::HEIGHT * scale - swatch) / 2,
    )
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
        //
        // **As wide as a 2:1 diamond allows.** The owner asked for a bigger minimap (2026-11-04) and showed the game's own
        // bar, where the map is a quarter of the width. A flat diamond twice as wide as it is tall is bounded by the *panel's
        // height*, not by the screen: `2 x (panel - 12)` px wide fills the bar vertically, and a wider box than that would only
        // add empty corners. 160 was the old fixed width - under half of what the bar can hold at 1280.
        // **The diamond fills the bar, and the box is the diamond's own shape.** The owner: "bar is the same height, minimap
        // fills it vertically and proportionally grows horizontally" (2026-11-04). A 2:1 diamond that fills a 130-pixel-tall
        // interior is 260 pixels wide - that is the arithmetic, and a wider box would only add empty corners, while a wider
        // *diamond* would mean a taller bar.
        let minimap_h = panel_h.saturating_sub(12);
        let minimap_w = (minimap_h.saturating_mul(2)).min(width / 3).max(160);
        let minimap = (
            width.saturating_sub(8 + minimap_w) as i32,
            panel.1 + 6,
            minimap_w,
            minimap_h,
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

/// **The production bar** (D157): what the selected building is making, as a bar that fills, with the number waiting beside it.
///
/// Drawn from [`ruage_sim::Production`] and nothing else, so the picture cannot disagree with the queue: an empty building draws
/// an empty track (and no text), a job in progress fills `done / total`, and the count behind it says how long the wait is.
pub fn production_bar(
    frame: &mut crate::Frame,
    at: (i32, i32),
    width: u32,
    production: &ruage_sim::Production,
) {
    let (x, y) = at;
    frame.fill_rect(x, y, width as i32, 4, palette::HP_TRACK);
    if let Some((_, done, total)) = production.running {
        let fraction = if total == 0 {
            0.0
        } else {
            (done as f32 / total as f32).clamp(0.0, 1.0)
        };
        let lit = (width as f32 * fraction) as i32;
        frame.fill_rect(x, y, lit, 4, palette::PROGRESS);
    }
    if !production.waiting.is_empty() {
        let label = format!("+{}", production.waiting.len());
        frame.draw_text(
            x + width as i32 - crate::font::text_width(&label, 1) - 1,
            y + 5,
            &label,
            1,
            palette::HUD_TEXT,
        );
    }
}

/// The bottom panel's height: a selection list on the left, and room above the minimap on the right.
#[must_use]
pub fn hud_panel_height() -> u32 {
    PANEL_HEIGHT
}

/// One selection card's width, gap and height, in pixels. A card is the game's: a name, a portrait, the small bar under it
/// and the numbers beside it.
const CARD_W: i32 = 118;
const CARD_GAP: i32 = 4;
const CARD_H: i32 = 62;
/// The portrait's side, and the small health bar under it.
const PORTRAIT: i32 = 30;

/// The left edge of a selection **card**, and its top. Card 0 is nearest the panel's left edge (2026-11-04: the wide health
/// bar and the row list it fed were replaced by cards, so this is where a card starts rather than where a row's bar did).
#[must_use]
pub fn selection_row_origin(row: usize, frame_height: u32) -> (i32, i32) {
    let layout = Layout::of(960, frame_height);
    (
        layout.selected.0 + 6 + row as i32 * (CARD_W + CARD_GAP),
        layout.selected.1 + 4,
    )
}

/// How many cards fit across the selected zone.
fn selection_cards_that_fit(zone: (i32, i32, u32, u32)) -> usize {
    let room = zone.2 as i32 - 12;
    (room / (CARD_W + CARD_GAP)).max(0) as usize
}

/// **A number in hundredths, as a player reads it**: 400 is "4", 96 is "0.96", 1859 is "18.59". Integer only - the value
/// arrives as an integer and no float is created to print it.
fn card_number(value: i32) -> String {
    let whole = value / 100;
    let rest = value % 100;
    if rest == 0 {
        format!("{whole}")
    } else if rest % 10 == 0 {
        format!("{whole}.{}", rest / 10)
    } else {
        format!("{whole}.{rest:02}")
    }
}

/// **One card**: the name, a portrait, the small health bar with `n/m` under it, and the numbers beside it (owner,
/// 2026-11-04, from the game's own selection bar). A worker shows its work rate and a load in hand; a fighting unit shows
/// attack, both armours and reach; a resource node shows how much it holds.
fn draw_selection_card(
    frame: &mut crate::Frame,
    zone: (i32, i32, u32, u32),
    index: usize,
    row: &SelectedRow,
) {
    let (x, y) = (zone.0 + 6 + index as i32 * (CARD_W + CARD_GAP), zone.1 + 4);
    // The card's own ground, one tone above the panel's, so a row of cards reads as cards rather than as one surface.
    frame.fill_rect(x, y, CARD_W, CARD_H, palette::HUD_BAR);
    let title: String = row.name.to_uppercase().chars().take(16).collect();
    frame.draw_text(x + 4, y + 3, &title, 1, palette::HUD_TEXT);
    // The portrait: a framed square, empty until the renderer has art. A **placeholder that says so** is honest; a made-up
    // silhouette would read as content the pack does not have.
    let (px, py) = (x + 4, y + 13);
    frame.fill_rect(px, py, PORTRAIT, PORTRAIT, palette::PANEL);
    frame.fill_rect(px, py, PORTRAIT, 1, palette::MARK);
    frame.fill_rect(px, py + PORTRAIT - 1, PORTRAIT, 1, palette::MARK);
    frame.fill_rect(px, py, 1, PORTRAIT, palette::MARK);
    frame.fill_rect(px + PORTRAIT - 1, py, 1, PORTRAIT, palette::MARK);
    // **A husk is its building with a cross over it** (the owner, 2026-11-05). The pictogram is still the farm's - the husk
    // unit has no art yet - and the cross is what says *this one is spent*: two diagonals inside the frame, drawn a pixel at a
    // time because there is no line primitive and a diagonal is not a rectangle.
    if row.is_husk {
        for step in 1..PORTRAIT - 1 {
            frame.fill_rect(px + step, py + step, 1, 1, palette::REFUSED);
            frame.fill_rect(px + PORTRAIT - 1 - step, py + step, 1, 1, palette::REFUSED);
        }
    }
    // The small health bar under the portrait, and `n/m` beneath it - the bar the owner kept, and the wide one that went.
    let (bx, by) = (px, py + PORTRAIT + 3);
    frame.fill_rect(bx, by, PORTRAIT, 4, palette::HP_TRACK);
    let lit = (PORTRAIT as f32 * row.hp_fraction.clamp(0.0, 1.0)) as i32;
    let colour = if row.hp_fraction > 0.5 {
        palette::HP_GREEN
    } else {
        palette::HP_RED
    };
    frame.fill_rect(bx, by, lit, 4, colour);
    // **What the building is making** (D157), under the health bar it is not: a second bar, in its own colour, with the count
    // waiting beside it. Nothing is drawn for a unit or an empty building, so the card keeps its shape.
    if row.production.running.is_some() || !row.production.waiting.is_empty() {
        production_bar(frame, (px, by + 17), PORTRAIT as u32, &row.production);
    }
    let max = row.stats.map(|stats| stats.max_hp).unwrap_or(0);
    let hp = if max > 0 {
        format!("{}/{}", row.hp, max)
    } else {
        format!("HP {}", row.hp)
    };
    frame.draw_text(bx, by + 6, &hp, 1, palette::HUD_TEXT);

    // The numbers, beside the portrait.
    let (tx, mut ty) = (px + PORTRAIT + 6, y + 13);
    let line_gap = crate::font::HEIGHT + 2;
    let mut line = |frame: &mut crate::Frame, text: &str| {
        frame.draw_text(tx, ty, text, 1, palette::HUD_TEXT);
        ty += line_gap;
    };
    if let Some(stats) = row.stats {
        line(frame, &format!("ATK {}", card_number(stats.attack)));
        line(
            frame,
            &format!(
                "ARM {}/{}",
                card_number(stats.armour_melee),
                card_number(stats.armour_pierce)
            ),
        );
        if stats.range > 0 {
            line(frame, &format!("RNG {}", card_number(stats.range)));
        } else if stats.work_rate > 0 && stats.carry > 0 {
            line(frame, &format!("WRK {}", card_number(stats.work_rate)));
        } else {
            line(frame, &format!("SPD {}", card_number(stats.speed)));
        }
    }
    // A load in hand, in the pack's word for its column; then what a node still holds.
    if row.carry > 0 {
        let capacity = row.stats.map(|stats| stats.carry).unwrap_or(0);
        let label = match row.resource.as_deref() {
            Some(resource) if capacity > 0 => {
                format!("{} {}/{}", resource.to_uppercase(), row.carry, capacity)
            }
            Some(resource) => format!("{} {}", resource.to_uppercase(), row.carry),
            None => format!("LOAD {}", row.carry),
        };
        frame.draw_text(tx, ty, &label, 1, palette::HUD_TEXT);
    } else if let (Some(resource), true) = (row.resource.as_deref(), row.amount > 0) {
        frame.draw_text(
            tx,
            ty,
            &format!("{} {}", resource.to_uppercase(), row.amount),
            1,
            palette::HUD_TEXT,
        );
    }
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

/// **The minimap, drawn as the map is: a diamond.** Each tile is the small diamond the world's own projection gives it, the
/// fog colours it, and the camera's view is a slanted parallelogram outlined in white with an empty inside - the owner's
/// "rectangle (empty fill, white margin)", whose size is the proportion of the map the camera covers (2026-11-04).
fn draw_minimap(
    frame: &mut Frame,
    snapshot: &Snapshot,
    rect: (i32, i32, u32, u32),
    view: Option<[(f32, f32); 4]>,
) {
    let (mx, my, mw, mh) = rect;
    if mw == 0 || mh == 0 {
        return;
    }
    frame.fill_rect(mx, my, mw as i32, mh as i32, palette::PANEL);
    // **A tile's diamond, drawn a little larger than its tile.** The minimap used to paint exactly `step` pixels per tile, and
    // the rounding of neighbouring tile centres left one-pixel gaps - which ran along the tile grid's two diagonals and read as
    // a pair of crossing lines (the owner, 2026-11-05). A half-pixel of overlap on each axis closes the gap without changing
    // which tile is drawn where: neighbours now meet, and the map's outer edge grows by at most one pixel, under the border the
    // box draws afterwards.
    //
    // **The overlap, not a per-pixel fill.** Asking every pixel which tile it belongs to also removes the seam (and the test
    // checks either way), but it measured **twice** the HUD cost - 4.4 ms against 2.0 ms in a band of 960x720 - because
    // `minimap_tile` does its own float inverse per pixel. Overlapping diamonds do the same job with the same number of draws.
    // A tile's diamond: one step across is `sx` pixels. **Rounded up to whole pixels**, because a fractional half-width leaves a
    // one-pixel gap wherever the rounding of two neighbouring centres goes apart - and those gaps ran along the grid's two
    // diagonals, which is the pair of crossing lines the owner saw (2026-11-05). One pixel more per tile closes it; half a
    // pixel *less* than a whole pixel would not, and a larger overlap measured dearer for no extra coverage.
    let step = {
        let (x0, _) = minimap_point((0, 0), rect, snapshot.width, snapshot.height);
        let (x1, _) = minimap_point((1, 0), rect, snapshot.width, snapshot.height);
        let across = (x1 - x0).abs().max(1) as f32;
        // `rx` was always the full step and `ry` half of it - a diamond deliberately drawn larger than its tile so neighbours
        // overlap. The **floor of 1.5 on the height** is the seam fix: on a small minimap `across / 2` falls below one pixel,
        // `fill_diamond` then paints a single row per tile, and the missing rows are the gaps that read as crossing lines.
        (across.max(1.5), (across / 2.0).max(1.5))
    };
    for y in 0..snapshot.height {
        for x in 0..snapshot.width {
            let forest = snapshot
                .terrain
                .get(usize::from(y) * usize::from(snapshot.width) + usize::from(x))
                .copied()
                .unwrap_or(false);
            let colour = match snapshot.tile_vision(x, y) {
                ruage_sim::TileVision::Unseen => [10, 10, 14, 255],
                ruage_sim::TileVision::Explored => [34, 40, 34, 255],
                ruage_sim::TileVision::Visible if forest => [26, 54, 26, 255],
                ruage_sim::TileVision::Visible => [96, 128, 60, 255],
            };
            let point = minimap_point((x, y), rect, snapshot.width, snapshot.height);
            frame.fill_diamond(point, step, colour);
        }
    }
    // The edge of the box: unexplored ground is nearly black, so without it the minimap is a hole in the panel.
    frame.fill_rect(mx - 1, my - 1, mw as i32 + 2, 1, palette::MARK);
    frame.fill_rect(mx - 1, my + mh as i32, mw as i32 + 2, 1, palette::MARK);
    frame.fill_rect(mx - 1, my, 1, mh as i32, palette::MARK);
    frame.fill_rect(mx + mw as i32, my, 1, mh as i32, palette::MARK);
    // **Where the camera is looking.** A rectangle of the world on a diamond minimap is a parallelogram, so it is drawn as
    // four lines rather than a box; and its inside is left empty, because the ground underneath is what a player is reading.
    if let Some(corners) = view {
        // **Projected as it is, then clipped to the box.** Clamping the corners to the map bent the parallelogram into a
        // shape it is not (the owner saw it "reach over the edge of the minimap and it should keep its shape", 2026-11-04);
        // a camera half off the map draws half a box, cut at the frame's edge like everything else.
        let corners =
            corners.map(|corner| projected(corner, rect, snapshot.width, snapshot.height));
        let boxed = (mx, my, mw as i32, mh as i32);
        let inside = |point: (i32, i32)| {
            point.0 >= boxed.0
                && point.1 >= boxed.1
                && point.0 < boxed.0 + boxed.2
                && point.1 < boxed.1 + boxed.3
        };
        for index in 0..4 {
            let (from, to) = (corners[index], corners[(index + 1) % 4]);
            // A pixel outside the minimap's box is not the minimap's to paint: the panel, the border and the frame edge stay
            // as they are, and what is left of the box is still the box's own shape.
            let (clipped_from, clipped_to) = (from, to);
            if !inside(clipped_from) && !inside(clipped_to) {
                continue;
            }
            line_clipped(
                frame,
                clipped_from,
                clipped_to,
                [255, 255, 255, 255],
                &inside,
            );
            // The "white margin": a second, greyer line one pixel out, so the box reads as a frame over the ground rather
            // than as a line that happens to be white.
            line_clipped(
                frame,
                (from.0, from.1 + 1),
                (to.0, to.1 + 1),
                [150, 150, 150, 255],
                &inside,
            );
        }
    }
}

/// The same line, painted only where `inside` says: the minimap's view box is a shape over a picture, and a shape that runs
/// past its own frame is a shape that lies about the frame.
fn line_clipped(
    frame: &mut Frame,
    from: (i32, i32),
    to: (i32, i32),
    colour: [u8; 4],
    inside: &dyn Fn((i32, i32)) -> bool,
) {
    let (mut x, mut y) = from;
    let (dx, dy) = ((to.0 - from.0).abs(), -(to.1 - from.1).abs());
    let (sx, sy) = (
        if from.0 < to.0 { 1 } else { -1 },
        if from.1 < to.1 { 1 } else { -1 },
    );
    let mut error = dx + dy;
    loop {
        if inside((x, y)) {
            frame.put(x, y, colour);
        }
        if x == to.0 && y == to.1 {
            break;
        }
        let doubled = 2 * error;
        if doubled >= dy {
            error += dy;
            x += sx;
        }
        if doubled <= dx {
            error += dx;
            y += sy;
        }
    }
}

/// **The minimap is the map turned 45 degrees.** A tile maps to the diamond that is the game's own screen: tile (0, 0) is the
/// diamond's top point, tile (w-1, 0) its right, (w-1, h-1) its bottom and (0, h-1) its left - which is the projection the
/// world itself is drawn on, and the reason the owner asked for it ("hud should be also 45 degrees turned. Just like map").
///
/// The horizontal axis is `tx - ty` and the vertical `tx + ty`, exactly as the isometric world uses.
fn projected(tile: (f32, f32), rect: (i32, i32, u32, u32), width: u16, height: u16) -> (i32, i32) {
    let (x, y, w, h) = rect;
    let across = f32::from(width.max(1)).max(1.0) - 1.0;
    let down = f32::from(height.max(1)).max(1.0) - 1.0;
    // **2:1, like the world's own tiles.** A tile on the map is twice as wide as it is tall, and the owner asked for the
    // minimap to be "angled in the same way as map is" (2026-11-04): square diamonds read as a different game seen from the
    // same angle. The diamond spans the box's width and half of it vertically, centred in whatever box it is given.
    let span_w = (w.saturating_sub(1)) as f32;
    let span_h = span_w / 2.0;
    let half_w = span_w / 2.0;
    let sx = if across > 0.0 { half_w / across } else { 0.0 };
    let sy = if down > 0.0 {
        span_h / (2.0 * down)
    } else {
        0.0
    };
    let cx = x as f32 + span_w / 2.0;
    // `v` is `tx + ty`, which runs from 0 to `2 * down`: the top point of the diamond is the top of its own span, and the
    // span is centred in the box vertically.
    let cy = y as f32 + (h as f32 - span_h) / 2.0;
    (
        (cx + (tile.0 - tile.1) * sx).round() as i32,
        (cy + (tile.0 + tile.1) * sy).round() as i32,
    )
}

/// Where a tile stands on the minimap, in pixels.
#[must_use]
pub fn minimap_point(
    tile: (u16, u16),
    rect: (i32, i32, u32, u32),
    width: u16,
    height: u16,
) -> (i32, i32) {
    projected((f32::from(tile.0), f32::from(tile.1)), rect, width, height)
}

/// **The tile under a minimap pixel**, or nothing when the pixel is one of the box's empty corners: the minimap fills a
/// diamond inside a rectangle, and a click on the corner of the rectangle is a click on nothing.
#[must_use]
pub fn minimap_tile(
    pixel: (i32, i32),
    rect: (i32, i32, u32, u32),
    width: u16,
    height: u16,
) -> Option<(u16, u16)> {
    let (x, y, w, h) = rect;
    if w == 0 || h == 0 || width == 0 || height == 0 {
        return None;
    }
    let across = f32::from(width) - 1.0;
    let down = f32::from(height) - 1.0;
    // The same 2:1 geometry `projected` draws with, so a click lands on the tile the picture shows.
    let span_w = (w.saturating_sub(1)) as f32;
    let span_h = span_w / 2.0;
    let half_w = span_w / 2.0;
    let sx = if across > 0.0 { half_w / across } else { 0.0 };
    let sy = if down > 0.0 {
        span_h / (2.0 * down)
    } else {
        0.0
    };
    let cx = x as f32 + span_w / 2.0;
    let cy = y as f32 + (h as f32 - span_h) / 2.0;
    let dx = pixel.0 as f32 - cx;
    let dy = pixel.1 as f32 - cy;
    let (u, v) = (
        if sx > 0.0 { dx / sx } else { 0.0 },
        if sy > 0.0 { dy / sy } else { 0.0 },
    );
    // **The diamond, not the box.** `u` is tx - ty and `v` is tx + ty, so a point is on the map when its distance from the
    // diamond's centre, in both axes together, is within one tile: the box's four corners are empty ground and a click there
    // is a click on nothing.
    if across > 0.0 && down > 0.0 {
        let reach = u.abs() / across + (v - down).abs() / down;
        // A pixel is rounder than a tile here: the diamond is half as tall as it is wide, so one pixel is two tiles' worth
        // of `v` at the edge. The slack keeps a projected corner inside its own diamond.
        if reach > 1.0 + 1.5 / down.max(1.0) {
            return None;
        }
    }
    let tx = (u + v) / 2.0;
    let ty = (v - u) / 2.0;
    // **Rounded, not truncated**: a pixel is a whole number and a tile is barely one, so truncation puts a click one tile
    // short of where the picture drew it (measured: tile (32, 32) drew at (780, 703) and read back as (31, 31)).
    let clamp = |value: f32, most: f32| value.round().clamp(0.0, most) as u16;
    Some((clamp(tx, across), clamp(ty, down)))
}

/// **Where the camera is looking, drawn on the minimap.** The camera sees a rectangle of the world, which on a diamond
/// minimap is a slanted parallelogram - the owner's "rectangle (empty fill, white margin)". Its size is the proportion of the
/// map the camera covers, so zooming in shrinks it.
#[must_use]
pub fn minimap_view(
    camera: &Camera,
    rect: (i32, i32, u32, u32),
    width: u16,
    height: u16,
) -> [(i32, i32); 4] {
    let corners = camera.visible_corners().map(|corner| {
        // Clamped to the map: a camera looking off the edge is still somewhere on the minimap, at the edge.
        (
            corner.0.clamp(0.0, f32::from(width.saturating_sub(1))),
            corner.1.clamp(0.0, f32::from(height.saturating_sub(1))),
        )
    });
    corners.map(|corner| projected(corner, rect, width, height))
}

/// **The minimap's box on screen**, from the frame's size and the map's: the public door to the layout's right zone, so a
/// click handler and the drawing code ask the same question.
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
fn fit_minimap(layout: &Layout, _map_width: u16, _map_height: u16) -> (i32, i32, u32, u32) {
    // **The zone itself.** This used to shrink the picture to "two pixels a tile" and centre the result, which was right
    // when the minimap was a square grid of tiles: a 64-tile map drew 128x128 inside a 160-pixel box. With an isometric
    // diamond the projection already keeps the map's own proportions, so fitting it twice only made it small - the owner
    // asked twice for a bigger minimap and the second request ("fills it vertically and proportionally grows horizontally")
    // is exactly what handing the whole zone to the diamond does.
    layout.minimap
}

/// The middle of the outcome banner: a strip across the play area, in colour because the font has no words (D82).
#[must_use]
pub fn banner_point(frame_width: u32, frame_height: u32) -> (u32, u32) {
    (
        frame_width / 2,
        (frame_height.saturating_sub(hud_panel_height())) / 2,
    )
}

/// **Everything a player watches, in one place.** The top bar is the stock, the population and the speed; the bottom panel
/// is what is selected; the corner is the minimap; a decided match paints a banner across the middle. `Hud::of` builds the
/// numbers and this draws them - the window's job is to call it, not to decide what a HUD is.
pub fn hud(frame: &mut Frame, hud: &Hud, snapshot: &Snapshot, speed: crate::Speed) {
    // **One layout per frame**, used by every part: deriving it again in each helper was 316 us against 151-195.
    let layout = Layout::of(frame.width, frame.height);
    let bar = layout.bar.3.min(frame.height);
    if bar > 0 {
        frame.fill_rect(0, 0, frame.width as i32, bar as i32, palette::HUD_BAR);
        // **Two rows, not one**: the pack's word for the column on top (at one pixel) and the number under it, laid out from
        // the widths of both. With a 7-pixel glyph and a 30-pixel bar there is room for a name and a number and no room for
        // them to share a line, which is what the old layout made them do.
        let text_scale = 2;
        let swatch = 10;
        let (name_y, number_y, swatch_y) = stock_rows(frame.height);
        let (columns, population_x) = stock_columns(hud);
        for (column, (name, amount)) in hud.stock.iter().enumerate() {
            let x = columns.get(column).copied().unwrap_or(8);
            let colour = palette::STOCK[column.min(palette::STOCK.len() - 1)];
            frame.fill_rect(x, swatch_y, swatch, swatch, colour);
            frame.draw_text(x, name_y, name, 1, palette::HUD_TEXT);
            frame.draw_number(
                x + swatch + 6,
                number_y,
                *amount,
                text_scale,
                palette::HUD_TEXT,
            );
        }
        let population = format!("{}/{}", hud.population, hud.population_cap);
        frame.draw_text(
            population_x + 8,
            number_y,
            &population,
            text_scale,
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
        // **A card per selected thing** (owner, 2026-11-04: *"let's add a functionality where unit stats are shown in the
        // bottom central part. the big long health bar can be removed."*). The wide bar that ran the width of the zone is
        // gone; what a player reads is the game's own card - name, portrait, the small bar under it with `n/m`, and the
        // numbers beside it. Several units give several cards in a row, which is how a box selection reads.
        let cards = selection_cards_that_fit(layout.selected);
        for (index, row) in hud.selected.iter().take(cards).enumerate() {
            draw_selection_card(frame, layout.selected, index, row);
        }
    }

    // The speed selector sits under the minimap, where pacing belongs (D101): the bar's five slots are the game's.
    speed_selector_in(frame, &layout, speed);

    let rect = fit_minimap(&layout, snapshot.width, snapshot.height);
    if rect.2 > 0 && rect.3 > 0 && rect.0 >= 0 && rect.1 >= 0 {
        draw_minimap(frame, snapshot, rect, hud.view);
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
        // **A box with a word in it, not a stripe.** The first version painted an 8-pixel band across the whole screen and
        // no text at all - the owner saw it as "a strange green stripe" (2026-11-04), which is exactly what a full-width
        // colour band with nothing in it is. The font has letters now, so the banner can say which way the match went.
        let (bx, by) = banner_point(frame.width, frame.height);
        let label = match outcome {
            ruage_sim::MatchOutcome::Victory { winner } if winner == snapshot.player => "VICTORY",
            _ => "DEFEAT",
        };
        let scale = 2;
        let text = crate::font::text_width(label, scale);
        let (w, h) = (text + 36, crate::font::HEIGHT * scale + 14);
        let x = bx as i32 - w / 2;
        let y = by as i32 - h / 2;
        frame.fill_rect(x, y, w, h, colour);
        frame.draw_text(
            bx as i32 - text / 2,
            by as i32 - crate::font::HEIGHT * scale / 2,
            label,
            scale,
            palette::HUD_TEXT,
        );
    }
}
