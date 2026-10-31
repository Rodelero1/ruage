//! The presentation boundary: a read-only snapshot of a match, a camera, and a rasteriser.
//!
//! The renderer never touches `World`. It reads a [`Snapshot`], which is a plain description of what a
//! player can see - id, content id, position, health fraction, what it is doing - and a [`Camera`], which
//! is a transform from tiles to pixels. That is the whole reason this crate exists: the simulation cannot
//! learn what a screen is, and an asset pack later replaces the colours with sprites by changing one
//! function, not the game (D17, D55).
//!
//! Floats are allowed here and nowhere near the simulation (D9): pixels are not state.

pub mod font;
pub mod ui;

use ruage_data::Map;
use ruage_fixed::Fixed;
use ruage_sim::{EntityKind, MatchOutcome, Task, TileVision, World};

/// What a unit is doing, as far as a renderer cares: which animation to play, not which node it is
/// walking to. Derived from the task, never stored (D55).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pose {
    Idle,
    Walk,
    Work,
    Attack,
    Die,
    Garrisoned,
    Fly,
}

impl Pose {
    fn of(task: &Task) -> Pose {
        match task {
            Task::Idle => Pose::Idle,
            Task::Move { .. } | Task::Hunt { .. } | Task::Return { .. } => Pose::Walk,
            Task::Gather { .. } | Task::Build { .. } => Pose::Work,
            Task::Attack { .. } => Pose::Attack,
            Task::Garrisoned { .. } => Pose::Garrisoned,
            Task::Fly { .. } => Pose::Fly,
        }
    }
}

/// One entity, as the screen sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct EntityView {
    pub id: u32,
    pub player: u8,
    pub kind: EntityKind,
    /// The content id this entity is an instance of, when it has one.
    pub unit: Option<u32>,
    /// Position in tiles, continuous: a unit between two tiles is drawn between them.
    pub position: (f32, f32),
    /// 1.0 for a finished thing, a fraction for one being built, zero for a node.
    pub hp_fraction: f32,
    pub pose: Pose,
    /// What a worker is carrying, in whole resources.
    pub carry: u32,
    /// How far along a construction is, 0..=1; `None` for anything not being built.
    pub construction: Option<f32>,
    /// How many units are inside, for a building that holds any.
    pub garrison: u32,
    /// What is left in a resource node, in whole resources.
    pub amount: u32,
    /// Units a building is making, the one in progress included; zero for everything else.
    pub pending: usize,
    /// Whether this entity is **remembered** rather than watched: its ground is explored, nothing of the player's can
    /// see it now, and it is still in the picture because a building the player has seen does not disappear. Units are
    /// never remembered - an army does not leave a photograph behind - and the renderer dims what is remembered, because
    /// a memory drawn at full brightness reads as a live building.
    pub remembered: bool,
    /// The tiles this entity occupies, read from the pack's own radii (D76): a town centre is 4x4, a house 2x2,
    /// a villager one. The renderer draws this silhouette, so the screen shows the space the simulation
    /// reserves rather than a square of its own invention.
    pub footprint: (u32, u32),
}

/// Everything a renderer needs, and nothing it could use to change the match.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub tick: u64,
    pub player: u8,
    pub width: u16,
    pub height: u16,
    pub terrain: Vec<bool>,
    /// The player's stockpile, by column, in the pack's order.
    pub stock: Vec<u32>,
    pub population: u32,
    pub population_cap: u32,
    pub outcome: Option<MatchOutcome>,
    /// Fog: one bit per tile, row-major, of what the player sees *now*.
    pub visible: Vec<u64>,
    /// Fog: one bit per tile, row-major, of what the player has ever seen.
    pub explored: Vec<u64>,
    pub entities: Vec<EntityView>,
}

