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

    /// **The engine's own answer to "which source is next"** (D280), for the scenario language and for a fixture that
    /// prices the choice. One question, one answer: the nearest node of the same kind as `old` that this worker knows,
    /// can reach, and can work. The scenario's `gather` action needs exactly this (D198) - and until now it could only
    /// ask its own copy of the question, which is how the two pickers came to differ.
    pub fn next_source_near(
        &self,
        from: (Fixed, Fixed),
        old: EntityId,
        worker: EntityId,
    ) -> Option<EntityId> {
        self.nearest_like(from, old, worker)
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
        // **No crossing of kinds** (D291, the owner, 2026-11-19: *"The game code itself should never hardcode such move. That
        // should be handled by AI to repurpose idle villagers. Is it the case?"* - it is). The engine's own answer to "which
        // source is next" is the nearest node of the **same kind** within the worker's reach, which is what a spent tree uses to
        // find the next tree; repurposing a man across resource *types* is the AI's job, and the game ships AI parameters for
        // exactly that (`sn-retask-gather-amount`, `sn-max-retask-gather-amount`) plus a technology that improves it
        // (`up-retask-gatherers`). A forager with no bush in reach therefore **stands** - the idle villager the player (or the
        // AI) then re-tasks.
        // **Cheap first, expensive last, and only for the nearest few** (D280). Reachability costs a path search, and the
        // picker is asked every time a node runs out - so the order of the questions *is* the cost. Before this rule the
        // loop below asked `node_workable` of **every resource on the map** before it had sorted them: measured on the
        // acceptance map, **114 path questions a pick** and **13 ms of a tick**, and **66 routes to name one tree on a map
        // of 66** (`nine_times_the_wood_does_not_cost_nine_times_the_pick`). The expensive question is now asked of the
        // eight survivors, which is what a pick may cost: one place and one route each.
        //
        // The eight are the nearest **same-kind nodes the player has scouted and nobody else has claimed** - not the
        // nearest eight that happen to have a free tile. A wood inside a wall can sit in that list and be passed over,
        // which is the answer the old filter gave, one question cheaper.
        let mut candidates: Vec<(i64, u32, EntityId)> = Vec::new();
        for entity in &self.entities {
            if entity.id == old || entity.id == worker {
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
                // **A drained node is not a source** (D280). Matching on the column alone put a worker on an empty tree,
                // whose own re-task put him back here: measured, four villagers bounced between two spent bushes **every
                // tick** of the acceptance match (7,369 re-tasks by tick 14,400) at a route each, 59.8 s of a 155 s run.
                (EntityKind::Resource(slot), EntityKind::Resource(other)) => {
                    slot == other && entity.amount.raw() > 0
                }
                // A finished farm is **not** a bush (D291): the engine does not cross resource types, and the AI re-tasks.
                (EntityKind::Resource(_), EntityKind::Building) => false,
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
        // **The expensive question, asked of the eight.** A node is worked by standing **on** it (a carcass, a sheep) or
        // **beside** it (a tree, a mine, a farm). "On it" is a route to the node's own tile; "beside it" is a place that is
        // free *and* reachable - and asking `can_reach` about the node's own blocked tile was the other half of the trap:
        // the closest-tile planner answers almost any blocked goal with a whole-map search and a cheerful "yes", so a pick
        // bought a map for an answer it could not use.
        for (_, _, id) in candidates {
            let Some(entity) = self.find(id) else {
                continue;
            };
            let (tx, ty) = self.tile_of(entity.position);
            let workable = if self.tile_walkable(tx, ty) {
                self.can_reach(from, entity.position)
            } else {
                self.work_position(from, entity.position)
                    .is_some_and(|place| self.can_reach(from, place))
            };
            if workable {
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
        // **Already standing on the node's tile**: there is nothing to walk, and the search would answer `NoPath` for a start
        // that is its own goal. That used to be unreachable because the settle walked a man off a node's tile; it is reachable
        // now, because a crowd presses onto whatever ground is between it and the node.
        let planned = if self.tile_of(from) == self.tile_of(to) {
            Ok(Vec::new())
        } else if self.find(node).map(|entity| entity.kind) == Some(EntityKind::Building) {
            match self.work_position(from, to) {
                Some(stand) => self.plan_path(from, stand),
                None => self.plan_path(from, to),
            }
        } else {
            // The walk still ends at the neighbouring tile's centre - a route that ends anywhere else is a route the movement
            // rules may never call *arrived*, and a worker that never arrives never works. The last third of a tile is closed by
            // `settle` in the gather arm instead (D169).
            self.plan_path(from, to)
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
        // **A node behind a wall is refused, not walked at** (D241, kept by D279): the search answers an unreachable goal
        // with the nearest tile it settled on, so a route always exists - the question is whether it ends at the node's
        // **ground**. The measure is the gap to the footprint, not to the anchor point (D312): a route's points are tile
        // corners and an entity's position is wherever `Command::Spawn`, a scenario or a save put it, so a point measure
        // read the first free ring tile of a centre-anchored plot as 1.5 tiles away on *each* axis instead of 1.0 - and
        // refused a villager sent to a plot standing in the open (measured: 4.5 tile-squared against a 2-tile reach).
        if let Some(end) = route.last() {
            let reach = 2 * self.scale;
            if self.gap_to_footprint(*end, node) > reach * reach {
                self.share_failed.insert(node, self.tick);
                return Err(SimError::NoPath {
                    from: self.tile_of(from),
                    to: self.tile_of(to),
                });
            }
        }
        self.put_route(
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
        // **No place is taken, and none is pushed onto the route** (D293, owner 2026-11-19: *"Drop it and let the crowds jam"*).
        // The order used to end on a private seat round the node (`standoff_point`), which is what kept two men a body apart for
        // ever. The original hands out no seats: the route ends at the resource, the bodies are solid, and a man presses in until his
        // box is against the node's (D285) - which is what entitles him to take. `standoff_point` and the place book survive only
        // until the next cut removes them; nothing calls this one any more.
        // The load is the villager's, not the node's: re-tasking onto the next tree (or the next sheep) must not spill
        // what is already in its hands. It used to be zeroed here, which was invisible until a worker could move on by
        // itself.
        Ok(())
    }
}
