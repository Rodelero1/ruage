//! **What a player knows**: sightings, the vision mask, and the ground a scout has already looked at (D92).
//!
//! Moved out of `lib.rs` unchanged in D136. The anti-cheat is the point of the file: a rule may ask what its player has
//! *seen*, never what is on the map, so everything here is written into per-player memory rather than read from the world.

use std::collections::BTreeMap;

use crate::{Entity, EntityId, EntityKind, MAX_PLAYERS, Sighting, Task, TileVision, World};

impl World {
    /// What `player` last observed of `unit`: the count that was in sight and the tick that sighting happened.
    /// `None` means never seen - which is the whole point of a memory rather than a view (D92).
    pub fn sighting(&self, player: u8, unit: u32) -> Option<(u16, u64)> {
        if player == 0 || usize::from(player) > MAX_PLAYERS {
            return None;
        }
        self.memory[usize::from(player) - 1]
            .get(&unit)
            .map(|sighting| (sighting.count, sighting.at))
    }

    /// Take this tick's sightings: for every player, count the enemy units that player's own units and buildings can
    /// watch right now, and stamp each count with the tick.
    ///
    /// One pass over the entities with a per-player visibility mask, not a `vision` call per entity: `vision` scans
    /// every entity to answer for one tile, so asking it per entity is quadratic and was the shape that cost 37x in the
    /// share pass. The mask is the same one the renderer builds, and it is built once per player here.
    ///
    /// **Only sightings are written.** A unit that walks out of sight keeps the count and stamp it had, so a rule can
    /// ask "seen within thirty seconds" and get an honest answer; the alternative - clearing when nothing is in sight -
    /// would make the memory a view with extra steps.
    /// Switch the memory on for a player. Called by whatever installs behaviour - a scenario's rules - so a world
    /// nobody has given an AI does not pay for one (the same shape as `apply_policy`, D75/D94).
    pub fn observe_for(&mut self, player: u8) {
        if player >= 1 && usize::from(player) <= MAX_PLAYERS {
            self.observers[usize::from(player) - 1] = true;
        }
    }

    /// Whether this player keeps a memory at all.
    pub fn observes(&self, player: u8) -> bool {
        player >= 1 && usize::from(player) <= MAX_PLAYERS && self.observers[usize::from(player) - 1]
    }

    pub(crate) fn observe(&mut self) {
        // Nothing to record unless somebody is watching, and no rule needs an answer fresher than once a game second:
        // building the sight masks costs O(entities x sight area), which is the only part of this that is not free.
        if !self.observers.iter().any(|watching| *watching) {
            return;
        }
        if !self.tick.is_multiple_of(u64::from(self.tick_rate_hz)) {
            return;
        }
        // A player can only learn about somebody else, so a world where fewer than two players hold anything has
        // nothing to record - and finding that out costs one pass over the entities and no masks at all. Most tests,
        // and every single-player scenario, take this branch.
        let mut active = [false; MAX_PLAYERS];
        let mut holders = 0;
        for entity in &self.entities {
            let player = usize::from(entity.player);
            if entity.hp > 0 && (1..=MAX_PLAYERS).contains(&player) && !active[player - 1] {
                active[player - 1] = true;
                holders += 1;
            }
        }
        if holders < 2 {
            return;
        }
        let masks: Vec<(u8, Vec<u64>)> = (1..=MAX_PLAYERS as u8)
            .filter(|player| {
                active[usize::from(*player) - 1] && self.observers[usize::from(*player) - 1]
            })
            .map(|player| (player, self.visible_mask(player)))
            .collect();
        if masks.is_empty() {
            return;
        }
        let width = usize::from(self.map_size.0);
        let tick = self.tick;
        let mut seen: Vec<BTreeMap<u32, u16>> = (0..MAX_PLAYERS).map(|_| BTreeMap::new()).collect();
        for entity in &self.entities {
            if entity.kind != EntityKind::Unit || entity.hp <= 0 || entity.player == 0 {
                continue;
            }
            let Some(unit) = entity.unit else { continue };
            let tile = self.tile_of(entity.position);
            let index = usize::from(tile.1) * width + usize::from(tile.0);
            let (word, bit) = (index / 64, 1u64 << (index % 64));
            for (player, mask) in &masks {
                if *player == entity.player {
                    continue;
                }
                if mask.get(word).is_some_and(|w| w & bit != 0) {
                    *seen[usize::from(*player) - 1].entry(unit).or_insert(0) += 1;
                }
            }
        }
        for (player, sightings) in seen.iter().enumerate() {
            for (unit, count) in sightings {
                self.memory[player].insert(
                    *unit,
                    Sighting {
                        count: *count,
                        at: tick,
                    },
                );
            }
        }
    }

    /// What this player knows about a tile: never seen, remembered, or watched right now.
    pub fn vision(&self, player: u8, tile: (u16, u16)) -> TileVision {
        if player == 0 || usize::from(player) > MAX_PLAYERS {
            return TileVision::Unseen;
        }
        if !self.explored_bit(player, tile) {
            return TileVision::Unseen;
        }
        if self
            .entities
            .iter()
            .any(|entity| entity.player == player && self.covers(entity, tile))
        {
            TileVision::Visible
        } else {
            TileVision::Explored
        }
    }