impl Snapshot {
    /// Take a read-only picture of a match for one player.
    pub fn of(world: &World, map: &Map, player: u8) -> Snapshot {
        let scale = world.scale();
        let to_tiles = |value: Fixed| value.raw() as f32 / scale as f32;
        let width = map.width;
        let height = map.height;

        // How many units each building holds, counted once: the per-entity scan this replaces was O(n^2) on the
        // frame path (126 entities is 16k comparisons a frame, and a HUD drawing it twice a second makes an army's
        // worth of them). Ordered, because nothing in this project walks a hash map (determinism rule 3).
        let mut inside: std::collections::BTreeMap<u32, u32> = std::collections::BTreeMap::new();
        for other in world.entities() {
            if let Task::Garrisoned { building } = other.task {
                *inside.entry(building.0).or_default() += 1;
            }
        }

        // One grid for the whole snapshot: the simulation's own record of what this player has ever seen, and the same
        // bits `explored` below is built from. Asking `vision` per entity would scan every tile per entity, and asking
        // it per tile - which this used to do - scans every entity per tile.
        let remembered_ground = world.explored_mask(player);

        let mut entities = Vec::new();
        for entity in world.entities() {
            // A player is not shown what they cannot see: that is the only filter here, and it is the simulation's
            // answer, not the renderer's guess. A **building or a resource node** the player has seen before stays in
            // view as a memory once nothing watches it - the ground under it is still explored - while a unit does not:
            // an army that walks out of sight is gone, which is the whole point of scouting.
            let seen = world.visible_to(player, entity.id);
            let remembered = !seen
                && matches!(entity.kind, EntityKind::Building | EntityKind::Resource(_))
                && explored_at(&remembered_ground, width, scale, entity.position);
            if !seen && !remembered {
                continue;
            }
            let garrison = inside.get(&entity.id.0).copied().unwrap_or(0);
            let max_hp = match entity.unit {
                Some(unit) => world.max_hp_of(player, unit).unwrap_or(entity.hp),
                None => entity.hp,
            };
            let construction = entity
                .construction
                .map(|c| (c.progress_ticks as f32 / c.total_ticks.max(1) as f32).clamp(0.0, 1.0));
            entities.push(EntityView {
                id: entity.id.0,
                player: entity.player,
                kind: entity.kind,
                unit: entity.unit,
                position: (to_tiles(entity.position.0), to_tiles(entity.position.1)),
                hp_fraction: if max_hp > 0 {
                    (entity.hp as f32 / max_hp as f32).clamp(0.0, 1.0)
                } else {
                    0.0
                },
                pose: Pose::of(&entity.task),
                carry: (entity.carry.raw() / scale) as u32,
                construction,
                garrison,
                amount: (entity.amount.raw() / scale).max(0) as u32,
                pending: world.pending_orders(entity.id),
                footprint: entity
                    .unit
                    .and_then(|unit| world.footprint_of(unit))
                    .unwrap_or((1, 1)),
                remembered,
            });
        }

        // One entry per column **the pack has**, not the engine's ceiling of eight: the HUD draws a swatch per entry, and
        // four swatches for resources this content never heard of is a lie about the game.
        let columns = world.resources().map_or(0, ruage_data::Resources::count);
        let stock = (0..columns).map(|slot| world.stock(player, slot)).collect();
        let explored = world.explored_mask(player);

        Snapshot {
            tick: world.tick(),
            player,
            width,
            height,
            terrain: map
                .terrain
                .iter()
                .map(|tile| *tile == ruage_data::Terrain::Forest)
                .collect(),
            stock,
            population: world.population(player),
            population_cap: world.population_cap(player),
            outcome: world.outcome(),
            visible: world.visible_mask(player),
            explored,
            entities,
        }
    }

    /// Whether a tile is visible now, remembered, or unknown to this player.
    pub fn tile_vision(&self, x: u16, y: u16) -> TileVision {
        if x >= self.width || y >= self.height {
            return TileVision::Unseen;
        }
        let bit = usize::from(y) * usize::from(self.width) + usize::from(x);
        let set = |grid: &Vec<u64>| {
            grid.get(bit / 64)
                .is_some_and(|word| word & (1u64 << (bit % 64)) != 0)
        };
        if set(&self.visible) {
            TileVision::Visible
        } else if set(&self.explored) {
            TileVision::Explored
        } else {
            TileVision::Unseen
        }
    }
}

/// Tiles to pixels. Flat ground only: this content has no terrain heights, so there is nothing for a
/// projection to shear (D55).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    /// The top-left corner of the view, in tiles.
    pub origin: (f32, f32),
    /// Pixels per tile, before zoom.
    pub tile_pixels: f32,
    pub zoom: f32,
    pub viewport: (u32, u32),
}

impl Camera {
    /// A camera on the game's own projection: **2:1 isometric**, because that is what the original looks like. A tile is
    /// `tile_pixels` wide and half that high, and tile (1, 0) is down-right from tile (0, 0) while tile (0, 1) is
    /// down-left - the diamond grid of every Age of Empires screen.
    pub fn new(viewport: (u32, u32)) -> Camera {
        Camera {
            origin: (0.0, 0.0),
            tile_pixels: 96.0,
            zoom: 1.0,
            viewport,
        }
    }

    /// Centre the view on a tile. Done in *tile* space, so it holds at every zoom.
    pub fn centred_on(mut self, tile: (f32, f32)) -> Camera {
        self.origin = (tile.0, tile.1);
        self
    }

    fn scale(&self) -> f32 {
        self.tile_pixels * self.zoom
    }

    /// Half a tile's width and height in pixels: the diamond's radii.
    fn radii(&self) -> (f32, f32) {
        (self.scale() / 2.0, self.scale() / 4.0)
    }

    /// The pixel at the **centre of a tile's diamond**. Off-screen tiles land off-screen, which is the renderer's problem
    /// to clip, not the camera's to hide.
    ///
    /// Note the `+ ry`: the diamond whose *centre* is tile `t` is drawn half a tile below the corner that `(x - y, x + y)`
    /// points at, and `pick` subtracts exactly that offset. Getting this pair out of step is what makes clicks land one
    /// tile off, which is why they are tested together (`crates/view/tests/camera.rs`).
    pub fn project(&self, tile: (f32, f32)) -> (i32, i32) {
        let (rx, ry) = self.radii();
        let (dx, dy) = (tile.0 - self.origin.0, tile.1 - self.origin.1);
        let (w, h) = self.viewport;
        (
            ((w as f32 / 2.0) + (dx - dy) * rx).round() as i32,
            ((h as f32 / 2.0) + (dx + dy) * ry + ry).round() as i32,
        )
    }

