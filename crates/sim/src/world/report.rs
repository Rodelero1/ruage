//! **What a match reports about itself**: the per-player card, the stats behind it, and a saved game's techs (D145).
//!
//! Split out of `lib.rs` under D145, verbatim.
use ruage_data::Effects;

use crate::{CardItem, CardTarget, EntityId, EntityKind, SimError, UnitStats, World};
impl World {
    #[must_use]
    pub fn card(&self, player: u8, entity: EntityId) -> Vec<CardItem> {
        let Some(found) = self.find(entity) else {
            return Vec::new();
        };
        if found.player != player
            || found.kind != EntityKind::Building
            || found.construction.is_some()
        {
            return Vec::new();
        }
        let Some(unit) = found.unit else {
            return Vec::new();
        };
        let root = self.units.as_ref().map_or(unit, |units| units.root(unit));
        let mut items = Vec::new();
        // What this building makes.
        if let Some(economy) = self.economy.as_ref()
            && let Some(made) = economy.train.by_building.get(&root)
        {
            for candidate in made {
                let refusal = self.check_train(entity, *candidate).err();
                // Hidden, not refused: the age's answer is not a choice the player can take (see `SimError::is_gate`).
                if refusal.as_ref().is_some_and(|err| self.is_age_gate(err)) {
                    continue;
                }
                let costs = self
                    .units
                    .as_ref()
                    .and_then(|units| units.unit(*candidate))
                    .map(|stats| self.cost_slots(&stats.costs))
                    .unwrap_or_default();
                items.push(CardItem {
                    target: CardTarget::Unit(*candidate),
                    name: self
                        .role_name(*candidate)
                        .unwrap_or_else(|| format!("unit {candidate}")),
                    costs,
                    refusal: refusal.map(|err| err.to_string()),
                });
            }
        }
        // What it researches: a tech names the unit that researches it (`location`), which is 0 for a tech with no home.
        if let Some(techs) = self.techs.as_ref() {
            let mut researched: Vec<CardItem> = techs
                .techs
                .iter()
                .filter(|(_, tech)| u32::try_from(tech.location).ok() == Some(root))
                .filter_map(|(id, tech)| {
                    // One refusal, asked once: the gate check and the sentence are the same answer.
                    let refusal = self.check_research(entity, *id).err();
                    if refusal.as_ref().is_some_and(|err| self.is_age_gate(err)) {
                        return None;
                    }
                    Some(CardItem {
                        target: CardTarget::Tech(*id),
                        name: self
                            .tech_role_name(*id)
                            .map_or_else(|| format!("tech {id}"), str::to_owned),
                        costs: self.cost_slots(&tech.costs),
                        refusal: refusal.map(|err| err.to_string()),
                    })
                })
                .collect();
            // **A slot does not move when the money changes.** The first version put the affordable things first, so a
            // button jumped to another slot the moment it became affordable - which is how a player clicks the wrong one.
            // The order is the pack's: what the building makes, then its techs by id.
            researched.sort_by_key(|item| match item.target {
                CardTarget::Tech(id) | CardTarget::Unit(id) => id,
            });
            items.extend(researched);
        }
        items
    }

