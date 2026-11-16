//! **One table per player** (R6, D234): the effects that belong to a player, walked **once**, so every consumer asks the
//! same list instead of deciding for itself which effects are theirs.
//!
//! Five consumers used to walk three effect tables independently - `barred_by` and `civ_barred_unit` (the civ's tech-tree
//! effect, kinds 102 and 2), `modifiers_for` (the researched techs and the team bonus, kinds 0/4/5 matched by attribute
//! string), `farm_food_bonus` (resource rows on the farm line) and `construction_rate` (resource 195) - and each decided
//! *which effects belong to the player* by a slightly different rule. That is how a bonus ends up half-applied: the farm
//! line and the construction rate were each "one resource row somebody forgot", and both were found by accident.
//!
//! What is cached is the **rows**, not a resolved number: the five consumers fold them differently (one sums the farm line,
//! one takes set-then-add for the build rate, one adds and multiplies per unit class), so caching the fold would be caching
//! five different answers. Caching the rows makes the walk once per event and cannot change arithmetic by construction,
//! which is what the acceptance hash checks.

use std::collections::{BTreeMap, BTreeSet};

use crate::{MAX_PLAYERS, World};
use ruage_data::{Effects, Modifier};
use ruage_fixed::Fixed;

/// Where a row came from. The consumers do not all read the same sources - `modifiers_for` never reads the civ's
/// tech-tree effect, `construction_rate` reads it **before** the researched techs, and `farm_food_bonus` never reads it at
/// all - so the source travels with the row and each consumer filters what it folds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Source {
    /// The civ's tech-tree effect: what the player *cannot* have, and the per-civ numbers that are not a research.
    Tree,
    /// What the player has researched, in tech-id order.
    Research,
    /// The civ's team bonus: an effect nothing researches, held from the first tick.
    Team,
}

/// One kind-1 row: a resource id, its mode (`0` set, `1` add) and its value.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ResourceRow {
    pub(crate) source: Source,
    pub(crate) resource: i16,
    pub(crate) mode: i16,
    /// The value **resolved at the world's scale**, because the simulation computes in fixed-point only (D9) and a row that
    /// does not fit the scale is dropped here, exactly as each reader dropped it when it resolved in place.
    pub(crate) value: Fixed,
}

/// **Everything the five consumers ask about one player.** Rebuilt when a tech lands or the civ is set - the two events
/// that can change it - and never on a tick.
#[derive(Debug, Clone, Default)]
pub(crate) struct BonusTable {
    /// Attribute name -> the modifier rows naming it, in application order (researched techs, then the team bonus).
    attributes: BTreeMap<String, Vec<Modifier>>,
    /// Kind-1 rows in the order the two folding consumers expect: the tech tree first, then researched, then the team.
    resources: Vec<ResourceRow>,
    /// Kind-102 rows of the tech-tree effect: the techs this player may not research.
    barred_techs: BTreeSet<u32>,
    /// Kind-2 rows with `b == 0`: the units this player may not have.
    barred_units: BTreeSet<u32>,
}

impl BonusTable {
    /// The modifiers naming `attribute`, in application order: researched techs in id order, then the team bonus.
    pub(crate) fn modifiers(&self, attribute: &str) -> &[Modifier] {
        self.attributes.get(attribute).map_or(&[], Vec::as_slice)
    }

    /// Every kind-1 row, with its source, in the order the rows were walked.
    pub(crate) fn resources(&self) -> &[ResourceRow] {
        &self.resources
    }

    pub(crate) fn bars_tech(&self, tech: u32) -> bool {
        self.barred_techs.contains(&tech)
    }

    pub(crate) fn bars_unit(&self, unit: u32) -> bool {
        self.barred_units.contains(&unit)
    }

