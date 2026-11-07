//! **Research**: a tech's checks, its cost, and what finishing it changes.
//!
//! Split out of `lib.rs` under D144, verbatim: a pure move, proved by the acceptance hash.

use ruage_data::Unit;
use ruage_fixed::Fixed;

use crate::{
    EntityId, EntityKind, MAX_PLAYERS, Research, ResearchPlan, SimError, World, add_to_class,
    collect_modifiers, multiply_class, scale_fixed, scaled_health,
};

impl World {
    pub(crate) fn check_research(
        &self,
        building: EntityId,
        tech: u32,
    ) -> Result<ResearchPlan, SimError> {
        let entity = self
            .find(building)
            .ok_or(SimError::MissingEntity(building))?;
        if entity.kind != EntityKind::Building {
            return Err(SimError::TrainNeedsABuilding { id: building });
        }
        if entity.construction.is_some() {
            return Err(SimError::BuildingNotReady { id: building });
        }
        if entity.researching.is_some() {
            return Err(SimError::AlreadyResearching { id: building });
        }
        let techs = self.techs.as_ref().ok_or(SimError::NoTables)?;
        let stats = techs.get(tech).ok_or(SimError::MissingTech { id: tech })?;
        if self.researched(entity.player).contains(&tech) {
            return Err(SimError::AlreadyResearched { tech });
        }
        // One tech, one order: two buildings researching the same tech would both charge and the
        // second would apply an upgrade with nothing left to upgrade.
        if self.entities.iter().any(|other| {
            other.player == entity.player && other.researching.is_some_and(|job| job.tech == tech)
        }) {
            return Err(SimError::TechInProgress { tech });
        }
        // `location` is the unit that researches it (12 is a barracks, 87 an archery range). Zero
        // means "anywhere", which is how the data spells a tech with no home building.
        if let Ok(place) = u32::try_from(stats.location)
            && place != 0
            && entity
                .unit
                .map(|unit| self.units.as_ref().map_or(unit, |units| units.root(unit)))
                != Some(place)
        {
            return Err(SimError::CannotResearchHere {
                building: entity.unit.unwrap_or(0),
                tech,
            });
        }
        // An age needs buildings of the current age (D65). Checked before the price so the player is told
        // what is actually missing, and counted on finished buildings: a foundation is not an age.
        if let Some((need, families, singles)) = self.age_needs.get(&tech) {
            let singles_met = !singles.is_empty()
                && self.entities.iter().any(|other| {
                    other.player == entity.player
                        && other.kind == EntityKind::Building
                        && other.hp > 0
                        && other.construction.is_none()
                        && other.unit.is_some_and(|unit| {
                            let family = self.units.as_ref().map_or(unit, |units| units.root(unit));
                            singles.contains(&family)
                        })
                });
            let have = self.standing_buildings(entity.player, families);
            if !singles_met && have < *need {
                return Err(SimError::AgeNeedsBuildings { need: *need, have });
            }
        }
        // A civ's tech tree lists what it *cannot* have, and that outranks "not ready yet": the
        // British list carries the Hussar, the Eagle line and the Paladin, while no civ's list carries
        // Loom or an age - so these are restrictions, not permissions (see crates/data/src/civs.rs).
        // Checked before the prerequisites so a barred tech says so rather than asking for an age.
        if let Some(civ) = self.barred_by(entity.player, tech) {
            return Err(SimError::TechBarred { tech, civ });
        }
        // Prerequisites: every id a tech lists is a *tech* id (all 614 entries in the table resolve to
        // techs, even where the number also names a building), and the count matters: the dat's own
        // `min_required_techs` says how many of them are enough, and tech 120 ("Shadow Node for Age
        // Two") lists five buildings while needing two (D30).
        let missing: Vec<u32> = stats
            .required
            .iter()
            .filter_map(|required| u32::try_from(*required).ok())
            .filter(|required| !self.researched(entity.player).contains(required))
            .collect();
        let have = stats.required.len().saturating_sub(missing.len());
        if have < usize::from(stats.min_required)
            && let Some(missing) = missing.first().copied()
        {
            return Err(SimError::MissingPrerequisite {
                tech,
                missing,
                role: self.tech_role_name(missing).map(str::to_owned),
            });
        }
        let effects = self.effects.as_ref().ok_or(SimError::NoTables)?;
        if let Some(effect_id) = stats.effect_id
            && effects.get(effect_id).is_none()
        {
            return Err(SimError::MissingEffect { id: effect_id });
        }
        for cost in &stats.costs {
            let slot = self.cost_slot(cost)?;
            let need = u32::from(cost.amount);
            let have = self.stock(entity.player, slot);
            if have < need {
                return Err(SimError::NotEnough {
                    resource: self.slot_name(slot),
                    need,
                    have,
                });
            }
        }
        let ticks = u32::try_from(stats.research_time)
            .ok()
            .and_then(|seconds| seconds.checked_mul(self.tick_rate_hz))
            .ok_or(SimError::MissingTech { id: tech })?;
        Ok(ResearchPlan {
            costs: stats.costs.clone(),
            ticks,
            player: entity.player,
        })
    }

