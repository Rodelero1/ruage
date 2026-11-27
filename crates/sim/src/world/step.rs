//! **One tick**: the pass that reads the entity list and produces the next world.
//!
//! Split out of `lib.rs` under D145, verbatim. The function is still **1,566 lines** - that is stage 2's job (named phases);
//! moving the file only stops `lib.rs` being the place every reader has to scroll.

use std::collections::BTreeSet;

use ruage_fixed::Fixed;

use crate::{
    EntityId, EntityKind, Landing, MAX_PLAYERS, Rot, Route, Shot, SimError, Stance, Task, World,
    reaches, step_towards,
};
impl World {
    /// Advance exactly one tick: apply queued commands, integrate, increment the counter.
    ///
    /// Movement is integrated in id order with an exact per-tick delta, so a unit's position
    /// after N ticks is `N * speed / tick_rate` with no accumulated error.
    pub fn step(&mut self) -> Result<(), SimError> {
        self.drain_orders();

        let mut work = self.decide()?;

        // **The queue** (T12 stage 2, D229): `BaseWorld::update` walks the players array from index 0 - Gaia, then
        // 1-8 - and steps each player's objects inside that player's own turn, so a man killed by an earlier player's
        // blow does nothing later in the tick, and a man whose turn came first has already done his work. Ours applies
        // each player's share of the tick's work as one turn, in that order.
        let mut queue = work.bucket(self);
        for work_for_this_turn in queue.turns.iter_mut().take(TURNS) {
            let work_for_this_turn = std::mem::take(work_for_this_turn);
            // **No "empty turn" shortcut** (tried and withdrawn, D230): a turn of empty lists is *not* nothing to do -
            // `shepherd` resumes a gather on a corpse from `self.slaughter`, which is world state and not one of the lists,
            // so skipping the empty turns stopped a villager gathering the sheep he had just killed. Two animal fixtures
            // said so; the saving was 2.5 percentage points of a tick and not worth a second place the rule can live.
            self.apply_turn(work_for_this_turn, &mut work);
        }

        self.step_production()?;
        // **Before the movement pass** (D154): a gate that opens this tick is walkable for this tick's steps; a gate that
        // closes is solid for them.
        self.step_gates();
        self.move_bodies(std::mem::take(&mut work.deltas))?;
        // **The arrival's separation is a placement, not a step** (D356.1): the game writes the position
        // (`func_002697C0` -> `Method15(out_x, out_y)`), and a step here would be refused by the body it separates from.
        let now = self.tick;
        for (id, point) in std::mem::take(&mut work.seats) {
            // **A placement is not a walk.** The man is put down a body's width along the node's side, so the route he was
            // walking is behind him: keep it and every tick is a "past waypoint" re-plan, which measured **138,588** re-paths on
            // the acceptance scene against **12,847** before this rule. The route is cleared here; the gather arm gives him a
            // fresh one when he next needs to walk.
            self.paths.remove(&id);
            if let Some(entity) = self.find_mut(id) {
                entity.position = point;
                entity.last_moved = now;
            }
        }

        self.fly_shots(
            work.shots,
            work.landings,
            work.landed_arrows,
            &mut work.blows,
            &mut work.ejected,
        );
        self.step_world_timers();
        self.eject_garrisons(work.ejected);
        self.sweep_dead(&work.blows);
        self.tick += 1;

        // What each player saw this tick, stamped with the tick that has just become current, so that
        // `sighting(..).1 == tick()` means "watched right now" and the age of a belief is `tick() - at`.
        self.observe();
        Ok(())
    }

    /// **One player's turn**: the work `decide` collected for this player's objects, applied in the order one object's
    /// own update would do it - places, then the fight, then the harvest, the re-tasks, the switch, the herd, the
    /// delivery, the site, the blow, and the return to what a finished fighter was doing.
    ///
    /// `work` is here for the three accumulators that are not one player's - the blows, the shots and the garrisons that
    /// must be thrown out - which the later phases read.
    fn apply_turn(&mut self, mut turn: TurnWork, work: &mut TickWork) {
        // The split ran before any turn; by the time this one comes, an earlier player's blow may have killed one of
        // its actors - and the original's walk would not find him either, because he was destroyed in his own turn.
        turn.keep_the_standing(self);
        // **Before the re-tasks** (D204): a man the ring has refused for long enough is sent to another node this same
        // tick, so the count that decides it and the order that answers it are read from the same state.
        self.count_refused_squares(turn.square_refusals);

        self.resume_fights(turn.engaging, turn.turning, turn.homing);

        self.grant_takes(turn.chops, turn.takes);
        self.send_hunters(turn.hunts);
        self.retask_workers(turn.retasks);
        self.apply_switches(turn.switches);
        self.shepherd(turn.herds);
        self.deposit_loads(turn.deposits);
        self.finish_construction(turn.builders);
        self.land_blows(
            turn.strikes,
            &mut work.blows,
            &mut work.shots,
            &mut work.ejected,
        );
        self.resume_idle(turn.finished);
    }
    /// **Take the queued orders and apply them**, refusing rather than failing the match.
    fn drain_orders(&mut self) {
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
        self.stamp_group();
    }

    /// **Form the tick's group** (D207, narrowed by D237). A **soldier** ordered to move in this tick is a mate of every other
    /// soldier ordered with it: that is the original's path-ignore list, which filters a *formation's* members, and a moving
    /// formation walks over itself. **A villager is never a mate** - Forgotten Empires' own documentation puts villagers, trade
    /// carts and fishing ships in no formation (*"each of them takes their own path"*), and the owner's rule of 2026-11-16 is
    /// that a villager reroutes and never walks through another. Without this, a box-select and a right-click turn a
    /// selection of villagers into a crowd that passes through itself (measured 0.269 tiles apart where the standoff is 0.400).
    fn stamp_group(&mut self) {
        if self.ordered_now.is_empty() {
            return;
        }
        let group = std::mem::take(&mut self.ordered_now);
        for &id in &group {
            let mates: BTreeSet<EntityId> = if self
                .find(id)
                .and_then(|entity| entity.unit)
                .is_some_and(|unit| self.is_military(unit))
            {
                group
                    .iter()
                    .copied()
                    .filter(|other| {
                        *other != id
                            && self
                                .find(*other)
                                .and_then(|entity| entity.unit)
                                .is_some_and(|unit| self.is_military(unit))
                    })
                    .collect()
            } else {
                BTreeSet::new()
            };
            if let Some(route) = self.paths.get_mut(&id) {
                route.mates = mates;
            }
        }
    }

