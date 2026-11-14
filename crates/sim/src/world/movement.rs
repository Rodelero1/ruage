//! **Collision**: what a step may do about the ground and about the bodies standing on it.
//!
//! Moved out of `lib.rs` unchanged (D136). `collide` is the only entry point the tick uses; everything else answers one
//! question about a candidate step - is the ground under it walkable, which body is in the way, and what a mover does when
//! a body refuses to move.

use ruage_fixed::Fixed;

use crate::{
    Entity, EntityId, EntityKind, MAX_GROUND_REPLANS, MAX_REPATHS_PER_TICK, Passing, Repath,
    StepDecision, Task, World,
};

impl World {
    /// The bodies standing on one tile, from the sorted slice.
    fn bodies_at<'a>(
        &self,
        bodies: &'a [(i64, i64, EntityId, usize, i64)],
        tile: (i64, i64),
    ) -> &'a [(i64, i64, EntityId, usize, i64)] {
        let start = bodies.partition_point(|(x, y, _, _, _)| (*x, *y) < tile);
        let end = bodies.partition_point(|(x, y, _, _, _)| (*x, *y) <= tile);
        &bodies[start..end]
    }

    /// Where a body stands **now**: through its index while the entity list still holds it there, and by id when a spawn
    /// moved it during the tick.
    fn body_at(&self, index: usize, id: EntityId) -> Option<(Fixed, Fixed)> {
        match self.entities.get(index) {
            Some(entity) if entity.id == id => Some(entity.position),
            _ => self.position(id),
        }
    }

    /// The nearest body whose footprint the candidate position would stand inside, with its position - every step is
    /// decided against it (C).
    ///
    /// The **nine tiles around the landing point** are the whole search: no standoff in this content is wider than one
    /// tile (two 0.2 radii, and the one-tile fallback for content that gives none), so a body in a further tile cannot
    /// reach the candidate. Nearest wins, ties broken by id, so two runs pick the same body.
    fn blocker_at(
        &self,
        bodies: &[(i64, i64, EntityId, usize, i64)],
        id: EntityId,
        radius: i64,
        candidate: (Fixed, Fixed),
    ) -> Option<(EntityId, (Fixed, Fixed))> {
        let (tx, ty) = self.tile_of_raw(candidate);
        let mut best: Option<(EntityId, (Fixed, Fixed))> = None;
        // Nearest first, ties by id, so a crowd round one point always names the same body and two runs agree.
        let mut best_key: Option<(i64, u32)> = None;
        for dy in -1..=1i64 {
            for dx in -1..=1i64 {
                for (_, _, other, index, other_radius) in self.bodies_at(bodies, (tx + dx, ty + dy))
                {
                    if *other == id {
                        continue;
                    }
                    let Some(position) = self.body_at(*index, *other) else {
                        continue;
                    };
                    // The pair's standoff, from the two radii already resolved for this tick.
                    let reach = match radius + other_radius {
                        0 => self.scale,
                        sum => sum,
                    };
                    let (gap_x, gap_y) = (
                        position.0.raw() - candidate.0.raw(),
                        position.1.raw() - candidate.1.raw(),
                    );
                    let squared = gap_x * gap_x + gap_y * gap_y;
                    if squared >= reach * reach {
                        continue;
                    }
                    let key = (squared, other.0);
                    if best_key.is_none_or(|old| key < old) {
                        best = Some((*other, position));
                        best_key = Some(key);
                    }
                }
            }
        }
        best
    }

    /// Whether a unit may **walk** this step: the tile it lands in is walkable ground, and a diagonal does not cut a
    /// corner the search would refuse - the same both-corners rule `line_clear` applies.
    ///
    /// The route is tile-legal, but the mover moves in **raw space**: a diagonal step toward the next waypoint can enter a
    /// blocked tile's corner that the Bresenham line never touches, which is how villagers walked *through* a house once
    /// houses started blocking their ground (measured: standing on the house for 19 ticks, 2026-11-03). The **goal's own
    /// tile is exempt**, the same rule the search and the straight line use - a node's tile holds the thing being worked,
    /// so a gatherer must still be able to step onto its tree.
    fn step_is_walkable(
        &self,
        here: (Fixed, Fixed),
        step: (Fixed, Fixed),
        goal: Option<(u16, u16)>,
        radius: i64,
        entering: bool,
    ) -> bool {
        let (x, y) = (here.0.raw() + step.0.raw(), here.1.raw() + step.1.raw());
        // **The body, not the centre** (D176). The owner, twice: a villager stood a hand's width from the town centre with its 0.2 disc inside the
        // wall, because this test only asked whether the *centre* landed on a blocked tile. A disc against a square is the square grown by the
        // radius - the Forgotten Empires talk's *"shrink the unit to a point, expand every other obstruction"* - so the step is refused when its
        // landing would put the body inside, and allowed when it is already inside and moving *out* (a unit pushed into a wall, or a spawn that
        // arrives touching one, must still be able to leave).
        // **A unit walking into its own building is not a body looking for room**: `entering` means it is on its way through the door, so the
        // ground rule is the old one (the centre may step where a route to the door leads). Everything else obeys the disc.
        let radius = if entering { 0 } else { radius };
        let now = self.clearance_at(here, radius);
        let next = self.clearance_at((Fixed::from_raw(x), Fixed::from_raw(y)), radius);
        // **A body already inside may shift along the wall** (D192). The rule refused any step that made the overlap deeper, which
        // is right - except that a body wedged against a corner could deepen it by a thousandth on both axes and then had no legal
        // step at all: the content pack's walker sat **0.134** from a blocked tile with a 0.2 disc, one waypoint to go, for 2,000
        // ticks. A tenth of a tile of slack is enough to slide out and still far too little to dig in.
        if next < 0 && (now >= 0 || next < now - self.scale / 10) {
            return false;
        }
        // A world with no map knows no blocked ground: a bare `World::new` (a unit test of the mover, a fixture that only
        // wants collision) must still be able to walk.
        if self.walk.is_none() {
            return true;
        }
        let free = |tile: (i64, i64)| -> bool {
            let (Ok(x), Ok(y)) = (u16::try_from(tile.0), u16::try_from(tile.1)) else {
                return false;
            };
            // **The goal is not a hole in the ground.** This used to treat the goal tile as free whatever stood on it, so a
            // unit ordered at a tree, a gold mine or a building walked *into* it and stood inside the obstacle - and from a
            // blocked tile there is no route out, so it stayed there for ever. The owner saw exactly that twice (2026-11-04:
            // "militia still get stuck in the forrest", "scout cavalry still gets stuck in gold mine"). A goal on blocked
            // ground means "come as close as you can", which the pathfinder's own fallback already arranges.
            let _ = goal;
            self.tile_walkable(x, y)
        };
        let landing = (x.div_euclid(self.scale), y.div_euclid(self.scale));
        if !free(landing) {
            return false;
        }
        // **Only a step that crosses a tile can cut a corner.** This asked the two orthogonal neighbours of a *fractional*
        // step, which is a demand no unit can satisfy while hugging a wall: every sub-tile diagonal step wanted both sides
        // free and a villager walking past a house stopped dead at x = 3 (2026-11-04, `a_villager_walks_round_a_house`).
        let crosses = step.0.raw().abs() >= self.scale || step.1.raw().abs() >= self.scale;
        if step.0.raw() != 0 && step.1.raw() != 0 && crosses {
            let from = self.tile_of_raw(here);
            if !free((from.0 + step.0.raw().signum(), from.1))
                || !free((from.0, from.1 + step.1.raw().signum()))
            {
                return false;
            }
        }
        true
    }

    /// One step against the bodies in the way (D98a, C, D205).
    ///
    /// The original queries collisions **at the new position with the unit's radius**, and this is that query: a body
    /// blocks a disc of `radius[0]`, summed for the pair, or one whole tile when the content gives neither a radius.
    /// **Every** step is tested, including one that stays inside the tile it started in - a rule that is precise at a
    /// tile boundary and blind inside the tile let two bodies 0.04 tiles apart walk together, which is why the hybrid
    /// (C1a) was measured and dropped.
    ///
    /// The answer, in this order:
    /// 1. **clear** - take the step;
    /// 2. the blocker is the **order's target**, or stands on the ground an **exclusive** order asked for - refuse and
    ///    say nothing: the arrival rules finish the order from here, and a repath cannot put a unit anywhere better
    ///    than the tile it is already on (D52, D98a, D200);
    /// 3. the mover is **already inside** the blocker's footprint - take the step, or a pair a spawn put inside each
    ///    other can never separate;
    /// 4. a **unit** in the way is answered by the original's tie-break: the **higher id is told to pause** for a
    ///    uniform 0.25-1.50 s and keeps its path, while the lower id proceeds (D205); the paused body returns before the
    ///    repath, so its waypoint list is untouched;
    /// 5. refuse and classify: a body the route already avoids is *past* and costs no repath (or a blocked unit would
    ///    re-plan every tick), anything else is non-past and earns one;
    /// 6. the blocker is **at rest** - take the step, bounded by the pair's reach, and let the repath route round it: a
    ///    body that is not going anywhere will not move aside, and waiting on it is waiting for ever;
    /// 7. otherwise **wait**.
    ///
    /// **What D205 withdrew, and what killed it.** Rules that used to sit here: a give-way mutex (`wait_or_give_way`,
    /// rule 9 and its own-tile twin), an axis and perpendicular **slide** round a body (rule 5 and the own-tile slide),
    /// and a **make-room** nudge (rule 6) - and, beside this pass, `push_the_standing`'s displacement (D200). All four
    /// were answers to "two men want the same ground", invented here because the original's answer had not been read
    /// yet. The give-way could not move a pair at one body apart at all (the bound inside `give` is zero there, measured
    /// 2026-11-13: five of seven villagers `WaitOwnTile` for a whole match, the acceptance bank at **0 food**), the
    /// slide moved a man off his own line to make room for somebody else's, and the nudge moved standing men a twentieth
    /// of a tile (measured 2026-11-14: **161** displacements in the acceptance run, 147 of them builders at their own
    /// sites). The original pauses the loser instead - bounded by a countdown, so the pair cannot stand for ever -
    /// and `docs/movement-in-the-binary.md` has the four instructions that say so.
    pub(crate) fn collide(
        &self,
        pass: &mut Passing<'_>,
        index: usize,
        id: EntityId,
        dx: Fixed,
        dy: Fixed,
    ) -> (Fixed, Fixed, StepDecision) {
        if dx == Fixed::ZERO && dy == Fixed::ZERO {
            return (dx, dy, StepDecision::Idle);
        }
        let Some(entity) = self.entities.get(index) else {
            return (dx, dy, StepDecision::Idle);
        };
        // **Only a unit collides with a body.** An arrow is an entity that moves through the same pass, and the first
        // version stopped every shot dead over the first unit it flew across - ten arrows froze mid-air over the town
        // centre. A building does not walk either, so this stays a rule about bodies.
        if entity.kind != EntityKind::Unit {
            return (dx, dy, StepDecision::Take);
        }
        let here = entity.position;
        let radius = self.body_radius(entity.unit);
        let goal = self.goal_of(entity);
        let goal_tile = goal.map(|(tile, _)| tile);
        // **Ground first.** A step that lands on blocked ground, or cuts the corner of one, is not a step - the mover
        // slides along the axis that is clear, which is how it gets round a building instead of through it. This is the
        // **terrain** slide and it stays (D205 withdraws the slide round a *body*, not the one round a wall).
        let entering = self.entering.contains_key(&id);
        if !self.step_is_walkable(here, (dx, dy), goal_tile, radius, entering) {
            for (sx, sy) in [(dx, Fixed::ZERO), (Fixed::ZERO, dy)] {
                if sx == Fixed::ZERO && sy == Fixed::ZERO {
                    continue;
                }
                if self.step_is_walkable(here, (sx, sy), goal_tile, radius, entering) {
                    return (sx, sy, StepDecision::Slide);
                }
            }
            // **Ground, not a body** (D127). No slide means the route's next waypoint is now blocked - a house was built
            // across it, or the unit was nudged off the line the geometry planned through. Standing still was the old answer,
            // and it was for ever: nothing re-examined the route, so the unit pushed at the wall for the rest of the match
            // (the owner: *"I still see scout bumping into trees and getting stuck"*). `openempire` re-plans every 25 cycles
            // whether or not it can name the reason; this asks for one, at most twice per route.
            if let Some((goal_tile, _)) = goal {
                let stalls = self.paths.get(&id).map_or(0, |route| route.stalls);
                if stalls < MAX_GROUND_REPLANS && pass.repaths.len() < MAX_REPATHS_PER_TICK {
                    pass.repaths.push(Repath {
                        unit: id,
                        body: None,
                        goal: goal_tile,
                    });
                }
            }
            return (Fixed::ZERO, Fixed::ZERO, StepDecision::WaitGround);
        }
        let candidate = (
            Fixed::from_raw(here.0.raw() + dx.raw()),
            Fixed::from_raw(here.1.raw() + dy.raw()),
        );
        let Some((other, other_at)) = self.blocker_at(pass.bodies, id, radius, candidate) else {
            return (dx, dy, StepDecision::Take);
        };
        let stop = (Fixed::ZERO, Fixed::ZERO);
        // **Whose body it is does not matter** (D200, the owner: *"Don't make difference of friend or foe as in original
        // game"*). The original's collision block compares the collided unit's **id** with the order's target and nothing
        // else - no player, team or alliance test anywhere in it, and the query passes an owner sentinel rather than the
        // unit's player (`docs/movement-in-the-binary.md`, read 2026-11-13). So a friend is a non-past collision exactly
        // like an enemy: it blocks, and the answer to it is a repath.
        let a_unit = self
            .find(other)
            .is_some_and(|other| other.kind == EntityKind::Unit);
        // 2. The order's own business: the body it was sent at, or the ground it was sent to.
        if Self::order_target(entity) == Some(other) {
            return (stop.0, stop.1, StepDecision::WaitTarget);
        }
        if a_unit
            && let Some((goal_tile, true)) = goal
            && self.tile_of(other_at) == goal_tile
        {
            // **A body on the goal tile asks for a route round it, not for a wait, unless the mover is already beside
            // it** (D200). This clause used to be hostile-only and to hold the walker still, which is a freeze when the
            // body never moves: a villager ordered off a fresh foundation stood 1.011 tiles from his goal for 2,000 ticks
            // (D192). With no owner test the same clause would hold a man off a tile a *friend* occupies for ever, so it
            // repaths - the original's answer to a non-target collision. The `adjacent` guard is what keeps that from
            // spinning: a man already beside the tile he was sent to has *arrived* for every purpose the game has, and
            // re-planning there buys a search a tick (measured: 525 repaths in 600 ticks without it).
            let tile = self.tile_of_raw(here);
            let adjacent = (tile.0 - i64::from(goal_tile.0)).abs() <= 1
                && (tile.1 - i64::from(goal_tile.1)).abs() <= 1;
            if !adjacent && pass.repaths.len() < MAX_REPATHS_PER_TICK {
                pass.repaths.push(Repath {
                    unit: id,
                    body: Some(other),
                    goal: goal_tile,
                });
            }
            return (stop.0, stop.1, StepDecision::WaitGoal);
        }
        // 3. A body the mover is **already inside** is not an obstruction (D52: "a body already overlapped is not made
        //    worse by moving"). Refusing cannot separate a pair a spawn or a push put inside each other, and it froze a
        //    drifter standing on a friend: the leader steps first, so the follower always sees a body it cannot reach.
        //    Getting inside somebody from outside is what rule 1 prevents; once inside, movement is the way out.
        let other_radius = self.body_radius(self.find(other).and_then(|entity| entity.unit));
        let reach = match radius + other_radius {
            0 => self.scale,
            sum => sum,
        };
        let (gap_x, gap_y) = (
            other_at.0.raw() - here.0.raw(),
            other_at.1.raw() - here.1.raw(),
        );
        if gap_x * gap_x + gap_y * gap_y < reach * reach {
            return (dx, dy, StepDecision::TakeOverlapping);
        }
        // 4. **A unit in the way: the original's tie-break** (D205). Byte for byte (`0x8a80ed`-`0x8a8141`):
        //    `call [vtable+0x2b4]` on both units - the object handle - `cmp eax,esi / jle`, and only when *other > self*
        //    does the other get `setPause(rand*1.25/32767+0.25)`. `+0x2b0` writes **the path's pause timer and nothing
        //    else**, so the loser keeps its path, spends no waypoint and stands still until the timer runs out. Our
        //    `EntityId` is a monotonic spawn counter, so this is "the older unit wins every contested tile" - the owner
        //    accepted that cost with the decision.
        //
        //    Both sides of a pair run this rule and agree: the lower id tells the higher to pause and then proceeds, the
        //    higher id pauses itself. The winner's answer is whatever the rest of this function decides - a repath round
        //    the man, a bounded step while he is at rest, or a stop - so the rule adds no step of its own and a pair
        //    never overlaps: D131's reach still holds.
        if a_unit {
            if other.0 > id.0 {
                // Right of way: the other man is the one who stands, and this body goes on being answered below.
                pass.pauses.push(other);
            } else {
                // The higher id: told to pause. It keeps its route - no waypoint is spent and no repath is asked for -
                // and stands this tick out. Already-paused bodies never reach here (the pass skips them), which is the
                // original's "one float": a fresh collision sets the timer, a running timer is never re-rolled.
                pass.pauses.push(id);
                return (stop.0, stop.1, StepDecision::Paused);
            }
        }
        // 5. Refuse, and say why. A body the route already planned around is *past*; anything else is worth a repath,
        //    unless the unit is already on its goal's ring, where a repath only buys searches (D98a).
        if let Some((goal_tile, _)) = goal {
            let tile = self.tile_of_raw(here);
            let near_goal = (tile.0 - i64::from(goal_tile.0)).abs() <= 1
                && (tile.1 - i64::from(goal_tile.1)).abs() <= 1;
            // **The exemption expires when that body moves** (D205, the original's `past` test): a route that planned
            // round a man who has since walked somewhere else no longer knows about him, so the collision counts again and
            // the winner asks for a fresh route. A permanent pass was measured to freeze a pair for 2,500 ticks.
            let known = self.paths.get(&id).is_some_and(|route| {
                route.avoiding.get(&other).is_some_and(|planned_at| {
                    self.find(other).is_some_and(|body| {
                        let (gx, gy) = (
                            body.position.0.raw() - planned_at.0.raw(),
                            body.position.1.raw() - planned_at.1.raw(),
                        );
                        // Still on the ground the plan was made around: within a tile of where it stood.
                        gx * gx + gy * gy <= self.scale * self.scale
                    })
                })
            });
            if !near_goal && !known && pass.repaths.len() < MAX_REPATHS_PER_TICK {
                pass.repaths.push(Repath {
                    unit: id,
                    body: Some(other),
                    goal: goal_tile,
                });
            }
        }
        // 6. A body **at rest** cannot move aside, so waiting on it is waiting for ever (measured: a gatherer frozen from
        //    tick 40,000 to 400,000 beside an idle friend). The step is taken and the repath above routes round it from
        //    the next tick - the overlap is at most one footprint, and it resolves; a frozen unit never does.
        if !pass.moving.contains(&other) {
            // **A unit at rest is still a body** (D193, and D200 removed the owner test from it). The exemption below
            // exists because a resource node carries no radius, so `reach` falls back to a whole tile and a villager could
            // not stand beside a tree - and it was being applied to *units* too: a walker passed an idle enemy at **0.237**
            // tiles where two 0.2 radii need `0.4`. The bound applies to any unit, friend or not; nodes and buildings keep
            // the exemption, which is the case it was written for.
            if a_unit {
                // **A step may not end inside the pair's reach** (D131 A, the owner's call 2026-11-06: *"I think A
                // should be it"*). Halving rather than arithmetic: the constraint is a circle round the other body, so
                // the largest fitting step is found by trying the step and then 1/2, 1/4, ... of it - five integer
                // probes at worst, all in fixed point and no sqrt.
                let delta = self.bound_to_reach(here, other_at, (dx, dy), reach);
                if delta.0 != Fixed::ZERO || delta.1 != Fixed::ZERO {
                    return (delta.0, delta.1, StepDecision::TakeAtRest);
                }
                // **Right of way still has to get past him** (D205). The pair meets at exactly the reach, where a step
                // straight at the blocker bounds to zero and the re-plan cannot help - the search plans round *tiles*, and
                // in one tile it cannot see the body at all (measured on this branch: the winner `WaitBlocked` from tick 70
                // to tick 2,500 and the acceptance bank flat from 400 s to 2,400 s). A **tangential** step moves along the
                // circle the bound protects rather than across it, so it costs no overlap: the older unit slides past the
                // man it has just paused instead of standing until the pause ends and blocks him again.
        // **A body on my own tile is the one collision a tile plan cannot express**: the search would have to leave the tile
        // it starts in, so the re-plan drops such a blocker (`bodies` filters the start tile) and this man would stand for
        // ever - measured, 2,500 ticks at exactly the reach. The answer is a step *along* the reach, which keeps the pair
        // apart while the older unit gets past. Anywhere else the re-plan is the answer (D205).
        if self.tile_of_raw(other_at) == self.tile_of_raw(here)
            && let Some(aside) = self.tangential_step(
                pass, id, radius, here, (dx, dy), other_at, reach, goal_tile, entering,
            )
        {
            return (aside.0, aside.1, StepDecision::Slide);
        }
                // **A body at rest inside the reach leaves no step at all**, and this body has right of way (rule 4), so
                // it has nothing left to try: it stands where it is and the other man's pause is what will move the pair.
                return (stop.0, stop.1, StepDecision::WaitBlocked);
            }
            // **The bound does not apply to a body at rest** (measured: seven gather and rally tests failed when it did).
            // A resource node carries no radius, so `reach` falls back to a whole tile - and a villager must be able to
            // stand beside a tree to chop it, which the bound forbade: it stopped 5 tiles out, on the boundary circle of a
            // node it was walking to. The rule D131 is about is two bodies that both want the ground, not a mover against
            // something that will never yield.
            return (dx, dy, StepDecision::TakeAtRest);
        }
        // 7. Anything else: stand still for this tick. The blocker is moving, so the pair has an answer next tick - and if
        //    it is this body that lost the tie-break, rule 4 has already paused it. **No tangent here**: the answer to a body
        //    in the way is the re-plan (D205), which now keeps the pair's reach clear of it (`short_range_around`); a lateral
        //    step round a moving man was measured to break fifteen stance, gather and rally rules.
        // **A body on my own tile is the one collision a tile plan cannot express**: the search would have to leave the tile
        // it starts in, so the re-plan drops such a blocker (`bodies` filters the start tile) and this man would stand for
        // ever - measured, 2,500 ticks at exactly the reach. The answer is a step *along* the reach, which keeps the pair
        // apart while the older unit gets past. Anywhere else the re-plan is the answer (D205).
        if self.tile_of_raw(other_at) == self.tile_of_raw(here)
            && let Some(aside) = self.tangential_step(
                pass, id, radius, here, (dx, dy), other_at, reach, goal_tile, entering,
            )
        {
            return (aside.0, aside.1, StepDecision::Slide);
        }
        (stop.0, stop.1, StepDecision::WaitBlocked)
    }


    /// **A step along the circle the reach protects, for a body on the mover's own tile** (D205).
    ///
    /// That one collision is the only one a tile plan cannot express: the search would have to leave the tile it starts in,
    /// and the re-plan therefore filters the body out (`bodies` drops a blocker standing on the start tile) - measured
    /// 2026-11-14, the winner `WaitBlocked` for 2,500 ticks at exactly the reach with `bodies=[]`. A tangential step moves
    /// *along* the circle instead of across it, so it costs no overlap. Every other collision is the re-plan's job.
    #[allow(clippy::too_many_arguments)]
    fn tangential_step(
        &self,
        pass: &Passing<'_>,
        id: EntityId,
        radius: i64,
        here: (Fixed, Fixed),
        step: (Fixed, Fixed),
        other_at: (Fixed, Fixed),
        reach: i64,
        goal: Option<(u16, u16)>,
        entering: bool,
    ) -> Option<(Fixed, Fixed)> {
        for (tx, ty) in [(-step.1.raw(), step.0.raw()), (step.1.raw(), -step.0.raw())] {
            if tx == 0 && ty == 0 {
                continue;
            }
            let tangent = (Fixed::from_raw(tx), Fixed::from_raw(ty));
            if !self.step_is_walkable(here, tangent, goal, radius, entering) {
                continue;
            }
            let aside = self.bound_to_reach(here, other_at, tangent, reach);
            if aside.0 == Fixed::ZERO && aside.1 == Fixed::ZERO {
                continue;
            }
            let to = (
                Fixed::from_raw(here.0.raw() + aside.0.raw()),
                Fixed::from_raw(here.1.raw() + aside.1.raw()),
            );
            if self.blocker_at(pass.bodies, id, radius, to).is_some() {
                continue;
            }
            return Some(aside);
        }
        None
    }

    /// **A step may not end inside the pair's reach** (D131 option A, the owner's call 2026-11-06: *"I think A should be
    /// it"*, with his own memory that *"narrow passes with 2 units passing in opposing directions were a problem"*).
    ///
    /// Two rules used to hand a body its **full** step into another body's space: 8 (a blocker at rest) and 9 (give way).
    /// Measured on the one-tile corridor, the pair ended **7,372** raw units apart against the **8,000** their radii need -
    /// 628 inside, 1/32 of a tile. AoE2's answer to the same geometry is *"units bump into others, stop and repath around.
    /// No pushing allowed"* (Klingers, Meeting C++ 2025, page 9), so the pair is allowed to **stop**; what it is not
    /// allowed to do is overlap.
    ///
    /// Halving rather than arithmetic: the constraint is a circle around the other body, so the largest fitting step is
    /// found by trying the step and then 1/2, 1/4, ... of it - five integer probes at worst, all in fixed point, no sqrt.
    fn bound_to_reach(
        &self,
        here: (Fixed, Fixed),
        other: (Fixed, Fixed),
        step: (Fixed, Fixed),
        reach: i64,
    ) -> (Fixed, Fixed) {
        for shift in 0..=5 {
            let div = 1i64 << shift;
            let (nx, ny) = (step.0.raw() / div, step.1.raw() / div);
            let (gx, gy) = (
                other.0.raw() - (here.0.raw() + nx),
                other.1.raw() - (here.1.raw() + ny),
            );
            if gx * gx + gy * gy >= reach * reach {
                return (Fixed::from_raw(nx), Fixed::from_raw(ny));
            }
        }
        (Fixed::ZERO, Fixed::ZERO)
    }

    /// The entity an order is trying to reach, when the order has one. `Move` has none - its goal is a place, and that
    /// is why `collide` also asks `goal_of` (rule 2), which covers the order that names ground rather than a body.
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
