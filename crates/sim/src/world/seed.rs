//! **Building the opening position**: the map and the pack become a `World` (D6), and the one door every new entity
//! comes through.
//!
//! Split out of `lib.rs` under D144, verbatim: a pure move, proved by the acceptance hash.

use std::collections::BTreeSet;

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;

use crate::{
    Entity, EntityId, EntityKind, MAX_PLAYERS, SimError, Task, Walk, World, free_neighbour,
};

impl World {
    /// Build the opening position: one town centre per player, `villagers_per_player` villagers
    /// around it, stats resolved from the unit table.
    ///
    /// Everything here is decided by the map and the config — no literal ids, no default
    /// speeds (D6). Tiles are placed at their origin, so a tile at (x, y) is `x * scale`.
    pub fn seed_from(
        &mut self,
        map: &Map,
        economy: &Economy,
        units: &Units,
    ) -> Result<(), SimError> {
        // **Refuse a pack whose geometry cannot be squared in i64** (D128). Every range and reach test in the tick is
        // `dx * dx + dy * dy` on raw positions; at the shipped scale of 20,000 on a 64-tile map that is 3.2e12 against
        // i64's 9.2e18, but a fine scale on a big map would overflow - debug panics, release computes a wrong distance and
        // paths, shoots and collides on it. A load-time refusal turns that from a match-time mystery into a sentence.
        let furthest = i64::from(map.width.max(map.height).saturating_sub(1)) * self.scale;
        if furthest
            .checked_mul(furthest)
            .and_then(|dist| dist.checked_mul(2))
            .is_none()
        {
            return Err(SimError::GeometryOverflow {
                scale: self.scale,
                tiles: u32::from(map.width.max(map.height)),
            });
        }
        self.units = Some(units.clone());
        self.economy = Some(economy.clone());
        // **How long a husk stays** (D122): content's seconds, in ticks. A pack with no `husk_seconds` (or no farms) keeps
        // husks for ever, which is the behaviour this had before husks existed.
        self.husk_ticks = u64::from(economy.farm.husk_seconds) * u64::from(self.tick_rate_hz);
        // Which building classes keep a player alive (D59b). The manual's list of things you need not
        // destroy is a list of *classes* - towers 52, walls 27, gates 39, farms 49, relics 1 - so the
        // whole rule is "class 3 counts", with no per-building list to keep in step and nothing to
        // forget when the pack gains a mill.
        // Which unit families are animals, so a gather order can be pointed at one (D70). By family, so a
        // carcass or an upgraded variant answers the same way.
        self.animals.clear();
        for role in &economy.hunt.animal_roles {
            if let Some(unit) = economy.role.get(role).copied() {
                // The id as well as the family: the tick asks "is this a living animal" per worker per tick and
                // must not walk the unit table to answer it (item 3).
                self.animals.insert(unit);
                self.animals.insert(units.root(unit));
            }
        }
        // Animals that can fight back. A sheep cannot, so being struck is not a refusal for it - it is simply an
        // animal that has no quarrel (item 3).
        self.armed_animals.clear();
        for role in &economy.hunt.animal_roles {
            if let Some(unit) = economy.role.get(role).copied()
                && units
                    .unit(unit)
                    .is_some_and(|stats| !stats.attacks.is_empty())
            {
                self.armed_animals.insert(unit);
                self.armed_animals.insert(units.root(unit));
            }
        }
        self.herdable.clear();
        for role in &economy.hunt.herdable_roles {
            if let Some(unit) = economy.role.get(role).copied() {
                // The id as well as the family: the tick asks "is this livestock" on the damage path and on the
                // conversion pass, and neither may walk the unit table (items 3 and 5).
                self.herdable.insert(unit);
                self.herdable.insert(units.root(unit));
            }
        }
        self.carcasses.clear();
        for role in &economy.hunt.carcass_roles {
            if let Some(unit) = economy.role.get(role).copied() {
                // Both the id and its family: the sweep runs per entity per tick and must not consult the
                // unit table to answer "is this a body" (D70).
                self.carcasses.insert(unit);
                self.carcasses.insert(units.root(unit));
            }
        }
        // What death leaves: each animal's `dead_unit`, and which roles make meat with their last blow. Both are
        // resolved here so the tick reads two maps and no table (item 2).
        self.animal_bodies.clear();
        for role in economy
            .hunt
            .animal_roles
            .iter()
            .chain(&economy.hunt.herdable_roles)
        {
            if let Some(unit) = economy.role.get(role).copied() {
                let body = units.unit(unit).and_then(|stats| stats.dead_unit);
                // A body the pack cannot name is a lookup that finds nothing, not an error: 640 records name a
                // body no civ has (D69). Such an animal is simply swept away as it was before.
                if let Some(body) = body.filter(|body| units.unit(*body).is_some()) {
                    self.animal_bodies.insert(unit, body);
                    self.animal_bodies.insert(units.root(unit), body);
                }
                // How fast its body rots: `decay_rate` is food per second (boar 0.4, sheep and deer 0.25 - the pack
                // agrees with two published accounts), turned into raw units per tick once, here. A rate the pack
                // leaves unresolved yields no rot rather than an invented one.
                if let Some(per_tick) = units
                    .unit(unit)
                    .and_then(|stats| stats.decay_rate.resolve(self.scale, "decay_rate").ok())
                    .map(|per_second| per_second.raw() / i64::from(self.tick_rate_hz))
                    .filter(|per_tick| *per_tick > 0)
                {
                    self.animal_rot.insert(unit, per_tick);
                    self.animal_rot.insert(units.root(unit), per_tick);
                    // How long its body lies there: its own meat divided by its own rot rate. A boar is 340 food at
                    // 0.4 a second, so 850 s; a sheep 100 at 0.25, so 400 s. Derived, so a pack whose animals change
                    // changes the lifetime with them.
                    let meat =
                        i64::from(units.unit(unit).map_or(0, |stats| stats.food)) * self.scale;
                    let span = u64::try_from((meat / per_tick).max(0)).unwrap_or(0);
                    if span > 0 {
                        self.animal_span.insert(unit, span);
                        self.animal_span.insert(units.root(unit), span);
                    }
                }
            }
        }
        self.butchers.clear();
        for role in &economy.hunt.butcher_roles {
            if let Some(unit) = economy.role.get(role).copied() {
                self.butchers.insert(unit);
                self.butchers.insert(units.root(unit));
            }
        }
        // How far a unit sees, squared and resolved once (item 5). `los` is a pack column; squaring at seed keeps
        // the per-tick comparison to a distance test with no root and no table read.
        self.sight.clear();
        for unit in units.units.values() {
            if let Ok(los) = unit.los.resolve(self.scale, "los") {
                let reach = los.raw();
                self.sight.insert(unit.id, reach * reach);
            }
        }
        // Which herdables are already standing here (a scenario may place them), and the same question for a body
        // that content names later: both sets are rebuilt here rather than guessed.
        self.herdable_ids.clear();
        for entity in &self.entities {
            if entity.kind == EntityKind::Unit
                && entity.player == 0
                && entity
                    .unit
                    .is_some_and(|unit| self.herdable.contains(&unit))
            {
                self.herdable_ids.insert(entity.id);
            }
        }
        // The leash (D71): content's two numbers, resolved once into ticks and engine units, so the tick
        // loop never divides and never touches a float. `None` means content said nothing, which is not
        // zero - a zero-second interest would make every animal give up on the tick it was struck.
        self.leash_ticks = economy
            .hunt
            .leash_seconds
            .map(|seconds| u64::from(seconds) * u64::from(self.tick_rate_hz));
        self.leash_tiles = economy
            .hunt
            .leash_tiles
            .map(|tiles| i64::from(tiles) * self.scale);
        self.conquest_classes.clear();
        for class in &economy.conquest.surviving_classes {
            self.conquest_classes.insert(*class);
        }
        // Building prerequisites (D63), resolved once to families: role -> the family that must have been
        // built first. A pair whose roles the pack does not have is skipped, like every other content name.
        self.requires_built.clear();
        if let Some(units) = self.units.as_ref() {
            for (role, need_role) in &economy.build.requires {
                if let (Some(unit), Some(needed)) =
                    (economy.role.get(role), economy.role.get(need_role))
                {
                    self.requires_built
                        .insert(units.root(*unit), (units.root(*needed), need_role.clone()));
                }
            }
        }
        // Age requirements (D65), resolved once to families and keyed by tech id, because that is what a
        // research order carries.
        self.age_needs.clear();
        if let Some(units) = self.units.as_ref() {
            for (role, requirement) in &economy.age.requires {
                let Some(tech) = economy.tech_role.get(role) else {
                    continue;
                };
                let families: BTreeSet<u32> = requirement
                    .of
                    .iter()
                    .filter_map(|name| economy.role.get(name))
                    .map(|unit| units.root(*unit))
                    .collect();
                let singles: BTreeSet<u32> = requirement
                    .single
                    .iter()
                    .filter_map(|name| economy.role.get(name))
                    .map(|unit| units.root(*unit))
                    .collect();
                if requirement.count > 0 {
                    self.age_needs
                        .insert(*tech, (requirement.count, families, singles));
                }
            }
        }
        // Blast radii, resolved once. The *radius* is the gate: `blast_level` is carried and not
        // consumed, because what its value means is not published and every unit that has one has a
        // radius with it (D51).
        self.blast.clear();
        for (id, unit) in &units.units {
            let Ok(radius) = unit.blast_radius.resolve(self.scale, "blast_radius") else {
                continue;
            };
            if radius.raw() > 0 {
                self.blast.insert(*id, radius.raw());
            }
        }
        self.walk = Some(Walk::from_map(map));
        self.footprints.clear();
        for (role, mask) in &economy.footprint {
            if let Some(unit) = economy.role.get(role) {
                let root = units.root(*unit);
                self.footprints.insert(root, mask.clone());
            }
        }
        self.blocking.clear();
        for role in &economy.build.blocking_roles {
            let unit = economy
                .role
                .get(role)
                .copied()
                .ok_or_else(|| SimError::NoRole { role: role.clone() })?;
            self.blocking.insert(units.root(unit));
        }
        self.map_size = (map.width, map.height);
        let words = (usize::from(map.width) * usize::from(map.height)).div_ceil(64);
        self.explored = std::array::from_fn(|_| vec![0u64; words]);
        // The start names roles; the economy's role table is what turns them into units, so this
        // function never learns that a town centre is 109 (D17).
        let tc_stats = self.role_unit_in(economy, &economy.start.town_center)?;
        let villager_stats = self.role_unit_in(economy, &economy.start.villager)?;
        let town_center = units
            .unit(tc_stats)
            .ok_or(SimError::MissingUnit { id: tc_stats })?;
        let villager = units
            .unit(villager_stats)
            .ok_or(SimError::MissingUnit { id: villager_stats })?;

        let resolve = |unit: &ruage_data::Unit, id: u32| -> Result<Fixed, SimError> {
            unit.speed
                .resolve(self.scale, "speed")
                .map_err(|err| SimError::Unresolved {
                    id,
                    detail: err.to_string(),
                })
        };
        let tc_speed = resolve(town_center, tc_stats)?;
        let villager_speed = resolve(villager, villager_stats)?;
        // **A scout, if the pack opens with one** (the owner's note, 2026-11-04). Resolved here, beside the villager's,
        // because the seeding loop below inserts into `self` and may not hold a borrow of it.
        let scout = match economy.start.scout.as_deref() {
            Some(role) if economy.start.scouts_per_player > 0 => {
                let unit = economy
                    .role
                    .get(role)
                    .copied()
                    .ok_or_else(|| SimError::NoRole {
                        role: role.to_owned(),
                    })?;
                let stats = units.unit(unit).ok_or(SimError::MissingUnit { id: unit })?;
                Some((unit, resolve(stats, unit)?, stats.hp))
            }
            _ => None,
        };

        let mut centres = map.town_centers.clone();
        centres.sort_by_key(|centre| centre.player);

        // Node tiles are taken before anyone is placed, so a villager never starts inside a
        // mine or a berry bush. The nodes themselves are inserted after the players for ids.
        let mut used: std::collections::BTreeSet<(u16, u16)> =
            map.resources.iter().map(|node| (node.x, node.y)).collect();
        let mut next_id = 1u32;
        for centre in centres {
            let here = (centre.x, centre.y);
            used.insert(here);
            self.insert(Entity {
                id: EntityId(next_id),
                player: centre.player,
                kind: EntityKind::Building,
                unit: Some(tc_stats),
                position: self.tile(here),
                speed: tc_speed,
                velocity: (Fixed::ZERO, Fixed::ZERO),
                hp: town_center.hp,
                amount: Fixed::ZERO,
                task: Task::Idle,
                training: None,
                construction: None,
                researching: None,
                carry: Fixed::ZERO,
                variant: None,
            })?;
            next_id += 1;

            // **A scout, so the map gets looked at** (the owner's note, 2026-11-04). It is content's role, and its speed and
            // health are the unit's own: nothing here knows that a scout is a horseman.
            for _ in 0..economy.start.villagers_per_player {
                let spot = free_neighbour(map, here, &used).ok_or(SimError::NoSpace {
                    player: centre.player,
                })?;
                used.insert(spot);
                self.insert(Entity {
                    id: EntityId(next_id),
                    player: centre.player,
                    kind: EntityKind::Unit,
                    unit: Some(villager_stats),
                    position: self.tile(spot),
                    speed: villager_speed,
                    velocity: (Fixed::ZERO, Fixed::ZERO),
                    hp: villager.hp,
                    amount: Fixed::ZERO,
                    task: Task::Idle,
                    training: None,
                    construction: None,
                    researching: None,
                    carry: Fixed::ZERO,
                    variant: None,
                })?;
                next_id += 1;
            }

            if let Some((scout_unit, scout_speed, scout_hp)) = scout {
                for _ in 0..economy.start.scouts_per_player {
                    let spot = free_neighbour(map, here, &used).ok_or(SimError::NoSpace {
                        player: centre.player,
                    })?;
                    used.insert(spot);
                    self.insert(Entity {
                        id: EntityId(next_id),
                        player: centre.player,
                        kind: EntityKind::Unit,
                        unit: Some(scout_unit),
                        position: self.tile(spot),
                        speed: scout_speed,
                        velocity: (Fixed::ZERO, Fixed::ZERO),
                        hp: scout_hp,
                        amount: Fixed::ZERO,
                        task: Task::Idle,
                        training: None,
                        construction: None,
                        researching: None,
                        carry: Fixed::ZERO,
                        variant: None,
                    })?;
                    next_id += 1;
                }
            }
        }

        // Resource nodes are entities too: a villager targets one by id, and the amount has to
        // live in state because gathering spends it. Map order, not sorted, so two runs agree
        // for the same file.
        for node in &map.resources {
            self.insert(Entity {
                id: EntityId(next_id),
                player: 0,
                kind: EntityKind::Resource(node.slot),
                unit: None,
                position: self.tile((node.x, node.y)),
                speed: Fixed::ZERO,
                velocity: (Fixed::ZERO, Fixed::ZERO),
                // A tree has to be **felled** before it yields (the owner's rule, 2026-10-30): the dat gives it 20 hit
                // points (25, 30 for the variants) and a bush or a mine none.
                hp: node.hp,
                amount: Fixed::from_raw(i64::from(node.amount) * self.scale),
                task: Task::Idle,
                training: None,
                construction: None,
                researching: None,
                carry: Fixed::ZERO,
                variant: None,
            })?;
            next_id += 1;
        }
        // Teams: content may put players together, otherwise everyone is alone.
        let teams = economy.start.teams.clone();
        if !teams.is_empty() {
            self.teams_named = true;
            for index in 0..MAX_PLAYERS {
                let team = teams[index.min(teams.len() - 1)];
                self.team_by_player[index] = u8::try_from(team).unwrap_or(u8::MAX);
            }
        }

        // If the tables this needs are already loaded, the opening position belongs to these
        // entities; if they arrive later, the setters apply them then.
        self.apply_start_stock()?;
        self.grant_starting_techs()?;
        // The opening position is already watched: a player sees their own town centre from tick zero,
        // rather than from the first tick they happen to move something.
        self.stamp_all();
        Ok(())
    }

