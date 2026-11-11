//! **Gates** (D154): a building whose tiles are walkable while it is open.
//!
//! The owner's rules: *trigger radius — let's try 1* and *enemy standing in open gate will keep the gate open*. The game's own
//! rule, from its manual and the wiki: a gate *"opens automatically if a friendly unit approaches … and only closes again if
//! no unit stands right under the Gate"*, so *"enemy units can also pass or keep the Gate open on purpose"* (D153).
//!
//! Split out of `lib.rs` under D145, and new behaviour under D154.

use crate::{EntityId, EntityKind, World};

/// One gate whose state changed this tick: which it is, the tiles it covers, and whether it is now open.
struct GateChange {
    id: EntityId,
    tiles: Vec<(u16, u16)>,
    open: bool,
}

impl World {
    /// **Open the gates that should be open, close the rest** (D154).
    ///
    /// A gate opens when a unit of its owner - or of a player on its team - stands within **one tile** of it, and closes only
    /// when no unit of **any** side stands on its own tiles. That asymmetry is the rule, not an oversight: an open gate lets
    /// anyone through, so an enemy standing in one keeps it open (the owner's decision, and the game's: *"enemy units can
    /// also pass or keep the Gate open on purpose"*).
    ///
    /// Cost: one walk of the entity list per tick, and only while gates exist - a world with no gate pays one
    /// `is_empty()` per tick. The walk grid is written **only when a gate changes state**, so a gate that stays open is
    /// free after the tick it opened.
    pub(crate) fn step_gates(&mut self) {
        if self.gates.is_empty() {
            return;
        }
        // Every unit's tile and owner, once: a gate asks about all of them, and re-walking the entity list per gate made the
        // first version O(gates x units).
        let mut units: Vec<(u8, (i64, i64))> = Vec::new();
        for entity in &self.entities {
            if entity.kind != EntityKind::Unit || entity.hp <= 0 {
                continue;
            }
            if matches!(entity.task, crate::Task::Garrisoned { .. }) {
                continue;
            }
            let tile = self.tile_of_raw(entity.position);
            units.push((entity.player, tile));
        }

        let mut changes: Vec<GateChange> = Vec::new();
        for entity in &self.entities {
            let Some(unit) = entity.unit else { continue };
            let root = self.units.as_ref().map_or(unit, |units| units.root(unit));
            if !self.gates.contains(&root) {
                continue;
            }
            let tiles = self.solid_tiles(entity);
            if tiles.is_empty() {
                continue;
            }
            let mut inside = false;
            let mut ally_near = false;
            for (player, (x, y)) in &units {
                let on_gate = tiles
                    .iter()
                    .any(|(tx, ty)| i64::from(*tx) == *x && i64::from(*ty) == *y);
                if on_gate {
                    // **Any** side, which is what keeps an enemy-held gate open.
                    inside = true;
                }
                if self.same_side(*player, entity.player) {
                    let near = tiles.iter().any(|(tx, ty)| {
                        (i64::from(*tx) - *x).abs() <= 1 && (i64::from(*ty) - *y).abs() <= 1
                    });
                    if near {
                        ally_near = true;
                    }
                }
            }
            let should_be_open = ally_near || inside;
            let is_open = self.open_gates.contains(&entity.id);
            if should_be_open != is_open {
                changes.push(GateChange {
                    id: entity.id,
                    tiles,
                    open: should_be_open,
                });
            }
        }

        for GateChange { id, tiles, open } in changes {
            if open {
                if let Some(walk) = self.walk.as_mut() {
                    for (x, y) in &tiles {
                        walk.unblock(*x, *y);
                    }
                }
                self.open_gates.insert(id);
            } else {
                if let Some(walk) = self.walk.as_mut() {
                    for (x, y) in &tiles {
                        walk.block(*x, *y);
                    }
                }
                self.open_gates.remove(&id);
            }
        }
    }

    /// Whether two players are on the same side: itself, or the same team when the match names teams.
    pub(crate) fn same_side(&self, a: u8, b: u8) -> bool {
        if a == b {
            return true;
        }
        if !self.teams_named {
            return false;
        }
        let index = |player: u8| usize::from(player).saturating_sub(1);
        match (
            self.team_by_player.get(index(a)),
            self.team_by_player.get(index(b)),
        ) {
            (Some(left), Some(right)) => left == right,
            _ => false,
        }
    }
}
