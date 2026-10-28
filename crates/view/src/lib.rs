//! The presentation boundary: a read-only snapshot of a match, a camera, and a rasteriser.
//!
//! The renderer never touches `World`. It reads a [`Snapshot`], which is a plain description of what a
//! player can see - id, content id, position, health fraction, what it is doing - and a [`Camera`], which
//! is a transform from tiles to pixels. That is the whole reason this crate exists: the simulation cannot
//! learn what a screen is, and an asset pack later replaces the colours with sprites by changing one
//! function, not the game (D17, D55).
//!
//! Floats are allowed here and nowhere near the simulation (D9): pixels are not state.

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

        let mut entities = Vec::new();
        for entity in world.entities() {
            // A player is not shown what they cannot see: that is the only filter here, and it is the
            // simulation's answer, not the renderer's guess.
            if !world.visible_to(player, entity.id) {
                continue;
            }
            let garrison = world
                .entities()
                .iter()
                .filter(|other| {
                    other.task
                        == (Task::Garrisoned {
                            building: entity.id,
                        })
                })
                .count() as u32;
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
            });
        }

        let stock = (0..ruage_sim::MAX_RESOURCES)
            .map(|slot| world.stock(player, slot))
            .collect();
        let mut explored = vec![0u64; (usize::from(width) * usize::from(height)).div_ceil(64)];
        for y in 0..height {
            for x in 0..width {
                if world.vision(player, (x, y)) != TileVision::Unseen {
                    let bit = usize::from(y) * usize::from(width) + usize::from(x);
                    if let Some(word) = explored.get_mut(bit / 64) {
                        *word |= 1u64 << (bit % 64);
                    }
                }
            }
        }

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
    pub fn new(viewport: (u32, u32)) -> Camera {
        Camera {
            origin: (0.0, 0.0),
            tile_pixels: 16.0,
            zoom: 1.0,
            viewport,
        }
    }

    pub fn centred_on(mut self, tile: (f32, f32)) -> Camera {
        let (w, h) = self.viewport;
        self.origin = (
            tile.0 - (w as f32 / self.scale() / 2.0),
            tile.1 - (h as f32 / self.scale() / 2.0),
        );
        self
    }

    fn scale(&self) -> f32 {
        self.tile_pixels * self.zoom
    }

    /// The pixel a tile's top-left corner lands on. Off-screen tiles land off-screen, which is the
    /// renderer's problem to clip, not the camera's to hide.
    pub fn project(&self, tile: (f32, f32)) -> (i32, i32) {
        (
            ((tile.0 - self.origin.0) * self.scale()).round() as i32,
            ((tile.1 - self.origin.1) * self.scale()).round() as i32,
        )
    }

    /// The tile under a pixel. The inverse of `project`, to the tile.
    pub fn pick(&self, pixel: (i32, i32)) -> (i32, i32) {
        (
            (pixel.0 as f32 / self.scale() + self.origin.0).floor() as i32,
            (pixel.1 as f32 / self.scale() + self.origin.1).floor() as i32,
        )
    }

    pub fn scroll(&mut self, tiles: (f32, f32)) {
        self.origin.0 += tiles.0;
        self.origin.1 += tiles.1;
    }

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
/// its fallback, which is why they live here and not in the content pack (D55).
mod palette {
    pub const GRASS: [u8; 4] = [96, 128, 64, 255];
    pub const FOREST: [u8; 4] = [32, 64, 32, 255];
    pub const UNSEEN: [u8; 4] = [8, 8, 12, 255];
    pub const EXPLORED: [u8; 4] = [64, 72, 64, 255];
    pub const PLAYER_ONE: [u8; 4] = [48, 96, 220, 255];
    pub const PLAYER_TWO: [u8; 4] = [200, 64, 48, 255];
    pub const NEUTRAL: [u8; 4] = [120, 120, 120, 255];
    pub const RESOURCE: [u8; 4] = [180, 150, 90, 255];
    pub const BUILDING: [u8; 4] = [200, 200, 200, 255];
}

pub(crate) fn player_colour(player: u8) -> [u8; 4] {
    match player {
        1 => palette::PLAYER_ONE,
        2 => palette::PLAYER_TWO,
        _ => palette::NEUTRAL,
    }
}

/// Draw a snapshot. Tiles are rectangles, units are dots, and nothing here knows what a unit *is* beyond
/// its kind and its player (D55).
pub fn render(snapshot: &Snapshot, camera: &Camera) -> Frame {
    let mut frame = Frame::new(camera.viewport.0, camera.viewport.1);
    let step = camera.scale();

    // Every tile the camera can see, and only those: a 64x64 map at 4x zoom is 4096 tiles, most of them
    // off screen, and drawing them would be most of the frame's cost.
    let first = camera.pick((0, 0));
    let last = camera.pick((frame.width as i32 - 1, frame.height as i32 - 1));
    for y in first.1.max(0)..=last.1.min(i32::from(snapshot.height) - 1) {
        for x in first.0.max(0)..=last.0.min(i32::from(snapshot.width) - 1) {
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
                TileVision::Visible => palette::GRASS,
            };
            let (px, py) = camera.project((x as f32, y as f32));
            frame.fill_rect(px, py, step.ceil() as i32, step.ceil() as i32, colour);
        }
    }

    // Resources first: they are the ground, a unit standing on one should cover it.
    for entity in &snapshot.entities {
        if matches!(entity.kind, EntityKind::Resource(_)) {
            let (px, py) = camera.project(entity.position);
            let size = (step * 0.6) as i32;
            frame.fill_rect(px + 2, py + 2, size.max(2), size.max(2), palette::RESOURCE);
        }
    }
    for entity in &snapshot.entities {
        match entity.kind {
            EntityKind::Resource(_) | EntityKind::Projectile => {}
            EntityKind::Building => {
                let (px, py) = camera.project(entity.position);
                let size = (step * 0.9) as i32;
                frame.fill_rect(px + 1, py + 1, size.max(3), size.max(3), palette::BUILDING);
            }
            EntityKind::Unit => {
                let (px, py) = camera.project(entity.position);
                let dot = (step * 0.4).max(2.0) as i32;
                frame.fill_rect(
                    px + dot / 2,
                    py + dot / 2,
                    dot.max(2),
                    dot.max(2),
                    player_colour(entity.player),
                );
            }
        }
    }
    frame
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