    /// **The nearest tile this player has never seen**, which is the only question a scout asks. Bounded: a scout that
    /// crossed an already-explored half of the map would otherwise be sent to the far corner on every order, so the search
    /// is a square of `limit` tiles around the unit and the nearest unseen tile in it wins. `explored_bit` is a bitset
    /// read - no entity scan, unlike `vision`, which asks the same question one tile at a time.
    #[must_use]
    pub fn nearest_unseen(&self, player: u8, from: (u16, u16), limit: u16) -> Option<(u16, u16)> {
        let (width, height) = self.map_size;
        let (x, y) = (i32::from(from.0), i32::from(from.1));
        let reach = i32::from(limit);
        let mut best: Option<(i64, (u16, u16))> = None;
        for ty in (y - reach).max(0)..=(y + reach).min(i32::from(height) - 1) {
            for tx in (x - reach).max(0)..=(x + reach).min(i32::from(width) - 1) {
                let (Ok(tx16), Ok(ty16)) = (u16::try_from(tx), u16::try_from(ty)) else {
                    continue;
                };
                if self.explored_bit(player, (tx16, ty16)) {
                    continue;
                }
                // No walkability test here: a scout may be sent at a tile it cannot stand on, and the order is refused at
                // the pathfinder with the engine's own sentence, which is a better answer than a silent detour.
                let dx = i64::from(tx) - i64::from(x);
                let dy = i64::from(ty) - i64::from(y);
                let key = dx * dx + dy * dy;
                if best
                    .is_none_or(|(best_key, best_tile)| (key, (tx16, ty16)) < (best_key, best_tile))
                {
                    best = Some((key, (tx16, ty16)));
                }
            }
        }
        best.map(|(_, tile)| tile)
    }

    /// The tiles this player has **ever** seen, as one bit per tile in row-major order - the grid the fog keeps, handed
    /// out so a renderer does not have to ask `vision` per tile (which scans every entity) or per entity (which scans
    /// every tile). The simulation already maintains it; this is only the door.
    pub fn explored_mask(&self, player: u8) -> Vec<u64> {
        let (width, height) = self.map_size;
        let mut mask = vec![0u64; (usize::from(width) * usize::from(height)).div_ceil(64)];
        if player == 0 || usize::from(player) > MAX_PLAYERS {
            return mask;
        }
        if let Some(explored) = self.explored.get(usize::from(player) - 1) {
            mask[..explored.len()].copy_from_slice(explored);
        }
        mask
    }

    /// Whether this player can see that entity: their own always, anybody else's only while it stands
    /// on ground an entity of theirs is watching.
    pub fn visible_to(&self, player: u8, entity: EntityId) -> bool {
        let Some(entity) = self.find(entity) else {
            return false;
        };
        if entity.player == player {
            return true;
        }
        let tile = self.tile_of(entity.position);
        self.vision(player, tile) == TileVision::Visible
    }

    /// The tiles this player sees *right now*, as one bit per tile in row-major order. Built from the
    /// union of the sight discs rather than by asking `vision` per tile, which is what a renderer
    /// needs: one pass per frame, not one pass per tile.
    pub fn visible_mask(&self, player: u8) -> Vec<u64> {
        let (width, height) = self.map_size;
        let mut mask = vec![0u64; (usize::from(width) * usize::from(height)).div_ceil(64)];
        if width == 0 || player == 0 || usize::from(player) > MAX_PLAYERS {
            return mask;
        }
        for entity in self.entities.iter().filter(|e| e.player == player) {
            let radius = self.sight_of(entity);
            if radius <= 0 {
                continue;
            }
            let (x, y) = self.tile_of_raw(entity.position);
            for dy in -radius..=radius {
                let ty = y + dy;
                if ty < 0 || ty >= i64::from(height) {
                    continue;
                }
                for dx in -radius..=radius {
                    if dx * dx + dy * dy > radius * radius {
                        continue;
                    }
                    let tx = x + dx;
                    if tx < 0 || tx >= i64::from(width) {
                        continue;
                    }
                    let bit = usize::try_from(ty * i64::from(width) + tx).unwrap_or(0);
                    if let Some(word) = mask.get_mut(bit / 64) {
                        *word |= 1u64 << (bit % 64);
                    }
                }
            }
        }
        mask
    }

    /// The sight radius of an entity in whole tiles: its own unit's `los`, rounded up. Zero for an
    /// entity with no unit (a handmade mover) or one whose table entry has no sight.
    pub(crate) fn sight_of(&self, entity: &Entity) -> i64 {
        let Some(unit) = entity.unit else { return 0 };
        let Some(units) = self.units.as_ref() else {
            return 0;
        };
        let Some(record) = units.unit(unit) else {
            return 0;
        };
        let Ok(los) = record.los.resolve(self.scale, "los") else {
            return 0;
        };
        let raw = los.raw();
        if raw <= 0 {
            return 0;
        }
        // `div_ceil` is unsigned-only on stable, and the raw value is positive by the guard above.
        (raw + self.scale - 1) / self.scale
    }

    /// Whether this entity's sight covers `tile`.
    fn covers(&self, entity: &Entity, tile: (u16, u16)) -> bool {
        if entity.player == 0
            || matches!(
                entity.kind,
                EntityKind::Projectile | EntityKind::Resource(_)
            )
            || matches!(entity.task, Task::Garrisoned { .. })
        {
            return false;
        }
        let radius = self.sight_of(entity);
        if radius <= 0 {
            return false;
        }
        let (x, y) = self.tile_of_raw(entity.position);
        let dx = x - i64::from(tile.0);
        let dy = y - i64::from(tile.1);
        dx * dx + dy * dy <= radius * radius
    }
    fn explored_bit(&self, player: u8, tile: (u16, u16)) -> bool {
        let (width, height) = self.map_size;
        if tile.0 >= width || tile.1 >= height {
            return false;
        }
        let bit = usize::from(tile.1) * usize::from(width) + usize::from(tile.0);
        self.explored[usize::from(player) - 1]
            .get(bit / 64)
            .is_some_and(|word| word & (1u64 << (bit % 64)) != 0)
    }
}
