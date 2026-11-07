//! **Garrison**: who may go inside, and what coming out means (D145).
//!
//! Split out of `lib.rs` under D145, verbatim.
use crate::{EntityId, EntityKind, SimError, Task, World};
impl World {
    /// The checks a research order has to survive: is this a finished building, is it the building
    /// the tech names, is the tech unknown / already had / already being researched here, and can
    /// the player pay. Returns the price and how many ticks it takes.
    /// The checks a garrison order has to survive: a unit, a finished building of the same player, room
    /// inside it, and the unit close enough to walk in - the capacity is the building's own data field.
    pub(crate) fn check_garrison(
        &self,
        unit: EntityId,
        building: EntityId,
    ) -> Result<(), SimError> {
        let source = self.find(unit).ok_or(SimError::MissingEntity(unit))?;
        if source.kind != EntityKind::Unit {
            return Err(SimError::NotAUnit { id: unit });
        }
        if matches!(source.task, Task::Garrisoned { .. }) {
            return Err(SimError::Garrisoned { id: unit });
        }
        let shelter = self
            .find(building)
            .ok_or(SimError::MissingEntity(building))?;
        if shelter.kind != EntityKind::Building {
            return Err(SimError::TrainNeedsABuilding { id: building });
        }
        if shelter.construction.is_some() {
            return Err(SimError::BuildingNotReady { id: building });
        }
        if shelter.player != source.player {
            return Err(SimError::FriendlyFire { id: building });
        }
        // Only a building that fights takes units in from outside. A production building - barracks,
        // archery range, stable, siege workshop, dock - holds nothing but what it has just trained, and
        // a unit that has left it can never come back. The data draws the line itself: exactly those
        // five carry a garrison capacity and no attack array, while the town centre, towers and castles
        // carry both (D38).
        let units_for_hp = self.units.as_ref().ok_or(SimError::NoTables)?;
        if let Some(stats) = shelter.unit.and_then(|built| units_for_hp.unit(built))
            && (stats.attacks.is_empty()
                || stats
                    .range
                    .resolve(self.scale, "range")
                    .map_or(0, |range| range.raw())
                    <= 0)
        {
            return Err(SimError::CannotGarrisonHere { id: building });
        }
        if let Some(stats) = shelter.unit.and_then(|built| units_for_hp.unit(built)) {
            let max = self.max_hp(shelter.player, stats.id).unwrap_or(stats.hp);
            let floor = (i64::from(max) * self.garrison_eject_at.raw()) / self.scale;
            if i64::from(shelter.hp) <= floor {
                return Err(SimError::GarrisonTooDamaged { id: building });
            }
        }
        let units = self.units.as_ref().ok_or(SimError::NoTables)?;
        let capacity = shelter
            .unit
            .and_then(|built| units.unit(built))
            .map_or(0, |stats| u32::from(stats.garrison));
        let inside = u32::try_from(
            self.entities
                .iter()
                .filter(|entity| {
                    entity.player == source.player
                        && entity.kind == EntityKind::Unit
                        && entity.task == (Task::Garrisoned { building })
                })
                .count(),
        )
        .unwrap_or(u32::MAX);
        if inside >= capacity {
            return Err(SimError::GarrisonFull {
                id: building,
                capacity,
            });
        }
        // Standing on the building's tile or beside it is close enough. A unit across the map is not
        // teleported in: move it first.
        let scale = self.scale;
        let here = (
            u16::try_from(source.position.0.raw() / scale).unwrap_or(0),
            u16::try_from(source.position.1.raw() / scale).unwrap_or(0),
        );
        let there = (
            u16::try_from(shelter.position.0.raw() / scale).unwrap_or(0),
            u16::try_from(shelter.position.1.raw() / scale).unwrap_or(0),
        );
        if here.0.abs_diff(there.0) > 1 || here.1.abs_diff(there.1) > 1 {
            return Err(SimError::TooFar { id: unit });
        }
        Ok(())
    }

    /// Let everyone out of a building, where they stand. Shared by `Ungarrison` and by a player
    /// demolishing their own building, so the two cannot drift apart.
    pub(crate) fn ungarrison(&mut self, building: EntityId) {
        let leaving: Vec<EntityId> = self
            .entities
            .iter()
            .filter(|entity| entity.task == (Task::Garrisoned { building }))
            .map(|entity| entity.id)
            .collect();
        for id in leaving {
            if let Some(entity) = self.find_mut(id) {
                entity.task = Task::Idle;
            }
        }
    }
}