    /// The tile under a pixel: the exact inverse of `project`, so a click lands on the diamond the player sees.
    pub fn pick(&self, pixel: (i32, i32)) -> (i32, i32) {
        let (rx, ry) = self.radii();
        let (w, h) = self.viewport;
        let sx = (pixel.0 as f32 - w as f32 / 2.0) / rx;
        let sy = (pixel.1 as f32 - h as f32 / 2.0 - ry) / ry;
        // x - y = sx and x + y = sy.
        let x = (sx + sy) / 2.0 + self.origin.0;
        let y = (sy - sx) / 2.0 + self.origin.1;
        // **Round, not floor.** `project` puts a tile at the *centre* of its diamond, so a diamond covers the
        // continuous range `tile - 0.5 .. tile + 0.5`; flooring is right only for a projection that puts a tile at
        // its top-left corner, and it sent every click in a diamond's upper-left half one tile off. Every centre
        // round-trips either way, which is why this hid behind a centre-only test
        // (`crates/view/tests/camera.rs::a_click_just_inside_a_diamond_picks_that_tile`).
        (x.round() as i32, y.round() as i32)
    }

    pub fn scroll(&mut self, tiles: (f32, f32)) {
        self.origin.0 += tiles.0;
        self.origin.1 += tiles.1;
    }

    /// The window changed size. **Only the viewport changes**: the camera's position is its own, so a resize must not
    /// re-centre it - rebuilding a `Camera` on every draw was the defect that made panning and zooming impossible while
    /// looking perfectly innocent.
    pub fn resized(&mut self, viewport: (u32, u32)) {
        self.viewport = viewport;
    }

    /// The zoom levels the game has, furthest first. A wheel notch is a notch: multiplying by a factor accumulates
    /// rounding and leaves the picture at 0.83x, which is not a zoom level anybody asked for.
    pub const ZOOM_LEVELS: [f32; 3] = [0.5, 1.0, 2.0];

    fn step_zoom(&mut self, towards: f32) {
        let current = self.zoom;
        let next = Self::ZOOM_LEVELS
            .iter()
            .copied()
            .filter(|level| {
                if towards > current {
                    *level > current + f32::EPSILON
                } else {
                    *level < current - f32::EPSILON
                }
            })
            .fold(None, |best: Option<f32>, level| {
                Some(best.map_or(level, |best| {
                    if (level - current).abs() < (best - current).abs() {
                        level
                    } else {
                        best
                    }
                }))
            });
        if let Some(next) = next {
            self.zoom = next;
        }
    }

    /// One notch closer, staying at the closest level at the end of the range.
    pub fn zoom_in(&mut self) {
        self.step_zoom(f32::INFINITY);
    }

    /// One notch further out, staying at the furthest level.
    pub fn zoom_out(&mut self) {
        self.step_zoom(f32::NEG_INFINITY);
    }

    /// A free zoom factor, for a probe or an animation. The player's control uses the levels above.
    pub fn zoom_by(&mut self, factor: f32) {
        self.zoom = (self.zoom * factor).clamp(0.25, 4.0);
    }
}

