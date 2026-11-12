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

    /// One step against the bodies in the way (D98a, C).
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
    ///    than the tile it is already on (D52, D98a);
    /// 3. the mover is **already inside** the blocker's footprint - take the step, or a pair a spawn put inside each
    ///    other can never separate;
    /// 4. the blocker stands on the **mover's own tile** and is itself moving - wait: the plan cannot route around a
    ///    tile it starts in, and the body ahead will move on;
    /// 5. one axis alone is clear - **slide** along it;
    /// 6. **make room**, once per route: half the way plus a quarter tile to one side, terrain-checked (C1b);
    /// 7. otherwise refuse and classify: a body the route already avoids is *past* and costs no repath (or a blocked
    ///    unit would re-plan every tick), anything else is non-past and earns one;
    /// 8. the blocker is **at rest** - take the step and let the repath route round it: a body that is not going
    ///    anywhere will not move aside, and waiting on it is waiting for ever;
    /// 9. otherwise **wait**, unless the blocker refused a step at this unit on the same tick - a mutual block gives way
    ///    in entity order, so a jam is never permanent.
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
        // slides along the axis that is clear, which is how it gets round a building instead of through it.
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
        // **Whose body is it** (D131 A, D189): the reach and the goal-tile refusal below are about **two players'** units. Two of
        // your own villagers are not walls - they pass while walking (D189) and separate where they stand - and treating them as
        // one froze a man told to step off a fresh foundation for 2,000 ticks, 1.011 tiles from the tile he was sent to, because an
        // idle neighbour stood on it (measured; the house stayed at hp 1).
        let hostile = entity.player >= 1
            && self
                .find(other)
                .is_some_and(|other| other.player >= 1 && other.player != entity.player);
        // 2. The order's own business: the body it was sent at, or the ground it was sent to.
        if Self::order_target(entity) == Some(other) {
            return (stop.0, stop.1, StepDecision::WaitTarget);
        }
        if hostile
            && let Some((goal_tile, true)) = goal
            && self.tile_of(other_at) == goal_tile
        {
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
        // **Whose ground is it** (D131 A): the reach is a bar between **two players' units**, because that is the pass the
        // owner remembers being a problem (*"narrow passes with 2 units passing in opposing directions"*). Two of your own
        // villagers brushing past each other is how AoE2 moves a crowd - the deck's own page 9 says *"formations allow for
        // some friendly overlap"* - and bounding those as well measured a match that no longer resolved: 164 entities
        // against 141, both players alive at 1,800 s where the baseline had eliminated one, refusals 574 -> 1,021.
        // **Your own units are not walls** (D189, D192). The owner chose this between the two ways out of the crowd problem
        // (*"1"*): his bodies may overlap while walking, and they are separated where they stand. Without it, ten men sent at one
        // bush cannot be delivered - the men behind cannot pass the men in front, and the settle, which moves a body directly,
        // either steps inside a neighbour (measured 1.962 and 0.144 tiles out) or strands a man outside the band.
        //
        // **The last step used to be the exception, and that exception was the bug.** A walker was held for any friendly body on
        // the tile his order asked for, so a villager told to step off a fresh foundation stood **1.011 tiles** from his goal for
        // 2,000 ticks while an idle neighbour stood there, and the house stayed at hp 1. A friend is not a wall on the last step
        // either; the settle and the work places are what keep men apart where it matters.
        //
        // **Enemy bodies are unchanged**: the reach above and `bound_to_reach` still hold two players' units apart, which is the
        // pass D131 was about.
        if !hostile {
            return (dx, dy, StepDecision::Take);
        }
        let give = |step: (Fixed, Fixed), why: StepDecision| {
            let delta = if hostile {
                self.bound_to_reach(here, other_at, step, reach)
            } else {
                step
            };
            (delta.0, delta.1, why)
        };
        // 4. A body standing on the **mover's own tile** is not something a plan can route around: the search would have
        //     to leave the tile it starts in, and what it found was a nine-waypoint route that walked backwards. The unit
        //     waits instead - which is what a follower does while the body ahead of it moves on. **Only while it moves**:
        //     a body at rest will not get out of the way, and waiting on it is waiting for ever.
        if self.tile_of_raw(other_at) == self.tile_of_raw(here) && pass.moving.contains(&other) {
            if self.wait_or_give_way(id, other, &mut pass.refused) {
                return give((dx, dy), StepDecision::GiveWayOwnTile);
            }
            return (stop.0, stop.1, StepDecision::WaitOwnTile);
        }
        // 5. One axis alone, in a fixed order so two runs slide the same way.
        for (sx, sy) in [(dx.raw(), 0i64), (0, dy.raw())] {
            if sx == 0 && sy == 0 {
                continue;
            }
            let slide = (
                Fixed::from_raw(here.0.raw() + sx),
                Fixed::from_raw(here.1.raw() + sy),
            );
            if self.blocker_at(pass.bodies, id, radius, slide).is_none() {
                return (
                    Fixed::from_raw(sx),
                    Fixed::from_raw(sy),
                    StepDecision::Slide,
                );
            }
        }
        // 6. Make room: **once per route**. A nudge is a way past one obstruction, not a way of life - uncapped, every
        //    crowded unit steps aside for every other and a base turns into a mosh pit that never arrives (measured:
        //    89 wood / 20 food / 50 gold at 1,600 s against 729 / 792 / 370).
        let gave_way = self.paths.get(&id).is_some_and(|route| route.gave_way);
        if !gave_way {
            let quarter = self.scale / 4;
            let length = dx.raw().abs().max(dy.raw().abs()).max(1);
            for turn in [1i64, -1i64] {
                let (rx, ry) = if turn > 0 {
                    (-dy.raw(), dx.raw())
                } else {
                    (dy.raw(), -dx.raw())
                };
                let (nx, ny) = (
                    dx.raw() / 2 + rx * quarter / length,
                    dy.raw() / 2 + ry * quarter / length,
                );
                let aside = (
                    Fixed::from_raw(here.0.raw() + nx),
                    Fixed::from_raw(here.1.raw() + ny),
                );
                // Terrain: a nudge may stay on the ground the body already holds (a body can stand on a node's tile),
                // but it may not step onto blocked ground to get out of somebody's way.
                let (x, y) = self.tile_of_raw(aside);
                if (x, y) != self.tile_of_raw(here) {
                    let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y)) else {
                        continue;
                    };
                    if !self.is_free_ground(x, y) {
                        continue;
                    }
                }
                if self.blocker_at(pass.bodies, id, radius, aside).is_none() {
                    pass.made_room.push(id);
                    return (
                        Fixed::from_raw(nx),
                        Fixed::from_raw(ny),
                        StepDecision::MakeRoom,
                    );
                }
            }
        }
        // 7. Refuse, and say why. A body the route already planned around is *past*; anything else is worth a repath,
        //    unless the unit is already on its goal's ring, where a repath only buys searches (D98a).
        if let Some((goal_tile, _)) = goal {
            let tile = self.tile_of_raw(here);
            let near_goal = (tile.0 - i64::from(goal_tile.0)).abs() <= 1
                && (tile.1 - i64::from(goal_tile.1)).abs() <= 1;
            let known = self
                .paths
                .get(&id)
                .is_some_and(|route| route.avoiding.contains(&other));
            if !near_goal && !known && pass.repaths.len() < MAX_REPATHS_PER_TICK {
                pass.repaths.push(Repath {
                    unit: id,
                    body: Some(other),
                    goal: goal_tile,
                });
            }
        }
        // 8. A body **at rest** cannot move aside, so waiting on it is waiting for ever (measured: a gatherer frozen from
        //    tick 40,000 to 400,000 beside an idle friend). The step is taken and the repath above routes round it from
        //    the next tick - the overlap is at most one footprint, and it resolves; a frozen unit never does.
        if !pass.moving.contains(&other) {
            // **A hostile unit at rest is still a body** (D193, measured). The exemption below exists because a resource node carries
            // no radius, so `reach` falls back to a whole tile and a villager could not stand beside a tree - but it was being
            // applied to *units* too, and a walker passed an idle enemy at **0.237** tiles where two 0.2 radii need `0.4`. The bound
            // applies to a hostile unit; nodes, buildings and friends keep the exemption.
            let a_unit = self
                .find(other)
                .is_some_and(|entity| entity.kind == EntityKind::Unit);
            if hostile && a_unit {
                return give((dx, dy), StepDecision::TakeAtRest);
            }
            // **The bound does not apply to a body at rest** (measured: seven gather and rally tests failed when it did).
            // A resource node carries no radius, so `reach` falls back to a whole tile - and a villager must be able to
            // stand beside a tree to chop it, which the bound forbade: it stopped 5 tiles out, on the boundary circle of a
            // node it was walking to. The rule D131 is about is two bodies that both want the ground (rules 4 and 9), not
            // a mover against something that will never yield.
            return (dx, dy, StepDecision::TakeAtRest);
        }
        // 9. Otherwise wait - unless the blocker refused a step at this unit on the same tick, in which case one of the
        //    two has to give way or the pair is jammed. The give-way step is **bounded by the reach** (D131 A): a jam is
        //    never permanent, and a pair never overlaps.
        if self.wait_or_give_way(id, other, &mut pass.refused) {
            return give((dx, dy), StepDecision::GiveWay);
        }
        (stop.0, stop.1, StepDecision::WaitBlocked)
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

    /// Stand still for a body in the way - unless that body refused a step at **this** one on the same tick, in which
    /// case this one gives way and takes its step.
    ///
    /// Entity order decides which of a pair gives way (the entity list is sorted by id), and a jam neither unit can
    /// resolve is never permanent: measured on the one-wood-tree fixture, two villagers side by side in one tile waited
    /// for each other from tick 15 to tick 2,000.
    fn wait_or_give_way(
        &self,
        id: EntityId,
        other: EntityId,
        refused: &mut Vec<(EntityId, EntityId)>,
    ) -> bool {
        if refused
            .iter()
            .any(|(unit, body)| *unit == other && *body == id)
        {
            return true;
        }
        refused.push((id, other));
        false
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
