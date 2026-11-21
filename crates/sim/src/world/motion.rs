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
            let mut blockers: Vec<crate::ground::Clear> = Vec::new();
            let (taken, why) =
                self.walk_the_step(id, direction, step, &mut pauses, &mut blockers)?;
            self.record_step(index, id, why);
            // **A non-past collision buys the mover a route round it** (D281). The original's epilogue for a collision it
            // counted is `"We have %d non-past collisions ... repathing"`: the mover re-plans, and the tiles its own bodies
            // hold are what the plan has to get round. It runs whether or not this step was allowed, because the winner of
            // a head-on keeps his step *and* re-plans - that is what makes a crowd spread rather than stack.
            if !blockers.is_empty() {
                self.replan_around_bodies(id, &blockers);
            }
            // **A ground refusal buys one new route** - the original re-plans on a terrain collision, and the throttle keeps
            // a wall the goal is buried behind from costing a whole-map search every tick.
            if why == StepDecision::WaitGround {
                self.replan_after_ground(id);
            }
            // **A walk whose target it has touched is over** (D286): the original's collision block says *"That collision is
            // with our target, so we're done"* and the order ends there. Other tasks that name a target - a gather, a build,
            // a fight - stand beside it and keep working, so only a walk is finished here; their own arrival rules decide.
            if why == StepDecision::WaitTarget
                && matches!(
                    self.find(id).map(|entity| entity.task),
                    Some(Task::Move {
                        target: Some(_),
                        ..
                    })
                )
            {
                self.end_walk(id);
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
                // **The drawn duration, not one tick less** (D282). The pass stands a body for as many ticks as the counter
                // holds, so writing `ticks - 1` turned the game's own 0.25 s floor into 0.20 s at 20 Hz - measured, the
                // loser's stand fell outside `the_loser_stands_for_the_drawn_pause`'s band.
                entity.pause_remaining = ticks;
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
        blockers: &mut Vec<crate::ground::Clear>,
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
            // **A step that would end inside the order's own target stops on its face** (D286). The collision with the
            // target ends the order, and the cut keeps the body at the game's contact distance instead of up to one step
            // short of it - which is what makes a gatherer's box gap zero (he is *at* the node) and a walk end exactly at
            // the radius the owner's saves measure.
            if let Some(target) = self.find(id).and_then(Self::order_target)
                && target != id
                && let Some(entity) = self.find(id)
                && let Some(contact) =
                    self.target_contact(here, landing, target, self.body_radius(entity.unit))
            {
                here = contact;
                why = StepDecision::WaitTarget;
                break;
            }
            if let Some(refusal) = self.refusal(id, here, landing, pauses, blockers) {
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

    /// **Plan the same goal again, round the bodies that refused the step** (D281).
    ///
    /// The goal is the route's own last point, so the re-plan does not drift toward a tile centre. A search that fails leaves the
    /// old route in place and the man stands, which is the original's answer to a wall of bodies he cannot get round, and what the
    /// pause is for.
    ///
    /// **The re-plan takes a FRESH stamp** (D320). `path->createdAt` is the past test's only input, and in the original a re-plan
    /// is a **new path**, so a collider that has not moved since *that* plan is past and is walked through.
    ///
    /// Keeping the old stamp made a stopped man permanently non-past: the walker re-planned on every tick it touched him, got the
    /// same route straight through him, and ended **0 raw** from the man it was passing (measured at tick 176 of
    /// `replan::a_stopped_friend_is_only_walked_through_after_the_replan`), which reads to a player as one villager standing inside
    /// another for ever.
    fn replan_around_bodies(&mut self, id: EntityId, blockers: &[crate::ground::Clear]) {
        let Some(route) = self.paths.get(&id) else {
            return;
        };
        let Some(goal) = route.points.last().copied() else {
            return;
        };
        // A new path, so a new stamp: a body that has stood still since this moment is past (`path->createdAt >= lastMoveTime`).
        let created = self.tick;
        let mates = route.mates.clone();
        let Some(from) = self.position(id) else {
            return;
        };
        if let Ok(points) = self.plan_path_avoiding(from, goal, blockers) {
            self.put_route(
                id,
                crate::Route {
                    points,
                    created,
                    mates,
                },
            );
            self.repath_count += 1;
        }
    }

    /// **The order is done**: the body is idle where it stands and its route goes with the order.
    fn end_walk(&mut self, id: EntityId) {
        if let Some(entity) = self.find_mut(id) {
            entity.task = Task::Idle;
        }
        self.paths.remove(&id);
        self.stalled.remove(&id);
    }

    /// Drop the waypoint the body is standing on.
    fn spend_waypoint(&mut self, id: EntityId) {
        if let Some(route) = self.paths.get_mut(&id)
            && !route.points.is_empty()
        {
            route.points.remove(0);
        }
    }

    /// **The refusals**, in the original's order. `Some(decision)` means the step is not taken.
    fn refusal(
        &self,
        id: EntityId,
        here: (Fixed, Fixed),
        landing: (Fixed, Fixed),
        pauses: &mut Vec<EntityId>,
        blockers: &mut Vec<crate::ground::Clear>,
    ) -> Option<StepDecision> {
        let entity = self.find(id)?;
        // **Only a unit is stopped by anything.** An arrow is an entity that moves through the same pass, and it flies over
        // the field - a shot whose last step lands on the building it was fired at must not be refused by the ground under
        // it. A building does not walk either.
        if entity.kind != EntityKind::Unit {
            return None;
        }
        let own = self.body_radius(entity.unit);
        let target = Self::order_target(entity);
        // 1. **A collision with the order's own target ends the order** (`0x127d`, *"That collision is with our target, so
        //    we're done"*). The test is the game's own: the mover's box against the thing's box (`FUN_00840830`), grown by its
        //    radius. A walk that names a house therefore stops with its body against the wall, and one that names a man stops
        //    at his shoulder - the same distance for both of the owner's clicks.
        if target.is_some_and(|thing| thing != id && self.body_touches(thing, landing, own)) {
            return Some(StepDecision::WaitTarget);
        }
        // 2. **The ground.** Impassable terrain is the game's **point test** (`FUN_008aa480` - a tile, no radius), which is
        //    what `tile_walkable` answers. The *things* on the ground are **boxes**, and the mover's box may not enter them
        //    (`FUN_00840830` grows the query box by its radius): that is the half a centre-on-the-border walk was missing,
        //    and it is the same ruler the plan uses, so a route the plan handed over is a route the step accepts.
        // **A world with no map knows no blocked ground** - a bare `World::new` (a unit test of the mover, a fixture that
        // only wants collision) must still be able to walk.
        let (tx, ty) = self.tile_of_raw(landing);
        let on_map = (u16::try_from(tx), u16::try_from(ty));
        let walkable =
            self.ground.is_none() || matches!(on_map, (Ok(x), Ok(y)) if self.tile_walkable(x, y));
        if !walkable {
            return Some(StepDecision::WaitGround);
        }
        // **And the objects' boxes, by the game's own test**: `FUN_00840830` grows the query box by the mover's radius and
        // collides on `<=`, so a landing that *touches* a tree, a wall or a footprint is refused - not only one that
        // penetrates it. Two clauses, because the game never has a body inside an object (its placement phase prevents it)
        // while ours can start that way, and a plain yes/no would nail such a body to the spot:
        //
        // 1. a landing that touches an object tile the body does **not** already touch is refused;
        // 2. a landing that takes the body **deeper** into one it does touch is refused too, so it may slide out but not in.
        // **The ruling is in, and the step does not follow it yet** (`docs/parity-plan.md` P3, D330): the binary's test collides on
        // **touching** (`FUN_00840830`, four `<=`), while this clause refuses only a landing that *penetrates* a new object. D306 settled
        // the apparent contradiction - the owner measured a packed trebuchet threading a one-tile hole, so touching-as-collision and a
        // passable one-tile gap are both true - which means the box this test grows is **not** the body's nominal radius: a body whose
        // radius is half a tile still has to fit a lane one tile wide. P3 therefore lands with the plan/step agreement, and the
        // one-tile-gap fixture has to stay green through it.
        if let Some(ground) = self.ground.as_ref() {
            let landing_depth =
                ground.object_depth(self.scale, (landing.0.raw(), landing.1.raw()), own);
            if landing_depth > 0 {
                let here_depth = ground.object_depth(self.scale, (here.0.raw(), here.1.raw()), own);
                if landing_depth > here_depth {
                    return Some(StepDecision::WaitGround);
                }
            }
        }
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
            // **The path-ignore list** (`0x1242`, *"That's a path ignore unit"*): the mover's own group, and only while
            // its members are **moving** - the binary builds that list from the route's objects whose `isMoving`
            // (`+0x2a0`) answers 1 or whose `+0x150` answers 1, and a stopped unit's velocity is zero. So a stopped mate
            // is not ignored: he blocks, which is what "units give up their formation when the stop button is pressed" means.
            let moving = self
                .paths
                .get(&other.id)
                .is_some_and(|route| !route.points.is_empty())
                && other.pause_remaining == 0;
            if moving && self.is_mate(id, other.id) {
                continue;
            }
            // 3. A collider that has not moved since this route was planned is *past*: the route already accounts for it.
            if self.is_past(id, other.id) {
                continue;
            }
            // **A body that has stopped is not a ghost** (D281). D279 read its silence as an exemption - a paused man or one
            // with no route left was walked through - and that made every stopped man pass-through-able, which is the
            // opposite of the original: its path-ignore list is the mover's *moving* mates, so a stopped unit **blocks**.
            // What answers the collision is the re-plan below (the collider's box joins the avoid list); the pause is what
            // stops the pair colliding again while the route is re-made.
            // **The clearance, not the tile** (D282), and it is a **box** (D286): the other body's own radius, grown by the
            // mover's. A tile list could not express 0.4 of a tile and the route cut the corner.
            let half = match self.body_radius(other.unit) {
                0 => self.scale / 2,
                own_other => own_other,
            };
            if !blockers
                .iter()
                .any(|clear| clear.at == (other.position.0.raw(), other.position.1.raw()))
            {
                blockers.push(crate::ground::Clear {
                    at: (other.position.0.raw(), other.position.1.raw()),
                    half: (half, half),
                    reach: own,
                });
            }
            // 4. **The higher id is told to pause** (`cmp eax,esi / jle`, `0x8a80ff`; `setPause(0.25-1.50 s)` at
            //    `0x8a8141`). Nothing else stops the mover: the original's epilogue **never writes a position**, so the
            //    step is taken and the radius retraction it computes is discarded (`0x8a7fe7`). D279 added a second brake
            //    here - *"the older unit wins and walks on, the younger stands"*, `blocked = true` - which the binary does
            //    not have. In a crowd every man stood for every lower-id neighbour and nobody drained: confirmed by the
            //    owner on the test branch (*"villagers get stuck"*) and by the acceptance crew stopping at **7 then 7**.
            // **A pause that is running is not re-armed** - and the one written *this* tick is still only in `pauses` while the
            // pass runs, so both have to be checked. Without the guard the mover re-arms his friend's pause on every refused
            // tick and the loser of a head-on never resumes: measured before it, **1876 ticks (93.8 s)** against the original's
            // 0.25-1.50 s.
            if other.id.0 > id.0 && other.pause_remaining == 0 && !pauses.contains(&other.id) {
                pauses.push(other.id);
            }
        }
        None
    }

    /// The entity an order is trying to reach, when the order has one. **A walk has one now** (D286): the object the click
    /// landed on, if it landed on anything - which is what lets the original's *"That collision is with our target, so we're
    /// done"* arm fire for a move at all.
    fn order_target(entity: &Entity) -> Option<EntityId> {
        match entity.task {
            Task::Attack { target, .. } => Some(target),
            Task::Gather { node, .. } => Some(node),
            Task::Return { drop, .. } => Some(drop),
            Task::Build { site } => Some(site),
            Task::Move { target, .. } => target,
            _ => None,
        }
    }
}
