//! **The step pass** (D279): one whole step a tick, and four refusals.
//!
//! This replaces `collide`'s ten-answer state machine with the original's own answer to every case:
//!
//! 1. **the order's own target** - a man sent at a body arrives beside it, and never walks through it;
//! 2. **the ground** - a step that lands inside a blocked tile is not taken; the body asks for one new route, throttled;
//! 3. **a body at rest since the route was planned** is *past* (`path+0x10` against `lastMoveTime`) and the mover walks
//!    on - that is what ends a head-on once the loser has stood for a tick;
//! 4. **anything else** refuses the step and **pauses the higher id** for 0.25-1.50 s (`setPause`, `0x8a80ed`-`0x8a8141`),
//!    which is the original's `cmp eax,esi / jle` and the whole of its tie-break.
//!
//! **The step itself is whole.** The owner's rule - *"man stands or walks. No acceleration or deceleration"* - is
//! implemented literally: the body travels exactly `speed / tick_rate` raw units along its polyline, crossing waypoints
//! inside the step and spending each one as it passes. A step ends early only when a refusal stops it.
//!
//! **What is deliberately absent**: the slide, `TakeAtRest`, the avoid set, the re-plan budget, the step fold, the
//! substituted waypoint and the per-body counters. Every one of them was invented in this tree and withdrawn by D279.

use ruage_fixed::Fixed;

use crate::{
    Entity, EntityId, EntityKind, SimError, StepDecision, Task, World, integer_sqrt, step_towards,
};

/// **How often a ground refusal may buy a new route**, in ticks.
///
/// The original re-plans on a terrain collision every update; that is affordable for it and not for a debug build of
/// this engine, where a whole-map search is thousands of tile visits. One search per man per second is far more than a
/// route needs - ground does not move - and it keeps a refused walker from spending a search on every tick.
const REPLAN_TICKS: u64 = 20;

impl World {
    /// **The movement pass.** Applies the steps the task layer asked for, in turn order, and answers each one.
    pub(crate) fn move_bodies(
        &mut self,
        deltas: Vec<(usize, EntityId, Fixed, Fixed)>,
    ) -> Result<(), SimError> {
        // Bodies told to pause by a collision this tick. Drawn and written **after** the pass: the draw comes from the
        // seeded stream and the pass only reads the world.
        let mut pauses: Vec<EntityId> = Vec::new();
        let mut moved: Vec<EntityId> = Vec::new();
        for (index, id, dx, dy) in deltas {
            if dx == Fixed::ZERO && dy == Fixed::ZERO {
                self.record_step(index, id, StepDecision::Idle);
                self.stalled.remove(&id);
                continue;
            }
            // A paused body stands its ticks out and keeps its route: the pause is a timer, not a re-plan.
            let paused = self
                .entities
                .get(index)
                .is_some_and(|entity| entity.id == id && entity.pause_remaining > 0);
            if paused {
                if let Some(entity) = self.entities.get_mut(index) {
                    entity.pause_remaining = entity.pause_remaining.saturating_sub(1);
                }
                self.record_step(index, id, StepDecision::Paused);
                continue;
            }
            // **The step is the body's own `speed / tick_rate`, not the length of the vector it was handed** (D279). The
            // task layer clips its delta at the next waypoint (`step_towards` lands exactly), so a waypoint 287 raw units
            // away arrived here as a 287-unit "step" and the tick walked 287 - a partial step by any measure. The delta is a
            // direction; the length is the rule.
            // A **unit** walks its own `speed / tick_rate`; everything else that moves through this pass - an arrow, a
            // mangonel's stone - is given its step by its own task (`projectile_step`), so the vector it was handed *is* the
            // step. Reading `speed` for those left every shot frozen at its archer's feet.
            let by_task = integer_sqrt(dx.raw() * dx.raw() + dy.raw() * dy.raw());
            let step = match self.find(id) {
                Some(entity) if entity.kind != EntityKind::Unit => by_task,
                Some(entity) => match entity.speed.div_exact(i64::from(self.tick_rate_hz.max(1))) {
                    Ok(step) => step.raw(),
                    Err(err) => {
                        return Err(SimError::Displacement {
                            id,
                            detail: err.to_string(),
                        });
                    }
                },
                None => 0,
            };
            let direction = if dx == Fixed::ZERO && dy == Fixed::ZERO {
                (Fixed::ZERO, Fixed::ZERO)
            } else {
                let length = integer_sqrt(dx.raw() * dx.raw() + dy.raw() * dy.raw()).max(1);
                (
                    Fixed::from_raw(dx.raw() * step / length),
                    Fixed::from_raw(dy.raw() * step / length),
                )
            };
            let (taken, why) = self.walk_the_step(id, direction, step, &mut pauses)?;
            self.record_step(index, id, why);
            // **A ground refusal buys one new route** - the original re-plans on a terrain collision, and the throttle keeps
            // a wall the goal is buried behind from costing a whole-map search every tick.
            if why == StepDecision::WaitGround {
                self.replan_after_ground(id);
            }
            if taken != (Fixed::ZERO, Fixed::ZERO) {
                moved.push(id);
            }
            // A refusal is a count against the order, and forty of them finish it (`finish_blocked_orders`). Nothing else
            // is counted: there is no per-route budget left to spend.
            if taken == (Fixed::ZERO, Fixed::ZERO) && why.is_a_wait() {
                let waited = self
                    .stalled
                    .get(&id)
                    .copied()
                    .unwrap_or(0)
                    .saturating_add(1);
                self.stalled.insert(id, waited);
            } else {
                self.stalled.remove(&id);
            }
        }
        // The clock, for the *past* test: a body that took a step this tick is the only thing that makes another body's
        // route stale.
        let now = self.tick;
        for id in moved {
            if let Some(entity) = self.find_mut(id) {
                entity.last_moved = now;
            }
            self.stamp_entity(id);
        }
        // **The loser of a collision is paused.** One draw per body per tick, from the seeded stream: the same body can be
        // named twice (both sides see each other) and the first write is the one that counts.
        let mut drawn = std::collections::BTreeSet::new();
        let rate = i64::from(self.tick_rate_hz);
        for id in pauses {
            if !drawn.insert(id) {
                continue;
            }
            let standing = self.find(id).is_some_and(|entity| {
                entity.kind == EntityKind::Unit && entity.pause_remaining == 0
            });
            if !standing {
                continue;
            }
            // The original: `rand()*1.25/32767 + 0.25` seconds. Its own `rand()` wrapper logs *"not considered sync
            // safe"*, so this draws from our seeded stream - the same interval, reproducible.
            let roll = self.sim_rng.next_u32();
            let millis = crate::PAUSE_MIN_MS + i64::from(roll % crate::PAUSE_SPREAD_MS);
            let ticks = u16::try_from((millis * rate / 1_000).max(1)).unwrap_or(u16::MAX);
            if let Some(entity) = self.find_mut(id) {
                entity.pause_remaining = ticks.saturating_sub(1);
            }
        }
        Ok(())
    }

