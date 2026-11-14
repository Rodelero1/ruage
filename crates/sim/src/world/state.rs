//! **A world's fingerprint**: the hash that must not move when nothing changed, and the stamp that feeds it (D145).
//!
//! Split out of `lib.rs` under D145, verbatim.
use crate::{
    EntityId, EntityKind, FNV_OFFSET_BASIS, MAX_PLAYERS, RallyPoint, Stance, Task, World, fnv_bytes,
};
impl World {
    /// Canonical hash over the state. Encoding is explicit little-endian: no `usize`, no
    /// platform-dependent layout, so the number means the same thing everywhere.
    pub fn state_hash(&self) -> u64 {
        let mut hash = FNV_OFFSET_BASIS;
        fnv_bytes(&mut hash, &self.tick.to_le_bytes());
        // What a player has watched changes what a rule may do, so it is state (D92). Iterated through `BTreeMap`s
        // for the same reason everything else is: a hash-order walk would make two identical matches hash apart.
        for (player, watching) in self.observers.iter().enumerate() {
            if *watching {
                fnv_bytes(&mut hash, b"observer");
                fnv_bytes(&mut hash, &[player as u8]);
            }
        }
        for (player, sightings) in self.memory.iter().enumerate() {
            for (unit, sighting) in sightings {
                fnv_bytes(&mut hash, b"sighting");
                fnv_bytes(&mut hash, &[player as u8]);
                fnv_bytes(&mut hash, &unit.to_le_bytes());
                fnv_bytes(&mut hash, &sighting.count.to_le_bytes());
                fnv_bytes(&mut hash, &sighting.at.to_le_bytes());
            }
        }
        // The civilisation is hashed because it changes what the world will allow: two runs that
        // differ in it refuse different techs, and a replay that lost it would diverge silently. Only
        // when set, so a world that names no civ hashes exactly as it did before civs existed.
        // What a player has ever built changes what the world will allow, so it is state (D63).
        for (id, queue) in &self.build_queue {
            fnv_bytes(&mut hash, b"build-queue");
            fnv_bytes(&mut hash, &id.0.to_le_bytes());
            for (unit, at) in queue {
                fnv_bytes(&mut hash, &unit.to_le_bytes());
                fnv_bytes(
                    &mut hash,
                    &[at.0 as u8, (at.0 >> 8) as u8, at.1 as u8, (at.1 >> 8) as u8],
                );
            }
        }
        for (player, family) in &self.built {
            fnv_bytes(&mut hash, &[*player]);
            fnv_bytes(&mut hash, &family.to_le_bytes());
        }
        for (id, rally) in &self.rallies {
            fnv_bytes(&mut hash, b"rally");
            fnv_bytes(&mut hash, &id.0.to_le_bytes());
            match rally {
                RallyPoint::Inside => fnv_bytes(&mut hash, &[1u8]),
                RallyPoint::Tile(x, y) => {
                    fnv_bytes(&mut hash, &[2u8]);
                    fnv_bytes(&mut hash, &x.to_le_bytes());
                    fnv_bytes(&mut hash, &y.to_le_bytes());
                }
            }
        }
        for (id, other) in &self.patrols {
            fnv_bytes(&mut hash, b"patrol");
            fnv_bytes(&mut hash, &id.0.to_le_bytes());
            fnv_bytes(&mut hash, &other.0.raw().to_le_bytes());
            fnv_bytes(&mut hash, &other.1.raw().to_le_bytes());
        }
        for (villager, animal) in &self.slaughter {
            fnv_bytes(&mut hash, b"slaughter");
            fnv_bytes(&mut hash, &villager.0.to_le_bytes());
            fnv_bytes(&mut hash, &animal.0.to_le_bytes());
        }
        for (id, leash) in &self.leash {
            fnv_bytes(&mut hash, b"leash");
            fnv_bytes(&mut hash, &id.0.to_le_bytes());
            fnv_bytes(&mut hash, &leash.home.0.raw().to_le_bytes());
            fnv_bytes(&mut hash, &leash.home.1.raw().to_le_bytes());
            fnv_bytes(&mut hash, &leash.until.to_le_bytes());
            fnv_bytes(&mut hash, &[u8::from(leash.chasing)]);
        }
        for (id, (stance, post)) in &self.stances {
            fnv_bytes(&mut hash, b"stance");
            fnv_bytes(&mut hash, &id.0.to_le_bytes());
            fnv_bytes(
                &mut hash,
                &[match stance {
                    Stance::Aggressive => 0u8,
                    Stance::Defensive => 1,
                    Stance::StandGround => 2,
                    Stance::NoAttack => 3,
                }],
            );
            fnv_bytes(&mut hash, &post.0.raw().to_le_bytes());
            fnv_bytes(&mut hash, &post.1.raw().to_le_bytes());
        }
        for (id, task) in &self.resuming {
            fnv_bytes(&mut hash, b"resume");
            fnv_bytes(&mut hash, &id.0.to_le_bytes());
            if let Task::Hunt { to } = task {
                fnv_bytes(&mut hash, &to.0.raw().to_le_bytes());
                fnv_bytes(&mut hash, &to.1.raw().to_le_bytes());
            }
        }
        // **Husks are state**: when each empty farm began its while decides when it leaves the map, so two runs that do not
        // agree on it are two different matches - and a save that drops it keeps the husk for ever (D122).
        // **A chopped remainder is state**: it decides which tick the next hit point comes off, so two runs that disagree
        // about it fall the same tree on different ticks (D129).
        for (id, carry) in &self.fell_carry {
            fnv_bytes(&mut hash, b"fell");
            fnv_bytes(&mut hash, &id.0.to_le_bytes());
            fnv_bytes(&mut hash, &carry.to_le_bytes());
        }
        for (id, since) in &self.husks {
            fnv_bytes(&mut hash, b"husk");
            fnv_bytes(&mut hash, &id.0.to_le_bytes());
            fnv_bytes(&mut hash, &since.to_le_bytes());
        }
        for (id, waiting) in &self.queued {
            fnv_bytes(&mut hash, b"queued");
            fnv_bytes(&mut hash, &id.0.to_le_bytes());
            fnv_bytes(
                &mut hash,
                &u32::try_from(waiting.len())
                    .unwrap_or(u32::MAX)
                    .to_le_bytes(),
            );
            for job in waiting {
                // **A job is either a unit or a tech** (D157), and the tag goes into the hash: a queue holding unit 83 must not
                // hash the same as one holding tech 83.
                match job {
                    crate::Job::Train(training) => {
                        fnv_bytes(&mut hash, b"unit");
                        fnv_bytes(&mut hash, &training.unit.to_le_bytes());
                        fnv_bytes(&mut hash, &training.remaining_ticks.to_le_bytes());
                    }
                    crate::Job::Research { tech } => {
                        fnv_bytes(&mut hash, b"tech");
                        fnv_bytes(&mut hash, &tech.to_le_bytes());
                    }
                }
            }
        }
        // A stopped queue is state, not an event: two runs that disagree about *why* production is halted have
        // diverged, and a replay that dropped the reason would draw a different panel from the same commands.
        for (id, reason) in &self.training_pauses {
            fnv_bytes(&mut hash, b"paused");
            fnv_bytes(&mut hash, &id.0.to_le_bytes());
            fnv_bytes(&mut hash, &[*reason as u8]);
        }
        if self.teams_named {
            fnv_bytes(&mut hash, b"teams");
            fnv_bytes(&mut hash, &self.team_by_player);
        }
        // Who is in the match, hashed only when somebody is: a world built by hand for a movement
        // test has no players and must hash exactly as it did before rosters existed.
        // Fog is state: a player who has seen a tile knows it forever, so two runs that disagree
        // about the map they have uncovered have diverged. Hashed only when something has been seen,
        // so a world built by hand for a movement test hashes exactly as before.
        for (index, grid) in self.explored.iter().enumerate() {
            if grid.iter().all(|word| *word == 0) {
                continue;
            }
            fnv_bytes(&mut hash, b"explored");
            fnv_bytes(&mut hash, &[u8::try_from(index).unwrap_or(u8::MAX)]);
            for word in grid {
                fnv_bytes(&mut hash, &word.to_le_bytes());
            }
        }
        if self.roster.iter().any(|present| *present) {
            fnv_bytes(&mut hash, b"roster");
            for present in self.roster {
                fnv_bytes(&mut hash, &[u8::from(present)]);
            }
        }
        for (index, civ) in self.civ_by_player.iter().enumerate() {
            if let Some(civ) = civ {
                fnv_bytes(&mut hash, b"civ");
                fnv_bytes(&mut hash, &[u8::try_from(index).unwrap_or(u8::MAX)]);
                fnv_bytes(&mut hash, &civ.to_le_bytes());
            }
        }
        fnv_bytes(&mut hash, &(self.entities.len() as u32).to_le_bytes());
        for row in &self.stock {
            for value in row {
                fnv_bytes(&mut hash, &value.to_le_bytes());
            }
        }
        for (player, techs) in &self.researched {
            fnv_bytes(&mut hash, &[*player]);
            fnv_bytes(&mut hash, &(techs.len() as u32).to_le_bytes());
            for tech in techs {
                fnv_bytes(&mut hash, &tech.to_le_bytes());
            }
        }
        for (id, route) in &self.paths {
            fnv_bytes(&mut hash, &id.0.to_le_bytes());
            fnv_bytes(&mut hash, &(route.points.len() as u32).to_le_bytes());
            for (x, y) in &route.points {
                fnv_bytes(&mut hash, &x.raw().to_le_bytes());
                fnv_bytes(&mut hash, &y.raw().to_le_bytes());
            }
            // A route's stall count decides **whether it may be re-planned again**, so it is state: two runs that disagree
            // about it diverge the moment an obstruction appears (D127). Written only when non-zero, so a world that never
            // stalls hashes as it did before this existed.
            if route.stalls != 0 {
                fnv_bytes(&mut hash, b"stalls");
                fnv_bytes(&mut hash, &[route.stalls]);
            }
            // The bodies a route is planned around decide where the unit goes next, so they are state. Written only
            // when non-empty, so a world that never collides hashes exactly as one written before D98a.
            if !route.avoiding.is_empty() {
                fnv_bytes(&mut hash, b"route");
                fnv_bytes(&mut hash, &(route.avoiding.len() as u32).to_le_bytes());
                for other in &route.avoiding {
                    fnv_bytes(&mut hash, &other.0.to_le_bytes());
                }
            }
        }
        for entity in &self.entities {
            fnv_bytes(&mut hash, &entity.id.0.to_le_bytes());
            fnv_bytes(&mut hash, &entity.player.to_le_bytes());
            let (tag, resource) = match entity.kind {
                EntityKind::Projectile => (b'p', 0u8),
                EntityKind::Building => (b'b', 0u8),
                EntityKind::Unit => (b'u', 0),
                // The column, not a kind: a pack that renumbers its resources renumbers this tag
                // too, which is what "the state changed" means for a node.
                EntityKind::Resource(slot) => (b'r', u8::try_from(slot).unwrap_or(u8::MAX)),
            };
            fnv_bytes(&mut hash, &[tag, resource]);
            fnv_bytes(&mut hash, &entity.amount.raw().to_le_bytes());
            fnv_bytes(&mut hash, &entity.carry.raw().to_le_bytes());
            fnv_bytes(&mut hash, &entity.speed.raw().to_le_bytes());
            match entity.researching {
                Some(job) => {
                    fnv_bytes(&mut hash, &[1u8]);
                    fnv_bytes(&mut hash, &job.tech.to_le_bytes());
                    fnv_bytes(&mut hash, &job.remaining_ticks.to_le_bytes());
                    fnv_bytes(&mut hash, &[u8::from(job.fresh)]);
                }
                None => fnv_bytes(&mut hash, &[0u8]),
            }
            match entity.training {
                Some(job) => {
                    fnv_bytes(&mut hash, &[1u8]);
                    fnv_bytes(&mut hash, &job.unit.to_le_bytes());
                    fnv_bytes(&mut hash, &job.remaining_ticks.to_le_bytes());
                    fnv_bytes(&mut hash, &job.hp.to_le_bytes());
                    fnv_bytes(&mut hash, &job.speed.raw().to_le_bytes());
                    fnv_bytes(&mut hash, &[u8::from(job.fresh)]);
                }
                None => fnv_bytes(&mut hash, &[0u8]),
            }
            match entity.construction {
                Some(construction) => {
                    fnv_bytes(&mut hash, &[1u8]);
                    fnv_bytes(&mut hash, &construction.progress_ticks.to_le_bytes());
                    fnv_bytes(&mut hash, &construction.total_ticks.to_le_bytes());
                    fnv_bytes(&mut hash, &construction.max_hp.to_le_bytes());
                }
                None => fnv_bytes(&mut hash, &[0u8]),
            }
            match entity.task {
                Task::Idle => fnv_bytes(&mut hash, &[0u8]),
                Task::Move { to } => {
                    fnv_bytes(&mut hash, &[7u8]);
                    fnv_bytes(&mut hash, &to.0.raw().to_le_bytes());
                    fnv_bytes(&mut hash, &to.1.raw().to_le_bytes());
                }
                Task::Hunt { to } => {
                    // Its own tag, not `Move`'s: two units walking to the same tile in the same
                    // position are doing the same thing only if one of them is not looking for a fight.
                    fnv_bytes(&mut hash, &[8u8]);
                    fnv_bytes(&mut hash, &to.0.raw().to_le_bytes());
                    fnv_bytes(&mut hash, &to.1.raw().to_le_bytes());
                }
                Task::Garrisoned { building } => {
                    fnv_bytes(&mut hash, &[6u8]);
                    fnv_bytes(&mut hash, &building.0.to_le_bytes());
                }
                Task::Fly {
                    target,
                    shooter,
                    damage,
                    aim,
                    blast,
                } => {
                    fnv_bytes(&mut hash, &[5u8]);
                    fnv_bytes(&mut hash, &target.0.to_le_bytes());
                    fnv_bytes(&mut hash, &shooter.0.to_le_bytes());
                    fnv_bytes(&mut hash, &damage.to_le_bytes());
                    fnv_bytes(&mut hash, &aim.0.raw().to_le_bytes());
                    fnv_bytes(&mut hash, &aim.1.raw().to_le_bytes());
                    fnv_bytes(&mut hash, &blast.raw().to_le_bytes());
                }
                Task::Attack {
                    target,
                    damage,
                    reload_ticks,
                    cooldown_ticks,
                    range,
                    min_range,
                } => {
                    fnv_bytes(&mut hash, &[4u8]);
                    fnv_bytes(&mut hash, &target.0.to_le_bytes());
                    fnv_bytes(&mut hash, &damage.to_le_bytes());
                    fnv_bytes(&mut hash, &reload_ticks.to_le_bytes());
                    fnv_bytes(&mut hash, &cooldown_ticks.to_le_bytes());
                    fnv_bytes(&mut hash, &range.raw().to_le_bytes());
                    fnv_bytes(&mut hash, &min_range.raw().to_le_bytes());
                }
                Task::Build { site } => {
                    fnv_bytes(&mut hash, &[3u8]);
                    fnv_bytes(&mut hash, &site.0.to_le_bytes());
                }
                Task::Return {
                    node,
                    drop,
                    slot,
                    rate_per_tick,
                    carry_cap,
                } => {
                    fnv_bytes(&mut hash, &[2u8]);
                    fnv_bytes(&mut hash, &node.0.to_le_bytes());
                    fnv_bytes(&mut hash, &drop.0.to_le_bytes());
                    fnv_bytes(&mut hash, &[u8::try_from(slot).unwrap_or(u8::MAX)]);
                    fnv_bytes(&mut hash, &rate_per_tick.raw().to_le_bytes());
                    fnv_bytes(&mut hash, &carry_cap.to_le_bytes());
                }
                Task::Gather {
                    node,
                    rate_per_tick,
                    carry_cap,
                    progress,
                } => {
                    fnv_bytes(&mut hash, &[1u8]);
                    fnv_bytes(&mut hash, &node.0.to_le_bytes());
                    fnv_bytes(&mut hash, &rate_per_tick.raw().to_le_bytes());
                    fnv_bytes(&mut hash, &carry_cap.to_le_bytes());
                    fnv_bytes(&mut hash, &progress.raw().to_le_bytes());
                }
            }
            match entity.variant {
                Some(variant) => {
                    fnv_bytes(&mut hash, &[1u8]);
                    fnv_bytes(&mut hash, &variant.to_le_bytes());
                }
                None => fnv_bytes(&mut hash, &[0u8]),
            }
            match entity.unit {
                Some(unit) => {
                    fnv_bytes(&mut hash, &[1u8]);
                    fnv_bytes(&mut hash, &unit.to_le_bytes());
                }
                None => fnv_bytes(&mut hash, &[0u8]),
            }
            fnv_bytes(&mut hash, &entity.position.0.raw().to_le_bytes());
            fnv_bytes(&mut hash, &entity.position.1.raw().to_le_bytes());
            fnv_bytes(&mut hash, &entity.velocity.0.raw().to_le_bytes());
            fnv_bytes(&mut hash, &entity.velocity.1.raw().to_le_bytes());
            // **A pause is state** (D205): it decides which tick a man may move again, so two runs that disagree about it
            // walk different trajectories from the next tick. Written only when non-zero, so a world where nobody has
            // collided yet hashes exactly as it did before pauses existed.
            if entity.pause_remaining != 0 {
                fnv_bytes(&mut hash, b"pause");
                fnv_bytes(&mut hash, &entity.pause_remaining.to_le_bytes());
            }
            fnv_bytes(&mut hash, &entity.hp.to_le_bytes());
        }
        hash
    }

