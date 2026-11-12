//! **Fighting**: who is in reach, one blow, one volley, and the aiming that decides whether a shot lands (D145).
//!
//! Split out of `lib.rs` under D145, verbatim. The strike rule itself is D142: a shot is measured **box to box**, and
//! `in_strike_range` is the one helper both the tick's blow and the route-ending test call.
use ruage_data::Unit;
use ruage_fixed::Fixed;

use crate::{
    Entity, EntityId, EntityKind, OCTANTS, Route, Shot, SimError, Stance, StepDecision, Task,
    World, damage_between, integer_sqrt, scale_fixed, splitmix64,
};
impl World {
    /// The checks an attack order has to survive.
    pub(crate) fn check_attack(
        &self,
        attacker: EntityId,
        target: EntityId,
    ) -> Result<(i32, u32, Fixed, Fixed), SimError> {
        let source = self
            .find(attacker)
            .ok_or(SimError::MissingEntity(attacker))?;
        let victim = self.find(target).ok_or(SimError::MissingEntity(target))?;
        let rules = self.damage_rules.ok_or(SimError::NoTables)?;
        if matches!(source.kind, EntityKind::Resource(_)) {
            return Err(SimError::Unarmed { id: attacker });
        }
        if matches!(source.task, Task::Garrisoned { .. }) {
            return Err(SimError::Garrisoned { id: attacker });
        }
        if matches!(
            victim.kind,
            EntityKind::Resource(_) | EntityKind::Projectile
        ) || matches!(victim.task, Task::Garrisoned { .. })
        {
            // A tree is not a target and neither is an arrow: one cannot bleed and the other is a
            // consequence, not a combatant. Nor is a unit inside a building - the building is the
            // target, and the unit goes with it.
            return Err(SimError::NotATarget { id: target });
        }
        if self.effective_stance(attacker) == Stance::NoAttack {
            return Err(SimError::StanceForbidsAttack { id: attacker });
        }
        // Your own livestock is not friendly fire: once a sheep is yours, your own shepherd has to be able to kill
        // it, or the slaughter cannot happen at all (D70 item 5 - the animal joins the player whose villager is
        // walking it, which is the same player). A teammate's sheep is a different matter: that is still a fight.
        let livestock = victim
            .unit
            .is_some_and(|unit| self.herdable.contains(&unit));
        let same_player = source.player == victim.player;
        if same_player && !livestock {
            return Err(SimError::FriendlyFire { id: target });
        }
        // Allies are not targets either: two players on one team do not fight each other. The check is skipped for
        // the livestock case above, where "the same side" is the *same player* and the sheep is its shepherd's to
        // kill; a **teammate's** sheep still needs an ally to be hostile, which it is not.
        if !same_player && self.team_of(source.player) == self.team_of(victim.player) {
            return Err(SimError::AlliedTarget { id: target });
        }
        let units = self.units.as_ref().ok_or(SimError::NoTables)?;
        // The **variant** is what the worker is while it works: a villager on a hunt task is VMHUN, and VMHUN carries a
        // bow (`range = 3.0`) where the base villager carries a knife (`range = 0.0`). Reading `unit` alone made every
        // hunter fight at arm's length, which is the owner's first correction (2026-10-29).
        let (attacker_stats, victim_stats) = match (source.variant.or(source.unit), victim.unit) {
            (Some(a), Some(b)) => (
                units.unit(a).ok_or(SimError::MissingUnit { id: a })?,
                units.unit(b).ok_or(SimError::MissingUnit { id: b })?,
            ),
            _ => return Err(SimError::Unarmed { id: attacker }),
        };
        if attacker_stats.attacks.is_empty() {
            return Err(SimError::Unarmed { id: attacker });
        }
        // Both sides' attacks and armours are adjusted by what their owners have researched: an
        // upgrade is resolved here, at order time, so the tick loop still never reads a table.
        let attacks = self.adjusted(source.player, attacker_stats, "attack");
        let armours = self.adjusted(victim.player, victim_stats, "armour");
        let damage = damage_between(&attacks, &armours, &rules);
        let reload_ticks = attacker_stats
            .reload
            .resolve(self.scale, "reload")
            .map(|reload| reload.raw() * i64::from(self.tick_rate_hz) / self.scale)
            .ok()
            .and_then(|ticks| u32::try_from(ticks).ok())
            .filter(|ticks| *ticks > 0)
            .ok_or(SimError::Unarmed { id: attacker })?;
        // Range is attribute 1, in tiles, and the upgrades that move it (Fletching and its line, +1
        // each) are kind-4 additions - so it resolves exactly like speed: base, then the owner's
        // modifiers at the engine scale. Zero stays zero: a melee unit has no reach to extend.
        let base = attacker_stats
            .range
            .resolve(self.scale, "range")
            .map_err(|err| SimError::Unresolved {
                id: attacker_stats.id,
                detail: err.to_string(),
            })?;
        let range = scale_fixed(
            base,
            self.scale,
            &self.modifiers_for(source.player, attacker_stats, "range"),
        );
        // The dead zone: a siege engine cannot shoot what stands next to it, and the number is the
        // unit's own (a mangonel 3 tiles, a bombard cannon 5, everything else 0), moved by any tech that
        // sets attribute 20 (D44).
        let min_base = attacker_stats
            .min_range
            .resolve(self.scale, "min_range")
            .map_err(|err| SimError::Unresolved {
                id: attacker_stats.id,
                detail: err.to_string(),
            })?;
        let min_range = scale_fixed(
            min_base,
            self.scale,
            &self.modifiers_for(source.player, attacker_stats, "minimum_range"),
        );
        Ok((damage, reload_ticks, range, min_range))
    }

