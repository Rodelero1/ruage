//! **Training**: what a building may make, and what appears when the queue finishes.
//!
//! Split out of `lib.rs` under D144, verbatim: a pure move, proved by the acceptance hash.

use ruage_fixed::Fixed;

use crate::{
    Command, Entity, EntityId, EntityKind, Job, RallyPoint, SimError, Task, TrainPlan, Training,
    TrainingPause, World,
};

impl World {
    /// The checks a training order has to survive. Readiness comes before trainability on
    /// purpose: a house that is still going up should be told it is not finished, not that it
    /// makes nothing.
    pub(crate) fn check_train(&self, building: EntityId, unit: u32) -> Result<TrainPlan, SimError> {
        let entity = self
            .find(building)
            .ok_or(SimError::MissingEntity(building))?;
        if entity.kind != EntityKind::Building {
            return Err(SimError::TrainNeedsABuilding { id: building });
        }
        let building_unit = entity
            .unit
            .ok_or(SimError::BuildingNotReady { id: building })?;
        if entity.construction.is_some() {
            return Err(SimError::BuildingNotReady { id: building });
        }
        // A building holds `queue_length` units at once: the one being made and those behind it. The
        // order is refused only when there is no room, because the queue is content's number, not the
        // engine's (D33).
        // Orders can be waiting to be applied (`command_apply = "next_tick"`), and a queue limit that
        // ignored them would accept five in one tick and then hold all five - the limit is about what
        // the building will hold, not about what has been applied so far.
        let pending = self
            .queue
            .iter()
            .filter(|command| matches!(command, Command::Train { building: queued, .. } if *queued == building))
            .count();
        let held = u32::from(entity.training.is_some())
            + u32::try_from(self.queued.get(&building).map_or(0, Vec::len)).unwrap_or(u32::MAX)
            + u32::try_from(pending).unwrap_or(u32::MAX);
        let limit = self
            .economy
            .as_ref()
            .map_or(1, |economy| economy.train.queue_length);
        if held >= limit {
            return Err(SimError::QueueFull {
                id: building,
                length: limit,
            });
        }
        let economy = self.economy.as_ref().ok_or(SimError::NoTables)?;
        // The same availability rule as building: a trebuchet needs the tech that makes it available,
        // and "no way to have it yet" outranks "not enough wood".
        if let Some(civ) = self.civ_barred_unit(entity.player, unit) {
            return Err(SimError::UnitBarred { unit, civ });
        }
        if !self.unit_available(entity.player, unit) {
            return Err(SimError::UnitUnavailable { unit });
        }
        // Population is not an ordering rule: a queue may hold more than the player can support, and
        // the *training* pauses at the ceiling instead (D34). Refusing here would make the ceiling a
        // wall you cannot queue behind, which is not how the game plays.
        // An upgrade target is not available on its own: a barracks trains pikemen only after the
        // Pikeman tech. Without this, `[train.entry]` naming the spear line would let a Dark Age
        // barracks train halberdiers.
        if let Some(tech) = self.upgrade_tech.get(&unit).copied()
            && !self.researched(entity.player).contains(&tech)
        {
            return Err(SimError::UnitNotUpgraded { unit, tech });
        }
        // A barracks upgraded by an age is unit 498, and `[train.entry]` names 12.
        let building_root = self
            .units
            .as_ref()
            .map_or(building_unit, |units| units.root(building_unit));
        // The line counts: `[train.entry]` says a barracks trains spearmen, and a barracks keeps doing
        // that as the line upgrades, so a pikeman matches through the family root.
        let unit_root = self.units.as_ref().map_or(unit, |units| units.root(unit));
        if !(economy.train.allows(building_unit, unit)
            || economy.train.allows(building_root, unit)
            || economy.train.allows(building_unit, unit_root)
            || economy.train.allows(building_root, unit_root))
        {
            return Err(SimError::NotTrainableHere {
                building: building_unit,
                unit,
            });
        }
        let units = self.units.as_ref().ok_or(SimError::NoTables)?;
        let stats = units.unit(unit).ok_or(SimError::MissingUnit { id: unit })?;
        let ticks = u32::try_from(stats.train_time)
            .ok()
            .and_then(|seconds| seconds.checked_mul(self.tick_rate_hz))
            .filter(|ticks| *ticks > 0)
            .ok_or(SimError::NotTrainableHere {
                building: building_unit,
                unit,
            })?;

        for cost in &stats.costs {
            let slot = self.cost_slot(cost)?;
            let need = u32::from(cost.amount);
            let have = self.stock(entity.player, slot);
            if have < need {
                return Err(SimError::NotEnough {
                    resource: self.slot_name(slot),
                    need,
                    have,
                });
            }
        }

        Ok(TrainPlan {
            costs: stats.costs.clone(),
            ticks,
            hp: stats.hp,
            speed: stats
                .speed
                .resolve(self.scale, "speed")
                .unwrap_or(Fixed::ZERO),
            player: entity.player,
        })
    }

