//! A scenario: timed `Command`s that name *what*, not *which id*.
//!
//! This is the replay format for step 5 and the scripted opponent in one. Times are seconds (D16),
//! converted with the match's own tick rate, so the same script means the same match at any rate.
//!
//! Steps resolve selectors against the world when they are due — "the first villager", "the nearest
//! tree", "beside my town centre" — because ids belong to seeding order and would rot the moment the
//! map changed. A step that cannot be resolved is reported, not silently skipped.

use serde::Deserialize;

use crate::{Command, EntityId, EntityKind, World};

/// The scenario format's own version. It is not a config file: it describes a match, so it moves
/// at its own pace and does not share `config/`'s schema constant.
pub const SCENARIO_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub schema: u32,
    pub duration_seconds: f64,
    pub step: Vec<Step>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Step {
    pub at_seconds: f64,
    pub player: u8,
    #[serde(flatten)]
    pub kind: StepKind,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StepKind {
    /// Put one of this player's villagers on a resource, named as the content pack names it.
    Gather {
        villager: usize,
        resource: String,
        /// Which of the nearest nodes of that kind: 0 is nearest. Three villagers told 0, 1 and 2
        /// work three different trees instead of queueing on one and idling when it runs out.
        #[serde(default)]
        nth: usize,
    },
    /// Put up a building beside one of this player's own buildings of the same type.
    Build {
        villager: usize,
        unit: u32,
        where_: Option<String>,
    },
    /// Train a unit at a named building.
    Train { building: String, unit: u32 },
    /// Send this player's newest unit of a type at the nearest enemy.
    Attack { attacker: u32, target: String },
    /// Stand a villager down.
    Stop { villager: usize },
}

/// What happened to a step, kept so the run can be reported rather than guessed at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The step ran and the simulation accepted the order.
    Ran,
    /// The step was due but the world could not satisfy it. Carries the reason.
    Refused(String),
    /// The step has not come due yet.
    Pending,
}

/// Runs a scenario against a world, one tick at a time.
pub struct Runner {
    steps: Vec<(u64, u8, StepKind)>,
    next: usize,
    pub outcomes: Vec<(f64, Outcome)>,
}

impl Runner {
    pub fn new(scenario: &Scenario, tick_rate_hz: u32) -> Result<Self, String> {
        if scenario.schema != SCENARIO_SCHEMA {
            return Err(format!(
                "the scenario is schema {}, this build reads {SCENARIO_SCHEMA}",
                scenario.schema
            ));
        }
        let mut steps = Vec::with_capacity(scenario.step.len());
        for step in &scenario.step {
            if !step.at_seconds.is_finite() || step.at_seconds < 0.0 {
                return Err(format!(
                    "a step is due at {} seconds, which is not a time",
                    step.at_seconds
                ));
            }
            let tick = (step.at_seconds * f64::from(tick_rate_hz)).round();
            let tick = u64::try_from(tick as i64)
                .map_err(|_| format!("a step at {} s does not fit in a tick", step.at_seconds))?;
            steps.push((tick, step.player, step.kind.clone()));
        }
        steps.sort_by_key(|(tick, player, _)| (*tick, *player));
        Ok(Self {
            steps,
            next: 0,
            outcomes: Vec::new(),
        })
    }

    pub fn pending(&self) -> bool {
        self.next < self.steps.len()
    }

    /// Run every step that is due at or before the current tick.
    pub fn due(&mut self, world: &mut World) {
        while let Some((at, player, kind)) = self.steps.get(self.next).cloned() {
            if world.tick() < at {
                return;
            }
            let outcome = match resolve(world, player, &kind) {
                Ok(command) => match world.submit_checked(command) {
                    Ok(()) => Outcome::Ran,
                    Err(err) => Outcome::Refused(err.to_string()),
                },
                Err(reason) => Outcome::Refused(reason),
            };
            self.outcomes
                .push((at as f64 / f64::from(world.tick_rate_hz()), outcome));
            self.next += 1;
        }
    }
}

