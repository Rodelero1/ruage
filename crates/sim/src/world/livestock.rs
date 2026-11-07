//! **Animals**: what a dead hunt leaves behind, and a sheep becoming a villager's (D145).
//!
//! Split out of `lib.rs` under D145, verbatim.
use ruage_fixed::Fixed;

use crate::{EntityId, EntityKind, MAX_PLAYERS, Rot, Task, World};
impl World {
    /// A herdable changes hands by **line of sight** (D70, item 5): a *unit's* sight, not a building's, immediate,
    /// herdables only. Unowned animals join the first player who sees them; an owned one is stolen when another
    /// player sees it and **no unit of its owner's does**; a tie goes to the side with more units near it.
    ///
    /// Cost: one walk of the units times the herdables alive, and nothing at all when no herdable stands - which is
    /// the early return below. The counts are a fixed array per animal, so no allocation happens per player.
    pub(crate) fn step_conversion(&mut self) {
        if self.herdable_ids.is_empty() || self.sight.is_empty() {
            return;
        }
        // Copies out of the borrow: the decisions below change tasks and owners.
        let seers: Vec<(u8, (Fixed, Fixed), i64)> = self
            .entities
            .iter()
            .filter(|entity| entity.kind == EntityKind::Unit && entity.player > 0 && entity.hp > 0)
            .filter(|entity| !matches!(entity.task, Task::Garrisoned { .. }))
            // An animal is not a witness: a sheep has a line of sight of its own, and counting it would let every
            // owned animal keep itself - which is exactly what the first version did (seen = [1, 0, ...]).
            .filter(|entity| !entity.unit.is_some_and(|unit| self.animals.contains(&unit)))
            .filter_map(|entity| {
                let reach = entity.unit.and_then(|unit| self.sight.get(&unit))?;
                Some((entity.player, entity.position, *reach))
            })
            .collect();
        if seers.is_empty() {
            return;
        }
        let animals: Vec<(EntityId, u8, (Fixed, Fixed))> = self
            .herdable_ids
            .iter()
            .filter_map(|id| {
                self.find(*id)
                    .filter(|entity| entity.hp > 0)
                    .map(|entity| (entity.id, entity.player, entity.position))
            })
            .collect();
        // (animal, new owner, whether it was somebody's already): only a *steal* disturbs the herder, because an
        // unowned animal joining the player whose villager is already walking it is the ordinary case.
        let mut taken: Vec<(EntityId, u8, bool)> = Vec::new();
        for (id, owner, position) in animals {
            let mut seen: [u32; MAX_PLAYERS] = [0; MAX_PLAYERS];
            for (player, at, reach) in &seers {
                let dx = at.0.raw() - position.0.raw();
                let dy = at.1.raw() - position.1.raw();
                if dx * dx + dy * dy <= *reach {
                    seen[usize::from(*player) - 1] += 1;
                }
            }
            // The owner's own sight keeps the animal: that is the manual's second sentence.
            if owner > 0 && seen[usize::from(owner) - 1] > 0 {
                continue;
            }
            // Among the players who can see it, the one with most units near wins; a tie goes to the lower player
            // number, so two runs of the same match agree.
            let mut best: Option<(u8, u32)> = None;
            for (index, count) in seen.iter().enumerate() {
                if *count == 0 {
                    continue;
                }
                let player = u8::try_from(index + 1).unwrap_or(u8::MAX);
                if player == owner {
                    continue;
                }
                if best.is_none_or(|(_, best_count)| *count > best_count) {
                    best = Some((player, *count));
                }
            }
            if let Some((player, _)) = best {
                taken.push((id, player, owner > 0));
            }
        }
        for (id, player, stolen) in taken {
            if let Some(entity) = self.find_mut(id) {
                entity.player = player;
            }
            if stolen {
                // **Stolen**: it belongs to somebody else now, so it walks to nobody's drop site and the villager
                // that was herding it stops rather than following a stranger's sheep home. That window - the walk,
                // while the animal is alive - is the one D70 kept pointing at.
                self.paths.remove(&id);
                self.leash.remove(&id);
                let herders: Vec<EntityId> = self
                    .entities
                    .iter()
                    .filter(|entity| matches!(entity.task, Task::Gather { node, .. } if node == id))
                    .map(|entity| entity.id)
                    .collect();
                for herder in herders {
                    self.stand_down(herder);
                    self.slaughter.remove(&herder);
                }
            }
            self.stamp_entity(id);
        }
    }