    /// Pay for a unit and put it in the building's single production slot.
    pub(crate) fn start_train(&mut self, building: EntityId, unit: u32) -> Result<(), SimError> {
        let plan = self.check_train(building, unit)?;
        for cost in &plan.costs {
            let slot = self.cost_slot(cost)?;
            let index = usize::from(plan.player).wrapping_sub(1);
            let Some(row) = self.stock.get_mut(index) else {
                return Err(SimError::PlayerOutOfRange {
                    player: plan.player,
                });
            };
            row[slot] -= u32::from(cost.amount);
        }
        let job = Training {
            unit,
            remaining_ticks: plan.ticks,
            hp: plan.hp,
            speed: plan.speed,
            fresh: true,
        };
        let idle = self
            .find(building)
            .is_some_and(|entity| entity.training.is_none() && entity.researching.is_none());
        if idle && !self.queued.contains_key(&building) {
            let Some(entity) = self.find_mut(building) else {
                return Err(SimError::MissingEntity(building));
            };
            entity.training = Some(job);
        } else {
            self.queued
                .entry(building)
                .or_default()
                .push(Job::Train(job));
        }
        Ok(())
    }

    /// **Start the head of each building's queue** (D157) where nothing is running.
    ///
    /// The queue holds units and techs in one order, so this is the only place that decides what runs next: a tech behind two
    /// units runs third, and a unit behind a tech waits for the tech. A job that can no longer be paid for is **dropped with a
    /// refusal** rather than silently skipped - the player paid for the queue's order, not for the engine's convenience.
    pub(crate) fn start_next_jobs(&mut self) -> Result<(), SimError> {
        if self.queued.is_empty() {
            return Ok(());
        }
        let idle: Vec<EntityId> = self
            .entities
            .iter()
            .filter(|entity| {
                entity.kind == EntityKind::Building
                    && entity.hp > 0
                    && entity.construction.is_none()
                    && entity.training.is_none()
                    && entity.researching.is_none()
            })
            .map(|entity| entity.id)
            .filter(|id| self.queued.contains_key(id))
            .collect();
        for id in idle {
            let Some(job) = self
                .queued
                .get_mut(&id)
                .and_then(|queue| (!queue.is_empty()).then(|| queue.remove(0)))
            else {
                continue;
            };
            if self.queued.get(&id).is_some_and(Vec::is_empty) {
                self.queued.remove(&id);
            }
            match job {
                Job::Train(training) => {
                    if let Some(entity) = self.find_mut(id) {
                        entity.training = Some(training);
                    }
                }
                Job::Research { tech } => match self.check_research(id, tech) {
                    Ok(plan) => self.begin_research(id, tech, plan)?,
                    Err(why) => self.refusal = Some((id, why, self.tick)),
                },
            }
        }
        Ok(())
    }