    /// Resolve the numbers at order time and start swinging.
    pub(crate) fn start_attack(
        &mut self,
        attacker: EntityId,
        target: EntityId,
    ) -> Result<(), SimError> {
        let (damage, reload_ticks, range, min_range) = self.check_attack(attacker, target)?;
        let Some(source) = self.find(attacker) else {
            return Err(SimError::MissingEntity(attacker));
        };
        let position = source.position;
        let Some(victim) = self.find(target).map(|e| e.position) else {
            return Err(SimError::MissingEntity(target));
        };
        // **An identical re-order changes nothing.** The base personality re-sends its army every twelve seconds; the
        // next lines used to re-plan the route from wherever the soldier now stands, which is how a line converging on
        // one building ends up shuffling between equally-near stand tiles - "they strike, then withdraw". When the task
        // is already this fight, with these numbers, the route and the reload both stay as they are.
        let kept = match source.task {
            Task::Attack {
                target: was,
                damage: was_damage,
                reload_ticks: was_reload,
                cooldown_ticks: was_cooldown,
                range: was_range,
                min_range: was_min,
            } if was == target
                && was_damage == damage
                && was_reload == reload_ticks
                && was_range == range
                && was_min == min_range =>
            {
                Some(was_cooldown)
            }
            _ => None,
        };
        // A building does not walk: its reach is its range, and pathing from a blocked tile would
        // refuse every shot a town centre wants to take. The test is the kind, not the speed - a unit
        // with no speed still gets a route, which is what a stationary test fixture relies on.
        if source.kind != EntityKind::Building && kept.is_none() {
            // **End the route where the blow can land** (E1b). A building's own tile is blocked and a node's tile holds
            // the thing being worked, so a route planned *to* the target ends wherever collision first refuses - and a
            // crowd of attackers queues behind the first arrival instead of spreading over the target's edge.
            let (ddx, ddy) = (
                victim.0.raw() - position.0.raw(),
                victim.1.raw() - position.1.raw(),
            );
            let squared = ddx * ddx + ddy * ddy;
            let here = self.tile_of(position);
            let there = self.tile_of(victim);
            let adjacent = here.0.abs_diff(there.0) <= 1 && here.1.abs_diff(there.1) <= 1;
            let target_body = self.find(target);
            let contact = self.reach_raw(source.unit, target_body.and_then(|e| e.unit));
            let in_reach = if range.raw() > 0 {
                // **Box to box** (D142): the route ends where the blow can land, and that is the target's edge.
                target_body.is_some_and(|body| {
                    self.gap_squared_to(position, self.body_radius(source.unit), body)
                        <= range.raw() * range.raw()
                })
            } else {
                adjacent || squared <= contact * contact
            };
            let stand = if in_reach {
                None
            } else {
                self.stand_near(attacker, position, victim, min_range, range)
            };
            let route = match stand {
                Some(stand) => self.plan_path_for(attacker, position, stand)?,
                None if in_reach => Vec::new(),
                // Nothing better than the target itself: the old behaviour, and the collision rule will still stop it.
                None => self.plan_path_for(attacker, position, victim)?,
            };
            self.paths.insert(
                attacker,
                Route {
                    points: route,
                    ..Route::default()
                },
            );
        }
        if let Some(entity) = self.find_mut(attacker) {
            // The reload the fight has already run is kept (see `kept` above); a new fight starts the swing at once.
            entity.task = Task::Attack {
                target,
                damage,
                reload_ticks,
                cooldown_ticks: kept.unwrap_or(0),
                range,
                min_range,
            };
        }
        Ok(())
    }