/// A rendered frame: RGBA, one row after another.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Frame {
    pub fn new(width: u32, height: u32) -> Frame {
        Frame {
            width,
            height,
            pixels: vec![0; (width as usize) * (height as usize) * 4],
        }
    }

    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let at = ((y as usize) * (self.width as usize) + (x as usize)) * 4;
        [
            self.pixels[at],
            self.pixels[at + 1],
            self.pixels[at + 2],
            self.pixels[at + 3],
        ]
    }

    pub fn put(&mut self, x: i32, y: i32, colour: [u8; 4]) {
        if x < 0 || y < 0 || x as u32 >= self.width || y as u32 >= self.height {
            return;
        }
        let at = ((y as usize) * (self.width as usize) + (x as usize)) * 4;
        self.pixels[at..at + 4].copy_from_slice(&colour);
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, colour: [u8; 4]) {
        for dy in 0..h {
            for dx in 0..w {
                self.put(x + dx, y + dy, colour);
            }
        }
    }

    /// Fill the **diamond** of the given radii about `centre`, one scanline at a time.
    ///
    /// This is the shape the game's grid is made of: at 96-pixel tiles a diamond is 96 wide and 48 tall, and
    /// neighbouring diamonds meet edge to edge, so a field of them covers the screen with no seams and no
    /// overlaps. The old renderer filled `step x step` squares at each tile's point, which under the isometric
    /// projection paints overlapping blocks - one tile drawn over the corner of four others.
    ///
    /// Every `put` clips, so a diamond that runs off the frame is cheap rather than wrong.
    pub fn fill_diamond(&mut self, centre: (i32, i32), radii: (f32, f32), colour: [u8; 4]) {
        let (rx, ry) = radii;
        if rx <= 0.0 || ry <= 0.0 || self.width == 0 || self.height == 0 {
            return;
        }
        // Clip to the frame up front, in both directions. A diamond is mostly off-screen more often than not - every
        // tile at the edge of a 960x720 view is - and a `put` per off-screen pixel cost as much as a visible one:
        // measured at 2,568 us a frame against 634 us for the square it replaced, which is where this clipping came
        // from (`crates/view/tests/frame_cost.rs`).
        let top = (centre.1 - ry.ceil() as i32).max(0);
        let bottom = (centre.1 + ry.ceil() as i32).min(self.height as i32 - 1);
        if top > bottom {
            return;
        }
        for y in top..=bottom {
            let dy = (y - centre.1) as f32;
            let across = 1.0 - (dy.abs() / ry);
            if across <= 0.0 {
                continue;
            }
            let half = (rx * across).round() as i32;
            let left = (centre.0 - half).max(0);
            let right = (centre.0 + half).min(self.width as i32 - 1);
            if left > right {
                continue;
            }
            let row = y as usize * self.width as usize;
            let from = (row + left as usize) * 4;
            let to = (row + right as usize) * 4 + 4;
            for pixel in self.pixels[from..to].as_chunks_mut::<4>().0 {
                *pixel = colour;
            }
        }
    }

    /// Draw the **outline** of a diamond: two pixels per scanline, so a marker never hides what it marks.
    pub fn outline_diamond(&mut self, centre: (i32, i32), radii: (f32, f32), colour: [u8; 4]) {
        let (rx, ry) = radii;
        if rx <= 0.0 || ry <= 0.0 {
            return;
        }
        let top = centre.1 - ry.ceil() as i32;
        let bottom = centre.1 + ry.ceil() as i32;
        for y in top..=bottom {
            let dy = (y - centre.1) as f32;
            let across = 1.0 - (dy.abs() / ry);
            if across <= 0.0 {
                continue;
            }
            let half = (rx * across).round() as i32;
            self.put(centre.0 - half, y, colour);
            self.put(centre.0 + half, y, colour);
        }
    }

    /// Draw one glyph of the bitmap font: `rows` are its scanlines, `scale` multiplies each font pixel.
    ///
    /// Returns the width used, so a caller can place the next thing without keeping the font's metrics in its head.
    pub fn draw_glyph(
        &mut self,
        x: i32,
        y: i32,
        rows: &[u8; 5],
        scale: i32,
        colour: [u8; 4],
    ) -> i32 {
        let scale = scale.max(1);
        for (row, bits) in rows.iter().enumerate() {
            for column in 0..font::WIDTH {
                if bits & (1 << (font::WIDTH - 1 - column)) == 0 {
                    continue;
                }
                self.fill_rect(
                    x + column * scale,
                    y + row as i32 * scale,
                    scale,
                    scale,
                    colour,
                );
            }
        }
        font::WIDTH * scale
    }

    /// Draw a whole number, left to right. Zero prints one digit, because a blank field reads as a bug.
    pub fn draw_number(&mut self, x: i32, y: i32, value: u32, scale: i32, colour: [u8; 4]) -> i32 {
        let scale = scale.max(1);
        let mut digits = Vec::new();
        let mut rest = value;
        loop {
            digits.push(font::DIGITS[(rest % 10) as usize]);
            rest /= 10;
            if rest == 0 {
                break;
            }
        }
        let mut at = x;
        for glyph in digits.iter().rev() {
            self.draw_glyph(at, y, glyph, scale, colour);
            at += font::WIDTH * scale + font::GAP * scale;
        }
        at - x - font::GAP * scale
    }

    /// Draw "have/cap", which is how a player reads population.
    pub fn draw_ratio(
        &mut self,
        x: i32,
        y: i32,
        have: u32,
        cap: u32,
        scale: i32,
        colour: [u8; 4],
    ) -> i32 {
        let scale = scale.max(1);
        let mut at = x + self.draw_number(x, y, have, scale, colour) + font::GAP * scale;
        at += self.draw_glyph(at, y, &font::SLASH, scale, colour) + font::GAP * scale;
        at - x + self.draw_number(at, y, cap, scale, colour)
    }

    /// A binary PPM (P6): no compression, no dependency, and every image tool reads it. PNG is a
    /// conversion away and that conversion is not the renderer's business (D55).
    pub fn to_ppm(&self) -> Vec<u8> {
        let mut out = format!("P6\n{} {}\n255\n", self.width, self.height).into_bytes();
        out.reserve((self.width as usize) * (self.height as usize) * 3);
        for pixel in self.pixels.as_chunks::<4>().0 {
            out.extend_from_slice(&pixel[..3]);
        }
        out
    }
}

