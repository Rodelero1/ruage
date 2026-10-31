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
}

impl Hud {
    pub fn of(snapshot: &Snapshot, resources: &Resources, selection: &Selection) -> Hud {
        let stock = (0..snapshot.stock.len())
            .map(|slot| {
                let name = resources.name(slot).unwrap_or("?").to_owned();
                (name, snapshot.stock[slot])
            })
            .collect();
        let selected = selection
            .ids
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
    26
}

/// Where a stock column's swatch starts. **The pack's order is the display order**, so a pack with three resources
/// shows three swatches and the view never learns their names.
pub fn hud_swatch_x(column: usize) -> u32 {
    8 + column as u32 * 60
}

/// Draw the HUD over a frame: one swatch and one number per stockpile column, then population as `have/cap`.
///
/// It reads a `Snapshot` and nothing else (D55): the simulation carries no text, and every number here is one the
/// engine already keeps. D74 allowed this; D82 chose the bitmap font it is drawn with.
pub fn hud(frame: &mut Frame, snapshot: &Snapshot) {
    let bar = hud_bar_height().min(frame.height);
    if bar == 0 {
        return;
    }
    frame.fill_rect(0, 0, frame.width as i32, bar as i32, palette::HUD_BAR);
    let scale = 3;
    let text_y = (bar as i32 - crate::font::HEIGHT * scale) / 2;
    let swatch = 12;
    for (column, amount) in snapshot.stock.iter().enumerate() {
        let x = hud_swatch_x(column) as i32;
        let colour = palette::STOCK[column.min(palette::STOCK.len() - 1)];
        frame.fill_rect(x, (bar as i32 - swatch) / 2, swatch, swatch, colour);
        frame.draw_number(x + swatch + 6, text_y, *amount, scale, palette::HUD_TEXT);
    }
    let x = hud_swatch_x(snapshot.stock.len()) as i32;
    frame.draw_ratio(
        x,
        text_y,
        snapshot.population,
        snapshot.population_cap,
        scale,
        palette::HUD_TEXT,
    );
}
