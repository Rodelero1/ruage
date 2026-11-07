//! **Construction**: sites that can start, progress that can be made, and what happens when a site is abandoned.
//!
//! Split out of `lib.rs` under D144, verbatim: a pure move, proved by the acceptance hash.

use ruage_fixed::Fixed;

use crate::{BuildPlan, EntityId, EntityKind, SimError, World};

impl World {
    /// Cancel a foundation of mine whose tile turns out to hold an enemy building that is already
    /// underway, once I can see that tile (D61).
    ///
    /// This is the second half of the blind-placement rule: you may lay a foundation in fog where
    /// something stands, because the player chose blind, and the game cancels the dead placement when
    /// your own sight reaches it - not at the click, and not when a builder finally arrives. The
    /// resources come back (D58a), because the player was never given a chance to know.
    ///
    /// An enemy *untouched* foundation is not a reason to cancel: two bare foundations are decided by
    /// whoever puts a builder to work first (`cancel_overbuilt`). Only something already started or
    /// finished is a settled fact that a late placement cannot undo.
    pub(crate) fn settle_blind_sites(&mut self) {
        // Nothing under construction: the pass costs one set lookup. It used to cost a walk of every entity to ask the
        // same question, every tick, in every match (item 6).
        if self.foundations.is_empty() {
            return;
        }
        // A maintained set drifts: this is the assertion that catches it, and it costs nothing in release.
        debug_assert_eq!(
            self.foundations.len(),
            self.entities
                .iter()
                .filter(|entity| entity.construction.is_some())
                .count(),
            "the foundation set disagrees with the world"
        );
        let sites: Vec<(EntityId, u8, (Fixed, Fixed))> = self
            .entities
            .iter()
            .filter(|entity| {
                entity.kind == EntityKind::Building
                    && entity.construction.is_some()
                    && entity.hp > 0
            })
            .map(|entity| (entity.id, entity.player, entity.position))
            .collect();
        let mut doomed = Vec::new();
        for (id, player, position) in sites {
            let tile = (
                u16::try_from((position.0.raw() / self.scale).max(0)).unwrap_or(u16::MAX),
                u16::try_from((position.1.raw() / self.scale).max(0)).unwrap_or(u16::MAX),
            );
            if !self.unit_sees(player, tile) {
                continue;
            }
            let taken = self.entities.iter().any(|other| {
                other.id != id
                    && other.player != 0
                    && other.player != player
                    && other.kind == EntityKind::Building
                    && other.position == position
                    && other
                        .construction
                        .is_none_or(|construction| construction.progress_ticks > 0)
            });
            if taken {
                doomed.push(id);
            }
        }
        for id in doomed {
            self.cancel_site(id);
        }
    }

    /// Cancel every other player's building sharing this site's tile (D58).
    ///
    /// Two buildings can only share a tile because one of them was placed blind, and this is where the
    /// blind one loses: the builder who starts working takes the ground. The victim is killed rather than
    /// deleted so the ordinary death sweep releases its bookkeeping and its tile - a cancelled wall must
    /// not keep blocking, and a cancelled site must not keep a `paths` entry.
    pub(crate) fn cancel_overbuilt(&mut self, site: EntityId) {
        let Some((position, player)) = self
            .find(site)
            .map(|entity| (entity.position, entity.player))
        else {
            return;
        };
        let theirs: Vec<(EntityId, bool)> = self
            .entities
            .iter()
            .filter(|entity| {
                entity.id != site
                    && entity.player != player
                    && entity.player != 0
                    && entity.position == position
                    && matches!(entity.kind, EntityKind::Building)
            })
            // "Underway" means started or finished: the owner's rule is that whoever starts building
            // first wins, so a site the other side has already put a builder to work on is not a
            // newcomer's to take (D61).
            .map(|entity| {
                (
                    entity.id,
                    entity
                        .construction
                        .is_none_or(|construction| construction.progress_ticks > 0),
                )
            })
            .collect();
        if theirs.iter().any(|(_, underway)| *underway) {
            self.cancel_site(site);
            return;
        }
        for (victim, _) in theirs {
            self.cancel_site(victim);
        }
    }

    /// Cancel a foundation and pay its owner back (D58a). The price is recomputed from content rather than
    /// carried on the entity: costs are not per-player yet, so what was paid and what is returned cannot
    /// drift - and the day a discount exists, this is the one place that has to learn about it.
    pub(crate) fn cancel_site(&mut self, site: EntityId) {
        let Some((player, unit, alive)) = self
            .find(site)
            .map(|entity| (entity.player, entity.unit, entity.hp > 0))
        else {
            return;
        };
        // Idempotent: a site cancelled earlier in the same tick is still in the list until the sweep,
        // and refunding it twice would pay its owner for a building that only cost once.
        if !alive {
            return;
        }
        if let Some(unit) = unit {
            self.refund_unit_cost(player, unit);
        }
        if let Some(entity) = self.find_mut(site) {
            entity.hp = 0;
        }
    }

    /// Give `player` back what one of `unit` costs. Saturating, because a refund is not a way to overflow a
    /// stockpile.
    pub(crate) fn refund_unit_cost(&mut self, player: u8, unit: u32) {
        self.refund_unit_cost_part(player, unit, 1, 1);
    }