/// The colours. These are the *view's* own: an asset pack replaces the sprite lookup and these become
/// its fallback, which is why they live here and not in the content pack (D55). Public because a test that
/// asks "was a building drawn?" needs the colour it is drawn in, and a palette a test cannot name is a
/// palette whose meaning nobody can assert.
pub mod palette {
    pub const GRASS: [u8; 4] = [96, 128, 64, 255];
    pub const GRASS_LIGHT: [u8; 4] = [110, 144, 72, 255];
    pub const GRASS_DARK: [u8; 4] = [84, 116, 56, 255];
    /// The tones a grass tile may take. One per tile, chosen by a hash of the tile, so the ground has texture without
    /// storing any: it is a function of position, not of state, and two renders of one snapshot stay byte-identical.
    pub const GRASS_TONES: [[u8; 4]; 3] = [GRASS, GRASS_LIGHT, GRASS_DARK];
    pub const FOREST: [u8; 4] = [32, 64, 32, 255];
    pub const UNSEEN: [u8; 4] = [8, 8, 12, 255];
    pub const EXPLORED: [u8; 4] = [64, 72, 64, 255];
    pub const PLAYER_ONE: [u8; 4] = [48, 96, 220, 255];
    pub const PLAYER_TWO: [u8; 4] = [200, 64, 48, 255];
    pub const NEUTRAL: [u8; 4] = [120, 120, 120, 255];
    /// A resource node on open ground: a pile of something the map placed.
    pub const PILE: [u8; 4] = [180, 150, 90, 255];
    /// A tree's canopy, and the darker clump that gives it some shape.
    pub const CANOPY: [u8; 4] = [56, 104, 48, 255];
    pub const CANOPY_SHADE: [u8; 4] = [40, 80, 36, 255];
    /// A tree that has been worked out: the entity stays, the canopy does not.
    pub const STUMP: [u8; 4] = [92, 72, 44, 255];
    pub const BUILDING: [u8; 4] = [200, 200, 200, 255];
    /// The HUD: a dark bar, light text, and one swatch per stockpile column in the pack's own order.
    pub const HUD_BAR: [u8; 4] = [16, 16, 20, 255];
    pub const HUD_TEXT: [u8; 4] = [235, 235, 225, 255];
    /// Selection: a white ring under what is selected, a health bar over it, and where the last order went.
    pub const SELECTION: [u8; 4] = [255, 255, 255, 255];
    pub const HP_GREEN: [u8; 4] = [80, 200, 80, 255];
    pub const HP_RED: [u8; 4] = [200, 60, 60, 255];
    pub const ORDER: [u8; 4] = [240, 220, 90, 255];
    pub const STOCK: [[u8; 4]; 4] = [
        [150, 110, 70, 255],
        [190, 70, 60, 255],
        [220, 190, 70, 255],
        [150, 150, 155, 255],
    ];
}

/// The grass tone of a tile: a hash of its coordinates, folded into the palette. No state, no RNG stream, and the same
/// answer on every machine - which is what lets a frame be compared byte for byte.
fn grass_tone(x: u16, y: u16) -> [u8; 4] {
    let mixed =
        (u32::from(x).wrapping_mul(73_856_093) ^ u32::from(y).wrapping_mul(19_349_663)) >> 7;
    palette::GRASS_TONES[(mixed % palette::GRASS_TONES.len() as u32) as usize]
}

/// Whether the tile an entity stands on is one the player has ever seen. The simulation's explored grid is the answer;
/// a view that kept its own memory of the map would be a second, silently disagreeing map.
fn explored_at(mask: &[u64], width: u16, scale: i64, position: (Fixed, Fixed)) -> bool {
    let scale = scale.max(1);
    let (x, y) = (
        position.0.raw().div_euclid(scale),
        position.1.raw().div_euclid(scale),
    );
    if x < 0 || y < 0 || x > i64::from(u16::MAX) || y > i64::from(u16::MAX) {
        return false;
    }
    let bit = y as usize * usize::from(width) + x as usize;
    mask.get(bit / 64)
        .is_some_and(|word| word & (1u64 << (bit % 64)) != 0)
}

/// A remembered thing is drawn at 60% brightness: dim enough to read as memory against the dim ground it stands on, and
/// bright enough to see what it was.
fn as_memory(colour: [u8; 4]) -> [u8; 4] {
    // Widen before multiplying: `200 * 3` overflows a `u8`, and in a release build that would wrap to a bright,
    // wrong colour rather than fail.
    let dim = |channel: u8| (u16::from(channel) * 3 / 5) as u8;
    [dim(colour[0]), dim(colour[1]), dim(colour[2]), 255]
}

/// Whether the ground under a position is forest. Used to read a resource node as a tree: a wood is where trees are,
/// and that is the map's answer rather than a table of which stockpile column is timber.
fn on_forest_ground(snapshot: &Snapshot, position: (f32, f32)) -> bool {
    let x = position.0.floor();
    let y = position.1.floor();
    if x < 0.0 || y < 0.0 {
        return false;
    }
    let (x, y) = (x as usize, y as usize);
    if x >= usize::from(snapshot.width) || y >= usize::from(snapshot.height) {
        return false;
    }
    snapshot
        .terrain
        .get(y * usize::from(snapshot.width) + x)
        .copied()
        .unwrap_or(false)
}

