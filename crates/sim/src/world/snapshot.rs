//! **P1a: the world written down, and read back** (D235).
//!
//! P1's shape was ruled by the owner on 2026-11-14: the **converter lives in `tools/`**, and the engine only ever reads its
//! own state format - so no reader of the game's save file enters `crates/`. This module is the engine's half: the world as
//! one JSON document, and a load that proves it is the same world by ticking the same as the one it came from.
//!
//! **What is *not* in the document, deliberately.** The content pack's tables and rules (`techs`, `effects`, `civs`,
//! `resources`, `damage_rules`) are not the match's state - they are the game's rules - so a state carries the pack's
//! **identity** instead, and a load against a different pack is refused rather than silently changing what a unit does. The
//! two derived caches (`bonus_tables`, `sight_modifiers`) are not written either: they are rebuilt on load, which is what
//! makes a missing rebuild a test failure instead of a file that looks complete.
//!
//! **Why `serde` on `World` itself rather than a `WorldState` mirror.** A mirror is a second list of the world's fields, and
//! the second list is the one that goes stale - this project has paid for that shape twice already (the farm line, the
//! builder speed, D234). A derive cannot forget a field: a field added to `World` is in the file the next time the crate
//! compiles, and the round-trip fixture fails if the load does not restore it.

use crate::{MAX_PLAYERS, SimError, World};
use ruage_data::ContentPack;

/// The document: the pack that was in force, and the world's own fields flattened beside it, so a reader sees one object and a
/// diff between two states is a diff of field names.
#[derive(serde::Serialize)]
struct Written<'a> {
    pack: &'a str,
    #[serde(flatten)]
    world: &'a World,
}

#[derive(serde::Deserialize)]
struct Read {
    pack: String,
    #[serde(flatten)]
    world: World,
}

impl World {
    /// **The whole state, as JSON.** Named fields, not a byte layout: a diff between two states says which field changed.
    ///
    /// # Errors
    ///
    /// Only a serialisation failure, which would mean a field type that cannot be written.
    pub fn state(&self, pack: &ContentPack) -> Result<Vec<u8>, SimError> {
        // **MessagePack, not JSON, and the reason is the maps**: the state is keyed by numbers in a dozen places
        // (`BTreeMap<EntityId, Route>`, `researched: BTreeMap<u8, BTreeSet<u32>>`), JSON insists map keys are strings, and
        // the alternative was a `#[serde(with)]` helper on every one of them - a second list to keep in step, which is what
        // this module exists to avoid. MessagePack keeps the **names** (it is self-describing), so a decoded state still
        // reads as field names one per line.
        rmp_serde::to_vec_named(&Written {
            pack: &pack.identity(),
            world: self,
        })
        .map_err(|err| SimError::BadState {
            detail: err.to_string(),
        })
    }

    /// **A world from a state**, with the pack's tables put back and the derived caches rebuilt.
    ///
    /// # Errors
    ///
    /// `BadState` when a field the state does not name is missing - no field is defaulted, because a corrupt file must not
    /// become a match that diverges a hundred ticks later - and `PackMismatch` when the state was written under another pack.
    pub fn from_state(pack: &ContentPack, state: &[u8]) -> Result<Self, SimError> {
        let read: Read = rmp_serde::from_slice(state).map_err(|err| SimError::BadState {
            detail: err.to_string(),
        })?;
        let pack_identity = pack.identity();
        if read.pack != pack_identity {
            return Err(SimError::PackMismatch {
                state: read.pack,
                pack: pack_identity,
            });
        }
        let mut world = read.world;
        // **The rules, from the pack; the caches, rebuilt.** `load_civs` is deliberately *not* used here: it assigns each
        // player a civ out of `economy.start.civs`, and the start state is the match's, not the pack's - a load that took it
        // from the pack would give a world a different civ from the one the state names (the second failure this fixture
        // found). The civ table goes in, and the two derived caches are rebuilt from the state's own assignment.
        world.command_apply = pack.sim.command_apply;
        world.units = Some(pack.units.clone());
        world.economy = Some(pack.economy.clone());
        world.civs = Some(pack.civs.clone());
        world.load_combat(&pack.combat);
        world.load_resources(&pack.resources)?;
        world.load_research(&pack.techs, &pack.effects)?;
        for index in 0..MAX_PLAYERS {
            let player = u8::try_from(index + 1).unwrap_or(1);
            world.rebuild_bonuses(player);
            world.refresh_sight_modifiers(player);
        }
        Ok(world)
    }
}
