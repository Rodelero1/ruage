//! **Gathering**: which node a worker should take, and what happens when the one it had runs out.
//!
//! Moved out of `lib.rs` unchanged in D136. `nearest_like` is the replacement search - it asks whether a node can actually
//! be reached, which is item 9's rule and the reason a node behind a wall is not a target - and `start_gather` is the order
//! that pays for the walk.

use ruage_fixed::Fixed;

use ruage_data::MaybeFixed;

use crate::{EntityId, EntityKind, Route, SimError, Task, World, scale_fixed};

impl World {
    /// The nearest **other** node of the same kind: the same resource column for a tree or a mine, the same family for
    /// an animal (the next sheep, not a boar), the same role for a farm with food left in it. Reachability is part of
    /// the choice, because a node behind a wall is not somewhere a worker can be sent (item 9's rule).
    pub(crate) fn nearest_like(
        &self,
        from: (Fixed, Fixed),
        old: EntityId,
        worker: EntityId,
    ) -> Option<EntityId> {
        self.nearest_like_in(from, old, worker, false)
    }

    /// **The nearest other node of the same kind that nobody else is working** (D182). The owner's rule: when the ring round a square is full,
    /// the extra worker takes the next source rather than crowding in - so a node another villager is already gathering from does not count.
    pub(crate) fn nearest_like_free(
        &self,
        from: (Fixed, Fixed),
        old: EntityId,
        worker: EntityId,
    ) -> Option<EntityId> {
        self.nearest_like_in(from, old, worker, true)
    }

    /// Is some other living worker already gathering from this node?
    /// **Is this source taken?** - nine men a square (D186), not one (D182).
    ///
    /// D182's *"one worker a square"* is written here a second time, as the picker's question, and it is what made a villager
    /// whose patch ran dry **stand still beside a forest with places free**: every node with a single man on it looked full.
    /// Measured - a woodcutter with a large patch two tiles away, holding two men of nine, ended with `Task::Idle`.
    fn claimed_by_other(&self, node: EntityId, worker: EntityId) -> bool {
        let on_node = self
            .entities
            .iter()
            .filter(|other| {
                other.id != worker
                    && other.hp > 0
                    && matches!(
                        other.task,
                        Task::Gather { node: target, .. } | Task::Return { node: target, .. } if target == node
                    )
            })
            .count();
        on_node
            >= crate::touching_place_count(
                &self.resource_tiles(node),
                self.body_radius(self.find(worker).and_then(|entity| entity.unit)),
                self.scale,
            )
    }