    /// Put a finished unit somewhere, in the order D85 sets out: a free tile beside the building; if there is none,
    /// **inside the building** (its own `garrison` field, the number its garrison orders read); if there is no room
    /// there either, leave the job where it is and **say why** - the queue pauses with `TrainingPause::NoRoom` until
    /// the reason goes away. A recruit is never dropped and never quietly retried for ever.
    ///
    /// Returns true when the job left the building (so the queue behind it advances) and false when it could not.
    pub(crate) fn spawn_from_building(&mut self, building: EntityId) -> bool {
        let Some(job) = self.find(building).and_then(|e| e.training) else {
            return true;
        };
        let Some(source) = self.find(building) else {
            return true;
        };
        let player = source.player;
        let unit = job.unit;
        let scale = self.scale;
        let origin = (
            u16::try_from(source.position.0.raw() / scale).unwrap_or(0),
            u16::try_from(source.position.1.raw() / scale).unwrap_or(0),
        );
        // Somewhere to stand, or somewhere to shelter. A building with no garrison capacity (a market, a smith)
        // gives no second chance: the rule is "inside it or say why", and content decides which buildings can hold
        // anybody.
        let placement = match self.free_spot_near(origin) {
            Some(tile) => Some((tile, false)),
            None if self.garrison_room(building, player) => Some((origin, true)),
            None => None,
        };
        let Some((spot, inside)) = placement else {
            self.training_pauses.insert(building, TrainingPause::NoRoom);
            return false;
        };
        // Health and speed are resolved when the unit appears, so a tech researched while it was in the queue still
        // counts - and only once placement is decided: a blocked building re-enters this function every tick, and
        // resolving modifiers per tick for a recruit that cannot be placed was two thirds of the pause's cost (measured,
        // docs/reviews/2026-10-31-d85-no-room.md).
        let hp = self.max_hp(player, unit).unwrap_or(job.hp);
        let speed = self.effective_speed(player, unit).unwrap_or(job.speed);
        let position = self.tile(spot);
        let id = EntityId(self.entities.iter().map(|e| e.id.0).max().unwrap_or(0) + 1);
        let inserted = self.insert(Entity {
            id,
            player,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position,
            speed,
            velocity: (Fixed::ZERO, Fixed::ZERO),
            hp,
            amount: Fixed::ZERO,
            task: Task::Idle,
            training: None,
            construction: None,
            researching: None,
            carry: Fixed::ZERO,
            variant: None,
        });
        if inserted.is_err() {
            return false;
        }
        self.training_pauses.remove(&building);
        if inside {
            // Sheltered on the tick it was finished: it exists, it costs population, it dies with the building.
            if let Some(entity) = self.find_mut(id) {
                entity.task = Task::Garrisoned { building };
            }
        } else {
            // Where the finished unit goes: the rally decides, and the default is where it stands.
            let rally = self.rallies.get(&building).copied();
            match rally {
                Some(RallyPoint::Inside) => {
                    // Only a building that can hold units can keep one, and only while there is room.
                    let room = self
                        .find(building)
                        .and_then(|entity| entity.unit)
                        .and_then(|unit| self.units.as_ref().and_then(|units| units.unit(unit)))
                        .is_some_and(|stats| stats.garrison > 0);
                    if room && self.check_garrison(id, building).is_ok() {
                        if let Some(entity) = self.find_mut(id) {
                            entity.task = Task::Garrisoned { building };
                        }
                    } else if room {
                        // **Too far to walk straight in.** A garrison order is refused for a unit that is not at the door,
                        // so the finished unit walks to the building and enters on arrival (see `entering`). Without this a
                        // rally to "inside" did nothing at all whenever the spawn tile was not touching the building.
                        let goal = self
                            .find(building)
                            .map(|entity| self.tile_of(entity.position))
                            .map(|door| self.tile(door));
                        if let Some(to) = goal {
                            if let Some(entity) = self.find_mut(id) {
                                entity.task = Task::Move { to };
                            }
                            self.entering.insert(id, building);
                        }
                    }
                }
                Some(RallyPoint::Tile(x, y)) => {
                    let to = self.tile((x, y));
                    // A rally point on a resource is a gathering order: the villager goes there and works
                    // *that* node. Rallied onto gold with wood all round it, it gathers the gold or
                    // nothing - never the nearest tree (D40).
                    let node = self
                        .entities
                        .iter()
                        .find(|entity| {
                            matches!(entity.kind, EntityKind::Resource(_)) && entity.position == to
                        })
                        .map(|entity| entity.id);
                    let gathering = node.is_some_and(|node| self.start_gather(id, node).is_ok());
                    if !gathering {
                        // Nothing to work there, or this unit cannot work it (a spearman rallied onto a
                        // tree), or there is no way to reach it: go as close as the map allows.
                        self.walk_towards(id, to);
                    }
                }
                None => {}
            }
        }

        // **The job is done: clear the slot** (D157). This used to happen as a side effect of setting `training` to the next
        // queued unit; once the queue moved to `start_next_jobs` the slot stayed occupied, and the building spawned a villager
        // **every tick** - twenty of them in twenty ticks, which is how the queue test found it.
        if let Some(source) = self.find_mut(building) {
            source.training = None;
        }
        // The queue's head is taken by `start_next_jobs`, not here: the head may be a tech, and a unit that took a tech's place
        // would skip it.
        true
    }