/// The tiles a footprint covers, given the tile an entity stands on and the pack's footprint.
///
/// The dat's radius is a **half-extent about the anchor**, so a 4x4 building covers `anchor-2 ..= anchor+1` and a
/// 3x3 one covers `anchor-1 ..= anchor+1` - the anchor is the tile nearest the middle, and an even footprint has no
/// middle tile, which is why the extra tile goes to the +x/+y side. Returns inclusive `(min_x, min_y, max_x, max_y)`
/// in tiles, which is what a footprint is: a rectangle of ground, drawn as the diamond the projection implies and
/// clicked as the rectangle it is.
#[must_use]
pub fn footprint_tiles(anchor: (i32, i32), footprint: (u32, u32)) -> (i32, i32, i32, i32) {
    let half_x = (footprint.0 / 2) as i32;
    let half_y = (footprint.1 / 2) as i32;
    let across = footprint.0.max(1) as i32;
    let down = footprint.1.max(1) as i32;
    (
        anchor.0 - half_x,
        anchor.1 - half_y,
        anchor.0 - half_x + across - 1,
        anchor.1 - half_y + down - 1,
    )
}

/// Whether a tile is inside an entity's footprint. **One rule for one click**: `pick_at` has a `World` and the UI's
/// selection has a `Snapshot`, and when they each had their own rule a click on the body of a town centre selected
/// nothing while the window's picking knew better.
#[must_use]
pub fn footprint_covers(anchor: (i32, i32), footprint: (u32, u32), tile: (i32, i32)) -> bool {
    let (min_x, min_y, max_x, max_y) = footprint_tiles(anchor, footprint);
    tile.0 >= min_x && tile.0 <= max_x && tile.1 >= min_y && tile.1 <= max_y
}

/// The same rule, for a snapshot's own view of an entity.
#[must_use]
pub fn covers(entity: &EntityView, tile: (i32, i32)) -> bool {
    footprint_covers(
        (
            entity.position.0.floor() as i32,
            entity.position.1.floor() as i32,
        ),
        entity.footprint,
        tile,
    )
}

/// How far towards the viewer something stands. A diamond grid has a front - the bottom of the screen - so
/// `x + y` increases towards it, and a thing with a bigger `x + y` covers whatever is behind it.
///
/// A **building** uses the far corner of its footprint rather than its anchor: a 4x4 town centre covers ground up to
/// `anchor + 3`, so a unit standing at `anchor + 1` is behind the building's mass and only `anchor + 4` is in front of
/// it. Using the anchor would paint every unit standing in the building's own footprint over the building.
#[must_use]
pub fn depth_of(entity: &EntityView) -> f32 {
    match entity.kind {
        EntityKind::Building => {
            let anchor = (
                entity.position.0.floor() as i32,
                entity.position.1.floor() as i32,
            );
            let (_, _, max_x, max_y) = footprint_tiles(anchor, entity.footprint);
            (max_x + max_y) as f32
        }
        EntityKind::Resource(_) | EntityKind::Unit | EntityKind::Projectile => {
            entity.position.0 + entity.position.1
        }
    }
}

/// Everything on the ground, **back to front**: depth first, then id, so the order never depends on how the entity
/// list happened to be built. Sorting is not cosmetic on a diamond grid - without it a villager north of a house is
/// painted over the house, and the picture flickers as entities are added and removed.
#[must_use]
pub fn draw_order(snapshot: &Snapshot) -> Vec<&EntityView> {
    let mut order: Vec<&EntityView> = snapshot.entities.iter().collect();
    order.sort_by(|a, b| {
        depth_of(a)
            .total_cmp(&depth_of(b))
            .then_with(|| a.id.cmp(&b.id))
    });
    order
}

pub(crate) fn player_colour(player: u8) -> [u8; 4] {
    match player {
        1 => palette::PLAYER_ONE,
        2 => palette::PLAYER_TWO,
        _ => palette::NEUTRAL,
    }
}