    /// **One whole step along the route.** Travels `step` raw units, crossing the waypoints it passes and spending each
    /// one; stops at the first refusal and keeps whatever part of the step was already walked.
    ///
    /// Returns what was actually walked and the refusal that stopped it (`Take` when the whole step was taken).
    fn walk_the_step(
        &mut self,
        id: EntityId,
        direction: (Fixed, Fixed),
        step: i64,
        pauses: &mut Vec<EntityId>,
    ) -> Result<((Fixed, Fixed), StepDecision), SimError> {
        let Some(start) = self.position(id) else {
            return Ok(((Fixed::ZERO, Fixed::ZERO), StepDecision::Idle));
        };
        let mut here = start;
        let mut left = step;
        let mut why = StepDecision::Take;
        // The waypoints this tick's polyline would spend, kept until the step is known to stand: a refused step must give them
        // back, or the body would lose route it never walked.
        let mut spent: Vec<(Fixed, Fixed)> = Vec::new();
        // A walk crosses at most a handful of waypoints a tick; the bound is against a route that somehow never advances.
        for _ in 0..64 {
            if left <= 0 {
                break;
            }
            let waypoint = self.next_waypoint(id);
            let aim = match waypoint {
                Some(point) => point,
                // **No route left, and nothing spent yet**: the task layer's own direction, one whole step of it (an idle
                // body's drift, a shot in flight). If waypoints *were* spent this tick the body has arrived, and the rest of
                // the step is not owed to anybody - continuing along the stale heading overshot a goal by 287 raw units.
                None if here == start => (
                    Fixed::from_raw(here.0.raw() + direction.0.raw()),
                    Fixed::from_raw(here.1.raw() + direction.1.raw()),
                ),
                None => break,
            };
            let (gx, gy) = (aim.0.raw() - here.0.raw(), aim.1.raw() - here.1.raw());
            let gap = integer_sqrt(gx * gx + gy * gy);
            if gap == 0 {
                // Standing on the waypoint already: spend it and carry on.
                if let Some(point) = waypoint {
                    spent.push(point);
                    self.spend_waypoint(id);
                    continue;
                }
                break;
            }
            let take = gap.min(left);
            if take <= 0 {
                // Nothing left to spend on a leg: the loop is done. Without this, a leg whose rounded step is zero spins
                // the whole bound (64 iterations, each scanning every body) on every tick - measured, the acceptance match
                // went from 27 s to 338 s.
                break;
            }
            let (mx, my) = step_towards(here, aim, take);
            // **A whole step is `take` units long, not "whatever the per-axis rounding came to".** `step_towards` rounds each
            // axis separately, so a 45-degree heading at 800 units came out **798** - inside every tolerance except the
            // owner's own rule, which is *"man stands or walks"* and leaves no room for 2 raw units a tick of drift. Rescaling
            // the vector to the step's own length costs an integer square root and puts the error at half a unit.
            let (mx, my) = if gap > take && take > 0 {
                let length = integer_sqrt(mx * mx + my * my);
                if length > 0 && length != take {
                    let scale_axis = |value: i64| -> i64 {
                        let magnitude =
                            i128::from(value.abs()) * i128::from(take) + i128::from(length / 2);
                        let rounded = magnitude / i128::from(length);
                        i64::try_from(rounded).unwrap_or(value.abs()) * value.signum()
                    };
                    let (mut sx, mut sy) = (scale_axis(mx), scale_axis(my));
                    // **A step is never zero** while there is ground to cover: a component that rounds away leaves the body
                    // standing for a tick it was asked to walk, and the per-axis rounding did exactly that for a 1-unit take.
                    if sx == 0 && sy == 0 {
                        if mx.abs() >= my.abs() {
                            sx = mx.signum();
                        } else {
                            sy = my.signum();
                        }
                    }
                    // **Never longer than the step.** Two axes rounding up make the vector up to 1.4 raw units long, which is
                    // outside the ±1 the owner's rule allows; taking one unit off the longer axis brings it back under.
                    for _ in 0..4 {
                        let length = integer_sqrt(sx * sx + sy * sy);
                        if length <= take {
                            break;
                        }
                        if sx.abs() >= sy.abs() {
                            sx -= sx.signum();
                        } else {
                            sy -= sy.signum();
                        }
                    }
                    (sx, sy)
                } else {
                    (mx, my)
                }
            } else {
                (mx, my)
            };
            let landing = (
                Fixed::from_raw(here.0.raw() + mx),
                Fixed::from_raw(here.1.raw() + my),
            );
            if let Some(refusal) = self.refusal(id, here, landing, pauses) {
                // **A refused step is a stand, not a shorter step.** If the refusal comes after part of the tick was already
                // walked - a waypoint crossed and the next leg blocked - the body keeps its starting position for this tick.
                // Anything else leaves a tick that is neither a whole step nor a stand, which is the one thing the owner's
                // rule does not allow.
                why = refusal;
                here = start;
                // Give back the route the dry run spent, in the order it was spent.
                if let Some(route) = self.paths.get_mut(&id) {
                    for point in spent.drain(..).rev() {
                        route.points.insert(0, point);
                    }
                }
                break;
            }
            here = landing;
            left -= take;
            if let Some(point) = waypoint
                && take >= gap
            {
                spent.push(point);
                self.spend_waypoint(id);
            }
        }
        if here != start
            && let Some(entity) = self.find_mut(id)
        {
            entity.position = here;
        }
        Ok((
            (
                Fixed::from_raw(here.0.raw() - start.0.raw()),
                Fixed::from_raw(here.1.raw() - start.1.raw()),
            ),
            why,
        ))
    }