    /// Mark what one entity can see into its player's explored grid.
    ///
    /// Called when the entity arrives in the world and again on a tick when it actually moves - never
    /// in a pass over the whole list. Fog is monotonic (a tile, once seen, stays seen), so an entity
    /// whose position did not change has nothing to add: the first version of this stamped every disc
    /// every tick and cost 19x the whole tick at 2,126 units (D50a). A stamp is now 50 ns on a unit
    /// that moved and zero on one that did not.
    pub(crate) fn stamp_entity(&mut self, id: EntityId) {
        let (width, height) = self.map_size;
        if width == 0 || height == 0 {
            return;
        }
        let Some(entity) = self.find(id) else { return };
        if entity.player == 0 || usize::from(entity.player) > MAX_PLAYERS {
            return;
        }
        if matches!(
            entity.kind,
            EntityKind::Projectile | EntityKind::Resource(_)
        ) {
            return;
        }
        // Inside a building a unit is not on the map and does not see out of it; the building's own
        // sight already covers what it can watch.
        if matches!(entity.task, Task::Garrisoned { .. }) {
            return;
        }
        let radius = self.sight_of(entity);
        if radius <= 0 {
            return;
        }
        let (x, y) = self.tile_of_raw(entity.position);
        let row = &mut self.explored[usize::from(entity.player) - 1];
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
                if let Some(word) = row.get_mut(bit / 64) {
                    *word |= 1u64 << (bit % 64);
                }
            }
        }
    }

    /// Stamp every entity that is already in the world. Seeding and tests that hand-place entities use
    /// this; the tick never does, because it stamps the entities that moved as it moves them.
    pub(crate) fn stamp_all(&mut self) {
        let ids: Vec<EntityId> = self
            .entities
            .iter()
            .filter(|entity| entity.player > 0)
            .map(|entity| entity.id)
            .collect();
        for id in ids {
            self.stamp_entity(id);
        }
    }
}