/// Draw a snapshot. Everything on the ground is a **diamond** - the game's own 2:1 isometric grid - a building is
/// a diamond as big as its pack footprint, a unit is a dot in its player's colour, and nothing here knows what a
/// unit *is* beyond its kind, its player and the space it takes up (D55).
pub fn render(snapshot: &Snapshot, camera: &Camera) -> Frame {
    let mut frame = Frame::new(camera.viewport.0, camera.viewport.1);
    let (rx, ry) = camera.radii();
    let (w, h) = (frame.width as i32, frame.height as i32);

    // The tiles the camera can see. The visible region of a diamond grid is itself a diamond, so the range comes
    // from all four corners of the frame (a two-corner box clips the top and bottom of the picture), plus a tile
    // of margin because a tile's diamond sticks out half a tile past its centre.
    let corners = [
        camera.pick((0, 0)),
        camera.pick((w - 1, 0)),
        camera.pick((0, h - 1)),
        camera.pick((w - 1, h - 1)),
    ];
    let min_x = corners.iter().map(|c| c.0).min().unwrap_or(0) - 1;
    let max_x = corners.iter().map(|c| c.0).max().unwrap_or(-1) + 1;
    let min_y = corners.iter().map(|c| c.1).min().unwrap_or(0) - 1;
    let max_y = corners.iter().map(|c| c.1).max().unwrap_or(-1) + 1;

    for y in min_y.max(0)..=max_y.min(i32::from(snapshot.height) - 1) {
        for x in min_x.max(0)..=max_x.min(i32::from(snapshot.width) - 1) {
            let (x, y) = (x as u16, y as u16);
            let forest = snapshot
                .terrain
                .get(usize::from(y) * usize::from(snapshot.width) + usize::from(x))
                .copied()
                .unwrap_or(false);
            let colour = match snapshot.tile_vision(x, y) {
                TileVision::Unseen => palette::UNSEEN,
                TileVision::Explored => palette::EXPLORED,
                TileVision::Visible if forest => palette::FOREST,
                TileVision::Visible => grass_tone(x, y),
            };
            let centre = camera.project((x as f32, y as f32));
            frame.fill_diamond(centre, (rx, ry), colour);
        }
    }

    // One pass, back to front: a diamond grid has an unambiguous front, and sorting by depth is what makes a unit
    // standing in front of a building cover it while one standing behind is covered (`draw_order`).
    for entity in draw_order(snapshot) {
        match entity.kind {
            EntityKind::Projectile => {}
            EntityKind::Resource(_) => {
                let tint = |colour: [u8; 4]| {
                    if entity.remembered {
                        as_memory(colour)
                    } else {
                        colour
                    }
                };
                let centre = camera.project(entity.position);
                if on_forest_ground(snapshot, entity.position) {
                    // A tree: a canopy, with a darker clump for shape, or a stump where the wood is gone. The canopy
                    // is smaller than a tile, so a wood shows the forest floor between its trunks.
                    if entity.amount == 0 {
                        frame.fill_diamond(centre, (rx * 0.22, ry * 0.22), tint(palette::STUMP));
                    } else {
                        frame.fill_diamond(centre, (rx * 0.55, ry * 0.55), palette::CANOPY);
                        frame.fill_diamond(
                            (centre.0 - (rx * 0.3) as i32, centre.1 + (ry * 0.2) as i32),
                            (rx * 0.35, ry * 0.35),
                            palette::CANOPY_SHADE,
                        );
                    }
                } else {
                    frame.fill_diamond(centre, (rx * 0.6, ry * 0.6), tint(palette::PILE));
                }
            }
            EntityKind::Building => {
                // The silhouette is the pack's footprint, drawn as the diamond a block of that many tiles makes:
                // four tiles across is `4 * rx` half-width and `4 * ry` half-height, centred on the ground the
                // rectangle covers rather than on its anchor tile (D76).
                let (across, down) = entity.footprint;
                let anchor = (
                    entity.position.0.floor() as i32,
                    entity.position.1.floor() as i32,
                );
                let (min_x, min_y, max_x, max_y) = footprint_tiles(anchor, entity.footprint);
                let centre =
                    camera.project(((min_x + max_x) as f32 / 2.0, (min_y + max_y) as f32 / 2.0));
                let colour = if entity.remembered {
                    as_memory(palette::BUILDING)
                } else {
                    palette::BUILDING
                };
                frame.fill_diamond(centre, (rx * across as f32, ry * down as f32), colour);
            }
            EntityKind::Unit => {
                let centre = camera.project(entity.position);
                let dot = (rx * 0.4).max(2.0) as i32;
                frame.fill_rect(
                    centre.0 - dot / 2,
                    centre.1 - dot / 2,
                    dot.max(2),
                    dot.max(2),
                    player_colour(entity.player),
                );
            }
        }
    }
    frame
}

/// What the player has picked, and where they last sent it. The renderer's second input: `render` draws the world,
/// `render_with` draws the world and what the player is doing with it.
///
/// It carries **ids, not entities**, because the selection outlives a frame - a unit can die between the click and the
/// next draw, and a highlight that kept a copy of the entity would paint a ghost.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Highlights<'a> {
    pub selected: &'a [u32],
    pub order: Option<(f32, f32)>,
}

/// Draw a snapshot and the player's marks over it: a ring around every selected thing (its footprint for a building),
/// its health bar, and where the last order went.
pub fn render_with(snapshot: &Snapshot, camera: &Camera, highlights: &Highlights) -> Frame {
    let mut frame = render(snapshot, camera);
    draw_highlights(&mut frame, snapshot, camera, highlights);
    frame
}

