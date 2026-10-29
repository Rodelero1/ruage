//! What one run of one script on one seed is worth, in numbers (item 8).
//!
//! The balance engine's question is not "did this match look right" but "how fast did this build order reach
//! Feudal, on this seed, with this pack" - so every number here is **derived from what the world already
//! reports**: the stockpile, the entity list, the researched set and the outcome. No engine state is added, nothing
//! new is hashed, and the runner cannot become a second simulation that disagrees with the first.
//!
//! Integers only. The sim crate keeps floats out of its own source (the `no_float` gate), and a metric does not need
//! one: times are ticks, quantities are whole resources, and a rate is scaled by the run's own length.

use std::collections::BTreeSet;

use crate::{EntityId, EntityKind, World};

/// The numbers a batch run reports for one side.
#[derive(Debug, Clone)]
pub struct Metrics {
    player: u8,
    tick_rate_hz: u32,
    /// The villager family, for counting losses: content's start unit, not an id this module knows.
    villager: Option<u32>,
    /// The age techs a run watches for, resolved once by role.
    feudal: Option<u32>,
    castle: Option<u32>,
    /// Unit ids that count as military, from `[metrics] military_roles` (family-matched when asked).
    military: Vec<u32>,
    /// Per stockpile slot: what came in, and what went out. A deposit is the only thing that raises a stockpile, so
    /// the increases are gathered and the decreases are spent - and neither can be mistaken for the other.
    delivered: Vec<i64>,
    spent: Vec<i64>,
    last_stock: Vec<u32>,
    /// Every villager this run has ever seen, so a disappearance is a death rather than a gap in a sample.
    seen_villagers: BTreeSet<EntityId>,
    villagers_lost: u32,
    first_military: Option<u64>,
    feudal_at: Option<u64>,
    castle_at: Option<u64>,
    ended_at: Option<u64>,
    /// Ticks observed, which is what turns a total into a rate.
    ticks_observed: u64,
    /// How often the entity walks happen, in ticks: a measurement tool may be cheap, and one sample a second is
    /// plenty for "when did the first soldier appear".
    sample_every: u64,
    slot_count: usize,
}

impl Metrics {
    /// Watch `player` in `world`. Roles the pack cannot name are skipped: a metric is a measurement, not a rule of the
    /// game, so a missing soldier role shrinks the count rather than refusing the run.
    pub fn new(world: &World, player: u8, military_roles: &[String]) -> Self {
        let slot_count = world.resources().map_or(0, |resources| resources.count());
        let military = military_roles
            .iter()
            .filter_map(|role| world.role_unit(role))
            .collect();
        let last_stock = (0..slot_count)
            .map(|slot| world.stock(player, slot))
            .collect();
        Self {
            player,
            tick_rate_hz: world.tick_rate_hz(),
            villager: world.start_villager_unit(),
            feudal: world.tech_role("feudal_age"),
            castle: world.tech_role("castle_age"),
            military,
            delivered: vec![0; slot_count],
            spent: vec![0; slot_count],
            last_stock,
            seen_villagers: BTreeSet::new(),
            villagers_lost: 0,
            first_military: None,
            feudal_at: None,
            castle_at: None,
            ended_at: None,
            ticks_observed: 0,
            sample_every: u64::from(world.tick_rate_hz()).max(1),
            slot_count,
        }
    }

    /// Take one sample. Called after every step: the stockpile read is an array index, and the entity walk happens
    /// once a second.
    pub fn observe(&mut self, world: &World) {
        self.ticks_observed = self.ticks_observed.max(world.tick());
        for slot in 0..self.slot_count {
            let now = world.stock(self.player, slot);
            let before = self.last_stock[slot];
            if now > before {
                self.delivered[slot] += i64::from(now - before);
            } else if now < before {
                self.spent[slot] += i64::from(before - now);
            }
            self.last_stock[slot] = now;
        }
        if self.ended_at.is_none() && world.outcome().is_some() {
            self.ended_at = Some(world.tick());
        }
        if !world.tick().is_multiple_of(self.sample_every) {
            return;
        }
        let researched = world.researched(self.player);
        if self.feudal_at.is_none() && self.feudal.is_some_and(|tech| researched.contains(&tech)) {
            self.feudal_at = Some(world.tick());
        }
        if self.castle_at.is_none() && self.castle.is_some_and(|tech| researched.contains(&tech)) {
            self.castle_at = Some(world.tick());
        }
        for entity in world.entities() {
            if entity.kind != EntityKind::Unit {
                continue;
            }
            if entity.player == self.player
                && self
                    .villager
                    .is_some_and(|villager| world.is_family(entity.unit.unwrap_or(0), villager))
            {
                self.seen_villagers.insert(entity.id);
            }
            if self.first_military.is_none()
                && entity.player == self.player
                && entity.unit.is_some_and(|unit| {
                    self.military
                        .iter()
                        .any(|role| world.is_family(unit, *role))
                })
            {
                self.first_military = Some(world.tick());
            }
        }
        // A villager that is no longer anywhere was lost. Forgetting it keeps the set bounded and makes the count
        // exact rather than cumulative on every sample.
        let gone: Vec<EntityId> = self
            .seen_villagers
            .iter()
            .copied()
            .filter(|id| world.entity(*id).is_none())
            .collect();
        for id in gone {
            self.seen_villagers.remove(&id);
            self.villagers_lost += 1;
        }
    }

    /// Whole resources of `slot` that reached the stockpile.
    pub fn delivered(&self, slot: usize) -> i64 {
        self.delivered.get(slot).copied().unwrap_or(0)
    }

    /// ... and what left it again, by price.
    pub fn spent(&self, slot: usize) -> i64 {
        self.spent.get(slot).copied().unwrap_or(0)
    }

    /// Delivered per minute over the run so far, in whole resources. Integer arithmetic throughout: a one-minute run
    /// reports exactly what it delivered.
    pub fn per_minute(&self, slot: usize) -> i64 {
        if self.ticks_observed == 0 {
            return 0;
        }
        let ticks = i64::try_from(self.ticks_observed).unwrap_or(i64::MAX);
        self.delivered(slot) * 60 * i64::from(self.tick_rate_hz) / ticks
    }

    pub fn feudal_at(&self) -> Option<u64> {
        self.feudal_at
    }

    pub fn castle_at(&self) -> Option<u64> {
        self.castle_at
    }

    pub fn first_military_at(&self) -> Option<u64> {
        self.first_military
    }

    pub fn villagers_lost(&self) -> u32 {
        self.villagers_lost
    }

    pub fn ended_at(&self) -> Option<u64> {
        self.ended_at
    }
}