    /// Pay for a tech and start its timer. Paid only after every check passed, so a refused order
    /// spends nothing.
    pub(crate) fn start_research(&mut self, building: EntityId, tech: u32) -> Result<(), SimError> {
        let plan = self.check_research(building, tech)?;
        for cost in &plan.costs {
            let slot = self.cost_slot(cost)?;
            let index = usize::from(plan.player).wrapping_sub(1);
            let Some(row) = self.stock.get_mut(index) else {
                return Err(SimError::PlayerOutOfRange {
                    player: plan.player,
                });
            };
            row[slot] -= u32::from(cost.amount);
        }
        let Some(entity) = self.find_mut(building) else {
            return Err(SimError::MissingEntity(building));
        };
        entity.researching = Some(Research {
            tech,
            remaining_ticks: plan.ticks,
            fresh: true,
        });
        Ok(())
    }

    /// Every modifier of `attribute` that applies to `unit`, from the techs `player` has researched.
    pub(crate) fn modifiers_for(
        &self,
        player: u8,
        unit: &Unit,
        attribute: &str,
    ) -> Vec<&ruage_data::Modifier> {
        let Some(effects) = self.effects.as_ref() else {
            return Vec::new();
        };
        let mut chosen = Vec::new();
        if let Some(techs) = self.techs.as_ref() {
            for tech_id in self.researched(player) {
                let Some(effect_id) = techs.get(*tech_id).and_then(|tech| tech.effect_id) else {
                    continue;
                };
                collect_modifiers(effects, effect_id, unit, attribute, &mut chosen);
            }
        }
        // A civ's team bonus applies without being researched: it is a property of who the player is.
        for effect_id in &self.bonus_effects[usize::from(player).min(MAX_PLAYERS - 1)] {
            collect_modifiers(effects, *effect_id, unit, attribute, &mut chosen);
        }
        chosen
    }

    /// A unit's attacks or armours after every modifier its owner has researched.
    ///
    /// Additions are applied before multiplications, in researched-tech order, so the result cannot
    /// depend on the order techs happened to be researched in. A modifier matches on the unit id it
    /// names, on the unit's class, or on neither (-1 means "everything").
    pub(crate) fn adjusted(&self, player: u8, unit: &Unit, attribute: &str) -> Vec<(i16, i16)> {
        let base = if attribute == "attack" {
            &unit.attacks
        } else {
            &unit.armours
        };
        let chosen = self.modifiers_for(player, unit, attribute);

        let mut out = base.clone();
        for modifier in chosen.iter().filter(|m| m.op == ruage_data::ModOp::Add) {
            // An attack or armour change is a change *to a class*; one that names no class is
            // content we cannot place. Skipping it is inert today (an unlisted class loses to any
            // attack), but a later rule that lets a classless entry through would turn it into
            // "+99 to everything".
            if modifier.attribute_class < 0 {
                continue;
            }
            let amount = i16::try_from(modifier.value.round() as i64).unwrap_or(i16::MAX);
            add_to_class(&mut out, modifier.attribute_class, amount);
        }
        for modifier in chosen
            .iter()
            .filter(|m| m.op == ruage_data::ModOp::Multiply)
        {
            if modifier.attribute_class < 0 {
                continue;
            }
            multiply_class(&mut out, modifier.attribute_class, modifier.value);
        }
        out
    }