fn resolve(world: &World, player: u8, kind: &StepKind) -> Result<Command, String> {
    match kind {
        StepKind::Gather {
            villager,
            resource,
            nth,
        } => {
            let unit = nth_villager(world, player, *villager)
                .ok_or_else(|| format!("player {player} has no villager {villager}"))?;
            let slot = world.slot_of(resource).ok_or_else(|| {
                format!(
                    "the content pack defines no resource called {resource:?}, so this step names \
                     nothing a villager could gather"
                )
            })?;
            let node = nearest_resource(world, unit, slot, *nth).ok_or_else(|| {
                format!("player {player} has no {resource} node number {nth} to gather")
            })?;
            Ok(Command::Gather {
                villager: unit,
                node,
            })
        }
        StepKind::Build { villager, unit, .. } => {
            let worker = nth_villager(world, player, *villager)
                .ok_or_else(|| format!("player {player} has no villager {villager}"))?;
            let at = free_tile_near(world, player)
                .ok_or_else(|| format!("player {player} has no free ground by its town centre"))?;
            Ok(Command::Build {
                villager: worker,
                unit: *unit,
                at,
            })
        }
        StepKind::Train { building, unit } => {
            let wanted = match building.as_str() {
                "town_centre" => 109,
                "barracks" => 12,
                other => return Err(format!("unknown building {other:?}")),
            };
            let site = world
                .entities()
                .iter()
                .find(|entity| {
                    entity.player == player
                        && entity.kind == EntityKind::Building
                        && entity.unit == Some(wanted)
                        && entity.construction.is_none()
                })
                .map(|entity| entity.id)
                .ok_or_else(|| format!("player {player} has no finished {building}"))?;
            Ok(Command::Train {
                building: site,
                unit: *unit,
            })
        }
        StepKind::Attack { attacker, target } => {
            if target.as_str() != "nearest_enemy" {
                return Err(format!("unknown target selector {target:?}"));
            }
            let unit = world
                .entities()
                .iter()
                .filter(|entity| {
                    entity.player == player
                        && entity.kind == EntityKind::Unit
                        && entity.unit == Some(*attacker)
                })
                .map(|entity| entity.id)
                .max()
                .ok_or_else(|| format!("player {player} has no unit {attacker}"))?;
            let enemy = world
                .entities()
                .iter()
                .filter(|entity| {
                    entity.player != player && entity.kind == EntityKind::Unit && entity.player != 0
                })
                .map(|entity| entity.id)
                .next()
                .ok_or_else(|| format!("player {player} has no enemy to attack"))?;
            Ok(Command::Attack {
                attacker: unit,
                target: enemy,
            })
        }
        StepKind::Stop { villager } => {
            let unit = nth_villager(world, player, *villager)
                .ok_or_else(|| format!("player {player} has no villager {villager}"))?;
            Ok(Command::Stop { unit })
        }
    }
}

fn nth_villager(world: &World, player: u8, index: usize) -> Option<EntityId> {
    let villager_unit = world.start_villager_unit()?;
    world
        .entities()
        .iter()
        .filter(|entity| {
            entity.player == player
                && entity.kind == EntityKind::Unit
                && entity.unit == Some(villager_unit)
        })
        .map(|entity| entity.id)
        .nth(index)
}

/// The `nth` nearest node of a kind, ordered by (distance, id) so the choice is deterministic and
/// two villagers told `nth = 0` and `nth = 1` land on different nodes.
fn nearest_resource(world: &World, from: EntityId, slot: usize, nth: usize) -> Option<EntityId> {
    let here = world.position(from)?;
    let mut nodes: Vec<(i64, u32, EntityId)> = world
        .entities()
        .iter()
        .filter(|entity| entity.kind == EntityKind::Resource(slot))
        .map(|entity| {
            let dx = entity.position.0.raw() - here.0.raw();
            let dy = entity.position.1.raw() - here.1.raw();
            (dx * dx + dy * dy, entity.id.0, entity.id)
        })
        .collect();
    nodes.sort_unstable();
    nodes.get(nth).map(|(_, _, id)| *id)
}

/// The first free tile within two of this player's town centre, in a fixed scan order.
fn free_tile_near(world: &World, player: u8) -> Option<(u16, u16)> {
    let centre = world.entities().iter().find(|entity| {
        entity.player == player && entity.kind == EntityKind::Building && entity.unit == Some(109)
    })?;
    let scale = world.scale();
    let origin = (
        u16::try_from(centre.position.0.raw() / scale).ok()?,
        u16::try_from(centre.position.1.raw() / scale).ok()?,
    );
    for radius in 2..=6u16 {
        for dy in 0..=radius {
            for dx in 0..=radius {
                for (x, y) in [
                    (origin.0 + dx, origin.1 + dy),
                    (origin.0 + dx, origin.1.wrapping_sub(dy)),
                    (origin.0.wrapping_sub(dx), origin.1 + dy),
                    (origin.0.wrapping_sub(dx), origin.1.wrapping_sub(dy)),
                ] {
                    if world.is_free_ground(x, y) {
                        return Some((x, y));
                    }
                }
            }
        }
    }
    None
}

pub fn load_scenario_str(text: &str, path: &str) -> Result<Scenario, String> {
    toml::from_str(text).map_err(|err| format!("{path}: {err}"))
}

pub fn load_scenario_file(path: &std::path::Path) -> Result<Scenario, String> {
    let text = std::fs::read_to_string(path).map_err(|err| format!("{}: {err}", path.display()))?;
    load_scenario_str(&text, &path.display().to_string())
}

/// A step that could not run at all, for tests that want to know why.
pub fn describe(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Ran => "ran".to_owned(),
        Outcome::Refused(reason) => format!("refused: {reason}"),
        Outcome::Pending => "pending".to_owned(),
    }
}

impl World {
    /// The unit id a player's villagers use, from the economy tables.
    pub fn start_villager_unit(&self) -> Option<u32> {
        self.economy.as_ref().map(|economy| economy.start.villager)
    }

    /// Is this tile inside the map, walkable, and empty?
    pub fn is_free_ground(&self, x: u16, y: u16) -> bool {
        let Some(walk) = self.walk.as_ref() else {
            return false;
        };
        if !walk.contains(x, y) || walk.blocked(x, y) {
            return false;
        }
        let position = (
            ruage_fixed::Fixed::from_raw(i64::from(x) * self.scale),
            ruage_fixed::Fixed::from_raw(i64::from(y) * self.scale),
        );
        !self
            .entities()
            .iter()
            .any(|entity| entity.position == position)
    }
}