    /// The nearest enemy a hunting unit can see and may attack, by id order on a tie.
    ///
    /// A hunt scans on a schedule (`combat.aggro_scan_ticks`, staggered by id) because this is the one
    /// order-path cost that grows with the size of the world: a pass over every entity for every
    /// hunting unit. Four looks a second at 20 Hz is invisible to a player and a fifth of the cost.
    pub(crate) fn nearest_enemy(&self, hunter: &Entity) -> Option<EntityId> {
        if self.aggro_radius <= 0 {
            return None;
        }
        let scan = self.aggro_scan_ticks;
        if scan > 1 && !(self.tick + u64::from(hunter.id.0)).is_multiple_of(scan) {
            return None;
        }
        let radius_squared = self.aggro_radius * self.aggro_radius;
        let mut best: Option<(i64, EntityId)> = None;
        for other in &self.entities {
            if other.player == hunter.player || other.player == 0 || other.hp <= 0 {
                continue;
            }
            if matches!(other.kind, EntityKind::Resource(_) | EntityKind::Projectile) {
                continue;
            }
            if matches!(other.task, Task::Garrisoned { .. }) {
                continue;
            }
            if self.team_of(other.player) == self.team_of(hunter.player) {
                continue;
            }
            let dx = other.position.0.raw() - hunter.position.0.raw();
            let dy = other.position.1.raw() - hunter.position.1.raw();
            let squared = dx * dx + dy * dy;
            if squared > radius_squared {
                continue;
            }
            let closer = match best {
                None => true,
                Some((best_squared, best_id)) => {
                    squared < best_squared || (squared == best_squared && other.id < best_id)
                }
            };
            if closer {
                best = Some((squared, other.id));
            }
        }
        best.map(|(_, id)| id)
    }

    /// Start a building: check the price, pay it, place the site and send the builder.
    ///
    /// "Is this a building?" is answered from the data rather than a hard-coded class list: a
    /// building has no speed and a positive creation time. A unit with a speed walks; a unit with
    /// no creation time cannot be put up.
    /// An animal takes being struck as an order: it turns on whoever hit it (D70). Nothing else does - a
    /// villager shot at keeps gathering, which is the game's rule and not an oversight here.
    pub(crate) fn retaliate(&mut self, victim: EntityId, attacker: EntityId) {
        if attacker == victim {
            return;
        }
        let Some(victim_unit) = self
            .find(victim)
            .filter(|entity| entity.hp > 0)
            .and_then(|entity| entity.unit)
        else {
            return;
        };
        let is_animal = self.units.as_ref().is_some_and(|units| {
            let root = units.root(victim_unit);
            self.animals.contains(&root)
        });
        if !is_animal {
            return;
        }
        // A sheep has nothing to answer with, and trying would record an `Unarmed` refusal for every slaughtered
        // animal - noise that would bury the refusals that matter (item 3).
        if !self.armed_animals.contains(&victim_unit) {
            return;
        }
        // Already fighting this one back: re-issuing would reset its cooldown on every blow.
        if matches!(
            self.find(victim).map(|entity| entity.task),
            Some(Task::Attack { target, .. }) if target == attacker
        ) {
            return;
        }
        // Interest, renewed by every blow (D71). This happens *before* the early return above, because
        // "already fighting this one" is exactly the case where the clock matters: a boar struck every
        // two seconds must never lose interest, and one struck again on its way home must turn round.
        let renewed = self.renew_interest(victim);
        if renewed {
            if let Err(error) = self.start_attack(victim, attacker) {
                self.refusal = Some((victim, error, self.tick));
            }
            return;
        }
        // A reaction has no caller to return an error to, and dropping it is how a real defect stayed
        // invisible for two sessions - the old `let _ =` printed only behind an env var nobody set, and
        // the instrumented `eprintln!` never appeared because this path was never reached at all (D71).
        if let Err(error) = self.start_attack(victim, attacker) {
            self.refusal = Some((victim, error, self.tick));
        }
    }