    /// **The numbers a unit's card shows** (owner, 2026-11-04: *"let's add a functionality where unit stats are shown in
    /// the bottom central part ... For units see Selection-Bar.png"*).
    ///
    /// The unit table is private to the simulation, so the panel cannot read it; this is the accessor that lets it, and it
    /// answers in the owner's numbers rather than the dat's: an attack of 400 is four, and the two armours are melee (class 4)
    /// over pierce (class 3), which is the `0/0` an Archer card shows.
    ///
    /// **Attack is the larger of the two display classes**, not the largest number on the unit. A villager carries
    /// `[[11,3],[4,3],[13,6],[14,0]]`, and 6 is a bonus against buildings; its card says 3. The panel shows what a player
    /// reads, and a class-specific bonus is not a unit's attack.
    ///
    /// Fractional quantities are integer **hundredths** (speed 96 = 0.96 of a tile a second), so nothing here is a float and
    /// nothing here can desync a tick. Hit points and capacity are counts and stay whole.
    pub fn stats_of(&self, player: u8, unit_id: u32) -> Option<UnitStats> {
        let unit = self.units.as_ref()?.unit(unit_id)?;
        // Effective, not base: an owner's upgrades are what the panel must read (D112's rule, applied to every number).
        let attacks = self.adjusted(player, unit, "attack");
        let armours = self.adjusted(player, unit, "armour");
        let of_class = |list: &[(i16, i16)], class: i16| -> i32 {
            list.iter()
                .find(|(key, _)| *key == class)
                .map(|(_, value)| i32::from(*value))
                .unwrap_or(0)
        };
        // **`i128`, so the multiply cannot overflow before the divide** (D128): a card is drawn once per selection, and the
        // exact answer costs nothing here.
        let hundredths = |raw: i64| -> i32 {
            let value = i128::from(raw) * 100 / i128::from(self.scale.max(1));
            i32::try_from(value).unwrap_or(i32::MAX)
        };
        let fractional = |value: ruage_data::MaybeFixed, key: &str| -> i32 {
            match value.resolve(self.scale, key) {
                Ok(fixed) => hundredths(fixed.raw()),
                // An unresolved value is refused at load, so this arm is for a pack that slipped one through: the card
                // shows zero rather than inventing a number the simulation is not using.
                Err(_) => 0,
            }
        };
        Some(UnitStats {
            max_hp: self.max_hp(player, unit_id).ok()?,
            attack: of_class(&attacks, 3).max(of_class(&attacks, 4)) * 100,
            armour_melee: of_class(&armours, 4) * 100,
            armour_pierce: of_class(&armours, 3) * 100,
            range: fractional(unit.range, "range"),
            speed: fractional(unit.speed, "speed"),
            los: fractional(unit.los, "los"),
            work_rate: fractional(unit.work_rate, "work_rate"),
            carry: i32::from(unit.carry),
        })
    }

    /// Hand the world the tech and effect tables. Refused when either is empty, because an empty
    /// table is a content mistake that would otherwise surface as "every tech is missing".
    pub fn load_research(
        &mut self,
        techs: &ruage_data::Techs,
        effects: &Effects,
    ) -> Result<(), SimError> {
        if techs.techs.is_empty() {
            return Err(SimError::MissingTech { id: 0 });
        }
        if effects.effects.is_empty() {
            return Err(SimError::MissingEffect { id: 0 });
        }
        self.techs = Some(techs.clone());
        self.effects = Some(effects.clone());

        // What availability looks like in this pack: which units some tech can make available, and which
        // techs exist only to do that. Both are read once, from content.
        self.gated_units.clear();
        self.availability_techs.clear();
        for effect in effects.effects.values() {
            for command in &effect.commands {
                if command.kind == 2
                    && command.b == 1
                    && let Ok(unit) = u32::try_from(command.a)
                {
                    self.gated_units.insert(unit);
                }
            }
        }
        // Which tech upgrades into which unit. The lowest tech id wins when two effects could produce
        // the same unit, so the answer does not depend on iteration order.
        self.upgrade_tech.clear();
        for tech in techs.techs.values() {
            let Some(effect_id) = tech.effect_id else {
                continue;
            };
            let Some(effect) = effects.effects.get(&effect_id) else {
                continue;
            };
            for command in &effect.commands {
                if command.kind != 3 {
                    continue;
                }
                let (Ok(from), Ok(to)) = (u32::try_from(command.a), u32::try_from(command.b))
                else {
                    continue;
                };
                if from == to || to == 0 {
                    continue;
                }
                self.upgrade_tech
                    .entry(to)
                    .and_modify(|existing| *existing = (*existing).min(tech.id))
                    .or_insert(tech.id);
            }
        }
        for tech in techs.techs.values() {
            if tech.research_time != 0 || !tech.costs.is_empty() || tech.location >= 0 {
                continue;
            }
            // Bookkeeping techs are the ones with nothing to click: no price, no time, no building,
            // and an effect that either does nothing at all or only makes units available. The dat
            // spells both kinds of marker this way - "ARRG (make avail)" enables a building, and
            // "Shadow Barracks -- Age Two" *is* the statement that a barracks exists (tech 122, no
            // effect, required by 667, which the archery range requires). Without the second kind the
            // whole Feudal building tree is unreachable: 147 waits on 667, 667 waits on 122, and
            // nothing would ever grant it (D29).
            let bookkeeping = match tech.effect_id {
                None => true,
                Some(effect_id) => match effects.effects.get(&effect_id) {
                    None => true,
                    Some(effect) => {
                        effect.commands.is_empty()
                            || effect
                                .commands
                                .iter()
                                .all(|command| command.kind == 2 && command.b == 1)
                    }
                },
            };
            if bookkeeping {
                self.availability_techs.push(tech.id);
            }
        }

        self.grant_starting_techs()
    }
}