    /// Death leaves a body: an animal at or below zero health becomes its `dead_unit` **in place**, and it holds
    /// its meat only if the last blow was a butcher's (D70, item 2).
    ///
    /// In place, and on the same id, because the gatherer's task names that id: a villager working a sheep carries
    /// on with the carcass without a re-target, which is the whole reason the slaughter transform was shaped this
    /// way. Health, speed, velocity and the variant all go - a body is not a unit that may move or be attacked.
    pub(crate) fn promote_dead_animals(
        &mut self,
        blows: &[(EntityId, EntityId)],
        new_rots: &mut Vec<(EntityId, Rot)>,
    ) {
        if self.animal_bodies.is_empty() || blows.is_empty() {
            return;
        }
        // Only this tick's victims can have died on this tick, so the candidates are a handful rather than the
        // whole entity list: a pass over every entity per tick is exactly the shape a review exists to catch.
        let mut seen: Vec<EntityId> = Vec::new();
        for (victim, _) in blows.iter().rev() {
            if seen.contains(victim) {
                continue;
            }
            seen.push(*victim);
            let Some(index) = self.entities.iter().position(|entity| entity.id == *victim) else {
                continue;
            };
            let (hp, unit, amount, kind) = {
                let entity = &self.entities[index];
                (entity.hp, entity.unit, entity.amount, entity.kind)
            };
            if kind != EntityKind::Unit || hp > 0 {
                continue;
            }
            let Some(body) = unit.and_then(|unit| self.animal_bodies.get(&unit).copied()) else {
                continue;
            };
            // The last blow decides: a soldier's or a tower's kill leaves a body with nothing in it.
            let killer = blows
                .iter()
                .rev()
                .find(|(id, _)| id == victim)
                .map(|(_, attacker)| *attacker);
            let butchered = killer.is_some_and(|killer| {
                self.find(killer)
                    .and_then(|entity| entity.unit)
                    .is_some_and(|unit| self.butchers.contains(&unit))
            });
            let id = *victim;
            let entity = &mut self.entities[index];
            entity.unit = Some(body);
            entity.amount = if butchered { amount } else { Fixed::ZERO };
            entity.hp = 0;
            entity.speed = Fixed::ZERO;
            entity.velocity = (Fixed::ZERO, Fixed::ZERO);
            entity.variant = None;
            entity.task = Task::Idle;
            // Bookkeeping goes with the animal: a body chases nothing and walks nowhere. A body that holds meat
            // starts rotting at its species' rate; an empty one is left to the sweep's business for now.
            self.paths.remove(&id);
            self.leash.remove(&id);
            self.rots.remove(&id);
            // Every body has a lifetime, meat or no meat: a soldier's kill leaves a carcass with nothing in it, and
            // before this it stood on the map for the rest of the match (item 6). A body that holds meat rots it
            // away and goes when it is empty; an empty one simply lies for its species' span and then goes.
            if let Some(unit) = unit {
                let rate = self.animal_rot.get(&unit).copied().unwrap_or(0);
                let span = self.animal_span.get(&unit).copied().unwrap_or(0);
                if span > 0 {
                    new_rots.push((
                        id,
                        Rot {
                            per_tick: if butchered { rate } else { 0 },
                            expires: self.tick.saturating_add(span),
                        },
                    ));
                }
            }
        }
    }
}