    /// The first tile a finished unit can stand on, in a 5x5 box round its building and in a fixed scan order, or
    /// `None` when the box is full.
    ///
    /// One pass over the world marks the box's occupied tiles and the candidates are read from that mask. The older
    /// shape asked `entities.iter().any(..)` once per candidate, so a *blocked* building - the case that now repeats
    /// every tick until a tile frees - cost twenty-five scans of the whole world per tick. A garrisoned unit is inside
    /// and occupies nothing.
    fn free_spot_near(&self, origin: (u16, u16)) -> Option<(u16, u16)> {
        const SIDE: i32 = 5;
        let mut taken = [false; (SIDE * SIDE) as usize];
        // The box in raw units, checked before any division: `tile_of_raw` divides, and the whole world is outside this
        // five-tile square, so comparing raw coordinates first is the difference between two divisions per entity per
        // tick and two per entity *in* the box (measured: the pause cost 2.4 us/tick at 148 entities before this).
        let scale = self.scale;
        let (lo_x, hi_x) = (
            (i64::from(origin.0) - 2) * scale,
            (i64::from(origin.0) + 3) * scale,
        );
        let (lo_y, hi_y) = (
            (i64::from(origin.1) - 2) * scale,
            (i64::from(origin.1) + 3) * scale,
        );
        for entity in &self.entities {
            // Anything standing somewhere holds that square: a building's own tile, a tree, another unit. A
            // garrisoned entity is inside and its position is where it went in, so it holds nothing.
            if matches!(entity.task, Task::Garrisoned { .. }) {
                continue;
            }
            let (px, py) = (entity.position.0.raw(), entity.position.1.raw());
            if px < lo_x || px >= hi_x || py < lo_y || py >= hi_y {
                continue;
            }
            let (dx, dy) = (
                px / scale - i64::from(origin.0),
                py / scale - i64::from(origin.1),
            );
            taken[((dy + 2) * i64::from(SIDE) + (dx + 2)) as usize] = true;
        }
        let walk = self.walk.as_ref()?;
        for dy in 0..=2i32 {
            for dx in 0..=2i32 {
                for (sx, sy) in [(dx, dy), (-dx, dy), (dx, -dy), (-dx, -dy)] {
                    let cand = (i32::from(origin.0) + sx, i32::from(origin.1) + sy);
                    if cand.0 < 0 || cand.1 < 0 {
                        continue;
                    }
                    let (cx, cy) = (
                        u16::try_from(cand.0).unwrap_or(u16::MAX),
                        u16::try_from(cand.1).unwrap_or(u16::MAX),
                    );
                    if !walk.contains(cx, cy) || walk.blocked(cx, cy) {
                        continue;
                    }
                    if taken[((sy + 2) * SIDE + (sx + 2)) as usize] {
                        continue;
                    }
                    // **Nobody is born inside a building.** The town centre's yard is walkable, so a busy spawn area used to
                    // put a finished unit in its own building's footprint - and a unit standing there cannot walk in to
                    // garrison, so a rally to the building silently did nothing (the owner's scout made the area busy enough
                    // to expose it, 2026-11-04).
                    if self.entities.iter().any(|entity| {
                        entity.kind == EntityKind::Building
                            && entity.hp > 0
                            && self.footprint_area(entity).contains(&(cx, cy))
                    }) {
                        continue;
                    }
                    return Some((cx, cy));
                }
            }
        }
        None
    }
}
