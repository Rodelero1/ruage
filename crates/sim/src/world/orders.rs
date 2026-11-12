//! **Orders in and out**: validation before a command is accepted, and `apply`, which carries one out (D145).
//!
//! Split out of `lib.rs` under D145, verbatim. The split is the file, not the logic: `validate` and `apply` still have to
//! agree about every command, so they sit next to each other rather than 3,000 lines apart.
use ruage_fixed::Fixed;

use crate::{
    Command, CommandApply, Entity, EntityId, EntityKind, MAX_PLAYERS, RallyPoint, SimError, Stance,
    StepDecision, Task, World,
};
impl World {
    /// Submit a command, refusing one that cannot be applied *at all* before it is queued. A bad
    /// order should be an error in the caller's face, not a villager who idles for ten minutes.
    pub fn submit_checked(&mut self, command: Command) -> Result<(), SimError> {
        if let Err(error) = self.validate(&command) {
            // **A gather order refused for the ground is remembered** (D138). A script's gather action picks by distance
            // and asks nothing - a path search per candidate cost 37x the run - so `start_gather` is the one authority,
            // and this refusal is the only evidence the picker will ever get. Without the entry a rule would take the
            // same walled-off tree every cooldown for the whole match: measured, 29 refusals in 30 s.
            if let (Command::Gather { node, .. }, SimError::NoPath { .. }) = (&command, &error) {
                self.share_failed.insert(*node, self.tick);
            }
            return Err(error);
        }
        self.submit(command);
        Ok(())
    }

    pub(crate) fn validate(&self, command: &Command) -> Result<(), SimError> {
        // A finished match takes no more orders. The check is here as well as in `apply` so the
        // caller sees the refusal instead of a tick that silently does nothing.
        if self.outcome.is_some() {
            return Err(SimError::MatchOver);
        }
        match command {
            Command::Gather { villager, node } => {
                let worker = self
                    .find(*villager)
                    .ok_or(SimError::MissingEntity(*villager))?;
                if worker.kind != EntityKind::Unit {
                    return Err(SimError::NotAUnit { id: *villager });
                }
                let target = self.find(*node).ok_or(SimError::MissingEntity(*node))?;
                // What may be gathered from is content's answer, not the validator's: a node, or a
                // building the pack calls a farm. One helper, so the order cannot accept something
                // the validator refuses.
                self.gather_source(*node)?;
                // A walled-off node is refused here rather than discovered ten seconds later by a
                // villager walking into a wall and giving up.
                if let Some(position) = self.find(*villager).map(|e| e.position) {
                    self.plan_path_for(*villager, position, target.position)?;
                }
            }
            Command::Build { villager, unit, at } => {
                self.check_build(*villager, *unit, *at)?;
            }
            Command::JoinBuild { villager, site } => {
                self.check_join(*villager, *site)?;
            }
            // A line is judged by its **first** tile only, and the rest are skipped as they are tried (the owner's answer B:
            // an ineligible segment is not fatal). `start_build` already refuses a tile that cannot be built on, so the whole
            // drag fails only when its first tile does.
            Command::BuildLine {
                villager,
                unit,
                tiles,
            } => {
                let first = tiles.first().copied().ok_or(SimError::EmptyLine)?;
                self.check_build(*villager, *unit, first)?;
            }
            Command::Train { building, unit } => {
                self.check_train(*building, *unit)?;
            }
            Command::Research { building, tech } => {
                self.check_research(*building, *tech)?;
            }
            Command::Attack { attacker, target } => {
                self.check_attack(*attacker, *target)?;
            }
            // Stopping is always legal: there is nothing to be refused.
            Command::Rally { building, point } => {
                let entity = self
                    .find(*building)
                    .ok_or(SimError::MissingEntity(*building))?;
                if entity.kind != EntityKind::Building {
                    return Err(SimError::TrainNeedsABuilding { id: *building });
                }
                if let Some((x, y)) = point
                    && !self.on_map(*x, *y)
                {
                    return Err(SimError::OffMap { tile: (*x, *y) });
                }
            }
            Command::Garrison { unit, building } => {
                self.check_garrison(*unit, *building)?;
            }
            Command::Ungarrison { building } => {
                let entity = self
                    .find(*building)
                    .ok_or(SimError::MissingEntity(*building))?;
                if entity.kind != EntityKind::Building {
                    return Err(SimError::TrainNeedsABuilding { id: *building });
                }
            }
            Command::Stop { .. } => {}
            Command::Move { unit, to } => {
                self.check_order(*unit, *to)?;
            }
            Command::WalkTo { unit, to } => {
                self.check_order_at(*unit, *to)?;
            }
            Command::Stance { unit, .. } => {
                let entity = self.find(*unit).ok_or(SimError::MissingEntity(*unit))?;
                if entity.kind != EntityKind::Unit {
                    return Err(SimError::NotAUnit { id: *unit });
                }
            }
            Command::AttackMove { unit, to } | Command::Patrol { unit, to } => {
                self.check_order(*unit, *to)?;
            }
            Command::Delete { player, entity } => {
                let target = self.find(*entity).ok_or(SimError::MissingEntity(*entity))?;
                if target.player == 0 || target.player != *player {
                    return Err(SimError::NotYours {
                        id: *entity,
                        player: *player,
                    });
                }
                if !matches!(target.kind, EntityKind::Unit | EntityKind::Building) {
                    return Err(SimError::NotDeletable { id: *entity });
                }
            }
            // The bound belongs in the dry run too, or `submit_checked` accepts a ninth side and
            // the failure only appears a tick later inside the queue.
            Command::Spawn { player, .. } => {
                // 0 is neutral (no stockpile); the ceiling is what has to hold (D20).
                if usize::from(*player) > MAX_PLAYERS {
                    return Err(SimError::PlayerOutOfRange { player: *player });
                }
            }
        }
        Ok(())
    }