    /// One tick of swinging: count the reload down and land a blow when it reaches zero.
    ///
    /// The countdown happens before the check on purpose. Firing on the tick the counter *reaches*
    /// zero rather than the tick after makes a 2 s reload forty ticks at 20 Hz instead of
    /// forty-one, which only shows up when something is timed to the blow.
    pub(crate) fn swing(&mut self, attacker: EntityId) -> i32 {
        let Some(entity) = self.find_mut(attacker) else {
            return 0;
        };
        let Task::Attack {
            target,
            damage,
            reload_ticks,
            mut cooldown_ticks,
            range,
            min_range,
        } = entity.task
        else {
            return 0;
        };
        cooldown_ticks = cooldown_ticks.saturating_sub(1);
        let landed = if cooldown_ticks == 0 {
            cooldown_ticks = reload_ticks;
            damage
        } else {
            0
        };
        entity.task = Task::Attack {
            target,
            damage,
            reload_ticks,
            cooldown_ticks,
            range,
            min_range,
        };
        landed
    }

    /// What a blow is worth against *this* victim. A **herdable is livestock, not an opponent**: a butcher's strike -
    /// the shepherd's knife or the hunter's arrow - takes it outright, so a 7-health sheep dies of one blow rather than
    /// of three (the owner's rule). A boar or a deer is fought, which is what makes a boar dangerous.
    pub(crate) fn blow_against(&self, attacker: EntityId, target: EntityId, damage: i32) -> i32 {
        let Some(victim) = self.find(target) else {
            return damage;
        };
        if !victim
            .unit
            .is_some_and(|unit| self.herdable.contains(&unit))
        {
            return damage;
        }
        let butcher = self
            .find(attacker)
            .and_then(|entity| entity.unit)
            .is_some_and(|unit| self.butchers.contains(&unit));
        if butcher {
            damage.max(victim.hp)
        } else {
            damage
        }
    }

    /// Turn a gather order into resolved state: the variant, the per-tick rate and the capacity
    /// are read from the tables here, once, and stored on the entity. Nothing in the tick loop
    /// looks at a table, and a replay cannot name its own numbers.
    /// How many arrows a building fires at once: its own, plus one for every whole multiple of its own
    /// damage per second that its garrison contributes. The published rule is
    /// `floor(sum of garrisoned units' dps / building dps)`, and only units that deal **pierce** damage
    /// from range count - so an archer adds, and a Throwing Axeman (melee damage at range) does not (D37).
    pub(crate) fn volley(&self, building: EntityId) -> u32 {
        let Some(entity) = self.find(building) else {
            return 1;
        };
        let Some(units) = self.units.as_ref() else {
            return 1;
        };
        let Some(stats) = entity.unit.and_then(|unit| units.unit(unit)) else {
            return 1;
        };
        let building_dps = self.pierce_dps(entity.player, stats);
        if building_dps <= 0 {
            return 1;
        }
        let mut garrison_dps = 0i64;
        for occupant in self.entities.iter() {
            if occupant.player != entity.player || occupant.kind != EntityKind::Unit {
                continue;
            }
            if occupant.task != (Task::Garrisoned { building }) {
                continue;
            }
            let Some(unit) = occupant.unit.and_then(|unit| units.unit(unit)) else {
                continue;
            };
            // Range plus pierce damage: a unit that fights at range but deals melee damage (a Mameluke,
            // a Throwing Axeman, a Gbeto) contributes nothing.
            if unit
                .range
                .resolve(self.scale, "range")
                .map_or(0, |range| range.raw())
                <= 0
            {
                continue;
            }
            garrison_dps += self.pierce_dps(occupant.player, unit);
        }
        let extra = u32::try_from(garrison_dps / building_dps).unwrap_or(0);
        1u32.saturating_add(extra)
    }

    /// Put a shot in the air. Its ticks-to-land are not stored: the arrow moves by a fixed step every
    /// tick until it reaches the position it was aimed at, which is exact for any speed that divides
    /// the tick rate and needs no accumulation.
    pub(crate) fn spawn_projectile(&mut self, shot: Shot) -> Result<EntityId, SimError> {
        let Shot {
            player,
            shooter,
            from,
            aim,
            target,
            damage,
            blast,
        } = shot;
        let id = EntityId(self.entities.iter().map(|e| e.id.0).max().unwrap_or(0) + 1);
        self.insert(Entity {
            last_step: StepDecision::Idle,
            id,
            player,
            kind: EntityKind::Projectile,
            unit: None,
            position: from,
            speed: Fixed::ZERO,
            velocity: (Fixed::ZERO, Fixed::ZERO),
            hp: 1,
            amount: Fixed::ZERO,
            task: Task::Fly {
                target,
                shooter,
                damage,
                aim,
                blast,
            },
            training: None,
            construction: None,
            researching: None,
            carry: Fixed::ZERO,
            variant: None,
        })?;
        Ok(id)
    }

