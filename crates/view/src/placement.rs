//! **Placing a wall with the mouse** (D156): the first left click starts the run, the second places it, and the ghost the
//! player sees is exactly the line the command sends.
//!
//! The owner: *"Walls can be placed vertically, horizontally mass at 45 degrees. First left click starts placement, second
//! click ends it."* And on ineligible ground: *"…when it is overlapping over ineligible area (mine, first, another building…)
//! it is getting red tone. With walls it works the same way with segments overlapping ineligible area getting red tone."*
//!
//! Deliberately **no window in here**: a `winit` loop cannot be driven from a test, so everything that can be wrong - which
//! tiles the run covers, which are red, what the second click sends - lives in this type and is tested. The window's job is
//! three calls: `click`, `hover`, `segments`.
//!
//! The line itself is **not** computed here: [`ruage_sim::World::wall_line`] is the one answer to "where does the wall go", so
//! the ghost and the command cannot disagree (D155).

use ruage_sim::{Command, EntityId, World};

use crate::{Camera, Frame};

/// **The ghost**, drawn over the ground: a half-transparent diamond per segment, outlined in the same colour - **green** where
/// the ground will take a wall, **red** where it will not (the owner: *"segments overlapping ineligible area getting red tone"*).
///
/// Takes segments rather than a placement so a caller that has already asked the world cannot draw a different line from the one
/// it will send.
pub fn draw_placement(frame: &mut Frame, camera: &Camera, segments: &[Segment]) {
    let radii = camera.radii();
    for segment in segments {
        let centre = camera.project((
            f32::from(segment.tile.0) + 0.5,
            f32::from(segment.tile.1) + 0.5,
        ));
        let colour = if segment.valid {
            crate::palette::PLACEMENT
        } else {
            crate::palette::PLACEMENT_REFUSED
        };
        frame.blend_diamond(centre, radii, colour, 96);
        frame.outline_diamond(centre, radii, colour);
    }
}

/// One tile of the ghost, and whether it could be built on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[must_use]
pub struct Segment {
    /// The tile, in drag order.
    pub tile: (u16, u16),
    /// `false` draws it **red**: the ground is taken, and the sim will skip this segment when the line is ordered.
    pub valid: bool,
}

/// A wall run being drawn: the building, the villager who will build it, and the two ends of the drag.
#[derive(Clone, Debug)]
pub struct Placement {
    villager: EntityId,
    unit: u32,
    /// `None` until the first click - before that there is no run, and nothing is drawn.
    from: Option<(u16, u16)>,
    to: (u16, u16),
}

impl Placement {
    /// A run about to be dragged, by `villager`, of `unit` (a building role id).
    #[must_use]
    pub fn new(villager: EntityId, unit: u32) -> Self {
        Self {
            villager,
            unit,
            from: None,
            to: (0, 0),
        }
    }

    /// Whether the next left click **starts** the run (rather than ending it).
    #[must_use]
    pub fn awaiting_start(&self) -> bool {
        self.from.is_none()
    }

    /// The building being placed.
    #[must_use]
    pub fn unit(&self) -> u32 {
        self.unit
    }

    /// The pointer moved. Before the first click this does nothing: there is no run to lengthen.
    pub fn hover(&mut self, tile: (u16, u16)) {
        if self.from.is_some() {
            self.to = tile;
        }
    }

    /// **A left click.** The first starts the run and returns `None`; the second returns the order to send - or `None` when
    /// nothing on the line can be built, in which case the run **stays up** so the player can drag somewhere else rather than
    /// losing the placement to a misplaced second click.
    pub fn click(&mut self, world: &World, tile: (u16, u16)) -> Option<Command> {
        if self.from.is_none() {
            self.from = Some(tile);
            self.to = tile;
            return None;
        }
        self.to = tile;
        if self.segments(world).iter().all(|segment| !segment.valid) {
            return None;
        }
        let command = Command::BuildLine {
            villager: self.villager,
            unit: self.unit,
            tiles: self.tiles(),
        };
        // The run is over: the next click starts a new one.
        self.from = None;
        Some(command)
    }

    /// The tiles the run covers, in drag order, as the ghost draws them.
    #[must_use]
    pub fn tiles(&self) -> Vec<(u16, u16)> {
        self.from
            .map_or_else(Vec::new, |from| World::wall_line(from, self.to))
    }

    /// The ghost: every tile of the run, and whether it is red.
    ///
    /// The question asked is [`World::is_free_ground`] - the *placement* predicate, terrain and standing entities - which is
    /// the same one a single building's ghost asks. A gate or a farm, whose rules differ, will need their own answer; a wall
    /// segment is worth exactly this question.
    #[must_use]
    pub fn segments(&self, world: &World) -> Vec<Segment> {
        self.tiles()
            .into_iter()
            .map(|tile| Segment {
                tile,
                valid: world.is_free_ground(tile.0, tile.1),
            })
            .collect()
    }
}