    fn nearest_like_in(
        &self,
        from: (Fixed, Fixed),
        old: EntityId,
        worker: EntityId,
        skip_claimed: bool,
    ) -> Option<EntityId> {
        let source = self.find(old)?;
        let kind = source.kind;
        // **The worker only chooses among what its player has scouted** (D148). Without this a node that runs dry sends the
        // villager to the next node *in the world*, which is how villagers crossed the map to berries nobody had found.
        let who = self.find(worker)?.player;
        // **And only what is nearby** (D149): the owner's reach, the worker's line of sight plus five tiles. Beyond it the
        // worker stands idle rather than walking the map.
        let reach = self.gather_reach(worker);
        let unit = source.unit;
        let family = unit.and_then(|unit| self.units.as_ref().map(|units| units.root(unit)));
        // A farm is a **building** that fills the same column a bush does, so "the next source of food" has to cross the
        // two kinds. Without this a forager whose berries are gone has nowhere to go: eighteen of them stood on the empty
        // berry patch at 2,400 s and the match never spent another stick of wood.
        let farm_slot = self
            .economy
            .as_ref()
            .and_then(|economy| economy.farm.resource.as_deref())
            .and_then(|name| self.slot_of(name));
        // **Cheap first, expensive last, and only for the nearest few.** Reachability costs a path search per candidate, and
        // this function is asked every time a node runs dry - over a map with 118 nodes that was 236 path searches for one
        // villager, and the acceptance run's slow ticks spent **1,336 path searches a tick** in exactly this loop. The sort is
        // distance-only; the walk is asked of the nearest eight, which is more than a worker will ever pass up in practice.
        let mut candidates: Vec<(i64, u32, EntityId)> = Vec::new();
        for entity in &self.entities {
            if entity.id == old || entity.id == worker {
                continue;
            }
            if matches!(entity.kind, EntityKind::Resource(_)) && !self.node_workable(entity.id) {
                continue;
            }
            if skip_claimed && self.claimed_by_other(entity.id, worker) {
                continue;
            }
            if !self.knows(who, entity) {
                continue;
            }
            let (dx, dy) = (
                entity.position.0.raw() - from.0.raw(),
                entity.position.1.raw() - from.1.raw(),
            );
            if dx * dx + dy * dy > reach * reach {
                continue;
            }
            let same = match (kind, entity.kind) {
                (EntityKind::Resource(slot), EntityKind::Resource(other)) => slot == other,
                // ... and a finished farm, for the column a farm fills.
                (EntityKind::Resource(slot), EntityKind::Building) => {
                    farm_slot == Some(slot)
                        && entity.construction.is_none()
                        && entity.amount.raw() > 0
                }
                (EntityKind::Unit, EntityKind::Unit) => {
                    // Another animal of the same family, and still alive: a carcass is not something to hunt.
                    entity.hp > 0
                        && unit.is_some_and(|unit| {
                            entity.unit.is_some_and(|other| {
                                other == unit
                                    || family.is_some_and(|family| {
                                        self.units
                                            .as_ref()
                                            .is_some_and(|units| units.root(other) == family)
                                    })
                            })
                        })
                }
                (EntityKind::Building, EntityKind::Building) => {
                    // Another finished farm with food left in it.
                    entity.construction.is_none()
                        && entity.amount.raw() > 0
                        && unit.is_some_and(|unit| {
                            entity.unit.is_some_and(|other| self.is_family(other, unit))
                        })
                }
                _ => false,
            };
            if !same {
                continue;
            }
            let dx = entity.position.0.raw() - from.0.raw();
            let dy = entity.position.1.raw() - from.1.raw();
            candidates.push((dx * dx + dy * dy, entity.id.0, entity.id));
        }
        candidates.sort_unstable();
        candidates.truncate(8);
        // Reachability is part of the choice, and a node's own tile is usually **blocked** (a tree, a mine, a farm), so the
        // question is whether a worker can reach somewhere it can *work* it - the same tile `work_position` picks when the
        // gather order is given. Asking `can_reach(node.position)` refused every tree, which is how the first version sent
        // every lumberjack to Idle.
        for (_, _, id) in candidates {
            let Some(entity) = self.find(id) else {
                continue;
            };
            if self.can_reach(from, entity.position)
                || self
                    .work_position(from, entity.position)
                    .is_some_and(|stand| self.can_reach(from, stand))
            {
                return Some(id);
            }
        }
        None
    }

    /// What may be gathered from an entity, and whether the thing being worked sets the pace: a farm
    /// does (its own work rate), a tree does not (the villager's variant does).
    ///
    /// The farm is content's idea (D27). The engine knows two kinds of gatherable thing only because
    /// `[farm]` names a building and the resource it yields.
    pub(crate) fn gather_source(
        &self,
        node: EntityId,
    ) -> Result<(String, Option<MaybeFixed>), SimError> {
        let entity = self
            .find(node)
            .cloned()
            .ok_or(SimError::MissingEntity(node))?;
        if let EntityKind::Resource(slot) = entity.kind {
            let name = self
                .resources
                .as_ref()
                .and_then(|resources| resources.name(slot))
                .ok_or_else(|| SimError::NoVariant {
                    task: format!("slot {slot}"),
                    detail: "the content pack names no resource in this column".to_owned(),
                })?;
            return Ok((name.to_owned(), None));
        }
        let economy = self.economy.as_ref().ok_or(SimError::NoTables)?;
        // A live animal is a source of food, and its meat is what it carries (D70). Content names which
        // units are animals, so the engine never learns what a sheep is. A body is not a source: only a
        // living animal yields, and what it leaves is the carcass rule's business (level 3 of this slice).
        if entity.kind == EntityKind::Unit
            && let Some(unit) = entity.unit
            && self.units.as_ref().is_some_and(|units| {
                let root = units.root(unit);
                // Alive, it is an animal; dead, it is a body - and a body is gathered from with no health at
                // all, which is why the carcass list is content rather than an inference (D70).
                self.animals.contains(&root) && entity.hp > 0 || self.carcasses.contains(&root)
            })
        {
            let resource = economy
                .hunt
                .resource
                .clone()
                .ok_or_else(|| SimError::NoVariant {
                    task: "hunt".to_owned(),
                    detail: "[hunt] names no resource for an animal to yield".to_owned(),
                })?;
            return Ok((resource, None));
        }
        let farm_unit = economy
            .farm
            .unit
            .as_ref()
            .and_then(|role| economy.role.get(role).copied());
        if farm_unit.is_none() || entity.kind != EntityKind::Building || entity.unit != farm_unit {
            return Err(SimError::NotAResource { id: node });
        }
        if entity.construction.is_some() {
            // A plot holds its food from the moment it is marked out, but nothing comes out of it
            // until it is finished.
            return Err(SimError::BuildingNotReady { id: node });
        }
        let resource = economy
            .farm
            .resource
            .clone()
            .ok_or_else(|| SimError::NoVariant {
                task: "farm".to_owned(),
                detail: "[farm] names no resource for the farm to yield".to_owned(),
            })?;
        // `use_villager_rate` picks who sets the pace: the building's own work rate, or the
        // variant's. It is a data question, so content holds the answer.
        let rate = (!economy.farm.use_villager_rate).then_some(economy.farm.rate);
        Ok((resource, rate))
    }