    /// **Walk the three tables for one player.** `tree`, `research` and `team` are the effect ids in the order each
    /// source states them; a missing table (a fixture with no civ rows) simply contributes nothing.
    pub(crate) fn build(
        tree: Option<u32>,
        research: impl Iterator<Item = u32>,
        team: impl Iterator<Item = u32>,
        effects: &Effects,
        scale: i64,
    ) -> Self {
        let mut table = Self::default();
        let push_effect = |table: &mut BonusTable, source: Source, effect_id: u32| {
            let Some(effect) = effects.get(effect_id) else {
                return;
            };
            for command in &effect.commands {
                match command.kind {
                    1 => {
                        if let Ok(value) = command.value.resolve(scale, "effect.value") {
                            table.resources.push(ResourceRow {
                                source,
                                resource: command.a,
                                mode: command.b,
                                value,
                            });
                        }
                    }
                    // **The bars come from the tech tree and nowhere else** (D23, D234): `barred_by` has always read that
                    // one effect, and two team bonuses in this pack carry a kind-102 row whose meaning is unverified
                    // (`docs/civ-bonuses.md`), so folding every source here would have answered an open question by
                    // accident. The acceptance hash caught exactly that when this was first written.
                    102 if matches!(source, Source::Tree) => {
                        // The barred id is carried in `value` (D23): the four selectors are -1. A row that does not resolve
                        // is not a bar, which is what the reader did with it before.
                        if let Ok(value) = command.value.resolve(scale, "effect.value")
                            && let Ok(id) = u32::try_from(value.raw() / scale)
                        {
                            table.barred_techs.insert(id);
                        }
                    }
                    2 if command.b == 0 && matches!(source, Source::Tree) => {
                        if let Ok(id) = u32::try_from(command.a) {
                            table.barred_units.insert(id);
                        }
                    }
                    _ => {}
                }
            }
            // Only the two sources `modifiers_for` reads contribute modifiers; the tech tree's kind-4/5 rows are
            // deliberately not added, because that consumer has never read them and adding them here would be a
            // behaviour change dressed as a refactor.
            if matches!(source, Source::Tree) {
                return;
            }
            for modifier in &effect.modifiers {
                table
                    .attributes
                    .entry(modifier.attribute.clone())
                    .or_default()
                    .push(modifier.clone());
            }
        };
        if let Some(tree) = tree {
            push_effect(&mut table, Source::Tree, tree);
        }
        for effect in research {
            push_effect(&mut table, Source::Research, effect);
        }
        for effect in team {
            push_effect(&mut table, Source::Team, effect);
        }
        table
    }
}

impl World {
    /// **Rebuild one player's table** (R6, D234). Called on the two events that can change it - a tech landing, and the
    /// civ or its tables being set - and never on a tick. A world with no effect table gets an empty table rather than a
    /// stale one, which is what a fixture that loads research after the civ would otherwise keep.
    pub(crate) fn rebuild_bonuses(&mut self, player: u8) {
        let slot = usize::from(player).min(MAX_PLAYERS - 1);
        let table = {
            let Some(effects) = self.effects.as_ref() else {
                self.bonus_tables[slot] = BonusTable::default();
                return;
            };
            let tree = self
                .civ_of(player)
                .and_then(|id| self.civs.as_ref()?.by_id(id))
                .and_then(|civ| civ.tech_tree_effect);
            let research: Vec<u32> = match self.techs.as_ref() {
                Some(techs) => self
                    .researched(player)
                    .iter()
                    .filter_map(|tech| techs.get(*tech).and_then(|tech| tech.effect_id))
                    .collect(),
                None => Vec::new(),
            };
            let team = self.bonus_effects[slot].clone();
            BonusTable::build(
                tree,
                research.into_iter(),
                team.into_iter(),
                effects,
                self.scale,
            )
        };
        self.bonus_tables[slot] = table;
    }

    /// **The table a consumer reads** (R6, D234). A player past the end of the table gets the last one, the same way every
    /// other per-player array in this engine is clamped.
    pub(crate) fn bonuses(&self, player: u8) -> &BonusTable {
        &self.bonus_tables[usize::from(player).min(MAX_PLAYERS - 1)]
    }
}