    /// The point a shot is really aimed at, after the archer's accuracy has had its say.
    ///
    /// Accuracy is a percentage (attribute 11): the dat gives an archer 80, a hand cannoneer 65, every
    /// building that shoots 100, and Thumb Ring sets bowmen to 100. A shot aimed true goes exactly where
    /// the archer meant; one that is not lands `accuracy_scatter` away, which the landing rule then makes
    /// a miss. The roll is derived from the tick and the shooter's id rather than drawn from a generator,
    /// so a replay makes exactly the same misses (D43).
    pub(crate) fn true_aim(&self, shooter: EntityId, aim: (Fixed, Fixed)) -> (Fixed, Fixed) {
        if self.accuracy_scatter <= 0 {
            return aim;
        }
        let (Some(units), Some(entity)) = (self.units.as_ref(), self.find(shooter)) else {
            return aim;
        };
        let Some(unit) = entity.unit.and_then(|unit| units.unit(units.root(unit))) else {
            return aim;
        };
        let accuracy = self.accuracy_of(entity.player, unit);
        if accuracy >= 100 {
            return aim;
        }
        let roll = splitmix64(
            self.tick()
                .wrapping_mul(0x9E37_79B9_7F4A_7C15)
                .wrapping_add(u64::from(entity.id.0)),
        );
        if accuracy > 0 && roll % 100 < u64::from(accuracy) {
            return aim;
        }
        // Off the mark: one of eight directions, because the scatter is a distance and a direction needs
        // no trigonometry to be deterministic.
        let (dx, dy) = OCTANTS[usize::try_from((roll >> 8) % 8).unwrap_or(0)];
        (
            Fixed::from_raw(aim.0.raw() + dx * self.accuracy_scatter),
            Fixed::from_raw(aim.1.raw() + dy * self.accuracy_scatter),
        )
    }

    /// Where to aim so that a shot meets a moving target: iterate the time of flight, which depends on the
    /// distance to the aim, which depends on the time of flight. Three passes settle it for any speed the
    /// game has, and every step is integer arithmetic on engine units, so two runs aim at the same point.
    ///
    /// The target's motion for this tick is read off its own movement rather than stored: `velocity` is a
    /// spawn-time field, and a unit's real direction is the step it is about to take.
    pub(crate) fn lead_shot(
        &self,
        from: (Fixed, Fixed),
        victim: (Fixed, Fixed),
        target: EntityId,
        projectile_step: i64,
    ) -> (Fixed, Fixed) {
        let (step_x, step_y) = self.motion_of(target);
        if (step_x, step_y) == (0, 0) || projectile_step <= 0 {
            return victim;
        }
        let mut aim = victim;
        for _ in 0..3 {
            let dx = aim.0.raw() - from.0.raw();
            let dy = aim.1.raw() - from.1.raw();
            let distance = integer_sqrt(dx * dx + dy * dy);
            let ticks = distance / projectile_step;
            aim = (
                Fixed::from_raw(victim.0.raw() + step_x * ticks),
                Fixed::from_raw(victim.1.raw() + step_y * ticks),
            );
        }
        aim
    }

    /// This player's accuracy with this unit, in percent: what the dat gives it, unless a tech sets
    /// attribute 11 to something else (Thumb Ring sets 100, City Rights 87).
    fn accuracy_of(&self, player: u8, unit: &Unit) -> u16 {
        let mut accuracy = unit.accuracy;
        for modifier in self.modifiers_for(player, unit, "accuracy") {
            if modifier.op == ruage_data::ModOp::Set {
                accuracy = u16::try_from(modifier.value.clamp(0.0, 100.0) as i64).unwrap_or(0);
            }
        }
        accuracy
    }

    /// Does this shooter lead its shots? Attribute 19 is the ballistics flag, and content sets it on the
    /// units a university's Ballistics teaches. Any positive set value counts (D42).
    pub(crate) fn has_ballistics(&self, shooter: EntityId) -> bool {
        let (Some(units), Some(entity)) = (self.units.as_ref(), self.find(shooter)) else {
            return false;
        };
        let Some(unit) = entity.unit.and_then(|unit| units.unit(units.root(unit))) else {
            return false;
        };
        self.modifiers_for(entity.player, unit, "ballistics")
            .iter()
            .any(|modifier| modifier.value > 0.0)
    }