/// The marks themselves, drawn over an existing frame. Rings are **outlines** so they never hide what they mark, and a
/// health bar sits above the silhouette, clear of the ring.
pub fn draw_highlights(
    frame: &mut Frame,
    snapshot: &Snapshot,
    camera: &Camera,
    highlights: &Highlights,
) {
    let (rx, ry) = camera.radii();
    for entity in &snapshot.entities {
        if !highlights.selected.contains(&entity.id) {
            continue;
        }
        let (across, down) = entity.footprint;
        let anchor = (
            entity.position.0.floor() as i32,
            entity.position.1.floor() as i32,
        );
        let (min_x, min_y, max_x, max_y) = footprint_tiles(anchor, entity.footprint);
        let centre = camera.project(((min_x + max_x) as f32 / 2.0, (min_y + max_y) as f32 / 2.0));
        let half = (rx * across as f32 * 0.9, ry * down as f32 * 0.9);
        frame.outline_diamond(centre, half, palette::SELECTION);
        // A health bar above the top vertex, green for what is left and red for what is gone.
        let bar_width = (rx * 0.6).max(8.0) as i32;
        let bar_x = centre.0 - bar_width / 2;
        let bar_y = centre.1 - half.1 as i32 - 8;
        let filled = (bar_width as f32 * entity.hp_fraction.clamp(0.0, 1.0)).round() as i32;
        frame.fill_rect(bar_x, bar_y, bar_width, 3, palette::HP_RED);
        if filled > 0 {
            frame.fill_rect(bar_x, bar_y, filled, 3, palette::HP_GREEN);
        }
    }
    if let Some(at) = highlights.order {
        let centre = camera.project(at);
        for offset in -4i32..=4 {
            frame.put(centre.0 + offset, centre.1, palette::ORDER);
            frame.put(centre.0, centre.1 + offset, palette::ORDER);
        }
    }
}

/// The clock that keeps a 20 Hz simulation and a free-running render apart (D56).
///
/// A frame asks how many ticks to run; the remainder is kept, so sixty frames a second at twenty ticks a
/// second is exactly three frames per tick and never drifts. Catch-up is capped, because a machine that
/// stalls for a second should not then run a thousand ticks inside one frame.
#[derive(Debug, Clone)]
pub struct Clock {
    tick_ms: f64,
    accumulator_ms: f64,
    max_catch_up: u32,
}

impl Clock {
    pub fn new(tick_rate_hz: u32) -> Clock {
        Clock {
            tick_ms: 1000.0 / f64::from(tick_rate_hz.max(1)),
            accumulator_ms: 0.0,
            max_catch_up: 5,
        }
    }

    /// How many simulation ticks this frame should run.
    pub fn ticks_for(&mut self, frame_ms: f64) -> u32 {
        self.accumulator_ms += frame_ms.max(0.0);
        let mut ticks = 0;
        while self.accumulator_ms >= self.tick_ms && ticks < self.max_catch_up {
            self.accumulator_ms -= self.tick_ms;
            ticks += 1;
        }
        // A frame that took longer than the cap can catch up is a frame the match skipped ahead on: the
        // accumulated time is dropped rather than repaid, or every later frame would try to catch up.
        if self.accumulator_ms > self.tick_ms * f64::from(self.max_catch_up) {
            self.accumulator_ms = 0.0;
        }
        ticks
    }

    /// How far between the last tick and the next, 0..1, for a renderer that interpolates.
    pub fn alpha(&self) -> f32 {
        (self.accumulator_ms / self.tick_ms).clamp(0.0, 1.0) as f32
    }
}

/// What stands under a tile: the **unit** if one is there, otherwise a building or resource. Own units win a tie, because
/// clicking a villager standing on a farm should select the villager.
///
/// Here rather than in the window binary for one reason: input is the part of a game that is easiest to get subtly wrong
/// (off-by-one tiles, wrong tie-breaking) and the hardest to test through a screen. This is a pure function of a world and
/// a tile, so it can be tested without one.
#[must_use]
pub fn pick_at(world: &ruage_sim::World, tile: (i32, i32)) -> Option<ruage_sim::EntityId> {
    let scale = world.scale();
    let mut best: Option<(u8, u32, ruage_sim::EntityId)> = None;
    for entity in world.entities() {
        // Anywhere on the footprint, not only on the anchor tile: a 4x4 town centre is drawn four tiles wide, so a
        // player who clicks the body of the building they can see must select it. The space is the pack's and so is
        // the size - nothing here knows what a town centre is.
        let anchor = (
            (entity.position.0.raw() / scale) as i32,
            (entity.position.1.raw() / scale) as i32,
        );
        let footprint = entity
            .unit
            .and_then(|unit| world.footprint_of(unit))
            .unwrap_or((1, 1));
        if !footprint_covers(anchor, footprint, tile) {
            continue;
        }
        let rank = if entity.kind == ruage_sim::EntityKind::Unit {
            0
        } else {
            1
        };
        // Same rank: the lower id, so the answer does not depend on iteration order.
        let key = (rank, entity.id.0);
        if best.is_none_or(|(best_rank, best_id, _)| key < (best_rank, best_id)) {
            best = Some((rank, entity.id.0, entity.id));
        }
    }
    best.map(|(_, _, id)| id)
}