    /// A **player's** gather order: the one place where the owner's *"if there is no space at half diameter ... try another nearest resource of
    /// the same kind"* is applied (D182).
    pub(crate) fn start_gather(
        &mut self,
        villager: EntityId,
        node: EntityId,
    ) -> Result<(), SimError> {
        self.start_gather_with(villager, node, true)
    }

    /// An **internal** re-task (a deposit completing, a node running dry, a share being reassigned). It does not spread workers: those paths run
    /// many times a tick, and spreading there took the acceptance run from 16 s to over 180 s - each attempt plans a route per candidate.
    pub(crate) fn start_gather_internal(
        &mut self,
        villager: EntityId,
        node: EntityId,
    ) -> Result<(), SimError> {
        self.start_gather_with(villager, node, false)
    }

    fn start_gather_with(
        &mut self,
        villager: EntityId,
        node: EntityId,
        spread: bool,
    ) -> Result<(), SimError> {
        // A deliberate order supersedes a share reassignment that was waiting on a deposit: without this, a villager told
        // to do something else would still be yanked to the column it was pulled towards, once its load went in.
        self.pending_gather.remove(&villager);
        // Whose techs apply: the villager's owner's (a rate modifier is the worker's, not the node's).
        let worker_player = self
            .find(villager)
            .map(|entity| entity.player)
            .ok_or(SimError::MissingEntity(villager))?;

        // **One farmer a farm** (the owner, 2026-11-04: *"Only 1 farmer per farm (can be constructed by multiple, but only 1
        // can farm once construction is finished)"*, and *"if there is no empty farm nearby, villager becomes idle"*).
        //
        // A farm is the one node that is also a workplace, so two villagers sent to one crop is not sharing a resource - it
        // is two men on four tiles, and the game does not allow it. The second order is therefore answered, not refused: the
        // villager goes to the **nearest free farm** of his own side, and stands idle when there is none.
        if self.find(node).is_some_and(|entity| self.is_farm_role(entity))
            && self.entities.iter().any(|other| {
                other.id != villager
                    && matches!(other.task, Task::Gather { node: taken, .. } | Task::Return { node: taken, .. } if taken == node)
            })
        {
            let here = self
                .find(villager)
                .map(|entity| entity.position)
                .ok_or(SimError::MissingEntity(villager))?;
            match self.nearest_free_farm(here, worker_player, villager) {
                Some(free) => return self.start_gather(villager, free),
                None => {
                    // No free plot: he stops where he is. Nothing is refused - the order was taken and answered - so the
                    // player sees a villager standing, which is what the owner asked for.
                    let worker = self
                        .find_mut(villager)
                        .ok_or(SimError::MissingEntity(villager))?;
                    worker.task = Task::Idle;
                    worker.variant = None;
                    self.paths.remove(&villager);
                    return Ok(());
                }
            }
        }
        // **The ring holds nine men** (D186, the owner's *"max 9 per each"*). The rule used to be *one worker a square*
        // (D182), which on a single tree meant **one** lumberjack and eighteen idle villagers - measured, 1 of 19. Nine slots
        // are the cap now, and a tenth man is answered the way a second one used to be: the nearest free source of the same
        // kind, or standing still when the map has none.
        let on_node = self
            .entities
            .iter()
            .filter(|other| {
                other.id != villager
                    && matches!(other.task, Task::Gather { node: taken, .. } | Task::Return { node: taken, .. } if taken == node)
            })
            .count();
        if spread
            && self
                .find(node)
                .is_some_and(|entity| matches!(entity.kind, EntityKind::Resource(_)))
            && on_node
                >= crate::touching_place_count(
                    &self.resource_tiles(node),
                    self.body_radius(self.find(villager).and_then(|entity| entity.unit)),
                    self.scale,
                )
        {
            let here = self
                .find(villager)
                .map(|entity| entity.position)
                .ok_or(SimError::MissingEntity(villager))?;
            if let Some(next) = self.nearest_like_free(here, node, villager) {
                return self.start_gather(villager, next);
            }
            let worker = self
                .find_mut(villager)
                .ok_or(SimError::MissingEntity(villager))?;
            worker.task = Task::Idle;
            worker.variant = None;
            self.paths.remove(&villager);
            return Ok(());
        }
        let (variant_id, rate_per_tick, carry_cap) = {
            let units = self.units.as_ref().ok_or(SimError::NoTables)?;
            let economy = self.economy.as_ref().ok_or(SimError::NoTables)?;

            let (resource, farm_rate) = self.gather_source(node)?;
            // The load has a column to land in, and that is checked here rather than discovered at
            // the deposit: an order naming a resource the stockpile has no column for is refused now.
            self.slot_of(&resource)
                .ok_or_else(|| SimError::UnknownResourceName {
                    name: resource.clone(),
                })?;
            // Which task the node presents. An **animal is hunted** (the hunter's bow: VMHUN carries `range = 3.0`)
            // and a **herdable is herded** (the shepherd's knife), which is the owner's rule and is why the pack names
            // both tasks. Everything else comes from content's resource -> task map, and a pack that names no hunt or
            // herd task falls back to that map rather than refusing the order.
            let animal_task = match self.find(node).map(|entity| (entity.kind, entity.unit)) {
                Some((EntityKind::Unit, Some(unit))) if self.herdable.contains(&unit) => {
                    Some("herd")
                }
                Some((EntityKind::Unit, Some(unit))) if self.animals.contains(&unit) => {
                    Some("hunt")
                }
                _ => None,
            };
            let task_name = animal_task
                .filter(|task| economy.gather.variant.contains_key(*task))
                .map(str::to_owned)
                .or_else(|| economy.gather.resource_task.get(&resource).cloned())
                .ok_or_else(|| SimError::NoVariant {
                    task: resource.clone(),
                    detail: "no [gather.resource_task] entry for this resource".to_owned(),
                })?;
            let variant_id =
                *economy
                    .gather
                    .variant
                    .get(&task_name)
                    .ok_or_else(|| SimError::NoVariant {
                        task: task_name.clone(),
                        detail: "no such task in [gather.variant]".to_owned(),
                    })?;
            let variant = units.unit(variant_id).ok_or_else(|| SimError::NoVariant {
                task: task_name.clone(),
                detail: format!("unit {variant_id} is not in the unit table"),
            })?;
            if variant.carry == 0 {
                return Err(SimError::NoVariant {
                    task: task_name.clone(),
                    detail: "a variant that cannot carry anything cannot gather".to_owned(),
                });
            }
            let rate = match farm_rate {
                Some(rate) => {
                    rate.resolve(self.scale, "farm.rate")
                        .map_err(|err| SimError::NoVariant {
                            task: "farm".to_owned(),
                            detail: err.to_string(),
                        })?
                }
                None => variant
                    .work_rate
                    .resolve(self.scale, "work_rate")
                    .map_err(|err| SimError::NoVariant {
                        task: task_name.clone(),
                        detail: err.to_string(),
                    })?,
            };
            // Gather rate and carry capacity are resolved *here*, when the order lands, from the
            // variant and the owner's modifiers: a load already in hand keeps the numbers it started
            // with, which is what "the resources have to be gathered first" means.
            let rate = scale_fixed(
                rate,
                self.scale,
                &self.modifiers_for(worker_player, variant, "gather_rate"),
            );
            // The carry cap is content arithmetic on a whole number, so it goes through the same
            // fixed-point helper as speed and gather rate rather than through `apply_scalar` (D67).
            let carry = scale_fixed(
                Fixed::from_raw(i64::from(variant.carry) * self.scale),
                self.scale,
                &self.modifiers_for(worker_player, variant, "carry"),
            );
            // Rounded, not truncated: 10 x 1.25 is 12.5 and the game's Wheelbarrow gives 13. A
            // truncating division here would quietly take a villager's load down by one.
            let carry_cap = u32::try_from((carry.raw() + self.scale / 2) / self.scale)
                .unwrap_or(u32::from(variant.carry));
            // Exactness is the D10 invariant: a rate that does not split into whole per-tick
            // amounts would drift, so it is an error rather than a rounded accumulation.
            let rate_per_tick = rate
                .div_exact(i64::from(self.tick_rate_hz))
                .map_err(|err| SimError::NoVariant {
                    task: task_name.clone(),
                    detail: err.to_string(),
                })?;
            (variant_id, rate_per_tick, carry_cap)
        };

        let from = self
            .find(villager)
            .map(|entity| entity.position)
            .ok_or(SimError::MissingEntity(villager))?;
        let to = self
            .find(node)
            .map(|entity| entity.position)
            .ok_or(SimError::MissingEntity(node))?;
        // A building's own tile is blocked, so a worker stands *beside* it. Planning to the centre
        // would walk the villager into the building, and the walk home would then start from a
        // blocked tile and never find a route.
        let planned = if self.find(node).map(|entity| entity.kind) == Some(EntityKind::Building) {
            match self.work_position(from, to) {
                Some(stand) => self.plan_path(from, stand, 0),
                None => self.plan_path(from, to, 0),
            }
        } else {
            // The walk still ends at the neighbouring tile's centre - a route that ends anywhere else is a route the movement
            // rules may never call *arrived*, and a worker that never arrives never works. The last third of a tile is closed by
            // `settle` in the gather arm instead (D169).
            self.plan_path(from, to, 0)
        };
        let route = match planned {
            Ok(route) => route,
            Err(error) => {
                // **A node nobody can work is remembered here** (D138). The picker asks nothing - a search per candidate
                // cost 37x the run - so this refusal is the only evidence it will ever get, and `share_failed` is the book
                // both the economy pass and the script read (0 A.D.'s `lastFailedGather`). Without it a rule would take
                // the same walled-off tree every cooldown for the whole match.
                self.share_failed.insert(node, self.tick);
                return Err(error);
            }
        };
        self.paths.insert(
            villager,
            Route {
                points: route,
                ..Route::default()
            },
        );

        let worker = self
            .find_mut(villager)
            .ok_or(SimError::MissingEntity(villager))?;
        worker.variant = Some(variant_id);
        worker.task = Task::Gather {
            node,
            rate_per_tick,
            carry_cap,
            progress: Fixed::ZERO,
        };
        // **His place on the ring, taken now, kept until he is re-tasked** (D186 addendum 5). The owner's words for what went
        // wrong: *"it feels rather random"* - and it was, because the settle re-chose a place every tick, so a man's
        // destination moved as his neighbour moved. A place is a property of the order: chosen when the order lands (here),
        // released when the order changes (`place_is_live` asks the task, so nothing needs sweeping).
        if matches!(
            self.find(node).map(|entity| entity.kind),
            Some(EntityKind::Resource(_))
        ) {
            // **A place, taken when the order lands** (D188): the geometry decides it, exactly as the settle would, so two men
            // ordered in the same tick cannot both be given the same spot.
            if let Some(crate::Standoff::Point(place)) =
                self.standoff_point(from, to, node, villager, &[], false)
            {
                self.reserve_gather_place(villager, node, place);
            }
        }
        // The load is the villager's, not the node's: re-tasking onto the next tree (or the next sheep) must not spill
        // what is already in its hands. It used to be zeroed here, which was invisible until a worker could move on by
        // itself.
        Ok(())
    }
}