    /// Hunts that found an enemy start the fight, patrols at their far end turn, defensive units go home.
    fn resume_fights(
        &mut self,
        engaging: Vec<(EntityId, EntityId)>,
        turning: Vec<EntityId>,
        homing: Vec<EntityId>,
    ) {
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
                .and_then(|from| self.plan_path(Some(id), from, other).ok());
            match route {
                Some(route) => {
                    self.put_route(
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
                .and_then(|from| self.plan_path(Some(id), from, post).ok());
            match route {
                Some(route) => {
                    self.put_route(
                        id,
                        Route {
                            points: route,
                            ..Route::default()
                        },
                    );
                    if let Some(entity) = self.find_mut(id) {
                        entity.task = Task::Move {
                            to: post,
                            target: None,
                        };
                    }
                }
                None => self.stand_down(id),
            }
        }
    }

    /// **Grant the harvest**: tree damage in `chops`, then the load in `takes`, in id order.
    fn grant_takes(&mut self, chops: Vec<EntityId>, takes: Vec<(EntityId, i64, Fixed)>) {
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
    }

    /// A hunter strikes the animal it was sent to hunt, remembering the gather order to resume on the body.
    fn send_hunters(&mut self, hunts: Vec<(EntityId, EntityId)>) {
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
    }

    /// **Fold in how long each man has stood refused a square** (D204). `decide` reads the world and cannot write it, so
    /// the counts travel as work like the re-tasks do; this is where they land. `None` forgets a man's count - he has a
    /// square, or has just been sent elsewhere - so a man who is seated is never charged for the wait at the node he left.
    fn count_refused_squares(&mut self, refusals: Vec<(EntityId, Option<(EntityId, u32)>)>) {
        for (id, refusal) in refusals {
            match refusal {
                Some((node, ticks)) => {
                    self.square_refused.insert(id, (node, ticks));
                }
                None => {
                    self.square_refused.remove(&id);
                }
            }
        }
    }

    /// The next node of the same kind, for a worker whose own ran out.
    fn retask_workers(&mut self, retasks: Vec<(EntityId, EntityId)>) {
        // The next node of the same kind, for a worker whose own ran out. An ordinary gather order: the load in hand is
        // kept, and the walk is planned by the same code a player's click uses.
        for (worker, node) in retasks {
            if let Err(error) = self.start_gather_internal(worker, node) {
                self.refusal = Some((worker, error, self.tick));
            }
        }
    }
    /// **A blow lands on the tick the attacker is in position**, then every `reload_ticks`, in order.
    fn land_blows(
        &mut self,
        strikes: Vec<(EntityId, EntityId)>,
        blows: &mut Vec<(EntityId, EntityId)>,
        shots: &mut Vec<Shot>,
        ejected: &mut Vec<EntityId>,
    ) {
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
                // in docs/archive/decisions-2026-11-24.md rather than guessed at here (D37).
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
    }

    /// Construction advances one builder-tick per tick, so the finish lands exactly on `create_time x tick_rate`.
    fn finish_construction(&mut self, builders: Vec<(EntityId, EntityId)>) {
        // **Hands are not linear, and the speed is counted per site, not per man** (D222). The game's own arithmetic -
        // `time = 3t / (n + 2)` - means `n` hands add `(n + 2) / 3` builder-ticks a tick: one hand is 1.0, two are 1.333,
        // three are 1.667, ten are 4.0, and no number of hands ever doubles the first one's rate. Counting per builder
        // was exactly linear, so a second hand halved the build; the count is taken once per site so a man who joined
        // late cannot be double-counted either.
        let mut hands: std::collections::BTreeMap<EntityId, (usize, Vec<EntityId>)> =
            std::collections::BTreeMap::new();
        for (builder, site) in builders {
            let entry = hands.entry(site).or_default();
            entry.0 += 1;
            entry.1.push(builder);
        }
        for (site, (count, builders)) in hands {
            // Thousandths of a builder-tick per tick: one hand is 3000, each extra hand 1000.
            let per_tick = 1000u32 * (u32::try_from(count).unwrap_or(u32::MAX).saturating_add(2));
            let builder = builders.first().copied().unwrap_or(EntityId(0));
            if self.work_one_tick(site, builder, per_tick) {
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
                    && let Err(why) = self.start_build_site(builder, unit, at)
                {
                    // A queued site is an order the engine gives itself, so its failure goes into state like any other
                    // (D88's rule). Two orders given in one tick are checked against a world where neither exists yet,
                    // so overlapping sites are accepted and the second can never start; `let _ =` here left the player's
                    // queued building gone with no record at all.
                    self.refusal = Some((builder, why, self.tick));
                }
            }
        }
    }

    /// **Deposit whole resources; the fraction stays in hand**, and the villager turns back for more.
    fn deposit_loads(&mut self, deposits: Vec<EntityId>) {
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
                    (Some(here), Some(there)) => self.plan_path(Some(id), here, there).ok(),
                    _ => None,
                };
                if let Some(route) = route {
                    self.put_route(
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
                if let Err(error) = self.start_gather_internal(id, next) {
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
                    && let Err(error) = self.start_gather_internal(id, next)
                {
                    self.refusal = Some((id, error, self.tick));
                }
            }
        }
    }

    /// A shepherd strikes the animal it was sent to work, where the animal stands (D116).
    fn shepherd(&mut self, herds: Vec<(EntityId, EntityId)>) {
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
            // **The game's own distance** (D285, D286): the gap between the two boxes, not between the two centres. A body
            // that walks at another stops with its box against the other's, and on a diagonal its centre is then farther
            // away than the radii sum - so the centre test said "not in reach" for a shepherd standing at its sheep.
            let gap = self.box_gap_sq(
                (here.0, here.1, self.body_radius_of(villager)),
                (there.0, there.1, self.node_radius(animal)),
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
            let beside = self.tile_of(here) == self.tile_of(there);
            if !beside && gap > i128::from(step) * i128::from(step) {
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
                        if let Err(error) = self.start_gather_internal(villager, animal) {
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
    }
    /// **Production and research**: one pass over the entities, because both timers are the same shape.
    fn step_production(&mut self) -> Result<(), SimError> {
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
            // **Dead is dead for the rest of the tick** (T12, U18): the original destroys a dead object in its own turn,
            // so a building killed before its turn hands nothing over. Ours removes the dead in a sweep after every
            // phase, so without this check a ruin delivers the recruit it had one tick left on. The owner is read here
            // too, so this is one lookup where the old code spent two.
            let Some(player) = self
                .find(id)
                .filter(|entity| entity.hp > 0)
                .map(|entity| entity.player)
            else {
                continue;
            };
            if capped[usize::from(player).wrapping_sub(1).min(MAX_PLAYERS - 1)] {
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
                // **The ceiling is read at the spawn, not once a tick** (D231). It is a fact about the world, and the
                // world changes as this phase's buildings finish: two barracks with one slot left must not both deliver,
                // which is what a `capped` computed before the first of them spawned did (measured: population 6 against
                // a cap of 5). Recomputed only when a job actually completes, so the cost is a scan per unit trained
                // rather than per building per tick.
                capped[usize::from(player).wrapping_sub(1).min(MAX_PLAYERS - 1)] =
                    self.population(player) >= self.population_cap(player);
            }
        }
        // Research: the same timer, and the effect lands when it reaches zero.
        for (id, player, tech) in researchers {
            // The same gate as production, and the same reason (T12): a building killed before its turn finishes no
            // research either.
            if self.find(id).is_none_or(|entity| entity.hp <= 0) {
                continue;
            }
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
        // **The next job in the queue** (D157): a building with nothing running takes the head of its queue - a unit or a tech,
        // whichever was ordered first. Last, so a job that finished this tick has already advanced the queue.
        self.start_next_jobs()?;
        Ok(())
    }

    /// Units whose job ended go back to what they were doing before it.
    fn resume_idle(&mut self, finished: Vec<EntityId>) {
        let mut resumed: Vec<EntityId> = Vec::new();
        for id in finished {
            if self.resuming.contains_key(&id) {
                // The fight ended and the unit still has a place to be: attack-move's whole point.
                resumed.push(id);
            } else {
                self.stand_down(id);
            }
        }
        for id in resumed {
            let Some(task) = self.resuming.remove(&id) else {
                continue;
            };
            match task {
                // The fight ended and the unit still has a place to be: attack-move's whole point.
                Task::Hunt { to } => {
                    let route = self
                        .find(id)
                        .map(|entity| entity.position)
                        .and_then(|from| self.plan_path(Some(id), from, to).ok());
                    match route {
                        Some(route) => {
                            self.put_route(
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
                // **A gather a step-aside interrupted** (D183): the same order, re-resolved against the tables, which is
                // what makes the worker go back to its node instead of standing idle beside it. If the node is gone the
                // order is refused, and the worker stands - the answer a gather to a dead node gives anyway.
                Task::Gather { node, .. } => {
                    if self.start_gather_internal(id, node).is_err() {
                        self.stand_down(id);
                    }
                }
                _ => self.stand_down(id),
            }
        }
    }

    /// **One tile past the nearest edge of a footprint**, from a tile inside it: the shortest way a builder standing on
    /// his own site can step off it. Plain arithmetic rather than a path search, because this runs on a tick.
    fn edge_out(&self, rect: (u16, u16, u16, u16), here: (u16, u16)) -> (u16, u16) {
        let (ax, ay, wide, tall) = rect;
        let left = here.0.saturating_sub(ax);
        let right = ax
            .saturating_add(wide)
            .saturating_sub(1)
            .saturating_sub(here.0);
        let top = here.1.saturating_sub(ay);
        let bottom = ay
            .saturating_add(tall)
            .saturating_sub(1)
            .saturating_sub(here.1);
        let least = left.min(right).min(top).min(bottom);
        if least == left {
            (ax.saturating_sub(1), here.1)
        } else if least == right {
            (ax.saturating_add(wide), here.1)
        } else if least == top {
            (here.0, ay.saturating_sub(1))
        } else {
            (here.0, ay.saturating_add(tall))
        }
    }

    /// **The movement pass**: collide, move, repath around bodies, spend reached waypoints.
    /// **Walk an arrived worker onto its working standoff** (D169, D180). The caller has already checked that the point is free
    /// ground, so this cannot put a worker inside anything.
    ///
    /// It used to *place* the worker there, in one tick, which is the owner's *"villagers jump towards the bush"*: a worker that
    /// arrives on the tile beside a node is 1.0 tile from its centre and the standoff is 0.70, so the placement threw it 0.3 of a
    /// tile - measured at **15,600 raw units in one tick** where its speed allows 1,000. It now takes the step the body could take:
    /// as far as its speed allows, straight at the standoff, and exactly onto it when the remainder fits in one step. The worker is
    /// at its tree one or two ticks later and never moves faster than it can walk.
    /// **How long a body must be refused before the body in its way is nudged** (D200). Two seconds at 20 Hz: long enough
    /// that a walker who can get round does get round - which is every case the evasion tests already cover, and why they
    /// keep passing - and short enough that a man who cannot is not lost for the match.
    const STALLED_TICKS: u32 = 40;

    fn fly_shots(
        &mut self,
        shots: Vec<Shot>,
        landings: Vec<Landing>,
        landed_arrows: Vec<EntityId>,
        blows: &mut Vec<(EntityId, EntityId)>,
        ejected: &mut Vec<EntityId>,
    ) {
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
    }

    /// The once-a-pass timers: leashes, herdable conversion, shares, spent nodes, husks.
    fn step_world_timers(&mut self) {
        self.step_leashes();
        // ... and a herdable changes hands as soon as somebody's unit can see it (item 5).
        self.step_conversion();
        // Shares: idle villagers follow the table content gives (the game's own AI shape).
        self.step_shares();
        self.step_spent_nodes();
        self.step_husks();
        // A building at or below its eject fraction throws its garrison out. They are unharmed - the
        // published rule - and they land where they already stand.
    }

    /// A building at or below its eject fraction throws its garrison out, unharmed and where they stand.
    fn eject_garrisons(&mut self, ejected: Vec<EntityId>) {
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
    }

    /// **The death sweep**: corpses, rot, the dead and their bookkeeping, and the match decision it carries.
    fn sweep_dead(&mut self, blows: &[(EntityId, EntityId)]) {
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
        self.promote_dead_animals(blows, &mut new_rots);
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
                && let Some(walk) = self.ground.as_mut()
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
            self.stalled_sites.remove(&id);
            self.slaughter.retain(|_, animal| *animal != id);
            self.herdable_ids.remove(&id);
        }

        // A seat can only disappear by dying, so the match is only ever decided on a tick where
        // something did. The idle tick pays one boolean for it.
        if something_died {
            self.decide_outcome();
        }
    }

    /// A task change decided in the pass becomes a route and a task after it.
    fn apply_switches(&mut self, switches: Vec<(EntityId, Task)>) {
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
                    (Some(from), Some(to)) => {
                        self.plan_path(Some(id), from, to).ok().or_else(|| {
                            self.approach_tile(self.tile_of(from), drop)
                                .and_then(|stand| {
                                    self.plan_path(Some(id), from, self.tile(stand)).ok()
                                })
                        })
                    }
                    _ => None,
                };
                if let Some(route) = route {
                    self.put_route(
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
    }

    /// **Decide the tick, change nothing**: one pass over the entities in id order, filling a `TickWork`.
    fn decide(&self) -> Result<TickWork, SimError> {
        let tick_rate = i64::from(self.tick_rate_hz);

        // Decide this tick's work in id order, then apply it. Deciding first keeps the second
        // pass free of lookups, so an entity cannot be moved *and* read from in the same tick in
        // an order-dependent way.
        // (index, id, dx, dy): the index is the fast path and the id is the check. `insert` keeps
        // the list sorted by id, so anything spawned later in the same tick shifts every index after
        // it - the id is what makes that detectable instead of silently moving the wrong entities.
        let mut work = TickWork {
            deltas: Vec::with_capacity(self.entities.len()),
            ..TickWork::default()
        };
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
                        work.finished.push(entity.id);
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
                    // **Box to box, in both arms** (D142, D286). The dat's range is measured between the bodies' edges, so a
                    // 4x4 town centre is hit from its wall rather than its middle - and melee is the same measure with a range
                    // of zero. The centre distance cannot answer it: a mover stopped by the target's **box** on a diagonal is
                    // farther away by its centres than the two radii sum, so a radii test said "not in reach" while the bodies
                    // were touching. Measured, a villager stood against a sheep with its boxes touching and no blow struck.
                    let edge = self.gap_squared_to(
                        entity.position,
                        self.body_radius(entity.unit),
                        target_body,
                    );
                    let in_reach = if range.raw() > 0 {
                        edge <= range.raw() * range.raw()
                    } else {
                        adjacent || edge <= 0
                    };
                    let _ = contact;
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
                            work.homing.push(entity.id);
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
                        work.strikes.push((entity.id, target));
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
                    let Some((target, site_unit, farm)) = self
                        .find(site)
                        .map(|s| (s.position, s.unit, self.is_farm_role(s)))
                    else {
                        work.finished.push(entity.id);
                        continue;
                    };
                    // **A builder works from outside the site, and never from on it** (D183 §3). A footprint's own tiles
                    // are walkable while it is a ghost, so "beside the building" is measured against the whole footprint
                    // rather than its anchor tile - a builder standing on his own site would hold it up waiting for
                    // himself, which is exactly what a house that took one builder-tick in five hundred looked like.
                    let here = self.tile_of(entity.position);
                    let rect = self.footprint_rect(site);
                    let (inside, gap) = match rect {
                        Some((ax, ay, wide, tall)) => (
                            here.0 >= ax
                                && here.0 < ax.saturating_add(wide)
                                && here.1 >= ay
                                && here.1 < ay.saturating_add(tall),
                            // Chebyshev distance from this tile to the nearest tile of the rectangle.
                            (
                                if here.0 < ax {
                                    ax - here.0
                                } else if here.0 >= ax.saturating_add(wide) {
                                    here.0 - (ax.saturating_add(wide) - 1)
                                } else {
                                    0
                                },
                                if here.1 < ay {
                                    ay - here.1
                                } else if here.1 >= ay.saturating_add(tall) {
                                    here.1 - (ay.saturating_add(tall) - 1)
                                } else {
                                    0
                                },
                            ),
                        ),
                        None => {
                            let there = self.tile_of(target);
                            (false, (here.0.abs_diff(there.0), here.1.abs_diff(there.1)))
                        }
                    };
                    let reach = self.reach_raw(entity.unit, site_unit);
                    let (gap_x, gap_y) =
                        (i64::from(gap.0) * self.scale, i64::from(gap.1) * self.scale);
                    // **The builder is a body, not a tile** (D250). Two questions, both about his own circle:
                    //
                    // * `inset` - how far his centre is **inside** the footprint (negative outside it), which is what
                    //   "at least half a diameter inside the farm plot" means;
                    // * `apart` - how far his centre is **outside** it, which is what "clear of the wall" means.
                    //
                    // The tile rule that used to decide this let a builder start a house from **0.02 tiles** outside its
                    // footprint; the first stroke then made those tiles block with his own body **0.18** inside them,
                    // which is the crossing the owner's rule forbids (measured, D250).
                    let radius = self.body_radius_of(entity.id);
                    let room = rect.map(|(ax, ay, wide, tall)| {
                        let scale = self.scale;
                        let (left, top) = (i64::from(ax) * scale, i64::from(ay) * scale);
                        let (right, bottom) = (
                            left + i64::from(wide) * scale,
                            top + i64::from(tall) * scale,
                        );
                        let (x, y) = (entity.position.0.raw(), entity.position.1.raw());
                        let inset = (x - left).min(right - x).min(y - top).min(bottom - y);
                        let (ox, oy) = (
                            (left - x).max(x - right).max(0),
                            (top - y).max(y - bottom).max(0),
                        );
                        (inset, (ox * ox + oy * oy).isqrt())
                    });
                    // **A farm is built from inside, every other building from outside** (D250, D183 §3 and §7). A farm
                    // is a floor whose site-clear gate does not apply, so nothing is held up by its builder standing on
                    // it, and the game's builder seeds the plot under his own feet.
                    // **Never from inside the footprint** otherwise, however close it is: the reach term is measured from
                    // the nearest footprint tile, which is zero for a builder standing on the site - and a body whose
                    // circle overlaps the edge is not clear of the wall the first stroke is about to raise.
                    let adjacent = match (farm, room) {
                        (true, Some((inset, _))) => inset >= radius,
                        (false, Some((_, apart))) => {
                            !inside
                                && apart >= radius
                                && ((gap.0 <= 1 && gap.1 <= 1)
                                    || gap_x * gap_x + gap_y * gap_y <= reach * reach)
                        }
                        (_, None) => {
                            !inside
                                && ((gap.0 <= 1 && gap.1 <= 1)
                                    || gap_x * gap_x + gap_y * gap_y <= reach * reach)
                        }
                    };
                    if adjacent {
                        work.builders.push((entity.id, site));
                        (Fixed::ZERO, Fixed::ZERO)
                    } else {
                        let delta = entity.speed.div_exact(tick_rate).map_err(|err| {
                            SimError::Displacement {
                                id: entity.id,
                                detail: err.to_string(),
                            }
                        })?;
                        let next = if farm {
                            // **The plot's middle**: a farm's builder belongs inside it, so his aim is the centre of the
                            // plot rather than a way out of it (D250). *The same* middle the build order routed him to
                            // (`World::footprint_centre`): for an even-width plot a tile's centre and the plot's centre are
                            // half a tile apart, and two definitions of "the middle" is one too many (found by the
                            // adversarial review, D209).
                            self.footprint_centre(site).unwrap_or(target)
                        } else if inside {
                            // **The shortest way out**, one tile past the nearest edge. A builder left standing on the
                            // site would never start it, and the route it walked is spent, so this has to be a place
                            // rather than a waypoint.
                            match rect {
                                Some((ax, ay, wide, tall)) => {
                                    self.tile_centre(self.edge_out((ax, ay, wide, tall), here))
                                }
                                None => target,
                            }
                        } else {
                            self.next_waypoint(entity.id).unwrap_or(target)
                        };
                        let (dx, dy) = step_towards(entity.position, next, delta.raw());
                        (Fixed::from_raw(dx), Fixed::from_raw(dy))
                    }
                }
                Task::Return { drop, .. } => {
                    let Some((target, drop_unit)) = self.find(drop).map(|n| (n.position, n.unit))
                    else {
                        work.finished.push(entity.id);
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
                    // **A load leaves at the wall** (D246, the owner's rule 3). The test used to be three loose clauses -
                    // tile adjacency by index, the *pair's* reach (0.4) against the building's **anchor point**, and
                    // `touches_footprint`'s whole tile of slack - and between them a carrier banked a load **0.281 tiles**
                    // from the town centre. It is now the distance to the **footprint** against his own radius, with **one
                    // step of slack**: the disc is refused the step that would put it inside the wall, so the closest he can
                    // stop is one step short of touching - measured, **4,155 raw** against a **4,000** radius, a step being
                    // **758**. "At the wall" is within the radius plus his own step.
                    let radius = self.body_radius_of(entity.id);
                    let gap = self.footprint_gap(entity.position, drop);
                    let _ = (here, there, drop_unit);
                    // **The wall, and nothing looser** (D246, D313). The old second clause - bank from anywhere within
                    // `radius + a quarter tile` once the route ran out - was written to rescue a carrier whose neighbouring
                    // tile was taken, and it did: measured, one never banked at all in 400,000 ticks
                    // (`research::two_players_can_be_different_civilisations`, 890 of 900). But it also licensed a bank from
                    // 0.45 tiles, which is past the owner's own rule, and the rescue is no longer needed now that a step may
                    // **touch** a wall (D306): the carrier walks the last stretch below and banks at it.
                    let at_the_wall = gap.is_some_and(|gap| gap <= radius + delta.raw());
                    if at_the_wall {
                        work.deposits.push(entity.id);
                        (Fixed::ZERO, Fixed::ZERO)
                    } else {
                        // **A carrier with no waypoint aims at the yard, not at the building's anchor** (D200). The anchor
                        // is blocked ground, so stepping at it is refused for ever: measured, a loaded carrier at
                        // `(11.85, 21.14)` in `Return` for **400,000 ticks** with ten units in his hands, the bank stuck at
                        // 790 of 900. `approach_tile` is the walkable tile beside the footprint - it exists for exactly this.
                        // **The last stretch is walked to the wall, not to a tile centre** (D246). A tile centre beside a
                        // footprint is half a tile from it, so a carrier aiming at one can never come within his own radius -
                        // measured, one carrier never banked at all in 400,000 ticks (`research::
                        // two_players_can_be_different_civilisations`, the bank stuck at 890 of 900).
                        let next = self.next_waypoint(entity.id).unwrap_or_else(|| {
                            self.wall_point(entity.position, drop, radius)
                                .unwrap_or(target)
                        });
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
                    let Some((target, node_unit)) = self.find(node).map(|n| (n.position, n.unit))
                    else {
                        // The node is gone (a later increment may delete emptied nodes).
                        work.finished.push(entity.id);
                        continue;
                    };
                    // **A spent node is done with, wherever the man stands** (D279): the re-task used to live inside the
                    // `at_node` branch, so a worker walking *to* a node that emptied under him kept walking to it for the
                    // rest of the match. The next source of the same kind, or the order ends.
                    if self.find(node).is_some_and(|n| {
                        matches!(n.kind, EntityKind::Resource(_)) && n.amount.raw() <= 0
                    }) {
                        match self.nearest_like(entity.position, node, entity.id) {
                            Some(next) => work.retasks.push((entity.id, next)),
                            None => work.finished.push(entity.id),
                        }
                        continue;
                    }
                    let is_resource = matches!(
                        self.find(node).map(|n| n.kind),
                        Some(EntityKind::Resource(_))
                    );
                    // **"At the node" is the game's box gap** (D285): the gap between the villager's box and the node's,
                    // against a range of zero - the boxes touch. This replaces four of our own rules (`touching` with its
                    // tolerance, `own_place` and its eighth of a tile, `ring_open`, `beside_adjacent`): the game has one
                    // distance and they were four approximations of it.
                    let body = self.box_gap_sq(
                        (
                            entity.position.0,
                            entity.position.1,
                            self.body_radius(entity.unit),
                        ),
                        (target.0, target.1, self.node_radius(node)),
                    );
                    // **As close as the ground lets him be** (D286). The game's arrival is the box gap against the order's
                    // range, and for a job that is zero - the boxes touch; its movement is continuous, so the tick before
                    // contact is already at it. Ours moves in whole steps, and a step that would end inside the node's box is
                    // **cut at the face**, so the man comes to rest on it - but the settle can seat him a fraction short,
                    // and an equality then tells a man standing against a mine that he is not at it: measured, a miner with
                    // `progress 0` and the bank stuck at 100 for 400,000 ticks. One step is the quantized form of the same
                    // rule, and it is the "a radius, not an equality" shape C2 already settled elsewhere.
                    let step = entity
                        .speed
                        .div_exact(i64::from(self.tick_rate_hz).max(1))
                        .map_or(0, |delta| delta.raw());
                    let at_node = body <= i128::from(step) * i128::from(step);
                    // **No seat is walked to any more** (D293, owner 2026-11-19: *"Drop it and let the crowds jam"*). The settle
                    // arm used to walk a gatherer the last half tile onto a private place round the node, one man per point, so no
                    // two men ever pressed. The original hands out no seats: a man goes to the resource he was given, the bodies are
                    // solid, and friends press into each other with no floor at all (D290). `at_node` (the box gap against the step,
                    // D285) is what entitles him to take.
                    //
                    // **And the crowd is still spread, by the rule that spread it in the game** (D204, kept by D291: the engine may
                    // re-target within a kind, never across). With the seat book gone this is the only thing that moves a man off a
                    // node that will not seat him, and its trigger is now the **refusal count itself** - a seat that never existed
                    // cannot refuse him. `STALLED_TICKS` keeps it from being the per-tick re-plan D182 measured at 35 s -> 400 s.
                    // **Refused means arrived and still not taking**, not merely on the way: a man walking in has nothing to look at
                    // yet, and counting his walk would re-task every gatherer 40 ticks after he set off (measured: three of three
                    // `gather_touching` fixtures red for exactly that).
                    let arrived = self
                        .paths
                        .get(&entity.id)
                        .is_none_or(|route| route.points.is_empty());
                    if is_resource && arrived && !at_node {
                        let waited = self
                            .square_refused
                            .get(&entity.id)
                            .copied()
                            .filter(|(at, _)| *at == node)
                            .map_or(0, |(_, count)| count);
                        if waited >= Self::STALLED_TICKS {
                            match self.nearest_like_free(entity.position, node, entity.id) {
                                Some(next) => {
                                    work.square_refusals.push((entity.id, None));
                                    work.retasks.push((entity.id, next));
                                }
                                // **Nowhere else has room**: he stands and looks again after the same wait.
                                None => work.square_refusals.push((entity.id, Some((node, 0)))),
                            }
                        } else {
                            work.square_refusals
                                .push((entity.id, Some((node, waited.saturating_add(1)))));
                        }
                    } else if self.square_refused.contains_key(&entity.id) {
                        // He is on his node, or away from it: the count belongs to the node he has left.
                        work.square_refusals.push((entity.id, None));
                    }

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
                        work.herds.push((entity.id, node));
                    }

                    // A **huntable** animal is shot, not gathered from, and the shot is ordered **before** the walk:
                    // a hunter that first walked onto the boar would be inside knife range at the first swing, and
                    // the bow (VMHUN, `range = 3.0`) would never matter. A herdable is walked home first and killed at
                    // the drop site, which is the herding pass's business.
                    if self.animal_at(node) == Some(false) {
                        work.hunts.push((entity.id, node));
                    }

                    // **An arriving man's place is a placement, not a shared point** (D356's ruling 1, the game's own rule -
                    // `func_002699A0`: step out by the pair's radii plus a hundredth, refusing any candidate another body
                    // holds). The owner, 2026-11-22: *"the second man steps to the next free spot near the same tree/mine, and
                    // only goes idle if there is none"*. Measured on the acceptance scene the day this landed: **144,534**
                    // overlapping pair-ticks, deepest a **full footprint** (8,000 raw), a gatherer pair closing to **5,600**
                    // against the **8,000** their bodies need. The step is tangential so he stays at the node (a ring tile is
                    // half a tile off the face and a man sent to one stops taking), and only the later man yields.
                    if at_node
                        && let Some(seat) = self.arrival_seat(entity.id, node, entity.position)
                    {
                        work.seats.push((entity.id, seat));
                        (Fixed::ZERO, Fixed::ZERO)
                    } else if at_node {
                        // Standing still at the thing being worked is what entitles a worker to take from it.
                        // **A standing tree is chopped down first.** Until its hit points are gone it yields nothing;
                        // when they are, it falls where it stands and is then cut for wood (the owner's rule). A bush or
                        // a mine has no hit points, so this branch never runs for them.
                        // Only a **resource node** is felled. A farm is a building with hit points of its own, and
                        // chopping it was how the first version of this left every farmer standing in a field.
                        let standing = self
                            .find(node)
                            .is_some_and(|n| matches!(n.kind, EntityKind::Resource(_)) && n.hp > 0);
                        if standing {
                            work.chops.push(entity.id);
                            (Fixed::ZERO, Fixed::ZERO)
                        } else {
                            let (take, kept) = self.take_amount(entity);
                            let node_left = self.find(node).map(|n| n.amount.raw()).unwrap_or(0);
                            work.takes.push((entity.id, take, kept));

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
                                work.switches.extend(
                                    self.return_task(entity, node, rate_per_tick, carry_cap)
                                        .map(|task| (entity.id, task)),
                                );
                            }
                            if take == 0
                                && empty_node
                                && !work.switches.iter().any(|(id, _)| *id == entity.id)
                            {
                                // Nothing in hand and nothing left here: look for the next node of the same kind before
                                // standing still. A tree holds 100 wood, so without this a lumberjack works one tree and
                                // idles for the rest of the match - which is what stopped the acceptance match at 139
                                // wood (the owner's rule: "move to next tree, next bush, next pile of gold").
                                // No finish when the man is already going home: `resume_idle` runs after the switches,
                                // so a finish here would stand down a carrier the load rule just sent home - measured,
                                // an eleventh man holding two wood idled on a dried patch while the bank waited.
                                let here = entity.position;
                                match self.nearest_like(here, node, entity.id) {
                                    Some(next) => work.retasks.push((entity.id, next)),
                                    None => work.finished.push(entity.id),
                                }
                            }
                            (Fixed::ZERO, Fixed::ZERO)
                        }
                    } else if matches!(entity.task, Task::Gather { progress, .. } if progress.raw() > 0)
                    {
                        // **A man already taking from the node does not walk at it** (D246, the owner's rule 4). This is the
                        // last step the fixture caught: he was in reach, taking wood, and still spent the tick walking
                        // **0.011 tiles** toward the node - `settling` was false because he stood on his own place, so the
                        // step fell through to here.
                        (Fixed::ZERO, Fixed::ZERO)
                    } else {
                        let delta = entity.speed.div_exact(tick_rate).map_err(|err| {
                            SimError::Displacement {
                                id: entity.id,
                                detail: err.to_string(),
                            }
                        })?;
                        // **With no waypoint, walk to his own place, not at the node** (D248). The node's tile is blocked
                        // ground and `step_towards` at it is refused for ever: measured, a woodcutter froze on a tile
                        // boundary (x = 24.5) with `progress 0` and 8,000,000 in his node, the bank stuck at 890 of 900 after
                        // 400,000 ticks. The place is walkable ground next to the node, which is where he is going anyway.
                        // **The node's own tile is never the aim** (D248). It is blocked ground, so a route that ends on it
                        // sends the man at a wall for ever: measured, a woodcutter pressed against the tree's tile with
                        // **clearance 528** and his own place 0.03 tiles away, re-planning 5,214 times in 400,000 ticks and
                        // never moving. His place is walkable ground beside the node, which is where the order points.
                        let next = match self.next_waypoint(entity.id) {
                            // **The order points at the node** (D293): there is no place beside it to prefer.
                            Some(point) if self.tile_of(point) == self.tile_of(target) => point,
                            Some(point) => point,
                            None => target,
                        };
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
                        work.landings.push(Landing {
                            shooter,
                            target,
                            damage,
                            aim,
                            blast,
                        });
                        work.landed_arrows.push(entity.id);
                    }
                    (Fixed::from_raw(dx), Fixed::from_raw(dy))
                }
                // Inside a building: no movement, no work, and no cooldown runs - a sheltered unit is
                // simply not on the map.
                Task::Move { to, .. } => {
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
                    // Arrival is exact here; an order that a standing body makes impossible is finished by
                    // `finish_blocked_orders`, which knows *why* it cannot arrive rather than guessing from distance.
                    // **Arrival is the reach, and nothing else** (P5, 2026-11-19). Two of our own rules used to end this walk:
                    // `beside` (finished within 1.25 tiles of a goal that holds a body, D200) and the forty-tick
                    // `finish_blocked_orders` (D200). The original has neither - wedged villagers never resolve
                    // (`docs/archive/research/gather-seating-evidence.md`) and its press depth is unbounded because the separation it computes is
                    // discarded (`docs/archive/research/press-depth-evidence.md`, D290) - so a walk that cannot arrive stands, its route intact,
                    // until the player re-tasks.
                    if reaches(entity.position, to, delta.raw()) {
                        work.finished.push(entity.id);
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
                        work.engaging.push((entity.id, enemy));
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
                                work.turning.push(entity.id);
                            } else {
                                work.finished.push(entity.id);
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
            work.deltas.push((index, entity.id, dx, dy));
        }

        // **The blow pass runs in the original's order, not the id's** (T12, U18). `BaseWorld::update` walks the players
        // array from index 0 - Gaia, then 1-8 - and steps each player's objects in slot order, so the first blow between
        // two players belongs to the lower player's man. Entity ids are creation order, which inside one player *is* the
        // slot; what id order loses is the player, and with it which of two mutually lethal men dies.
        work.strikes
            .sort_by_key(|(attacker, _)| self.turn_key(*attacker));

        Ok(work)
    }

    /// **Where an entity's turn falls in the tick** (T12, U18): the players array by index - Gaia at 0, then 1-8 - and
    /// each player's objects in slot order. An id that is not in the world sorts last, which keeps the key total without
    /// inventing a place for it.
    fn turn_key(&self, id: EntityId) -> (u8, u32) {
        self.entities
            .binary_search_by_key(&id, |entity| entity.id)
            .map_or((u8::MAX, id.0), |at| (self.entities[at].player, id.0))
    }

    /// **Which turn an object acts in** (T12 stage 2): its owner's index - Gaia at 0, then the players - or `None` when no
    /// turn will find it, which is an object that is gone or one whose health has already run out. A resource node is not
    /// a special case: it has `hp = 0` by construction and it still lays its gather ring, so "dead" is "no health and not
    /// a node" rather than a bare health test.
    fn turn_slot(&self, id: EntityId) -> Option<usize> {
        let at = self
            .entities
            .binary_search_by_key(&id, |entity| entity.id)
            .ok()?;
        let entity = self.entities.get(at)?;
        let acts = entity.hp > 0 || matches!(entity.kind, EntityKind::Resource(_));
        let slot = usize::from(entity.player);
        (acts && slot < TURNS).then_some(slot)
    }
}

/// **What one tick decided to do, before anything is applied** (D146): the vectors `decide` fills and the phases consume.
///
/// It exists so `step` reads as a list of phases rather than 480 lines of declaration and one loop. Nothing here is hashed or
/// carried between ticks: it dies with the call.
#[derive(Default)]
struct TickWork {
    deltas: Vec<(usize, EntityId, Fixed, Fixed)>,
    takes: Vec<(EntityId, i64, Fixed)>,
    chops: Vec<EntityId>,
    finished: Vec<EntityId>,
    switches: Vec<(EntityId, Task)>,
    herds: Vec<(EntityId, EntityId)>,
    deposits: Vec<EntityId>,
    builders: Vec<(EntityId, EntityId)>,
    strikes: Vec<(EntityId, EntityId)>,
    retasks: Vec<(EntityId, EntityId)>,
    hunts: Vec<(EntityId, EntityId)>,
    blows: Vec<(EntityId, EntityId)>,
    landed_arrows: Vec<EntityId>,
    landings: Vec<Landing>,
    shots: Vec<Shot>,
    ejected: Vec<EntityId>,
    engaging: Vec<(EntityId, EntityId)>,
    turning: Vec<EntityId>,
    homing: Vec<EntityId>,
    /// Men who chose a place on a node's ring this tick and must now keep it (D186 addendum 5).
    places: Vec<(EntityId, EntityId, (Fixed, Fixed))>,
    /// **Men whose place at a node must be written, not stepped** (D356's ruling 1). The game's separation is a
    /// *placement*: `func_002697C0` calls the child's own set-position after `func_002699A0` accepts a point. A step would
    /// be refused by the very body it is separating from - measured, a man held the seat 7,000 raw away and never moved, the
    /// delta refused every tick. So this is applied after the movement pass, as a write.
    seats: Vec<(EntityId, (Fixed, Fixed))>,
    /// **How long each man has stood refused a square** (D204): `Some((node, ticks))` is a man still refused at that
    /// node, `None` is a man who has a square - or has just been sent to another one - so his count goes. Carried here
    /// because `decide` reads the world and never writes it; `count_refused_squares` folds these in after the loop.
    square_refusals: Vec<(EntityId, Option<(EntityId, u32)>)>,
}

/// **How many turns a tick has** (T12 stage 2): Gaia at index 0, then the eight players `BaseWorld::update` walks.
const TURNS: usize = MAX_PLAYERS + 1;

/// **One player's share of the tick's work**: what `decide` decided for that player's objects, in the order one object's
/// own update does it.
#[derive(Default)]
struct TurnWork {
    places: Vec<(EntityId, EntityId, (Fixed, Fixed))>,
    square_refusals: Vec<(EntityId, Option<(EntityId, u32)>)>,
    engaging: Vec<(EntityId, EntityId)>,
    turning: Vec<EntityId>,
    homing: Vec<EntityId>,
    chops: Vec<EntityId>,
    takes: Vec<(EntityId, i64, Fixed)>,
    hunts: Vec<(EntityId, EntityId)>,
    retasks: Vec<(EntityId, EntityId)>,
    switches: Vec<(EntityId, Task)>,
    herds: Vec<(EntityId, EntityId)>,
    deposits: Vec<EntityId>,
    builders: Vec<(EntityId, EntityId)>,
    strikes: Vec<(EntityId, EntityId)>,
    finished: Vec<EntityId>,
}

/// **The tick's work, split into turns** (T12 stage 2), built once per tick so the split costs one pass over each list
/// rather than one pass per player per list.
#[derive(Default)]
struct Queue {
    turns: Vec<TurnWork>,
}

impl TickWork {
    /// **Split the work into one turn per player.** An item whose actor this play is not - another player's object, or one
    /// whose health ran out earlier in the tick - is **dropped**, which is the rule the original gets for free by walking
    /// a player's objects only and destroying a dead one in its own turn.
    fn bucket(&mut self, world: &World) -> Queue {
        let mut queue = Queue {
            turns: (0..TURNS).map(|_| TurnWork::default()).collect(),
        };
        // Two shapes: a list of ids, and a list of tuples whose first element is the actor.
        macro_rules! bucket {
            ($field:ident) => {
                for item in self.$field.drain(..) {
                    if let Some(slot) = world.turn_slot(item) {
                        queue.turns[slot].$field.push(item);
                    }
                }
            };
            ($field:ident, $at:tt) => {
                for item in self.$field.drain(..) {
                    if let Some(slot) = world.turn_slot(item.$at) {
                        queue.turns[slot].$field.push(item);
                    }
                }
            };
        }
        bucket!(places, 0);
        bucket!(square_refusals, 0);
        bucket!(engaging, 0);
        bucket!(turning);
        bucket!(homing);
        bucket!(chops);
        bucket!(takes, 0);
        bucket!(hunts, 0);
        bucket!(retasks, 0);
        bucket!(switches, 0);
        bucket!(herds, 0);
        bucket!(deposits);
        bucket!(builders, 0);
        bucket!(strikes, 0);
        bucket!(finished);
        queue
    }
}

impl TurnWork {
    /// **Drop what this turn will not find.** The bucket pass ran before any turn began, so it could only judge health at
    /// the tick's opening; an earlier player's blow may have killed one of this turn's actors since. The owner half cannot
    /// change during a tick, so this is the health half and nothing else.
    fn keep_the_standing(&mut self, world: &World) {
        macro_rules! keep {
            ($field:ident) => {
                self.$field.retain(|item| world.turn_slot(*item).is_some());
            };
            ($field:ident, $at:tt) => {
                self.$field
                    .retain(|item| world.turn_slot(item.$at).is_some());
            };
        }
        keep!(places, 0);
        keep!(square_refusals, 0);
        keep!(engaging, 0);
        keep!(turning);
        keep!(homing);
        keep!(chops);
        keep!(takes, 0);
        keep!(hunts, 0);
        keep!(retasks, 0);
        keep!(switches, 0);
        keep!(herds, 0);
        keep!(deposits);
        keep!(builders, 0);
        keep!(strikes, 0);
        keep!(finished);
    }
}