    /// A unit's maximum health for this player: the table's value with every hit-point modifier
    /// applied, additions before multiplications. Health is *state*, so this is what a spawn uses and
    /// what a research grant is measured against.
    pub(crate) fn max_hp(&self, player: u8, unit_id: u32) -> Result<i32, SimError> {
        let units = self.units.as_ref().ok_or(SimError::NoTables)?;
        let unit = units
            .unit(unit_id)
            .ok_or(SimError::MissingUnit { id: unit_id })?;
        Ok(scaled_health(
            unit,
            &self.modifiers_for(player, unit, "hit_points"),
        ))
    }

    /// A unit's speed for this player, at the engine scale.
    pub(crate) fn effective_speed(&self, player: u8, unit_id: u32) -> Result<Fixed, SimError> {
        let units = self.units.as_ref().ok_or(SimError::NoTables)?;
        let unit = units
            .unit(unit_id)
            .ok_or(SimError::MissingUnit { id: unit_id })?;
        let base = unit
            .speed
            .resolve(self.scale, "speed")
            .map_err(|err| SimError::Unresolved {
                id: unit_id,
                detail: err.to_string(),
            })?;
        let modifiers = self.modifiers_for(player, unit, "speed");
        Ok(scale_fixed(base, self.scale, &modifiers))
    }

    /// Apply a researched tech: mark it done and run its effect commands.
    ///
    /// Only **kind 3** is interpreted so far - an upgrade, "every unit `a` becomes unit `b`". Every
    /// other kind is left alone on purpose and counted in `docs/next-step.md`: applying half a tech
    /// silently would be worse than applying the half that is modelled, and the tests that use this
    /// path assert the effect they use contains nothing else.
    /// A tech has landed: apply it, then apply whatever availability it unlocked.
    pub(crate) fn finish_research(&mut self, player: u8, tech: u32) -> Result<(), SimError> {
        let food_before = self.farm_food_bonus(player);
        self.apply_tech(player, tech)?;
        self.grant_availability(player)?;
        // Farm upgrades are the one effect that changes ground already worked: the plot's food is a
        // budget, so what the tech adds goes onto every plot this player has that still has food in
        // it. A spent plot stays spent - its food is eaten, and an upgrade does not bring it back.
        let added = self.farm_food_bonus(player) - food_before;
        if added > 0 {
            // **Proportionally** (the owner's rule, 2026-10-30): a plot that is half eaten gains half of what the tech
            // adds. A farm is a budget, not a tank that is refilled - the extra food belongs to the *plot*, and what the
            // villager has already taken is gone.
            let was_total = self
                .economy
                .as_ref()
                .map_or(0, |economy| i64::from(economy.farm.food_per_farm))
                + food_before;
            self.grow_plots(player, added, was_total);
        }
        Ok(())
    }

    /// Add `food` to every plot of this player's that still holds some, in id order so the result is
    /// a function of state. Content decides which building is a plot, exactly as it does everywhere
    /// else; a pack with no farm yields nothing here.
    fn grow_plots(&mut self, player: u8, food: i64, was_total: i64) {
        let Some(farm_unit) = self
            .economy
            .as_ref()
            .and_then(|economy| economy.farm.unit.as_ref())
            .and_then(|role| self.economy.as_ref()?.role.get(role).copied())
        else {
            return;
        };
        let scale = self.scale;
        for entity in &mut self.entities {
            if entity.player == player
                && entity.kind == EntityKind::Building
                && entity.unit == Some(farm_unit)
                && entity.amount.raw() > 0
            {
                // The plot's share of the upgrade, rounded **down**: a quarter-eaten plot gains a quarter of the extra.
                let remaining = entity.amount.raw() / scale;
                let gain = if was_total > 0 {
                    (food * remaining / was_total).max(0)
                } else {
                    0
                };
                entity.amount = Fixed::from_raw(entity.amount.raw() + gain * scale);
            }
        }
    }

