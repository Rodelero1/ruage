//! **Villager shares**: the once-a-second pass that moves one villager toward an under-manned resource (D145).
//!
//! Split out of `lib.rs` under D145, verbatim.
use crate::{EntityId, EntityKind, MAX_PLAYERS, Task, World};
impl World {
    /// The player's town centre, as the anchor the share table measures from: the building of the start's town-centre
    /// family. `None` when the player has none left, in which case the caller measures from the worker itself.
    fn base_of(&self, player: u8) -> Option<EntityId> {
        let centre = self.start_town_center_unit()?;
        self.entities
            .iter()
            .find(|entity| {
                entity.player == player
                    && entity.kind == EntityKind::Building
                    && entity.unit.is_some_and(|unit| self.is_family(unit, centre))
            })
            .map(|entity| entity.id)
    }

    /// Enforce the share table, once a second. One villager per pass - the table is a preference, not a teleport - and
    /// only idle villagers or ones on an over-manned resource are moved, so a player's own orders survive.
    ///
    /// Cost: one walk of the entities per call, gated on a table existing; the walk counts villagers and finds the
    /// nearest node of the chosen resource in a second pass over the resource nodes only.
    pub(crate) fn step_shares(&mut self) {
        if self.shares.is_empty() {
            return;
        }
        let every = u64::from(self.tick_rate_hz).max(1);
        if !self.tick.is_multiple_of(every) {
            return;
        }
        self.shares_scan_tick = self.tick;
        let villager_unit = match self.start_villager_unit() {
            Some(unit) => unit,
            None => return,
        };
        // Who is working what, and who is free.
        // `(id, player, which column it is working, and whether it is *free*)`. The flag is not `slot.is_none()`:
        // a villager laying a foundation or walking to one has no column either, and taking its build queue away
        // because the table wanted another lumberjack broke `build_queue.rs` the first time this pass ran.
        let mut workers: Vec<(EntityId, u8, Option<usize>, bool)> = Vec::new();
        let mut per_player: [usize; MAX_PLAYERS] = [0; MAX_PLAYERS];
        for entity in &self.entities {
            if entity.kind != EntityKind::Unit || entity.player == 0 {
                continue;
            }
            let is_villager = entity
                .unit
                .is_some_and(|unit| self.is_family(unit, villager_unit));
            if !is_villager {
                continue;
            }
            per_player[usize::from(entity.player) - 1] += 1;
            // A villager standing at a **dead** node with nothing in hand is idle in everything but the name of its
            // task - 0 A.D. gives that state its own name (`FINDINGNEWTARGET`) and lets it find the next resource. It is
            // the state this economy used to freeze in: the whole run stopped at 8 units with food in the stockpile,
            // because the workers *looked* employed. Counting them as idle is what lets the share table move them.
            let (slot, free) = match entity.task {
                Task::Gather { node, .. } => {
                    // Dead means "nothing left to take". **Not** `hp <= 0`: every resource node on the map is spawned with
                    // `hp = 0` (a tree is not a thing with health), so testing hp here made every gatherer look idle - and
                    // the share pass then re-issued a gather order to it once a second for the whole match, which is why
                    // villagers never settled on a tree. Only a *unit* has death by hp.
                    let dead = self.find(node).is_none_or(|target| {
                        target.amount.raw() <= 0
                            || (target.kind == EntityKind::Unit && target.hp <= 0)
                    });
                    if dead && entity.carry.raw() <= 0 {
                        (None, true)
                    } else {
                        (self.node_slot(node), false)
                    }
                }
                Task::Return { node, .. } => (self.node_slot(node), false),
                Task::Idle => (None, true),
                // **A builder is reserved.** This is the AI's `up-assign-builders` (Feudal Age.per assigns three to a
                // market, two to a blacksmith; Castle Age.per six to a town-centre foundation): once a villager is on a
                // site it is not free and it is not on a resource column, so neither the idle arm nor the pull arm can
                // take it. It stays that way until the site finishes, is cancelled, or the build path gives up on it.
                // Everything else - a walk, a fight - is work in progress and not the table's business either.
                _ => (None, false),
            };
            workers.push((entity.id, entity.player, slot, free));
        }
        for player in 1..=MAX_PLAYERS as u8 {
            let count = per_player[usize::from(player) - 1];
            if count == 0 {
                continue;
            }
            let Some((_, targets)) = self
                .shares
                .iter()
                .rev()
                .find(|(at, _)| *at <= count)
                .cloned()
            else {
                continue;
            };
            // Actual against target, per slot, in whole villagers. Every column counts, not only the listed ones: a
            // villager left on a resource the table gives 0% is over-manned and is what gets moved.
            let slots = self.resources.as_ref().map_or(0, |pack| pack.count());
            let percent_of = |slot: usize| {
                targets
                    .iter()
                    .find(|(known, _)| *known == slot)
                    .map_or(0, |(_, percent)| *percent)
            };
            let mut deficit: Vec<(i32, usize)> = (0..slots)
                .map(|slot| {
                    let percent = percent_of(slot);
                    let wanted = (i32::try_from(count).unwrap_or(0) * percent) / 100;
                    let actual = i32::try_from(
                        workers
                            .iter()
                            .filter(|(_, who, slot_of, _)| *who == player && slot_of == &Some(slot))
                            .count(),
                    )
                    .unwrap_or(0);
                    (wanted - actual, slot)
                })
                .collect();
            deficit.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
            let Some((need, slot)) = deficit.first().copied().filter(|(need, _)| *need > 0) else {
                continue;
            };
            let _ = need;
            // **Only an idle villager is moved**, and never twice within the retry window. Moving somebody who is
            // already working means a fresh path search per second (measured: 29 ms each, 11.7 s over 400 game-seconds
            // against 319 ms without the pass), and re-issuing the same order every second is not a policy anyway: a
            // villager who has been told where to go is no longer the engine's business. Shares converge through the
            // villagers who *arrive* - one per pass - and through training.
            // The first free villager **that is also off the retry window**, with no site queued. Every part of that
            // belongs inside the search: asking for "the first free one" and then rejecting it spends the pass on the
            // same villager - which is exactly how this pass failed, with three idle villagers and the same one chosen
            // every second while the other two stood there.
            let free = workers
                .iter()
                .find(|(id, who, _, free)| {
                    *who == player
                        && *free
                        && !self.build_queue.get(id).is_some_and(|q| !q.is_empty())
                        && !self
                            .share_moved
                            .get(id)
                            .is_some_and(|at| self.tick.saturating_sub(*at) < self.share_move_ticks)
                })
                .map(|(id, _, _, _)| *id);
            // Nobody free: the table can still **rebalance**. Take one villager off the column that is over its share,
            // bounded by the same retry window. Without this the shares only ever describe the *first* assignment, and a
            // 32-villager economy sat on 35 wood for 1,000 game-seconds while fourteen villagers fed a granary that
            // nothing was spending.
            let poached = free.or_else(|| {
                let mut over: Vec<(i32, usize)> = (0..slots)
                    .filter_map(|other| {
                        let percent = percent_of(other);
                        let wanted = (i32::try_from(count).unwrap_or(0) * percent) / 100;
                        let actual = i32::try_from(
                            workers
                                .iter()
                                .filter(|(_, who, slot_of, _)| {
                                    *who == player && slot_of == &Some(other)
                                })
                                .count(),
                        )
                        .unwrap_or(0);
                        (actual > wanted).then_some((actual - wanted, other))
                    })
                    .collect();
                over.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
                let (_, from) = over.first().copied()?;
                workers
                    .iter()
                    .find(|(id, who, slot_of, _)| {
                        *who == player
                            && slot_of == &Some(from)
                            && !self.build_queue.get(id).is_some_and(|q| !q.is_empty())
                            && !self.share_moved.get(id).is_some_and(|at| {
                                self.tick.saturating_sub(*at) < self.share_move_ticks
                            })
                    })
                    .map(|(id, _, _, _)| *id)
            });
            let Some(worker) = poached else {
                continue;
            };
            if self.find(worker).is_none() {
                continue;
            }
            // The worker may be carrying: then the load goes to a drop site **first** and the new order waits in
            // `pending_gather`. A villager pulled off wood with ten wood in its arms and sent straight to a farm has
            // thrown those ten away, which is not what a player sees.
            // **The share table is a question about the base, not about the spot the villager happens to stand on**
            // (D149): "eight on wood" means wood near *your* town centre. Measured from the worker instead, a villager
            // spawned away from its base found nothing within its own reach and never moved (the policy test caught it).
            let anchor = self.base_of(player).unwrap_or(worker);
            let node = match self.nodes_of_slot(anchor, slot, 3).into_iter().next() {
                Some(node) => node,
                None => continue,
            };
            self.share_moved.insert(worker, self.tick);
            // **One lookup, not four** (D128): this runs per villager per tick. The entity list does not change here, so the
            // three scans the first version made were pure waste - and one of them was an `expect` that only a broken
            // invariant could ever have triggered.
            let Some(worker_entity) = self.find(worker) else {
                continue;
            };
            let carrying = worker_entity.carry.raw() > 0;
            let task = worker_entity.task;
            if carrying
                && let Task::Gather {
                    node: old,
                    rate_per_tick,
                    carry_cap,
                    ..
                } = task
                && let Some(worker_entity) = self.find(worker)
                && let Some(returning) =
                    self.return_task(worker_entity, old, rate_per_tick, carry_cap)
            {
                if let Some(entity) = self.find_mut(worker) {
                    entity.task = returning;
                }
                self.pending_gather.insert(worker, node);
                let _ = need;
                continue;
            }
            if let Err(error) = self.start_gather(worker, node) {
                self.share_failed.insert(node, self.tick);
                self.refusal = Some((worker, error, self.tick));
            } else {
                self.share_failed.remove(&node);
            }
            let _ = need;
        }
    }
}