    /// **Plan the same goal again from where the body stands**, at most once every `REPLAN_TICKS`.
    ///
    /// The goal is the route's own last point, so a re-plan does not drift toward a tile centre; a search that fails leaves
    /// the old route in place, and the man stands - which is the original's answer to ground it cannot get round.
    fn replan_after_ground(&mut self, id: EntityId) {
        let Some(route) = self.paths.get(&id) else {
            return;
        };
        if self.tick.saturating_sub(route.created) < REPLAN_TICKS {
            return;
        }
        let Some(mut goal) = route.points.last().copied() else {
            return;
        };
        // **A place the body cannot get to is given up for the tile beside it** (D279). The last waypoint of a gather route
        // is the worker's reserved place, which is an off-grid point - and a point can sit somewhere no *step* reaches while
        // its tile looks fine. Refused twice in a row, the aim falls back to that tile's centre, which is ground the search
        // itself certified. Without this the same hopeless route was re-planned every 20 ticks for the rest of the match:
        // measured, a woodcutter frozen 1.36 tiles from his tree from tick 100 to 800.
        let stalled = self.stalled.get(&id).copied().unwrap_or(0);
        let scale = self.scale;
        let on_grid = goal.0.raw().rem_euclid(scale) == scale / 2
            && goal.1.raw().rem_euclid(scale) == scale / 2;
        if stalled >= u32::try_from(REPLAN_TICKS).unwrap_or(u32::MAX) && !on_grid {
            goal = self.tile_centre(self.tile_of(goal));
        }
        let mates = route.mates.clone();
        let Some(from) = self.position(id) else {
            return;
        };
        if let Ok(points) = self.plan_path(from, goal) {
            self.put_route(
                id,
                crate::Route {
                    points,
                    created: 0,
                    mates,
                },
            );
            self.repath_count += 1;
        }
    }