    /// Mark a tech researched and apply it. Does not chase availability techs; `finish_research` is the
    /// entry point that does.
    fn apply_tech(&mut self, player: u8, tech: u32) -> Result<(), SimError> {
        // What the player's units are worth *before* this tech, so the grant is the difference the
        // tech made rather than a full heal: a wounded unit keeps its wound.
        let before: Vec<(EntityId, u32, i32)> = self
            .entities
            .iter()
            .filter(|entity| entity.player == player)
            .filter_map(|entity| {
                let unit = entity.unit?;
                Some((
                    entity.id,
                    unit,
                    self.max_hp(player, unit).unwrap_or(entity.hp),
                ))
            })
            .collect();

        self.researched.entry(player).or_default().insert(tech);

        // Health and speed are state, so every unit the player already has gains them at once.
        for (id, unit_id, old_max) in before {
            let new_max = self.max_hp(player, unit_id)?;
            let speed = self.effective_speed(player, unit_id)?;
            let grant = new_max - old_max;
            if let Some(entity) = self.find_mut(id) {
                // A site that is still going up stores the maximum it will finish at, so it has to
                // hear about the new one too - otherwise a house whose build overlapped the tech
                // would finish at the health it was quoted when the order landed.
                if let Some(mut construction) = entity.construction {
                    construction.max_hp = new_max;
                    entity.construction = Some(construction);
                }
                entity.hp = entity.hp.saturating_add(grant).clamp(1, new_max);
                entity.speed = speed;
            }
        }

        let Some(effect_id) = self
            .techs
            .as_ref()
            .and_then(|techs| techs.get(tech))
            .and_then(|stats| stats.effect_id)
        else {
            return Ok(());
        };
        let commands: Vec<(i16, i16, i16)> = self
            .effects
            .as_ref()
            .and_then(|effects| effects.get(effect_id))
            .map(|effect| {
                effect
                    .commands
                    .iter()
                    .map(|command| (command.kind, command.a, command.b))
                    .collect()
            })
            .ok_or(SimError::MissingEffect { id: effect_id })?;

        for (kind, a, b) in commands {
            if kind != 3 {
                continue;
            }
            let (Ok(from), Ok(to)) = (u32::try_from(a), u32::try_from(b)) else {
                // A negative argument is "nothing", not an id: kind 3 always names both units.
                continue;
            };
            self.upgrade_units(player, from, to)?;
        }
        Ok(())
    }

    /// Replace every unit of `player`'s that is a `from` with a `to`, keeping the fraction of health
    /// it had: a spearman at half health becomes a swordsman at half health, not a healed one.
    fn upgrade_units(&mut self, player: u8, from: u32, to: u32) -> Result<(), SimError> {
        // Both caps go through the modifier table, so an upgrade does not undo a health tech.
        let old_max = self.max_hp(player, from).unwrap_or(0);
        let new_max = self.max_hp(player, to)?;
        let speed = self.effective_speed(player, to)?;

        let ids: Vec<EntityId> = self
            .entities
            .iter()
            .filter(|entity| entity.player == player && entity.unit == Some(from))
            .map(|entity| entity.id)
            .collect();
        for id in ids {
            let Some(entity) = self.find_mut(id) else {
                continue;
            };
            let scaled = if old_max > 0 {
                // Ceiling division: a unit that had *any* health left keeps at least one point.
                let numerator = i64::from(entity.hp) * i64::from(new_max);
                i32::try_from((numerator + i64::from(old_max) - 1) / i64::from(old_max))
                    .unwrap_or(new_max)
            } else {
                new_max
            };
            entity.hp = scaled.clamp(1, new_max);
            entity.unit = Some(to);
            entity.speed = speed;
        }
        Ok(())
    }
}