    pub(crate) fn find(&self, id: EntityId) -> Option<&Entity> {
        self.entities.iter().find(|e| e.id == id)
    }

    pub(crate) fn find_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.entities.iter_mut().find(|e| e.id == id)
    }

    /// Queue a command. With `command_apply = "immediate"` it takes effect at once; otherwise
    /// it is held until the next `step`, which is what makes a command log replayable.
    pub fn submit(&mut self, command: Command) {
        match self.command_apply {
            CommandApply::Immediate => {
                // The caller cannot observe the error here; `step` will not see these again.
                let _ = self.apply(command);
            }
            CommandApply::NextTick => self.queue.push(command),
        }
    }

    pub(crate) fn apply(&mut self, command: Command) -> Result<(), SimError> {
        if self.outcome.is_some() {
            return Err(SimError::MatchOver);
        }
        match command {
            Command::Spawn {
                id,
                player,
                kind,
                unit,
                position,
                speed,
                hp,
                amount,
            } => {
                // Spawn is the only command that names a player directly, so the bound is enforced
                // here too: an entity for player 9 would otherwise sit in the world until something
                // tried to hand it a stockpile, and then panic on the array.
                // Player 0 is neutral - resource nodes live there and it has no stockpile - so the
                // bound is only the ceiling. The movement hook in `spawn_now` uses player 0.
                if usize::from(player) > MAX_PLAYERS {
                    return Err(SimError::PlayerOutOfRange { player });
                }
                if kind == EntityKind::Unit
                    && unit.is_some_and(|unit| {
                        self.herdable.contains(&unit)
                            || self
                                .units
                                .as_ref()
                                .is_some_and(|units| self.herdable.contains(&units.root(unit)))
                    })
                {
                    self.herdable_ids.insert(id);
                }
                // **Who owns this ground?** Units standing inside the new footprint have to be given somewhere else *before*
                // the tiles turn solid, or their escape search cannot see a way out (a unit in the middle of a 3x3 has
                // eight blocked neighbours and stays put for the rest of the match).
                let movers: Vec<(EntityId, (Fixed, Fixed))> = if kind == EntityKind::Building {
                    let tiles = self.build_site_tiles(unit.unwrap_or(0), self.tile_of(position));
                    self.entities
                        .iter()
                        .filter(|entity| {
                            entity.kind == EntityKind::Unit
                                && entity.player != 0
                                && tiles.contains(&self.tile_of(entity.position))
                        })
                        .filter_map(|entity| {
                            let here = self.tile_of(entity.position);
                            self.escape_from(&tiles, here).map(|at| (entity.id, at))
                        })
                        .collect()
                } else {
                    Vec::new()
                };
                let inserted = self.insert(Entity {
                    last_step: StepDecision::Idle,
                    id,
                    player,
                    kind,
                    unit,
                    position,
                    speed,
                    // A thing that is placed is not a thing that is walking. This was `(speed, ZERO)`: a
                    // spawned body drifted east at its own speed for ever with `Task::Idle`, taking one
                    // step of 0.025 tile per tick - which is what the "creep" was. Every other way into
                    // the world starts at rest, and this one now does too (D64, D70).
                    velocity: (Fixed::ZERO, Fixed::ZERO),
                    hp,
                    // An animal placed with no amount is worth what the pack says it is worth: a sheep carries its 100
                    // food, not zero (D70). **Only an animal.** `meat_of` reads any unit's `food` field, and a farm's
                    // `food` is its 250-food capacity - so a spawned farm told "nothing left" came back full, and
                    // `is_expired_farm` never saw a husk that a command had made (the parked defect, closed here).
                    amount: Fixed::from_raw(
                        i64::from(if amount == 0 {
                            if unit.is_some_and(|unit| self.animals.contains(&unit)) {
                                self.meat_of(unit)
                            } else {
                                0
                            }
                        } else {
                            amount
                        }) * self.scale,
                    ),
                    task: Task::Idle,
                    training: None,
                    construction: None,
                    researching: None,
                    carry: Fixed::ZERO,
                    variant: None,
                });
                // **A building that appears pushes units off its ground**, exactly as one that is finished does: a villager
                // left inside a blocked footprint can never plan a route again (A* refuses to start on a blocked tile), so
                // it stands there for the rest of the match. `Command::Spawn` used not to, and six research tests were red
                // on it - a barracks spawned at (14, 14) swallowed the three starting villagers.
                if inserted.is_ok() {
                    for (id, at) in movers {
                        if let Some(entity) = self.find_mut(id) {
                            entity.position = at;
                        }
                        self.paths.remove(&id);
                    }
                }
                inserted
            }
            Command::Gather { villager, node } => self.start_gather(villager, node),
            Command::Build { villager, unit, at } => self.start_build(villager, unit, at),
            Command::JoinBuild { villager, site } => self.start_join(villager, site),
            Command::BuildLine {
                villager,
                unit,
                tiles,
            } => self.start_build_line(villager, unit, &tiles),
            Command::Train { building, unit } => self.start_train(building, unit),
            Command::Research { building, tech } => self.start_research(building, tech),
            Command::Attack { attacker, target } => self.start_attack(attacker, target),
            Command::Rally { building, point } => {
                match point {
                    None => {
                        self.rallies.remove(&building);
                    }
                    Some(tile) => {
                        // A rally on the building's own tile means "stay inside". It is the same tile
                        // the player would click on the building, so no second command is needed.
                        let inside = self
                            .find(building)
                            .map(|entity| self.tile_of(entity.position) == tile)
                            .unwrap_or(false);
                        let rally = if inside {
                            RallyPoint::Inside
                        } else {
                            RallyPoint::Tile(tile.0, tile.1)
                        };
                        self.rallies.insert(building, rally);
                    }
                }
                Ok(())
            }
            Command::Garrison { unit, building } => {
                // Inside, the unit keeps its position and its load: it is sheltered, not teleported.
                if let Some(entity) = self.find_mut(unit) {
                    entity.task = Task::Garrisoned { building };
                }
                self.paths.remove(&unit);
                Ok(())
            }
            Command::Ungarrison { building } => {
                self.ungarrison(building);
                Ok(())
            }
            Command::Stop { unit } => {
                self.paths.remove(&unit);
                self.patrols.remove(&unit);
                self.resuming.remove(&unit);
                self.stand_down(unit);
                Ok(())
            }
            Command::Stance { unit, stance } => {
                let Some(post) = self.find(unit).map(|entity| entity.position) else {
                    return Err(SimError::MissingEntity(unit));
                };
                if stance == Stance::Aggressive {
                    // The default is the absence of an entry, so an aggressive unit costs nothing.
                    self.stances.remove(&unit);
                } else {
                    self.stances.insert(unit, (stance, post));
                }
                // A unit told to stop fighting drops the fight it is in; a unit told to stand its
                // ground keeps the fight, because it may still be in reach.
                if stance == Stance::NoAttack
                    && matches!(
                        self.find(unit).map(|entity| entity.task),
                        Some(Task::Attack { .. })
                    )
                {
                    self.stand_down(unit);
                }
                Ok(())
            }
            Command::Move { unit, to } => {
                let goal = self.tile_centre(to);
                self.start_walk(unit, goal, false, false)
            }
            Command::WalkTo { unit, to } => self.start_walk(unit, to, false, false),
            Command::AttackMove { unit, to } => {
                let goal = self.tile_centre(to);
                self.start_walk(unit, goal, true, false)
            }
            Command::Patrol { unit, to } => {
                let goal = self.tile_centre(to);
                self.start_walk(unit, goal, true, true)
            }
            Command::Delete { entity, .. } => {
                // A deletion is a death and takes the death path: health to zero, and the end of the
                // tick does the rest - the route goes, the population frees. Removing the entity
                // here would run that path from a place where the entity list is supposed to be
                // fixed for the length of the tick.
                //
                // One deliberate difference from being razed by an enemy: whoever is inside comes
                // out alive. Demolishing your own town centre is how a player pulls villagers out of
                // it, and losing them would make the command a trap; an enemy's siege still kills
                // the garrison, which is what the death sweep does on its own (D46a).
                let is_building = self
                    .find(entity)
                    .is_some_and(|target| target.kind == EntityKind::Building);
                // Demolishing work in progress gives back the part you have not built (D60). Verified
                // against the game: an untouched foundation refunds in full, a half-built building
                // refunds about half, a finished one refunds nothing - you lose what you invested.
                let part = self.find(entity).and_then(|target| {
                    target
                        .construction
                        .map(|construction| (target.player, target.unit, construction))
                });
                if let Some((player, Some(unit), construction)) = part {
                    let total = construction.total_ticks;
                    if total > 0 {
                        let left = total.saturating_sub(construction.progress_ticks.min(total));
                        self.refund_unit_cost_part(player, unit, left, total);
                    }
                }
                if let Some(target) = self.find_mut(entity) {
                    target.hp = 0;
                }
                if is_building {
                    self.ungarrison(entity);
                }
                self.paths.remove(&entity);
                Ok(())
            }
        }
    }
}
