//! **Collision**: what a step may do about the ground and about the bodies standing on it.
//!
//! Moved out of `lib.rs` unchanged (D136). `collide` is the only entry point the tick uses; everything else answers one
//! question about a candidate step - is the ground under it walkable, which body is in the way, and what a mover does when
//! a body refuses to move.

use ruage_fixed::Fixed;

use crate::{
    Entity, EntityId, EntityKind, MAX_REPATHS_PER_TICK, Passing, Repath,
    StepDecision, Task, World,
};

impl World {
    /// **A corner is folded into the step, not landed on** (D252). The owner: *"villagers never slow down. They may stop or
    /// stop to repath ... so either they walk, or stand."*
    ///
    /// `step_towards` lands exactly on a point that is within one step. That is right for the goal and costs a walking man
    /// the rest of that tick's step at every bend: measured **15 of 20,000** villager body-ticks at 0.11-0.95 of a step, 14 of
    /// them a route transit on one walk. The obvious fix is wrong: spending the corner *before* the step makes the aim the
    /// point beyond it, and a straight line there **cuts the corner**, which the ground refuses - a walker wedged against a
    /// forest edge for **68 ticks**, sliding one raw unit a tick.
    ///
    /// So the corner is walked **through**, after the ordinary step: the remainder of the tick is spent along the next leg,
    /// one point at a time, spending each point it reaches. Bounded to two legs (a step is 0.04 of a tile for a villager),
    /// refused by ground or by a body, and it never folds past the **last** point - the goal is landed on exactly, because
    /// arrival is an equality at the task layer.
    pub(crate) fn fold_the_step(
        &mut self,
        asked: &[(EntityId, (Fixed, Fixed), i64)],
        reached: &[(EntityId, (Fixed, Fixed))],
        bodies: &[(i64, i64, EntityId, usize, i64)],
    ) {
        let apart = |a: (Fixed, Fixed), b: (Fixed, Fixed)| -> i64 {
            let (dx, dy) = (b.0.raw() - a.0.raw(), b.1.raw() - a.1.raw());
            crate::integer_sqrt(dx * dx + dy * dy)
        };
        for (id, here) in reached {
            let Some((_, start, delta)) = asked.iter().find(|(who, ..)| who == id).copied() else {
                continue;
            };
            let mut left = delta - apart(start, *here);
            if left <= 0 {
                continue;
            }
            // **Only a landing.** A refused or clipped step leaves the body short of its front point, and folding that would
            // walk it past ground the mover had just refused.
            let Some(front) = self.route_points(*id).first().copied() else {
                continue;
            };
            if apart(*here, front) > 1 {
                continue;
            }
            // **Only a walk folds** (D252). `Move` and `Hunt` are the tasks whose whole contract is "walk the route": the
            // goal is a place, and the only arrival rule is the goal itself. A gather, a build or a deposit *approaches* a
            // point that a second pass also steers at - `settle_workers` walks an unsquared villager onto his place's standoff
            // every tick - so folding a landed corner there put the two passes 800 raw units apart and they undid each other
            // for ever: measured, one gatherer oscillating between (394394,313527) and (395101,313901) from tick 38,000 to
            // 400,000, and a fixture that banks 400 food stuck at **200** with **1 repath**. An attacker's stop distance is a
            // range, not a route point, so it is out for the same reason.
            let folds = self.find(*id).is_some_and(|entity| {
                matches!(
                    entity.task,
                    crate::Task::Move { .. } | crate::Task::Hunt { .. }
                )
            });
            if !folds {
                continue;
            }
            let radius = self.body_radius(self.find(*id).and_then(|entity| entity.unit));
            let mut at = *here;
            for _ in 0..2 {
                if left <= 0 {
                    break;
                }
                // **The corner behind the fold is spent as the fold begins.** The body stands on it (checked above), and
                // leaving it in the list makes the next tick aim *back* at it: measured, a gatherer oscillated at every
                // corner and a 400,000-tick fixture's economy froze at **200 of 400** food with **1 repath**.
                if let Some(path) = self.paths.get_mut(id)
                    && path.points.len() > 1
                {
                    path.points.remove(0);
                }
                let points = self.route_points(*id);
                // **The last point is the goal**, and it is landed on, not folded past: an empty remainder means the corner
                // the body reached was the goal.
                let Some(next) = points.first().copied() else {
                    break;
                };
                let gap = apart(at, next);
                let take = gap.min(left);
                let (dx, dy) = crate::step_towards(at, next, take);
                let target = (
                    Fixed::from_raw(at.0.raw() + dx),
                    Fixed::from_raw(at.1.raw() + dy),
                );
                // **The ground the fold lands on**, by the same rule the mover uses everywhere: the body's disc clear of
                // every blocked tile.
                if self.clearance_at(target, radius) < 0 {
                    break;
                }
                // ... and every body it could touch, by the pair's own reach.
                let tile = self.tile_of_raw(target);
                let blocked = (-1i64..=1).any(|dy| {
                    (-1i64..=1).any(|dx| {
                        self.bodies_at(bodies, (tile.0 + dx, tile.1 + dy))
                            .iter()
                            .any(|(_, _, other, _, other_radius)| {
                                *other != *id
                                    && apart(target, self.position(*other).unwrap_or(target))
                                        < radius + *other_radius
                            })
                    })
                });
                if blocked {
                    break;
                }
                if let Some(entity) = self.find_mut(*id) {
                    entity.position = target;
                }
                // **Sight follows the fold** (D50a): `stamp_entity` ran on the position the step landed on, so a fold that
                // moved the body further must re-stamp, or the ground it can now see goes unobserved for a tick. The stamp
                // is an OR into an explored bitset, so a second one costs nothing and cannot double-count.
                self.stamp_entity(*id);
                at = target;
                left -= take;
            }
        }
    }

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
    ) -> bool {
        let (x, y) = (here.0.raw() + step.0.raw(), here.1.raw() + step.1.raw());
        // **The body, not the centre** (D176). The owner, twice: a villager stood a hand's width from the town centre with its 0.2 disc inside the
        // wall, because this test only asked whether the *centre* landed on a blocked tile. A disc against a square is the square grown by the
        // radius - the Forgotten Empires talk's *"shrink the unit to a point, expand every other obstruction"* - so the step is refused when its
        // landing would put the body inside, and allowed when it is already inside and moving *out* (a unit pushed into a wall, or a spawn that
        // arrives touching one, must still be able to leave).
        // ground rule is the old one (the centre may step where a route to the door leads). Everything else obeys the disc.
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

    /// **The nearest point a body fits**, when a point sits inside a building's footprint grown by its radius (D262-D264).
    /// The game's distance to a footprint is `max(0, |dx| - radius_x)` per axis (`func_00269190`), so touching is legal and
    /// inside is not; a gather aim or a route endpoint that is inside can never be reached by a legal step.
    pub(crate) fn push_out_of_buildings(&self, at: (Fixed, Fixed), radius: i64) -> (Fixed, Fixed) {
        let scale = self.scale;
        let mut point = at;
        for building in self
            .entities
            .iter()
            .filter(|entity| entity.kind == EntityKind::Building)
        {
            let Some((ax, ay, w, h)) = self.footprint_rect(building.id) else {
                continue;
            };
            let left = i64::from(ax) * scale - radius;
            let top = i64::from(ay) * scale - radius;
            let right = (i64::from(ax) + i64::from(w)) * scale + radius;
            let bottom = (i64::from(ay) + i64::from(h)) * scale + radius;
            let (x, y) = (point.0.raw(), point.1.raw());
            if x <= left || x >= right || y <= top || y >= bottom {
                continue;
            }
            let (to_left, to_right) = (x - left, right - x);
            let (to_top, to_bottom) = (y - top, bottom - y);
            let nearest = to_left.min(to_right).min(to_top).min(to_bottom);
            point = if nearest == to_left {
                (Fixed::from_raw(left), point.1)
            } else if nearest == to_right {
                (Fixed::from_raw(right), point.1)
            } else if nearest == to_top {
                (point.0, Fixed::from_raw(top))
            } else {
                (point.0, Fixed::from_raw(bottom))
            };
        }
        point
    }

    /// **Where a body goes when the spot it was given is already taken** (D267) - the original's `func_002699A0`
    /// (`unit_action_resolve.cpp`, 80 lines). Its rule, one for one:
    ///
    /// * the step out is **`ext_a + ext_b + 0.01`** - the two radii plus one hundredth of a tile - so the pair **touches and
    ///   does not overlap**; the 0.01 is on the outside, which is why the original's units stand *just* clear of each other;
    /// * the side is one of **four** (`coord +/- step` on an axis), picked at random per attempt, with the other coordinate
    ///   jittered by a random offset across that side, so a group on one point fans out instead of queueing;
    /// * **fifty** attempts, and the first spot that is on the map, on ground the body fits on, and clear of every other body
    ///   is taken;
    /// * nothing moves an existing body: the newcomer is the one that steps aside (the caller writes this position), which is
    ///   why this is *placement*, not pushing, and D52's "no pushing" still holds.
    ///
    /// The original draws its side and offset from `func_001AAB30` (a random). This one is a xorshift seeded from the entity id
    /// and the tick: the same shape, identical on every machine and every run.
    pub(crate) fn place_apart(&self, at: (Fixed, Fixed), radius: i64, salt: u64) -> (Fixed, Fixed) {
        let reach_to = |other: &Entity, mine: i64| -> i64 {
            match mine + self.body_radius(other.unit) {
                0 => self.scale,
                sum => sum,
            }
        };
        let clear = |point: (Fixed, Fixed), mine: i64| -> bool {
            self.entities.iter().all(|other| {
                if other.kind != EntityKind::Unit || other.id.0 == salt as u32 {
                    return true;
                }
                let reach = reach_to(other, mine);
                let (dx, dy) = (
                    other.position.0.raw() - point.0.raw(),
                    other.position.1.raw() - point.1.raw(),
                );
                dx * dx + dy * dy >= reach * reach
            })
        };
        if clear(at, radius) {
            return at;
        }
        let scale = self.scale;
        let mut state = salt | 1;
        let mut next = move || -> i64 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            i64::try_from(state & 0xFFFF).unwrap_or(0)
        };
        for _ in 0..50 {
            let side = next() % 4;
            // The body that is already standing there decides how far along the side the newcomer may go.
            let span = self
                .entities
                .iter()
                .filter(|other| other.kind == EntityKind::Unit && other.id.0 != salt as u32)
                .map(|other| self.body_radius(other.unit).max(scale / 8))
                .max()
                .unwrap_or(scale / 2);
            let offset = next() % (2 * span) - span;
            // **The step out, on the pair's own measure**: the two radii plus a hundredth of a tile.
            let out = self
                .entities
                .iter()
                .filter(|other| other.kind == EntityKind::Unit && other.id.0 != salt as u32)
                .map(|other| match radius + self.body_radius(other.unit) {
                    0 => scale,
                    sum => sum,
                })
                .max()
                .unwrap_or(scale);
            let step = out + scale / 100;
            let (x, y) = (at.0.raw(), at.1.raw());
            let point = match side {
                0 => (Fixed::from_raw(x + step), Fixed::from_raw(y + offset)),
                1 => (Fixed::from_raw(x - step), Fixed::from_raw(y + offset)),
                2 => (Fixed::from_raw(x + offset), Fixed::from_raw(y + step)),
                _ => (Fixed::from_raw(x + offset), Fixed::from_raw(y - step)),
            };
            let (tx, ty) = self.tile_of_raw(point);
            let (Ok(tx), Ok(ty)) = (u16::try_from(tx), u16::try_from(ty)) else {
                continue;
            };
            if !self.tile_walkable(tx, ty) {
                continue;
            }
            // A body of this radius has to fit on the ground it lands on, and it has to be clear of every other body.
            if self.clearance_at(point, radius) >= 0 && clear(point, radius) {
                return point;
            }
        }
        at
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
        if !self.step_is_walkable(here, (dx, dy), goal_tile, radius) {
            // **The slide is still load-bearing, and this is where it is measured** (D264). Deleted, with the short-range planner
            // given a legal goal (so the planner is no longer the problem), the **animals family goes red: 3 of 11** -
            // `a_live_animal_is_a_gather_source`, `the_villager_follows_the_sheep`,
            // `the_shepherd_kills_the_sheep_and_the_gathering_resumes_on_the_body`. The reason is not the route: a gatherer's
            // last approach is a **place kept to within a fifths of a tile by `settle_workers`**, whose nudges are far under a
            // step, so a step rule that refuses anything but a whole step leaves him a fraction short and he never works.
            // The original answers the same case differently: **its whole movement is continuous** (0.04 tiles a tick, D258), so
            // "a fraction of a step" is not a category it has. Until our mover can spend a partial step on the gather approach -
            // or the place is reached by walking only - the slide stays.
            // **The terrain slide is gone** (D276/D279): the original applies its step whole and answers a blocked step
            // with a re-plan or a stand - *"no axis is ever dropped"*. The substituted waypoint above is what keeps a
            // refusal from becoming a stall, which is why the two changes land together.
            // **Ground, not a body** (D127). No slide means the route's next waypoint is now blocked - a house was built
            // across it, or the unit was nudged off the line the geometry planned through. Standing still was the old answer,
            // and it was for ever: nothing re-examined the route, so the unit pushed at the wall for the rest of the match
            // (the owner: *"I still see scout bumping into trees and getting stuck"*). `openempire` re-plans every 25 cycles
            // whether or not it can name the reason; this asks for one, at most twice per route.
            // **The waypoint this body could not reach is counted** (D279): the original substitutes a waypoint it will not
            // reach "this or the next update" instead of waiting on it, and it counts nothing per route - so the cap of two
            // ground re-plans goes with the substitution in, not after it.
            // DBG_PROBE: which kind of route the refused step belongs to (D280's question). Committed so it cannot be lost
            // to a `git checkout` - see the pitfall in the movement reference.
            if std::env::var("DBG_ORIGIN").is_ok() {
                let scale = self.scale;
                let (first, len) = self
                    .paths
                    .get(&id)
                    .map(|route| (route.points.first().copied(), route.points.len()))
                    .unwrap_or((None, 0));
                let kind = match first {
                    None => "none",
                    Some(point) => {
                        let aligned = |raw: i64| raw % scale == 0 || raw % scale == scale / 2;
                        if aligned(point.0.raw()) && aligned(point.1.raw()) {
                            "tile-chain"
                        } else {
                            "geometry"
                        }
                    }
                };
                eprintln!("ORIGIN kind={kind} len={len}");
            }
            pass.waits.push(id);
            if let Some((goal_tile, _)) = goal {
                if pass.repaths.len() < MAX_REPATHS_PER_TICK {
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
        // 1b. **A group walks over itself** (D207). The original drops an exempt collider outright: its classifier answers
        //     "past" for a unit on the route's exemption list and never pauses, re-paths or bounds the step, and that list
        //     is the mover's *moving selection-mates* (`FUN_008007f0`, read 2026-11-14). Its own developers describe the
        //     consequence on the bug forum: *"Military units moving in groups can go over each other."* So a mate is taken
        //     as ground that is not there - no reach, no wait - while everybody else stays solid.
        if a_unit && self.is_mate(id, other) {
            return (dx, dy, StepDecision::TakeOverlapping);
        }
        // 1c. **A collider that has not moved since this route was planned is past** (D210), and the original drops it:
        //     no pause, no repath, no bound. This is the second half of what ends a head-on - the loser is paused, stops
        //     moving, and from then on he is old news to a route planned after his last step. The order's own target is
        //     excluded: a man sent at something arrives beside it, he does not walk through it.
        if a_unit && Self::order_target(entity) != Some(other) && self.is_past(id, other) {
            return (dx, dy, StepDecision::Take);
        }
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
        //
        //    **D268 tried to remove this clause, because the original has no such thing, and the measurement refuted it**:
        //    with the clause gone the overlaps in a 36,000-tick match went **104,386 -> 167,025** pair-ticks and the ones
        //    deeper than half a footprint **51,676 -> 75,380**. Refusing a step does not separate a pair that is already
        //    inside - it congeals them, because the collision is still there next tick. The clause stays until the answer to
        //    a collision is a working **repath out of an overlap** (the original's own answer), which is the recorded next
        //    step, not a thing to wish for.
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
            // **A repath out of an overlap was tried and refuted** (D272). D52 lets an overlapping body keep moving, and D268
            // showed refusing it congeals a pair (104,386 -> 167,025 pair-ticks). Taking the owner's "2c" and queueing a repath
            // out was the third answer, and it is the worst of the three by this probe: **104,386 -> 337,570** pair-ticks,
            // deeper than half a footprint **51,676 -> 288,009**, and the "past" exemption balloons to **239,106** because the
            // churn keeps more units moving. So the clause stays exactly as D52 wrote it: the local optimum of this mover is
            // "keep moving", and what is wrong is upstream - a step is landing inside a body at all (D269's **7,500** against
            // 8,000), which no rule here can be blamed for.
            return (dx, dy, StepDecision::TakeOverlapping);
        }
        // 4. **A unit in the way: the original's tie-break** (D205). Byte for byte (`0x8a80ed`-`0x8a8141`):
        //    `call [vtable+0x2b4]` on both units - the object handle - `cmp eax,esi / jle`, and only when *other > self*
        //    does the other get `setPause(rand*1.25/32767+0.25)`. `+0x2b0` writes **the path's pause timer and nothing
        //    else**, so the loser keeps its path, spends no waypoint and stands still until the timer runs out. Our
        //    `EntityId` is a monotonic spawn counter, so this is "the older unit wins every contested tile" - the owner
        //    accepted that cost with the decision.
        //
        //    Only the **lower** id acts here: it tells the higher one to pause and then goes on to be answered by the rest
        //    of this function. The higher id does **not** pause itself - it walks on to the same rest of the function,
        //    which is where its own answer lives (a re-plan round the man, a step into a body at rest, or a stop) - because
        //    a body that pauses itself never asks for a route, and against a man who never moves that is a freeze for ever
        //    (measured 2026-11-14, `vision::an_enemy_is_only_visible_while_somebody_watches`). D131's reach no longer holds
        //    as a bound: D210 withdrew the clamp, and what ends an encounter is the pause and the *past* test.
        if a_unit && other.0 > id.0 {
            // Right of way: the other man is the one who stands, and this body goes on being answered below.
            pass.pauses.push(other);
        }
        // **The loser does not pause himself** (D210, corrected 2026-11-14). The original's block only ever calls
        // `setPause` on **the other** unit (`cmp eax,esi / jle` - "pause only when other > self"); the mover that
        // loses the comparison simply goes on to the rest of the block, which is where its own answer lives. Our first
        // version paused *itself* and returned, so it never asked for a route: measured on
        // `vision::an_enemy_is_only_visible_while_somebody_watches`, a scout stood `Paused` at (614150, 614150) from
        // tick 50 to tick 550 against a man with no order, because the body that won the tie-break never moved, so the
        // collision was still there every tick and the loser was stood up again for ever.
        //
        // A body paused *by the other's* turn still stands its ticks out - `move_bodies` checks `pass.pauses` before
        // the body's own step, and `pause_remaining` carries it across ticks - so the drawn 0.25-1.50 s is unchanged.
        // 5. Refuse, and say why. A body the route already planned around is *past*; anything else is worth a repath,
        //    unless the unit is already on its goal's ring, where a repath only buys searches (D98a).
        if let Some((goal_tile, _)) = goal {
            let tile = self.tile_of_raw(here);
            let near_goal = (tile.0 - i64::from(goal_tile.0)).abs() <= 1
                && (tile.1 - i64::from(goal_tile.1)).abs() <= 1;
            let known = self
                .paths
                .get(&id)
                .is_some_and(|route| route.avoiding.contains(&other));
            // **Near the goal is no longer a reason to stand still when the thing in the way is a body** (D237). The guard
            // was written for ground (a repath beside the goal buys nothing but searches, D98a), and it is exactly where a
            // gatherer lives: a man one tile from the tree that a friend blocks inside that last tile could not ask for a
            // route at all, so narrowing the mates rule starved the match - 149 entities to 139, `repaths` 432 to 1,401.
            // The pencil bends round a *disc*, so a body in the last tile is the one case where a near-goal search pays.
            if (!near_goal || a_unit) && !known && pass.repaths.len() < MAX_REPATHS_PER_TICK {
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
                // **No clamp against a body** (D210). `bound_to_reach` used to halve the step until it ended outside
                // `r1 + r2`, which is stricter than the game: the binary's only radius-against-radius retraction has its
                // result discarded at the movement site (`0x8a7fe7`), and its position setter clamps against the map border
                // alone. What ends the encounter is the pause (rule 4) and the *past* test above, not geometry.
                return (dx, dy, StepDecision::TakeAtRest);
            }
            // **The bound does not apply to a body at rest** (measured: seven gather and rally tests failed when it did).
            // A resource node carries no radius, so `reach` falls back to a whole tile - and a villager must be able to
            // stand beside a tree to chop it, which the bound forbade: it stopped 5 tiles out, on the boundary circle of a
            // node it was walking to. The rule D131 is about is two bodies that both want the ground, not a mover against
            // something that will never yield.
            return (dx, dy, StepDecision::TakeAtRest);
        }
        // 7. Anything else: stand still. The blocker is moving, so the pair has an answer next tick - and if it is this
        //    body that lost the tie-break, rule 4 has already paused it.
        (stop.0, stop.1, StepDecision::WaitBlocked)
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