    /// Drop the waypoint the body is standing on.
    fn spend_waypoint(&mut self, id: EntityId) {
        if let Some(route) = self.paths.get_mut(&id)
            && !route.points.is_empty()
        {
            route.points.remove(0);
        }
    }

    /// **The four refusals**, in the original's order. `Some(decision)` means the step is not taken.
    fn refusal(
        &self,
        id: EntityId,
        _here: (Fixed, Fixed),
        landing: (Fixed, Fixed),
        pauses: &mut Vec<EntityId>,
    ) -> Option<StepDecision> {
        let entity = self.find(id)?;
        // **Only a unit is stopped by anything.** An arrow is an entity that moves through the same pass, and it flies over
        // the field - a shot whose last step lands on the building it was fired at must not be refused by the ground under
        // it. A building does not walk either.
        if entity.kind != EntityKind::Unit {
            return None;
        }
        // 2. **The ground.** A step that lands on a blocked tile is not taken. The test is the **tile**, not the disc:
        //    the original's collision block answers a terrain collision by re-planning, and its radius test's result is
        //    discarded at the movement site (`0x8a7fe7`, D210) - a disc test here wedges a man 28 raw units short of a
        //    bush's corner, refusing every step that closes the gap, which froze a gatherer for 1,200 ticks in
        //    `a_touching_tile_is_enough_to_bank`. A graze of a rounding is not worth a freeze.
        // **A world with no map knows no blocked ground** - a bare `World::new` (a unit test of the mover, a fixture that
        // only wants collision) must still be able to walk.
        let (tx, ty) = self.tile_of_raw(landing);
        let on_map = (u16::try_from(tx), u16::try_from(ty));
        let walkable =
            self.ground.is_none() || matches!(on_map, (Ok(x), Ok(y)) if self.tile_walkable(x, y));
        if !walkable {
            return Some(StepDecision::WaitGround);
        }
        let target = Self::order_target(entity);
        let mut blocked = false;
        for other in self
            .entities
            .iter()
            .filter(|other| other.kind == EntityKind::Unit && other.id != id)
        {
            let reach = self.reach_raw(entity.unit, other.unit);
            let (dx, dy) = (
                other.position.0.raw() - landing.0.raw(),
                other.position.1.raw() - landing.1.raw(),
            );
            if dx * dx + dy * dy >= reach * reach {
                continue;
            }
            // 1. The body the order was given to is never walked through.
            if target == Some(other.id) {
                return Some(StepDecision::WaitTarget);
            }
            // A moving group walks over itself (the original's selection exemption, D207).
            if self.is_mate(id, other.id) {
                continue;
            }
            // 3. A collider that has not moved since this route was planned is *past*: the route already accounts for it.
            if self.is_past(id, other.id) {
                continue;
            }
            // **A body that is not going anywhere is past** (D279): one standing its pause out, or one with no route left
            // (a worker at his place, a man who has arrived). Without this, a crowd chains - a paused loser re-pauses
            // whoever walks into him - and the acceptance match went from **27 s to over nine minutes** in a debug build,
            // because a ring of villagers is permanently inside each other's reach. The original drops a collider its
            // classifier calls *past* and never pauses twice for the same obstruction; this is that rule with the only
            // evidence the model has for "moving".
            let going_somewhere = self
                .paths
                .get(&other.id)
                .is_some_and(|route| !route.points.is_empty());
            if other.pause_remaining > 0 || !going_somewhere {
                continue;
            }
            // 4. Anything else: **the older unit wins and walks on, the younger stands.** The original pauses the higher id
            //    (`cmp eax,esi / jle`) and *discards* the retraction its radius test computes (`0x8a7fe7`, D210), so the
            //    winner's step is taken through the loser - which is also why the pair cannot deadlock: one of them is
            //    always allowed to move.
            if other.id.0 > id.0 {
                pauses.push(other.id);
                continue;
            }
            blocked = true;
        }
        blocked.then_some(StepDecision::WaitBlocked)
    }

    /// The entity an order is trying to reach, when the order has one. `Move` has none - its goal is a place.
    fn order_target(entity: &Entity) -> Option<EntityId> {
        match entity.task {
            Task::Attack { target, .. } => Some(target),
            Task::Gather { node, .. } => Some(node),
            Task::Return { drop, .. } => Some(drop),
            Task::Build { site } => Some(site),
            _ => None,
        }
    }
}
