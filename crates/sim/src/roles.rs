//! **What a role is called, and what ground is free** (D198): three `World` queries the engine and the scenario language both need.
//!
//! They lived in `script.rs` until the scenario language moved out of the engine crate - the move found them, because the engine
//! itself calls all three (`metrics`, `movement`, `sites`, `shares`).

use crate::{EntityKind, World};

impl World {
    /// The unit id a player's villagers use: the `start.villager` role, resolved through the
    /// economy's role table.
    pub fn start_villager_unit(&self) -> Option<u32> {
        let economy = self.economy.as_ref()?;
        economy.role.get(&economy.start.villager).copied()
    }

    /// The unit id of the town centre a match starts from: the `start.town_center` role. Scripted
    /// steps place buildings beside it, and they must do that by role rather than by the word.
    pub fn start_town_center_unit(&self) -> Option<u32> {
        let economy = self.economy.as_ref()?;
        economy.role.get(&economy.start.town_center).copied()
    }

    /// Is this tile inside the map, walkable, and empty?
    pub fn is_free_ground(&self, x: u16, y: u16) -> bool {
        let Some(walk) = self.walk.as_ref() else {
            return false;
        };
        if !walk.contains(x, y) || walk.blocked(x, y) {
            return false;
        }
        let position = (
            ruage_fixed::Fixed::from_raw(i64::from(x) * self.scale),
            ruage_fixed::Fixed::from_raw(i64::from(y) * self.scale),
        );
        // **A unit does not make ground occupied.** Terrain and what stands on it permanently do; a villager is a person
        // who is somewhere for a moment, and treating people as obstacles left a player unable to place a market beside
        // their own town centre - 177 refusals in a row - once villagers started standing there to deposit.
        //
        // **And a spent farm is not ground that stands on itself** (D77): its husk is a floor, and the next farm is built
        // on it. This predicate used to say otherwise while `site_refusal` said the opposite, so the placement search
        // never offered a husk's tile and the farm rule refused with *"no free tile for a farm"* 105 times in one
        // acceptance run, with nine husks in its own base.
        !self.entities().iter().any(|entity| {
            entity.position == position
                && entity.kind != EntityKind::Unit
                && !self.is_expired_farm(entity.id)
        })
    }
}