    pub(crate) fn insert(&mut self, entity: Entity) -> Result<(), SimError> {
        if self.find(entity.id).is_some() {
            return Err(SimError::DuplicateEntity(entity.id));
        }
        if self.entities.len() >= self.max_entities as usize {
            return Err(SimError::TooManyEntities {
                max: self.max_entities,
            });
        }
        // A player is in the match from the moment their first entity exists, and stays in it even
        // after losing everything: "the enemy is gone" and "nobody was ever here" are different
        // facts, and only one of them ends a match (D46).
        if entity.player > 0 && entity.player as usize <= MAX_PLAYERS {
            self.roster[usize::from(entity.player) - 1] = true;
        }
        // A building that arrives finished - seeded by the opening, or placed by a scenario - has been
        // "built once" as surely as one a villager raised, so prerequisites treat them the same (D63).
        if entity.kind == EntityKind::Building
            && entity.construction.is_none()
            && let Some(unit) = entity.unit
        {
            let family = self.units.as_ref().map_or(unit, |units| units.root(unit));
            self.built.insert((entity.player, family));
        }
        let at = self
            .entities
            .iter()
            .position(|existing| existing.id > entity.id)
            .unwrap_or(self.entities.len());
        let id = entity.id;
        // A wall closes its tile from the moment it stands there - the tile is claimed by the insertion,
        // not by the completion, because a half-built wall is not walk-through either. The check reads the
        // entity being inserted rather than looking it up, which is the bug this line had first: an entity
        // cannot be found before it is in the list.
        if entity.kind == EntityKind::Building {
            for tile in self.solid_tiles(&entity) {
                if let Some(walk) = self.walk.as_mut() {
                    walk.block(tile.0, tile.1);
                }
            }
        }
        // **Every node blocks the tile it stands on while it holds anything** - a tree, a berry bush, a gold or stone
        // mine (the owner's rule, 2026-11-03: "each forest tile is blocking passage until tree cut and all wood
        // extracted"). The first version keyed it on hit points, which only a tree has, so a bush or a mine was a floor a
        // villager stood on to work it; it also stopped blocking a felled tree that still had wood in it, which is the
        // half of the rule the owner named. `step_spent_nodes` frees the tile when the node is emptied and removed.
        if matches!(entity.kind, EntityKind::Resource(_)) && entity.amount.raw() > 0 {
            let tile = self.tile_of(entity.position);
            if let Some(walk) = self.walk.as_mut() {
                walk.block(tile.0, tile.1);
            }
        }
        self.entities.insert(at, entity);
        // A thing that has just arrived is watched from the tick it arrives: fog follows insertion as
        // well as movement, and this is the one place both paths can be fed from.
        self.stamp_entity(id);
        Ok(())
    }
}