    /// Give `player` back `left / total` of what one of `unit` costs - the fraction of a site that is not
    /// built yet. Integer arithmetic floors, so a nearly finished building returns almost nothing and never
    /// returns more than was paid.
    pub(crate) fn refund_unit_cost_part(&mut self, player: u8, unit: u32, left: u32, total: u32) {
        let Some(costs) = self
            .units
            .as_ref()
            .and_then(|units| units.unit(unit))
            .map(|stats| stats.costs.clone())
        else {
            return;
        };
        let total = total.max(1);
        let mut refunds = Vec::with_capacity(costs.len());
        for cost in &costs {
            if let Ok(slot) = self.cost_slot(cost) {
                let amount = u32::from(cost.amount).saturating_mul(left) / total;
                refunds.push((slot, amount));
            }
        }
        let Some(row) = self.stock.get_mut(usize::from(player).wrapping_sub(1)) else {
            return;
        };
        for (slot, amount) in refunds {
            if let Some(have) = row.get_mut(slot) {
                *have = have.saturating_add(amount);
            }
        }
    }

    /// One tick of work on a site. Returns true when this tick finished the building. HP is
    /// derived from progress rather than accumulated, so a mill's 17.14 HP/s cannot drift.
    pub(crate) fn work_one_tick(&mut self, site: EntityId) -> bool {
        // The first tick of work takes the tile (D58). A site that stands on an enemy's foundation can
        // only be there because it was placed blind, and the builder who starts working cancels the one
        // who did not get there first. Cancelled before the borrow below, which needs `self` whole.
        let first_tick = self
            .find(site)
            .and_then(|entity| entity.construction)
            .is_some_and(|construction| construction.progress_ticks == 0);
        if first_tick {
            self.cancel_overbuilt(site);
        }
        let Some(building) = self.find_mut(site) else {
            return false;
        };
        let Some(mut construction) = building.construction else {
            return false;
        };
        construction.progress_ticks = construction.progress_ticks.saturating_add(1);
        let total = construction.total_ticks.max(1);
        let hp = if construction.progress_ticks >= total {
            construction.max_hp
        } else {
            let scaled = i64::from(construction.max_hp) * i64::from(construction.progress_ticks)
                / i64::from(total);
            i32::try_from(scaled).unwrap_or(1).max(1)
        };
        building.hp = hp;
        let done = construction.progress_ticks >= total;
        building.construction = Some(construction);
        done
    }

    /// The checks a build order has to survive: is it a building, can the player pay, is the
    /// tile free. Returns what the order will cost and how long it will take.
    pub(crate) fn check_build(
        &self,
        villager: EntityId,
        unit_id: u32,
        at: (u16, u16),
    ) -> Result<BuildPlan, SimError> {
        let units = self.units.as_ref().ok_or(SimError::NoTables)?;
        let stats = units
            .unit(unit_id)
            .ok_or(SimError::MissingUnit { id: unit_id })?;
        let speed = stats
            .speed
            .resolve(self.scale, "speed")
            .unwrap_or(Fixed::ZERO);
        if speed.raw() != 0 || stats.train_time <= 0 {
            return Err(SimError::NotABuilding { unit: unit_id });
        }
        let total = u32::try_from(stats.train_time)
            .ok()
            .and_then(|seconds| seconds.checked_mul(self.tick_rate_hz))
            .ok_or(SimError::NotABuilding { unit: unit_id })?;
        let worker = self
            .find(villager)
            .ok_or(SimError::MissingEntity(villager))?;

        // Availability outranks the price: "you cannot have this yet" is the answer, not "you are poor".
        if let Some(civ) = self.civ_barred_unit(worker.player, unit_id) {
            return Err(SimError::UnitBarred { unit: unit_id, civ });
        }
        if !self.unit_available(worker.player, unit_id) {
            return Err(SimError::UnitUnavailable { unit: unit_id });
        }
        // A building prerequisite is about history, not the map (D63): the requirement is that this
        // player has finished one at some point, so a razed blacksmith does not take the workshop away.
        if let Some((need, name)) = self.requires_built.get(&units.root(unit_id))
            && !self.built.contains(&(worker.player, *need))
        {
            return Err(SimError::MissingBuilding { need: name.clone() });
        }
        for cost in &stats.costs {
            let slot = self.cost_slot(cost)?;
            let need = u32::from(cost.amount);
            let have = self.stock(worker.player, slot);
            if have < need {
                return Err(SimError::NotEnough {
                    resource: self.slot_name(slot),
                    need,
                    have,
                });
            }
        }
        if let Some(why) = self.site_refusal(worker.player, unit_id, at) {
            return Err(why);
        }
        let position = self.tile(at);
        // Reachability is part of "can this order run": paying for a site nobody can walk to would
        // leave a construction that never progresses and wood the player cannot get back.
        self.plan_path(worker.position, position, 0)?;

        Ok(BuildPlan {
            total_ticks: total,
            // Derived, not copied: a building ordered after a health tech must be quoted the new
            // maximum, or it finishes at the old one.
            max_hp: self.max_hp(worker.player, unit_id).unwrap_or(stats.hp),
            costs: stats.costs.clone(),
            player: worker.player,
        })
    }
}
