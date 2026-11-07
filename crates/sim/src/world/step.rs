//! **One tick**: the pass that reads the entity list and produces the next world.
//!
//! Split out of `lib.rs` under D145, verbatim. The function is still **1,566 lines** - that is stage 2's job (named phases);
//! moving the file only stops `lib.rs` being the place every reader has to scroll.

use std::collections::BTreeSet;

use ruage_fixed::Fixed;

use crate::{
    EntityId, EntityKind, Landing, MAX_PLAYERS, Passing, Repath, Rot, Route, Shot, SimError,
    Stance, Task, World, find_path_avoiding, reaches, short_range, step_towards,
};
impl World {
    /// Advance exactly one tick: apply queued commands, integrate, increment the counter.
    ///
    /// Movement is integrated in id order with an exact per-tick delta, so a unit's position
    /// after N ticks is `N * speed / tick_rate` with no accumulated error.
    pub fn step(&mut self) -> Result<(), SimError> {
        let queued = std::mem::take(&mut self.queue);
        for command in queued {
            // A command that became impossible between being accepted and landing is **refused, not fatal**: the tile
            // somebody else took this tick, the tree that fell, the price that went up. The match must not end because two
            // orders in the same tick wanted the same square - which is exactly how the acceptance run died at tick 16,008
            // (`something already stands at (18, 17)`, a market and a blacksmith both choosing it).
            let actor = Self::command_entity(&command);
            if let Err(why) = self.apply(command) {
                self.refusal = Some((actor, why, self.tick));
            }
        }

        let tick_rate = i64::from(self.tick_rate_hz);

        // Decide this tick's work in id order, then apply it. Deciding first keeps the second
        // pass free of lookups, so an entity cannot be moved *and* read from in the same tick in
        // an order-dependent way.
        // (index, id, dx, dy): the index is the fast path and the id is the check. `insert` keeps
        // the list sorted by id, so anything spawned later in the same tick shifts every index after
        // it - the id is what makes that detectable instead of silently moving the wrong entities.
        let mut deltas: Vec<(usize, EntityId, Fixed, Fixed)> =
            Vec::with_capacity(self.entities.len());
        let mut takes: Vec<(EntityId, i64, Fixed)> = Vec::new();
        // Workers chopping a standing tree, and the hit points they take off it this tick.
        let mut chops: Vec<EntityId> = Vec::new();
        let mut finished: Vec<EntityId> = Vec::new();
        let mut switches: Vec<(EntityId, Task)> = Vec::new();
        // A villager working a herdable animal, and the animal: herded after the tick's own movement.
        let mut herds: Vec<(EntityId, EntityId)> = Vec::new();
        let mut deposits: Vec<EntityId> = Vec::new();
        let mut builders: Vec<(EntityId, EntityId)> = Vec::new();
        let mut strikes: Vec<(EntityId, EntityId)> = Vec::new();
        // Workers whose node ran dry and who have somewhere else to go, decided in the pass and started after it.
        let mut retasks: Vec<(EntityId, EntityId)> = Vec::new();
        // (hunter, animal): a huntable animal in front of a villager, which is an attack rather than a take.
        let mut hunts: Vec<(EntityId, EntityId)> = Vec::new();
        // Who hit whom this tick, in order: the last blow decides the meat, and the blow that kills is always one
        // of this tick's (a victim at or below zero health is promoted before the tick ends). Nothing is carried
        // between ticks, nothing is hashed, and it is empty in any world where nobody is fighting (item 2).
        let mut blows: Vec<(EntityId, EntityId)> = Vec::new();
        // Arrows that landed this tick. They are entities, so they leave the list rather than being
        // turned idle: an idle arrow would sit in the world forever.
        let mut landed_arrows: Vec<EntityId> = Vec::new();
        // (target, damage, where the arrow was aimed): the aim point decides whether the shot is a hit
        // or a miss, so it has to survive the flight (D42).
        let mut landings: Vec<Landing> = Vec::new();
        let mut shots: Vec<Shot> = Vec::new();
        // Whoever was hit this tick. A building among them may have crossed the eject threshold, and
        // that is where its garrison comes out (D37).
        let mut ejected: Vec<EntityId> = Vec::new();
        // (hunter, target): a hunt that found somebody this tick. Started after the pass, because
        // `start_attack` plans a route and the pass holds the entity list still.
        let mut engaging: Vec<(EntityId, EntityId)> = Vec::new();
        // Patrols that reached their far end and have to turn round.
        let mut turning: Vec<EntityId> = Vec::new();
        // Defensive units whose fight left their ground: they go back to the post.
        let mut homing: Vec<EntityId> = Vec::new();
        let projectile_step = self.projectile_step;

        // Indexed, not id-collected: resolving each id afterwards is a linear scan per entity,
        // which makes the whole tick O(n^2). The entity list does not change during this loop, so
        // an index stays valid and the second pass touches each entity once.
        for (index, entity) in self.entities.iter().enumerate() {
            let (dx, dy) = match entity.task {
                Task::Attack {
                    target,
                    range,
                    min_range,
                    ..
                } => {
                    let Some(target_body) = self
                        .find(target)
                        // A tree cannot bleed, and a **body** cannot be fought: a carcass keeps the entity's
                        // id, so "the target is gone" stopped being how a fight with an animal ends (item 2).
                        // Without this an army swings at a corpse for the rest of the match.
                        .filter(|v| !matches!(v.kind, EntityKind::Resource(_)) && v.hp > 0)
                    else {
                        // The target is gone - or is now a body. The attacker stands down on the next tick.
                        finished.push(entity.id);
                        continue;
                    };
                    let (victim, victim_unit) = (target_body.position, target_body.unit);
                    // A melee attacker strikes from the tile next to its target; anything with reach
                    // strikes from as far as that reach goes, and walks until it is inside it. Both
                    // are measured on the same squared distance, in engine units, so the rule is one
                    // comparison and no square root.
                    let here = self.tile_of(entity.position);
                    let there = self.tile_of(victim);
                    let adjacent = here.0.abs_diff(there.0) <= 1 && here.1.abs_diff(there.1) <= 1;
                    let dx = victim.0.raw() - entity.position.0.raw();
                    let dy = victim.1.raw() - entity.position.1.raw();
                    let squared = dx * dx + dy * dy;
                    // A melee attacker strikes from the next tile, diagonals included, or from where its footprint
                    // touches the target's (C2) - a body stopped short by a friend's footprint is still in contact.
                    let contact = self.reach_raw(entity.unit, victim_unit);
                    let in_reach = if range.raw() > 0 {
                        // **Box to box** (D142): the dat's range is measured between the bodies' edges, so a 4x4 town
                        // centre is hit from its wall rather than its middle.
                        self.gap_squared_to(
                            entity.position,
                            self.body_radius(entity.unit),
                            target_body,
                        ) <= range.raw() * range.raw()
                    } else {
                        adjacent || squared <= contact * contact
                    };
                    // A siege engine - and a skirmisher - has a dead zone: inside it the shot is not
                    // possible however close the target is, so the shooter backs away instead of
                    // standing there. The distance is measured **to the target's edge**, because two
                    // bodies cannot share a tile: a skirmisher's one-tile dead zone would otherwise
                    // never bite on a tile grid, where the closest a neighbour can stand is one tile
                    // from centre to centre (D44).
                    let padding = if min_range.raw() > 0 {
                        self.body_radii(entity.unit, victim_unit)
                    } else {
                        0
                    };
                    let dead = min_range.raw() + padding;
                    let too_close = min_range.raw() > 0 && squared < dead * dead;
                    // A stance decides how much of this fight is the unit's own business: it can refuse
                    // the chase (stand ground), refuse to leave its post (defensive), or refuse the
                    // fight altogether (no attack, which the validator already keeps out of here).
                    let stance = self.stance_of(entity.id);
                    let off_ground = stance == Stance::Defensive
                        && self.stances.get(&entity.id).is_some_and(|(_, post)| {
                            let dx = victim.0.raw() - post.0.raw();
                            let dy = victim.1.raw() - post.1.raw();
                            dx * dx + dy * dy > self.aggro_radius * self.aggro_radius
                        });
                    if stance == Stance::NoAttack || off_ground {
                        if off_ground {
                            homing.push(entity.id);
                        }
                        (Fixed::ZERO, Fixed::ZERO)
                    } else if too_close && stance == Stance::StandGround {
                        // It cannot shoot what is against it, and it may not step back either.
                        (Fixed::ZERO, Fixed::ZERO)
                    } else if too_close {
                        let delta = entity.speed.div_exact(tick_rate).map_err(|err| {
                            SimError::Displacement {
                                id: entity.id,
                                detail: err.to_string(),
                            }
                        })?;
                        // Away from the target, along the line it already lies on: the point reflected
                        // through the attacker's own position.
                        // **In `i128`, then clamped** (D128): `raw * 2` overflows i64 only for positions no map can hold,
                        // but the clamp costs one comparison and makes the reflection total.
                        let reflect = |here: i64, there: i64| -> Fixed {
                            let point = i128::from(here) * 2 - i128::from(there);
                            Fixed::from_raw(i64::try_from(point).unwrap_or(if point > 0 {
                                i64::MAX
                            } else {
                                i64::MIN
                            }))
                        };
                        let away = (
                            reflect(entity.position.0.raw(), victim.0.raw()),
                            reflect(entity.position.1.raw(), victim.1.raw()),
                        );
                        let (dx, dy) = step_towards(entity.position, away, delta.raw());
                        (Fixed::from_raw(dx), Fixed::from_raw(dy))
                    } else if in_reach {
                        strikes.push((entity.id, target));
                        (Fixed::ZERO, Fixed::ZERO)
                    } else if stance == Stance::StandGround {
                        // In reach or not, it does not walk to the fight.
                        (Fixed::ZERO, Fixed::ZERO)
                    } else {
                        let delta = entity.speed.div_exact(tick_rate).map_err(|err| {
                            SimError::Displacement {
                                id: entity.id,
                                detail: err.to_string(),
                            }
                        })?;
                        let next = self.next_waypoint(entity.id).unwrap_or(victim);
                        let (dx, dy) = step_towards(entity.position, next, delta.raw());
                        (Fixed::from_raw(dx), Fixed::from_raw(dy))
                    }
                }
                Task::Build { site } => {
                    let Some((target, site_unit)) = self.find(site).map(|s| (s.position, s.unit))
                    else {
                        finished.push(entity.id);
                        continue;
                    };
                    // Adjacent counts: the site's own tile is where the building will be, so a builder works from the
                    // tile beside it - or close enough for the two footprints to touch (C2), which is where a footprint
                    // collision leaves a builder with a friend already on the good tile.
                    let here = self.tile_of(entity.position);
                    let there = self.tile_of(target);
                    let adjacent = here.0.abs_diff(there.0) <= 1 && here.1.abs_diff(there.1) <= 1;
                    let (gap_x, gap_y) = (
                        target.0.raw() - entity.position.0.raw(),
                        target.1.raw() - entity.position.1.raw(),
                    );
                    let reach = self.reach_raw(entity.unit, site_unit);
                    let adjacent = adjacent || gap_x * gap_x + gap_y * gap_y <= reach * reach;
                    if adjacent {
                        builders.push((entity.id, site));
                        (Fixed::ZERO, Fixed::ZERO)
                    } else {
                        let delta = entity.speed.div_exact(tick_rate).map_err(|err| {
                            SimError::Displacement {
                                id: entity.id,
                                detail: err.to_string(),
                            }
                        })?;
                        let next = self.next_waypoint(entity.id).unwrap_or(target);
                        let (dx, dy) = step_towards(entity.position, next, delta.raw());
                        (Fixed::from_raw(dx), Fixed::from_raw(dy))
                    }
                }
                Task::Return { drop, .. } => {
                    let Some((target, drop_unit)) = self.find(drop).map(|n| (n.position, n.unit))
                    else {
                        finished.push(entity.id);
                        continue;
                    };
                    let delta = entity.speed.div_exact(tick_rate).map_err(|err| {
                        SimError::Displacement {
                            id: entity.id,
                            detail: err.to_string(),
                        }
                    })?;
                    // **Touching the drop site is enough** - any side, any corner (the owner's rule, 2026-10-29). Waiting
                    // for the final step *onto* the building's own tile is what parked twenty-seven villagers beside a
                    // mill and five woodcutters beside a lumber camp for a thousand game-seconds, each holding a load
                    // and each with one waypoint left that they never took.
                    let here = self.tile_of(entity.position);
                    let there = self.tile_of(target);
                    // Touching by tile, or by footprint (C2): a queue of villagers handing in at one town centre is
                    // stacked 0.4 tiles deep, and the ones at the back are inside the ring but not on a tile centre.
                    let (gap_x, gap_y) = (
                        target.0.raw() - entity.position.0.raw(),
                        target.1.raw() - entity.position.1.raw(),
                    );
                    let reach = self.reach_raw(entity.unit, drop_unit);
                    let touching = (here.0.abs_diff(there.0) <= 1 && here.1.abs_diff(there.1) <= 1)
                        || gap_x * gap_x + gap_y * gap_y <= reach * reach
                        // **Or standing on the building's ground.** A carrier walked into the yard by `approach_tile`
                        // was two tiles from the anchor and never banked, which cost a third of the economy.
                        || self.touches_footprint(entity.position, drop);
                    if touching {
                        deposits.push(entity.id);
                        (Fixed::ZERO, Fixed::ZERO)
                    } else {
                        let next = self.next_waypoint(entity.id).unwrap_or(target);
                        let (dx, dy) = step_towards(entity.position, next, delta.raw());
                        (Fixed::from_raw(dx), Fixed::from_raw(dy))
                    }
                }
                Task::Gather {
                    node,
                    rate_per_tick,
                    carry_cap,
                    progress,
                } => {
                    let _ = progress;
                    let Some((target, target_is_building, node_unit)) = self
                        .find(node)
                        .map(|n| (n.position, n.kind == EntityKind::Building, n.unit))
                    else {
                        // The node is gone (a later increment may delete emptied nodes).
                        finished.push(entity.id);
                        continue;
                    };
                    let here = self.tile_of(entity.position);
                    let there = self.tile_of(target);
                    // A node is stood on; a building is worked from the tile beside it, because its
                    // own tile is blocked and a worker that insisted on it would walk forever.
                    // **Standing beside a node you cannot stand on counts as working it.** A tree in the forest sits on
                    // blocked terrain, so a villager that walks up to it stops on the adjacent tile with one waypoint
                    // left and never arrives: `at_node` was false for ever, `take_amount` never ran, and - because the
                    // villager was carrying a partial load - the "find the next node" branch did not fire either. Four
                    // woodcutters did this for a thousand game-seconds beside two spent trees (2026-10-29), which is
                    // why the wood column produced nothing while the food column produced 1,994.
                    let beside = target_is_building || !self.tile_walkable(there.0, there.1);
                    // **Distance, not the exact point** (C2). A body stopped by its friend's footprint stands inside the
                    // node's own tile but never on its centre, so `position == target` was a deadlock: the second worker
                    // on a carcass took nothing for the rest of the match (measured: 89 wood / 20 food at 1,600 s). Being
                    // on the node's ground, or close enough for the two footprints to touch, is being at it.
                    let (gap_x, gap_y) = (
                        target.0.raw() - entity.position.0.raw(),
                        target.1.raw() - entity.position.1.raw(),
                    );
                    let squared = gap_x * gap_x + gap_y * gap_y;
                    let reach = self.reach_raw(entity.unit, node_unit);
                    let at_node = here == there
                        || squared <= reach * reach
                        || (beside
                            && here.0.abs_diff(there.0) <= 1
                            && here.1.abs_diff(there.1) <= 1);

                    // A herdable animal is walked to the drop site by the villager working it (D70). The
                    // node was already read above, so this costs one family lookup and no second scan of the
                    // world - the first version re-found the node per gatherer per tick, which is O(units).
                    if !self.herdable.is_empty()
                        && node_unit.is_some_and(|unit| {
                            self.units
                                .as_ref()
                                .is_some_and(|units| self.herdable.contains(&units.root(unit)))
                        })
                    {
                        herds.push((entity.id, node));
                    }

                    // A **huntable** animal is shot, not gathered from, and the shot is ordered **before** the walk:
                    // a hunter that first walked onto the boar would be inside knife range at the first swing, and
                    // the bow (VMHUN, `range = 3.0`) would never matter. A herdable is walked home first and killed at
                    // the drop site, which is the herding pass's business.
                    if self.animal_at(node) == Some(false) {
                        hunts.push((entity.id, node));
                    }

                    // Standing still at the thing being worked is what entitles a worker to take
                    // from it.
                    if at_node {
                        // **A standing tree is chopped down first.** Until its hit points are gone it yields nothing;
                        // when they are, it falls where it stands and is then cut for wood (the owner's rule). A bush or
                        // a mine has no hit points, so this branch never runs for them.
                        // Only a **resource node** is felled. A farm is a building with hit points of its own, and
                        // chopping it was how the first version of this left every farmer standing in a field.
                        let standing = self
                            .find(node)
                            .is_some_and(|n| matches!(n.kind, EntityKind::Resource(_)) && n.hp > 0);
                        if standing {
                            chops.push(entity.id);
                            (Fixed::ZERO, Fixed::ZERO)
                        } else {
                            let (take, kept) = self.take_amount(entity);
                            let node_left = self.find(node).map(|n| n.amount.raw()).unwrap_or(0);
                            takes.push((entity.id, take, kept));

                            // Two reasons to head home: the load is full, or the node just gave its
                            // last. The second matters because a partial load on a dead tree never
                            // reaches the cap - the villager would stand there for the rest of the
                            // match holding wood nobody can spend.
                            let in_hand = entity.carry.raw() + take;
                            let empty_node = node_left - take <= 0;
                            let full = in_hand >= i64::from(carry_cap) * self.scale;
                            // A **full load always goes home**, whether or not anything was taken this tick. Without the
                            // first clause a villager whose cap was reached by a re-task (a load carried onto the next
                            // node) stood at that node with no room to take anything, for the rest of the match.
                            let brimming = entity.carry.raw() >= i64::from(carry_cap) * self.scale;
                            if brimming
                                || (take > 0 && (full || empty_node))
                                || (take == 0 && empty_node && in_hand > 0)
                            {
                                switches.extend(
                                    self.return_task(entity, node, rate_per_tick, carry_cap)
                                        .map(|task| (entity.id, task)),
                                );
                            }
                            if take == 0 && empty_node {
                                // Nothing in hand and nothing left here: look for the next node of the same kind before
                                // standing still. A tree holds 100 wood, so without this a lumberjack works one tree and
                                // idles for the rest of the match - which is what stopped the acceptance match at 139
                                // wood (the owner's rule: "move to next tree, next bush, next pile of gold").
                                let here = entity.position;
                                match self.nearest_like(here, node, entity.id) {
                                    Some(next) => retasks.push((entity.id, next)),
                                    None => finished.push(entity.id),
                                }
                            }
                            (Fixed::ZERO, Fixed::ZERO)
                        }
                    } else {
                        let delta = entity.speed.div_exact(tick_rate).map_err(|err| {
                            SimError::Displacement {
                                id: entity.id,
                                detail: err.to_string(),
                            }
                        })?;
                        let next = self.next_waypoint(entity.id).unwrap_or(target);
                        let (dx, dy) = step_towards(entity.position, next, delta.raw());
                        (Fixed::from_raw(dx), Fixed::from_raw(dy))
                    }
                }
                Task::Fly {
                    target,
                    shooter,
                    damage,
                    aim,
                    blast,
                } => {
                    // The step is resolved once per tick, outside the borrow: `step_towards` needs no
                    // table, so the only thing to avoid is touching `self` inside the loop.
                    let (dx, dy) = step_towards(entity.position, aim, projectile_step);
                    if dx == 0 && dy == 0 {
                        // Landing is the only thing an arrow does. Whether it hurts anything is
                        // decided after the loop, against the target as it stands then: an arrow
                        // whose target died on the way finds nothing and does not pick another.
                        landings.push(Landing {
                            shooter,
                            target,
                            damage,
                            aim,
                            blast,
                        });
                        landed_arrows.push(entity.id);
                    }
                    (Fixed::from_raw(dx), Fixed::from_raw(dy))
                }
                // Inside a building: no movement, no work, and no cooldown runs - a sheltered unit is
                // simply not on the map.
                Task::Move { to } => {
                    let delta = entity.speed.div_exact(tick_rate).map_err(|err| {
                        SimError::Displacement {
                            id: entity.id,
                            detail: err.to_string(),
                        }
                    })?;
                    let next = self.next_waypoint(entity.id).unwrap_or(to);
                    let (dx, dy) = step_towards(entity.position, next, delta.raw());
                    // **Arrived is close enough, not exactly on it.** The last step of a walk is often clipped by the ground
                    // or by a body, so an exact-equality test left a unit pacing at the doorstep of its goal for ever - which
                    // is how a builder failed to "arrive" at a site and a barracks was never finished (2026-11-04).
                    if reaches(entity.position, to, delta.raw()) {
                        finished.push(entity.id);
                    }
                    (Fixed::from_raw(dx), Fixed::from_raw(dy))
                }
                Task::Hunt { to } => {
                    // Looking for a fight first: an enemy in reach is the order's business, and the
                    // walk is only how the unit got here. Two stances never look: one cannot fight,
                    // the other does not walk to a fight it was not given.
                    let may_engage = matches!(
                        self.stance_of(entity.id),
                        Stance::Aggressive | Stance::Defensive
                    );
                    if let Some(enemy) = may_engage.then(|| self.nearest_enemy(entity)).flatten() {
                        engaging.push((entity.id, enemy));
                        (Fixed::ZERO, Fixed::ZERO)
                    } else {
                        let delta = entity.speed.div_exact(tick_rate).map_err(|err| {
                            SimError::Displacement {
                                id: entity.id,
                                detail: err.to_string(),
                            }
                        })?;
                        let next = self.next_waypoint(entity.id).unwrap_or(to);
                        let (dx, dy) = step_towards(entity.position, next, delta.raw());
                        if dx == 0 && dy == 0 && entity.position == to {
                            // Arrival: a patrol turns round, an attack-move stops.
                            if self.patrols.contains_key(&entity.id) {
                                turning.push(entity.id);
                            } else {
                                finished.push(entity.id);
                            }
                        }
                        (Fixed::from_raw(dx), Fixed::from_raw(dy))
                    }
                }
                Task::Garrisoned { .. } => (Fixed::ZERO, Fixed::ZERO),
                Task::Idle => {
                    let dx = entity.velocity.0.div_exact(tick_rate).map_err(|err| {
                        SimError::Displacement {
                            id: entity.id,
                            detail: err.to_string(),
                        }
                    })?;
                    let dy = entity.velocity.1.div_exact(tick_rate).map_err(|err| {
                        SimError::Displacement {
                            id: entity.id,
                            detail: err.to_string(),
                        }
                    })?;
                    (dx, dy)
                }
            };
            deltas.push((index, entity.id, dx, dy));
        }

        // A hunt that found somebody starts the fight and remembers the walk it interrupted.
        for (hunter, target) in engaging {
            let Some(walk) = self.find(hunter).map(|entity| entity.task) else {
                continue;
            };
            if self.start_attack(hunter, target).is_ok() {
                self.resuming.insert(hunter, walk);
            }
        }
        // A patrol at its far end swaps ends: the tile it just reached becomes the one to come back
        // to, and the tile it came from becomes the goal.
        for id in turning {
            let Some(Task::Hunt { to }) = self.find(id).map(|entity| entity.task) else {
                continue;
            };
            let Some(other) = self.patrols.insert(id, to) else {
                continue;
            };
            let route = self
                .find(id)
                .map(|entity| entity.position)
                .and_then(|from| self.plan_path(from, other, 0).ok());
            match route {
                Some(route) => {
                    self.paths.insert(
                        id,
                        Route {
                            points: route,
                            ..Route::default()
                        },
                    );
                    if let Some(entity) = self.find_mut(id) {
                        entity.task = Task::Hunt { to: other };
                    }
                }
                // An end it cannot get back to is an order that has run out, not a unit that should
                // stand there forever pretending to patrol.
                None => {
                    self.patrols.remove(&id);
                    if let Some(entity) = self.find_mut(id) {
                        entity.task = Task::Idle;
                    }
                }
            }
        }

        // A defensive unit whose fight walked off its ground goes home; that is the difference
        // between holding a chokepoint and chasing an army across the map.
        for id in homing {
            let Some(post) = self.stances.get(&id).map(|(_, post)| *post) else {
                continue;
            };
            let route = self
                .find(id)
                .map(|entity| entity.position)
                .and_then(|from| self.plan_path(from, post, 0).ok());
            match route {
                Some(route) => {
                    self.paths.insert(
                        id,
                        Route {
                            points: route,
                            ..Route::default()
                        },
                    );
                    if let Some(entity) = self.find_mut(id) {
                        entity.task = Task::Move { to: post };
                    }
                }
                None => self.stand_down(id),
            }
        }

        // Grant takes in id order, each bounded by what the node still holds *at that moment*.
        // Reading the amount before the loop would let two villagers on a one-wood tree take one
        // wood each: the node would clamp at zero and the pair would keep two.
        for worker in chops {
            let node = match self.find(worker).map(|e| e.task) {
                Some(Task::Gather { node, .. }) => node,
                _ => continue,
            };
            // **The rate is per second, and the remainder is kept** (D129). The pack says 1.5 hit points a second; at
            // 20 Hz that is 75 thousandths a tick, so the carry hands over a whole point every 13.3 ticks. Multiplying by
            // the tick rate instead - which is what this did until D129 - made it 30 points a tick, and every tree in the
            // forest fell to the first blow.
            let milli_per_second = self
                .economy
                .as_ref()
                .map_or(0, |economy| economy.gather.fell_milli_per_second);
            let hz = i64::from(self.tick_rate_hz.max(1));
            let mut loss = 0;
            if milli_per_second > 0 {
                // The carry is in **thousandths of a hit point, per tick, scaled by the tick rate** - so one whole hit
                // point is `1_000 * hz` of it. A 1.5 hp/s chop at 20 Hz adds 1,500 a tick and crosses 20,000 every
                // 13.3 ticks.
                let whole = 1_000 * hz;
                let carry = self.fell_carry.entry(node).or_insert(0);
                *carry = carry.saturating_add(milli_per_second);
                loss = *carry / whole;
                *carry %= whole;
            }
            if let Some(target) = self.find_mut(node) {
                // A tree falls when its hit points reach zero and **stays where it stands** until the wood is gone.
                let loss = i32::try_from(loss).unwrap_or(i32::MAX);
                target.hp = (target.hp - loss).max(0);
            }
            if loss == 0 && milli_per_second == 0 {
                self.fell_carry.remove(&node);
            }
        }
        for (worker, take, kept) in takes {
            let node = match self.find(worker).map(|e| e.task) {
                Some(Task::Gather { node, .. }) => node,
                _ => continue,
            };
            if let Some(entity) = self.find_mut(worker)
                && let Task::Gather { progress, .. } = &mut entity.task
            {
                *progress = kept;
            }
            let granted = match self.find_mut(node) {
                Some(target) => {
                    let available = target.amount.raw();
                    let granted = take.min(available);
                    target.amount = Fixed::from_raw(available - granted);
                    // Only a **resource node** disappears when it is emptied. A farm is a building that content refills
                    // (Horse Collar and friends), and removing it here deleted the crop instead of the node.
                    if available - granted <= 0
                        && self
                            .find(node)
                            .is_some_and(|entity| matches!(entity.kind, EntityKind::Resource(_)))
                    {
                        self.spent_nodes.insert(node);
                    }
                    granted
                }
                None => 0,
            };
            if let Some(entity) = self.find_mut(worker) {
                entity.carry = Fixed::from_raw(entity.carry.raw() + granted);
            }
        }
        // A hunt: the villager attacks the animal, and the gather order is remembered so it resumes on the body.
        for (hunter, animal) in hunts {
            self.slaughter.entry(hunter).or_insert(animal);
            let already = matches!(
                self.find(hunter).map(|entity| entity.task),
                Some(Task::Attack { target, .. }) if target == animal
            );
            if !already && let Err(error) = self.start_attack(hunter, animal) {
                self.refusal = Some((hunter, error, self.tick));
            }
        }
        // The next node of the same kind, for a worker whose own ran out. An ordinary gather order: the load in hand is
        // kept, and the walk is planned by the same code a player's click uses.
        for (worker, node) in retasks {
            if let Err(error) = self.start_gather(worker, node) {
                self.refusal = Some((worker, error, self.tick));
            }
        }
        for (id, task) in switches {
            // A worker heading home needs a route to the drop site, not just the final position.
            if let Task::Return { drop, .. } = task {
                let ends = (
                    self.find(id).map(|entity| entity.position),
                    self.find(drop).map(|entity| entity.position),
                );
                // **Home is the edge of the building, not its anchor** - see `approach_tile`.
                // **The anchor's route first, the footprint's only when that route does not exist.** Aiming every carrier
                // at the building's edge changed the flow of a whole match (176 entities and a won match became 127 and a
                // draw), so the approach is a fallback for the case that used to fail outright - a drop site whose near
                // side is built over - and not a new route for every trip.
                let route = match ends {
                    (Some(from), Some(to)) => self.plan_path(from, to, 0).ok().or_else(|| {
                        self.approach_tile(self.tile_of(from), drop)
                            .and_then(|stand| self.plan_path(from, self.tile(stand), 0).ok())
                    }),
                    _ => None,
                };
                if let Some(route) = route {
                    self.paths.insert(
                        id,
                        Route {
                            points: route,
                            ..Route::default()
                        },
                    );
                }
            } else {
                self.paths.remove(&id);
            }
            if let Some(entity) = self.find_mut(id) {
                entity.task = task;
            }
        }
        // **A shepherd strikes the animal it was sent to work, where the animal stands** (D116, superseding D70's
        // herding). A gathered herdable used to walk itself to the nearest drop site with the villager trailing behind;
        // the owner dropped that: *"Sheep are more likely to be found by scout and player and AI are more likely to direct
        // sheep to the drop site themselves."* So the animal does not steer itself, moving it is an ordinary order, and the
        // shepherd's job is only the blow - **on arrival**, not on arrival at a drop site.
        //
        // The kill is an ordinary attack, so the blow carries its attacker and the last-blow rule decides the meat; death
        // promotes the animal to its body in place, and the gather order resumes on that body below. Arriving is not
        // killing.
        for (villager, animal) in herds {
            let (Some(here), Some(there)) = (
                self.find(villager).map(|entity| entity.position),
                self.find(animal).map(|entity| entity.position),
            ) else {
                continue;
            };
            let (gap_x, gap_y) = (there.0.raw() - here.0.raw(), there.1.raw() - here.1.raw());
            let squared = gap_x * gap_x + gap_y * gap_y;
            let reach = self.reach_raw(
                self.find(villager).and_then(|entity| entity.unit),
                self.find(animal).and_then(|entity| entity.unit),
            );
            // **In reach _plus one step_, or on the animal's own ground.** A body that walks at another stops when the
            // next step would enter the pair's standoff, so it comes to rest a step *short* of it - measured at 10,399
            // against a standoff of 10,000, which is why a strict `squared <= reach^2` never fired and the shepherd stood
            // beside its sheep for the rest of the match. This is the same rule the rest of the engine uses for arrival:
            // a radius, not an equality (C2).
            let step = self
                .find(villager)
                .and_then(|entity| {
                    entity
                        .speed
                        .div_exact(i64::from(self.tick_rate_hz).max(1))
                        .ok()
                })
                .map_or(0, |delta| delta.raw());
            let touch = reach + step;
            let beside = self.tile_of(here) == self.tile_of(there);
            if !beside && squared > touch * touch {
                continue;
            }
            self.slaughter.insert(villager, animal);
            let already = matches!(
                self.find(villager).map(|entity| entity.task),
                Some(Task::Attack { target, .. }) if target == animal
            );
            if !already && let Err(error) = self.start_attack(villager, animal) {
                self.refusal = Some((villager, error, self.tick));
            }
        }
        // A shepherd whose blow has landed goes back to the work it was sent to do: the animal it was gathering is
        // now a body, and the gather order resumes on the same id (D70, item 3).
        if !self.slaughter.is_empty() {
            let pairs: Vec<(EntityId, EntityId)> = self
                .slaughter
                .iter()
                .map(|(one, other)| (*one, *other))
                .collect();
            for (villager, animal) in pairs {
                let idle = matches!(
                    self.find(villager).map(|entity| entity.task),
                    Some(Task::Idle)
                );
                if !idle {
                    continue;
                }
                match self.find(animal).map(|entity| entity.hp) {
                    // Dead: the body is waiting to be worked.
                    Some(hp) if hp <= 0 => {
                        if let Err(error) = self.start_gather(villager, animal) {
                            self.refusal = Some((villager, error, self.tick));
                        }
                        self.slaughter.remove(&villager);
                    }
                    // Still alive and no longer swinging: try again, and give up on the entry if it will not start
                    // (a refusal is recorded; re-issuing it every tick would only bury it).
                    Some(_) => {
                        if let Err(error) = self.start_attack(villager, animal) {
                            self.refusal = Some((villager, error, self.tick));
                            self.slaughter.remove(&villager);
                        }
                    }
                    None => {
                        self.slaughter.remove(&villager);
                    }
                }
            }
        }
        // Deposit whole resources; the fraction stays in hand. Dropping the remainder instead
        // would quietly destroy a slice of every trip.
        for id in deposits {
            let Some((player, task)) = self.find(id).map(|e| (e.player, e.task)) else {
                continue;
            };
            let Task::Return {
                node,
                slot,
                rate_per_tick,
                carry_cap,
                ..
            } = task
            else {
                continue;
            };
            let raw = self.find(id).map(|e| e.carry.raw()).unwrap_or(0);
            let whole = raw / self.scale;
            let slot_player = usize::from(player).wrapping_sub(1);
            if whole > 0 && slot_player < self.stock.len() {
                let slot = &mut self.stock[slot_player][slot];
                *slot = slot.saturating_add(u32::try_from(whole).unwrap_or(u32::MAX));
            }
            let node_left = self.find(node).map(|n| n.amount.raw()).unwrap_or(0);
            let scale = self.scale;
            // Going back to work needs a fresh route from the drop site: without one the villager
            // walks at the node in a straight line and straight through whatever it went around.
            if node_left > 0 {
                let ends = (
                    self.find(id).map(|entity| entity.position),
                    self.find(node).map(|entity| entity.position),
                );
                let route = match ends {
                    (Some(here), Some(there)) => self.plan_path(here, there, 0).ok(),
                    _ => None,
                };
                if let Some(route) = route {
                    self.paths.insert(
                        id,
                        Route {
                            points: route,
                            ..Route::default()
                        },
                    );
                }
            }
            // The shares moved this worker while it was carrying: the bank has it now, so it takes up the new column.
            if let Some(next) = self.pending_gather.remove(&id) {
                if let Err(error) = self.start_gather(id, next) {
                    self.refusal = Some((id, error, self.tick));
                }
                continue;
            }
            let mut spent = false;
            if let Some(entity) = self.find_mut(id) {
                entity.carry = Fixed::from_raw(raw - whole * scale);
                if node_left > 0 {
                    entity.task = Task::Gather {
                        node,
                        rate_per_tick,
                        carry_cap,
                        progress: Fixed::ZERO,
                    };
                } else {
                    entity.task = Task::Idle;
                    spent = true;
                }
            }
            if spent {
                // The node is finished: go to the next one of the same kind rather than standing down (the owner's
                // rule). This is where a lumberjack used to stop for the rest of the match after one tree.
                let here = self.find(id).map(|entity| entity.position);
                if let Some(next) = here.and_then(|here| self.nearest_like(here, node, id))
                    && let Err(error) = self.start_gather(id, next)
                {
                    self.refusal = Some((id, error, self.tick));
                }
            }
        }
        // Construction advances one builder-tick per tick of work, so the finish lands exactly on
        // `create_time x tick_rate` even where HP per second is not a whole number (a mill is
        // 600 HP over 35 s).
        for (builder, site) in builders {
            if self.work_one_tick(site) {
                let finished = self
                    .find(site)
                    .map(|building| (building.player, building.unit));
                if let Some(building) = self.find_mut(site) {
                    building.construction = None;
                }
                self.foundations.remove(&site);
                // Finished once is finished for good, for the purposes of prerequisites (D63).
                if let Some((player, Some(unit))) = finished {
                    let family = self.units.as_ref().map_or(unit, |units| units.root(unit));
                    self.built.insert((player, family));
                }
                // The builder comes to rest *and forgets its route* (D64). Setting the task to idle was
                // not enough: the waypoints stayed, and a villager that finished a house on its doorstep
                // kept walking into the building at its own speed for as long as the match ran.
                self.stand_down(builder);
                self.paths.remove(&builder);
                // The next queued site starts on the tick the last one finished, so a villager with a
                // queue never stands idle between buildings (D66).
                let next = self
                    .build_queue
                    .get_mut(&builder)
                    .and_then(|queue| queue.pop_front());
                if self
                    .build_queue
                    .get(&builder)
                    .is_some_and(|queue| queue.is_empty())
                {
                    self.build_queue.remove(&builder);
                }
                if let Some((unit, at)) = next
                    && let Err(why) = self.start_build(builder, unit, at)
                {
                    // A queued site is an order the engine gives itself, so its failure goes into state like any other
                    // (D88's rule). Two orders given in one tick are checked against a world where neither exists yet,
                    // so overlapping sites are accepted and the second can never start; `let _ =` here left the player's
                    // queued building gone with no record at all.
                    self.refusal = Some((builder, why, self.tick));
                }
            }
        }
        // Melee: a blow lands on the tick the attacker is in position, then every `reload_ticks`.
        //
        // **Blows land in order, so two units cannot trade killing blows** (D88, the owner's decision: "follow
        // original"). Each blow is applied against the world as the previous ones left it, so a unit killed earlier
        // in this same loop never swings - it is still in the list (the death sweep runs at the end of the tick) but
        // its health is already at or below zero. A blow aimed at something that died earlier finds nothing to hit.
        // Who this pass has already killed. Everything dead before this pass was swept at the end of the last tick,
        // so this list is the whole of "already gone" - and it is normally empty, which is why the guard costs two
        // comparisons per blow rather than two scans of the world (`find` is a linear search; a hundred blows on a
        // 3,000-entity map would have paid 600,000 comparisons a tick for a fact this list already knows).
        let mut killed: Vec<EntityId> = Vec::new();
        for (attacker, target) in strikes {
            if killed.contains(&attacker) || killed.contains(&target) {
                continue;
            }
            let swung = self.swing(attacker);
            let landed = self.blow_against(attacker, target, swung);
            if landed <= 0 {
                continue;
            }
            let flight = self.projectile_speed();
            let Some(source) = self.find(attacker).map(|entity| {
                (
                    entity.position,
                    entity.player,
                    entity.kind,
                    entity.unit,
                    entity.task,
                )
            }) else {
                continue;
            };
            let blast = source
                .3
                .and_then(|unit| self.blast.get(&unit).copied())
                .unwrap_or(0);
            let Some(victim_position) = self.find(target).map(|entity| entity.position) else {
                continue;
            };
            // An arrow flies to a point, so the point is the whole of the aim: at the target's feet
            // without Ballistics, at where it will be with it (D42).
            let aim = if flight > 0 && self.has_ballistics(attacker) {
                self.lead_shot(source.0, victim_position, target, flight)
            } else {
                victim_position
            };
            // ... and accuracy decides whether the archer aims *true* at all. A shot that is not aimed
            // true lands the content's scatter away, which the landing rule turns into a miss (D43).
            let aim = self.true_aim(attacker, aim);
            // Only a *ranged* blow flies. The pack's shot speed used to be applied to every blow, so a
            // knife became an arrow in the air: it landed in the `landings` loop, which carried no
            // attacker, and melee damage was therefore unattributable (D71). A melee blow lands here,
            // on the tick it is swung, where `(attacker, target)` is known.
            let ranged = matches!(source.4, Task::Attack { range, .. } if range > Fixed::ZERO);
            if ranged && flight > 0 && source.0 != aim {
                // The shot exists now and lands later. It carries the damage resolved when the order
                // was given, so an upgrade that arrives mid-flight changes nothing about it. It is
                // *recorded* here and created below, because the entity list is index-stable for the
                // length of this tick and inserting into it mid-pass would shift every later index.
                // A building looses a volley rather than a shot: its own arrow plus whatever its
                // garrison adds. Each arrow carries the *building's* damage for now - the published
                // rule has extra arrows transmit the unit's attack, and that difference is recorded
                // in docs/decisions.md rather than guessed at here (D37).
                let arrows = if source.2 == EntityKind::Building {
                    self.volley(attacker)
                } else {
                    1
                };
                for _ in 0..arrows {
                    shots.push(Shot {
                        player: source.1,
                        shooter: attacker,
                        from: source.0,
                        aim,
                        target,
                        damage: landed,
                        blast: Fixed::from_raw(blast),
                    });
                }
                continue;
            }
            // A pack whose shots are instant: the blow lands where it was aimed, and a blast still
            // splashes around that point rather than hitting one unit (D51).
            if let Some(victim) = self.find_mut(target) {
                victim.hp -= landed;
                if victim.hp <= 0 {
                    killed.push(target);
                }
            }
            blows.push((target, attacker));
            // A blow is an order to an animal (D70): it turns on whoever struck it.
            self.retaliate(target, attacker);
            let blast = source
                .3
                .and_then(|unit| self.blast.get(&unit).copied())
                .unwrap_or(0);
            for victim in self.blast_victims(aim, Fixed::from_raw(blast), target) {
                if let Some(entity) = self.find_mut(victim) {
                    entity.hp -= landed;
                    if entity.hp <= 0 {
                        killed.push(victim);
                    }
                }
                ejected.push(victim);
            }
            ejected.push(target);
        }
        // Production: one job per building, finishing exactly on its create time. A unit with
        // nowhere to stand waits inside the building rather than being lost.
        // Production and research: one pass over the entities, because both timers are the same
        // shape and the second `collect` used to cost a whole extra scan of the world every tick.
        let mut trainers: Vec<EntityId> = Vec::new();
        // Who is at their ceiling, worked out once per player that has a building in production rather
        // than once per building per tick. A paused job keeps its remaining ticks: the queue waits for
        // a unit to die or a house to finish, which is the rule (D34).
        let mut capped: [bool; MAX_PLAYERS] = [false; MAX_PLAYERS];
        let mut researchers: Vec<(EntityId, u8, u32)> = Vec::new();
        for entity in &self.entities {
            if entity.construction.is_some() {
                continue;
            }
            if entity.training.is_some() {
                trainers.push(entity.id);
            }
            if let Some(job) = entity.researching {
                researchers.push((entity.id, entity.player, job.tech));
            }
        }
        {
            let mut seen: [bool; MAX_PLAYERS] = [false; MAX_PLAYERS];
            for id in &trainers {
                if let Some(player) = self.find(*id).map(|entity| entity.player) {
                    let index = usize::from(player).wrapping_sub(1);
                    if index < MAX_PLAYERS && !seen[index] {
                        seen[index] = true;
                        capped[index] = self.population(player) >= self.population_cap(player);
                    }
                }
            }
        }
        for id in trainers {
            if self.find(id).is_some_and(|entity| {
                capped[usize::from(entity.player)
                    .wrapping_sub(1)
                    .min(MAX_PLAYERS - 1)]
            }) {
                continue;
            }
            let ready = match self.find_mut(id) {
                Some(entity) => match entity.training {
                    Some(mut job) => {
                        if job.fresh {
                            job.fresh = false;
                            entity.training = Some(job);
                            false
                        } else {
                            job.remaining_ticks = job.remaining_ticks.saturating_sub(1);
                            let done = job.remaining_ticks == 0;
                            entity.training = Some(job);
                            done
                        }
                    }
                    None => false,
                },
                None => false,
            };
            if ready {
                self.spawn_from_building(id);
            }
        }
        // Research: the same timer, and the effect lands when it reaches zero.
        for (id, player, tech) in researchers {
            let landed = match self.find_mut(id) {
                Some(entity) => match entity.researching {
                    Some(mut job) => {
                        if job.fresh {
                            job.fresh = false;
                            entity.researching = Some(job);
                            false
                        } else {
                            job.remaining_ticks = job.remaining_ticks.saturating_sub(1);
                            let done = job.remaining_ticks == 0;
                            if done {
                                entity.researching = None;
                            } else {
                                entity.researching = Some(job);
                            }
                            done
                        }
                    }
                    None => false,
                },
                None => false,
            };
            if landed {
                self.finish_research(player, tech)?;
            }
        }
        let mut resumed: Vec<EntityId> = Vec::new();
        for id in finished {
            // A unit that arrived at the door of the building it was rallied into goes in, if the building still has room
            // for it - the walk was the garrison order's first half.
            if let Some(building) = self.entering.remove(&id) {
                if self.check_garrison(id, building).is_ok() {
                    if let Some(entity) = self.find_mut(id) {
                        entity.task = Task::Garrisoned { building };
                    }
                    continue;
                }
                self.stand_down(id);
                continue;
            }
            if self.resuming.contains_key(&id) {
                // The fight ended and the unit still has a place to be: attack-move's whole point.
                resumed.push(id);
            } else {
                self.stand_down(id);
            }
        }
        for id in resumed {
            let Some(task @ Task::Hunt { to }) = self.resuming.remove(&id) else {
                continue;
            };
            let route = self
                .find(id)
                .map(|entity| entity.position)
                .and_then(|from| self.plan_path(from, to, 0).ok());
            match route {
                Some(route) => {
                    self.paths.insert(
                        id,
                        Route {
                            points: route,
                            ..Route::default()
                        },
                    );
                    if let Some(entity) = self.find_mut(id) {
                        entity.task = task;
                    }
                }
                None => {
                    if let Some(entity) = self.find_mut(id) {
                        entity.task = Task::Idle;
                    }
                }
            }
        }

        let mut reached: Vec<(EntityId, (Fixed, Fixed))> = Vec::new();
        // Whatever actually moved this tick is what can see something new (D50a).
        let mut movers: Vec<EntityId> = Vec::new();
        // The occupancy grid is only worth building on a tick where something moves; an idle world pays
        // one `any` over the delta list, which is empty of movement in the common case (D52).
        // Who has a step to take this tick: a body with nothing to do cannot move aside for anybody, which is the
        // difference between waiting a tick and waiting for ever (rules 4 and 8 in `collide`).
        let moving: BTreeSet<EntityId> = deltas
            .iter()
            .filter(|(_, _, dx, dy)| *dx != Fixed::ZERO || *dy != Fixed::ZERO)
            .map(|(_, id, _, _)| *id)
            .collect();
        let bodies = self.body_tiles();
        let mut pass = Passing {
            bodies: &bodies,
            moving: &moving,
            repaths: Vec::new(),
            made_room: Vec::new(),
            refused: Vec::new(),
        };
        for (index, id, dx, dy) in deltas {
            let (dx, dy) = self.collide(&mut pass, index, id, dx, dy);
            if dx != Fixed::ZERO || dy != Fixed::ZERO {
                movers.push(id);
            }
            // Fast path: the entity at this index is still the one we decided for. If a spawn moved
            // it, fall back to a lookup by id - slower, but correct, and only on ticks where the
            // world changed shape.
            if self.entities.get(index).map(|entity| entity.id) != Some(id) {
                let position = self
                    .find(id)
                    .expect("entity list is stable within a tick")
                    .position;
                let moved = (
                    position
                        .0
                        .checked_add(dx)
                        .map_err(|_| SimError::Overflow(id))?,
                    position
                        .1
                        .checked_add(dy)
                        .map_err(|_| SimError::Overflow(id))?,
                );
                let entity = self.find_mut(id).expect("just found");
                entity.position = moved;
                reached.push((id, moved));
                continue;
            }
            let entity = &mut self.entities[index];
            entity.position.0 = entity
                .position
                .0
                .checked_add(dx)
                .map_err(|_| SimError::Overflow(id))?;
            entity.position.1 = entity
                .position
                .1
                .checked_add(dy)
                .map_err(|_| SimError::Overflow(id))?;
            reached.push((id, entity.position));
        }
        for id in movers {
            self.stamp_entity(id);
        }

        // Bodies in the way of this tick's steps are decided in the pass and applied after it, because a repath plans a
        // route and the entity list must stay still while it is walked (the same shape as shots).
        //
        // A route that made room has spent its one nudge; the next body in the way is answered by a repath.
        for id in pass.made_room {
            if let Some(route) = self.paths.get_mut(&id) {
                route.gave_way = true;
            }
            self.made_room_count += 1;
        }

        // Re-plan around the bodies that were in the way, now that the movement pass has finished with the entity
        // list. A repath keeps what the route already avoids - those bodies are still past - and adds the new one; a
        // search that fails leaves the old route in place, so the unit waits rather than walking into the body.
        if !pass.repaths.is_empty() {
            for Repath {
                unit: id,
                body,
                goal,
            } in pass.repaths
            {
                let Some(from) = self.position(id) else {
                    continue;
                };
                let start = self.tile_of(from);
                let mut avoiding = self
                    .paths
                    .get(&id)
                    .map(|route| route.avoiding.clone())
                    .unwrap_or_default();
                if let Some(other) = body {
                    avoiding.insert(other);
                }
                let obstacles: BTreeSet<(u16, u16)> = avoiding
                    .iter()
                    .filter_map(|other| {
                        self.position(*other).map(|position| self.tile_of(position))
                    })
                    // The ground the unit already stands on is not an obstacle to plan around: an avoid set that holds
                    // it makes the search leave the tile it starts in, which produced a nine-waypoint route backwards.
                    .filter(|tile| *tile != start)
                    .collect();
                // A nudge already spent is spent for the order, not for the plan: a repath must not hand the unit a
                // second one.
                let gave_way = self.paths.get(&id).is_some_and(|route| route.gave_way);
                // **The old route's last point is the goal**, so the repath aims where the order pointed rather than at the
                // centre of a tile; the geometry then answers the whole question, avoiding the bodies by `avoid` (D98a).
                let aim = self
                    .paths
                    .get(&id)
                    .and_then(|route| route.points.last().copied())
                    .unwrap_or_else(|| self.tile_centre(goal));
                let radius = self.body_radius(self.find(id).and_then(|entity| entity.unit));
                let planned = match self.walk.as_ref() {
                    Some(walk) => short_range(walk, self.scale, from, aim, radius, &obstacles)
                        .or_else(|| {
                            find_path_avoiding(walk, start, goal, &obstacles)
                                .map(|steps| self.corners(walk, start, steps, aim))
                        }),
                    None => None,
                };
                match planned {
                    Some(points) => {
                        // A ground stall spends one of the route's two re-plans; a body repath leaves the count alone.
                        let stalls = if body.is_none() {
                            self.paths.get(&id).map_or(0, |route| route.stalls) + 1
                        } else {
                            self.paths.get(&id).map_or(0, |route| route.stalls)
                        };
                        self.paths.insert(
                            id,
                            Route {
                                points,
                                avoiding,
                                gave_way,
                                stalls,
                            },
                        );
                        self.repath_count += 1;
                    }
                    None => {
                        // The body joins the route's avoid set even though the search failed - the original's list is
                        // filled *before* the repath, and this is why it matters: without it the same hopeless search
                        // is re-run every tick, at four whole-map explorations a tick in a crowded fight.
                        self.repath_failures += 1;
                        // **A failed re-plan spends one of the route's two attempts** (D135). D127 counted the successful
                        // ones only, so ground that answers "no" was asked again on the next tick and the next: measured on
                        // the D134 branch, 2,533 failed searches in 12,000 ticks - 8.1 s of a 20 s run - and every one of
                        // them a full search over the map. A body repath needs no counter: the body joins the avoid set
                        // below and the next request finds it already known.
                        // The count lives on the route, which is where D127 put it - and a unit that has **no** route and a
                        // goal it cannot reach is exactly the case that looped: `get_mut` returned `None`, nothing was
                        // counted, and the request came back every tick. So an entry is created for it.
                        if body.is_none() || !self.paths.contains_key(&id) {
                            let stalls = self.paths.get(&id).map_or(0, |route| route.stalls) + 1;
                            let route = self.paths.entry(id).or_default();
                            route.stalls = stalls;
                        }
                        if let (Some(route), Some(other)) = (self.paths.get_mut(&id), body) {
                            route.avoiding.insert(other);
                        }
                        // A reaction the engine gives itself has no caller: its refusal goes into state, where the
                        // dump can show it, rather than being dropped (D71's lesson).
                        self.refusal = Some((
                            id,
                            SimError::NoPath {
                                from: start,
                                to: goal,
                            },
                            self.tick,
                        ));
                    }
                }
            }
        }

        // A waypoint reached is a waypoint spent. Done after the move, so a unit that lands on it
        // this tick starts toward the next one on the next tick.
        // One definition of "reached", not two: the mover aims at `next_waypoint`, so the spend test asks it - an exact
        // point (A) and a tile centre are then the same question.
        let spent: Vec<EntityId> = reached
            .iter()
            .filter(|(id, position)| self.next_waypoint(*id).is_some_and(|way| *position == way))
            .map(|(id, _)| *id)
            .collect();
        for id in spent {
            if let Some(path) = self.paths.get_mut(&id) {
                path.points.remove(0);
            }
        }

        // Shots go up, land, and leave the list, all after the movement pass: the entity list is
        // index-stable for the length of that pass, and every one of these changes its shape.
        for shot in shots {
            // A full world loses the shot rather than the tick: a battle must not be stopped by the
            // entity ceiling.
            let _ = self.spawn_projectile(shot);
        }
        for Landing {
            shooter,
            target,
            damage,
            aim,
            blast,
        } in landings
        {
            // The arrow lands where it was aimed. It hurts the target only if the target is still
            // there - within half a tile of the landing point, which is the space a unit occupies.
            // Without this an arrow could not miss, and Ballistics would have nothing to change.
            let tolerance = self.scale / 2;
            let reachable = self.find(target).is_some_and(|victim| {
                let dx = victim.position.0.raw() - aim.0.raw();
                let dy = victim.position.1.raw() - aim.1.raw();
                dx * dx + dy * dy <= tolerance * tolerance
            });
            // **A missile whose launcher died while it was in the air does no damage** (D90, the owner's rule: "if
            // trebuchet goes missile and dies, missile causes 0 damage"). The shot keeps its flight - it is an entity
            // with its own lifetime and it lands where it was aimed - but the blow it carries is worth nothing. The
            // launcher may be gone (a death sweeps the entity) or still present at or below zero (killed earlier in
            // this same tick), so both are checked.
            if self.find(shooter).is_none_or(|entity| entity.hp <= 0) {
                ejected.push(target);
                continue;
            }
            let mut struck = false;
            // A hunter's arrow is a villager's blow too, so the same slaughter rule applies to it (the owner's second
            // correction): an arrow that kills a herdable leaves the carcass, exactly as the knife does.
            let damage = self.blow_against(shooter, target, damage);
            if let Some(victim) = self.find_mut(target)
                && victim.hp > 0
                && reachable
            {
                victim.hp -= damage;
                blows.push((target, shooter));
                struck = true;
            }
            // A shot is an order to an animal just as a knife is: whoever loosed it is now its quarry.
            if struck {
                self.retaliate(target, shooter);
            }
            // ... and a shot with a blast hurts what stands around the landing point, its owner's own
            // units included: a mangonel that could not hit its own line would not need protecting.
            for victim in self.blast_victims(aim, blast, target) {
                if let Some(entity) = self.find_mut(victim) {
                    entity.hp -= damage;
                }
                blows.push((victim, shooter));
                self.retaliate(victim, shooter);
                ejected.push(victim);
            }
            ejected.push(target);
        }
        for id in landed_arrows {
            self.entities.retain(|entity| entity.id != id);
        }
        // Animals react to the blows this tick landed, so the leash is judged after them (D71).
        self.step_leashes();
        // ... and a herdable changes hands as soon as somebody's unit can see it (item 5).
        self.step_conversion();
        // Shares: idle villagers follow the table content gives (the game's own AI shape).
        self.step_shares();
        self.step_spent_nodes();
        self.step_husks();
        // A building at or below its eject fraction throws its garrison out. They are unharmed - the
        // published rule - and they land where they already stand.
        for id in ejected {
            // Copy what is needed out of the borrow before touching anything mutable.
            let Some((kind, hp, player, unit)) = self
                .find(id)
                .map(|building| (building.kind, building.hp, building.player, building.unit))
            else {
                continue;
            };
            if kind != EntityKind::Building || hp <= 0 {
                continue;
            }
            let Some(unit) = unit else { continue };
            let max = self.max_hp(player, unit).unwrap_or(hp);
            let floor = (i64::from(max) * self.garrison_eject_at.raw()) / self.scale;
            if i64::from(hp) > floor {
                continue;
            }
            let inside: Vec<EntityId> = self
                .entities
                .iter()
                .filter(|entity| entity.task == (Task::Garrisoned { building: id }))
                .map(|entity| entity.id)
                .collect();
            for unit in inside {
                if let Some(entity) = self.find_mut(unit) {
                    entity.task = Task::Idle;
                }
            }
        }

        // The dead leave the world only at the end of the tick, once every phase has finished with
        // them: an entity that dies mid-tick still has a position to update and waypoints to spend.
        // Resource nodes are exempt - their HP is zero by design and their amount is what matters.
        // Route bookkeeping dies with its owner. Without this a unit that is killed mid-walk keeps
        // its remaining waypoints forever - a long match accumulates routes nobody walks, and the
        // state hash walks them, so a dead unit's path was part of the hash.
        // A foundation placed blind over something that is already being built is cancelled the moment
        // its owner can see the tile (D61). Done before the sweep so the cancellation dies this tick.
        self.settle_blind_sites();
        let mut buried: Vec<EntityId> = Vec::new();
        // A wall's tile, remembered before the sweep: after `retain` the entity is gone and there is no
        // position left to free.
        // Only a building that is **already dead** needs its ground freed, and asking every building every tick for its
        // whole footprint would be sixteen allocations a building a tick for nothing.
        let wall_tiles: Vec<(EntityId, (u16, u16))> = self
            .entities
            .iter()
            .filter(|entity| entity.kind == EntityKind::Building && entity.hp <= 0)
            .flat_map(|entity| {
                let id = entity.id;
                self.solid_tiles(entity)
                    .into_iter()
                    .map(move |tile| (id, tile))
            })
            .collect();
        // An animal that died of a blow leaves its body where it stood, and the body keeps the entity's id so the
        // gatherer's task stays valid across the blow (D70). Done before the sweep, which then keeps it because its
        // unit is a named carcass (item 2).
        // Bodies made this tick start rotting on the *next* one: a corpse that lost a tick of food as it appeared
        // would make every arithmetic assertion about meat off by exactly one tick of rot.
        let mut new_rots: Vec<(EntityId, Rot)> = Vec::new();
        self.promote_dead_animals(&blows, &mut new_rots);
        // A body is a unit with no health, and it must survive the pass that removes the dead (D70). The set
        // holds ids as well as families, so this costs a lookup and no table walk: the first version cloned
        // the whole unit table here, once per tick.
        let bodies = self.carcasses.clone();
        // Rot happens here rather than in a pass of its own: the sweep already touches every entity once per tick,
        // and a second walk of the world to take four food off a carcass would be the same mistake the leash review
        // caught. `rots` is taken out so the closure does not borrow `self` twice.
        let mut rots = std::mem::take(&mut self.rots);
        let mut rotted: Vec<EntityId> = Vec::new();
        let now = self.tick;
        self.entities.retain_mut(|entity| {
            if let Some(rot) = rots.get(&entity.id).copied()
                && rot.per_tick > 0
            {
                entity.amount = Fixed::from_raw((entity.amount.raw() - rot.per_tick).max(0));
            }
            let is_body = entity.kind == EntityKind::Unit
                && entity.unit.is_some_and(|unit| bodies.contains(&unit));
            // A body with nothing left in it is gone: nothing eats an empty carcass, and leaving it standing is the
            // "an emptied body stays for ever" hole this closes.
            // Spent: empty meat that was rotting away, or a body that has lain for its species' span - which is the
            // only lifetime an empty carcass has, since there is nothing in it to rot (item 6).
            let spent = is_body
                && rots.get(&entity.id).is_some_and(|rot| {
                    (rot.per_tick > 0 && entity.amount.raw() <= 0) || now >= rot.expires
                });
            let keep = (!matches!(entity.kind, EntityKind::Unit | EntityKind::Building)
                || entity.hp > 0
                || is_body)
                && !spent;
            if !keep {
                if spent {
                    rotted.push(entity.id);
                }
                buried.push(entity.id);
            }
            keep
        });
        for id in rotted {
            rots.remove(&id);
        }
        self.rots = rots;
        for (id, rot) in new_rots {
            self.rots.insert(id, rot);
        }
        // A garrison goes down with its shelter: a unit inside a building that dies has nowhere to be,
        // and the alternative - ejecting it onto the rubble - would let a razed town centre disgorge
        // fifteen villagers. Their ids join `buried`, because they have bookkeeping of their own.
        let razed: BTreeSet<EntityId> = buried.iter().copied().collect();
        let inside: BTreeSet<EntityId> = self
            .entities
            .iter()
            .filter(|entity| {
                matches!(entity.task, Task::Garrisoned { building } if razed.contains(&building))
            })
            .map(|entity| entity.id)
            .collect();
        self.entities.retain(|entity| !inside.contains(&entity.id));
        buried.extend(inside);
        let something_died = !buried.is_empty();
        // Everything keyed by an entity id goes with it. The route was the first of these (a dead
        // unit kept its waypoints forever); the queue, the rally, the stance, the patrol's far end and
        // the interrupted walk are the rest, and the state hash walks all of them.
        for (id, tile) in wall_tiles {
            if buried.contains(&id)
                && let Some(walk) = self.walk.as_mut()
            {
                walk.unblock(tile.0, tile.1);
            }
        }
        for id in buried {
            self.paths.remove(&id);
            self.rallies.remove(&id);
            self.queued.remove(&id);
            self.training_pauses.remove(&id);
            self.resuming.remove(&id);
            self.patrols.remove(&id);
            self.stances.remove(&id);
            self.build_queue.remove(&id);
            self.leash.remove(&id);
            self.slaughter.remove(&id);
            self.foundations.remove(&id);
            self.slaughter.retain(|_, animal| *animal != id);
            self.herdable_ids.remove(&id);
        }

        // A seat can only disappear by dying, so the match is only ever decided on a tick where
        // something did. The idle tick pays one boolean for it.
        if something_died {
            self.decide_outcome();
        }

        self.tick += 1;

        // What each player saw this tick, stamped with the tick that has just become current, so that
        // `sighting(..).1 == tick()` means "watched right now" and the age of a belief is `tick() - at`.
        self.observe();
        Ok(())
    }
}