    /// Everything a blast catches: units (not the aimed target, which the landing rule already dealt
    /// with, and not one sheltered inside a building), within `radius` of the landing point. Buildings
    /// take the aimed hit only - a mangonel's stone does not demolish a wall it passed near (D51).
    pub(crate) fn blast_victims(
        &self,
        aim: (Fixed, Fixed),
        radius: Fixed,
        aimed: EntityId,
    ) -> Vec<EntityId> {
        if radius.raw() <= 0 {
            return Vec::new();
        }
        let mut hit = Vec::new();
        for entity in &self.entities {
            if entity.id == aimed || entity.hp <= 0 || entity.kind != EntityKind::Unit {
                continue;
            }
            if matches!(entity.task, Task::Garrisoned { .. }) {
                continue;
            }
            let dx = entity.position.0.raw() - aim.0.raw();
            let dy = entity.position.1.raw() - aim.1.raw();
            if dx * dx + dy * dy <= radius.raw() * radius.raw() {
                hit.push(entity.id);
            }
        }
        hit
    }

    /// **Would this attacker's blow land from where it stands?** The question `step` asks before it swings, exposed so the
    /// rule has a test that does not need a played-out match (D142).
    #[must_use]
    pub fn in_strike_range(&self, attacker: EntityId, target: EntityId) -> bool {
        let (Some(source), Some(victim)) = (self.find(attacker), self.find(target)) else {
            return false;
        };
        let range = self.unit_range(source.unit);
        if range > 0 {
            // **Box to box** (D142): the dat's range is measured between the bodies' edges, so a 4x4 town centre is hit
            // from its wall. Centre to centre made every building two tiles nearer than it looks.
            let gap = self.gap_squared_to(source.position, self.body_radius(source.unit), victim);
            return gap <= range * range;
        }
        let here = self.tile_of(source.position);
        let there = self.tile_of(victim.position);
        here.0.abs_diff(there.0) <= 1 && here.1.abs_diff(there.1) <= 1
    }

    /// A unit's attack range in raw units, from the dat (0 for melee).
    fn unit_range(&self, unit: Option<u32>) -> i64 {
        self.units
            .as_ref()
            .and_then(|units| {
                unit.and_then(|unit| units.unit(units.root(unit)))
                    .map(|stats| (units, stats))
            })
            .and_then(|(_, stats)| stats.range.resolve(self.scale, "range").ok())
            .map_or(0, |range| range.raw())
    }

    /// **The squared gap between a body's box and a target's**, in engine units: zero when they touch or overlap (D142).
    ///
    /// A unit's box is a square of its own `radius` about its centre; a **building's is the rectangle its footprint
    /// covers**, which is anchored at a corner and is not centred on its position - `radius_x`/`radius_y` from the dat, so
    /// a town centre is four tiles wide and its wall is two tiles from its middle. The attacker's own radius shrinks the
    /// target's box, which is the same arithmetic as growing the attacker to a point.
    pub(crate) fn gap_squared_to(&self, from: (Fixed, Fixed), radius: i64, target: &Entity) -> i64 {
        let (left, top, right, bottom) = if target.kind == EntityKind::Building {
            let tiles = self.footprint_area(target);
            let (Some(x0), Some(x1)) = (
                tiles.iter().map(|tile| tile.0).min(),
                tiles.iter().map(|tile| tile.0).max(),
            ) else {
                return 0;
            };
            let (Some(y0), Some(y1)) = (
                tiles.iter().map(|tile| tile.1).min(),
                tiles.iter().map(|tile| tile.1).max(),
            ) else {
                return 0;
            };
            (
                i64::from(x0) * self.scale,
                i64::from(y0) * self.scale,
                (i64::from(x1) + 1) * self.scale,
                (i64::from(y1) + 1) * self.scale,
            )
        } else {
            let own = self.body_radius(target.unit);
            (
                target.position.0.raw() - own,
                target.position.1.raw() - own,
                target.position.0.raw() + own,
                target.position.1.raw() + own,
            )
        };
        let (left, top, right, bottom) =
            (left - radius, top - radius, right + radius, bottom + radius);
        let px = from.0.raw();
        let py = from.1.raw();
        let dx = if px < left {
            left - px
        } else if px > right {
            px - right
        } else {
            0
        };
        let dy = if py < top {
            top - py
        } else if py > bottom {
            py - bottom
        } else {
            0
        };
        dx * dx + dy * dy
    }
}
