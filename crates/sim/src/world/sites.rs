//! **Where a building may stand**: footprints, lanes, approach tiles, and where a walk ends (D145).
//!
//! Split out of `lib.rs` under D145, verbatim.
use std::collections::{BTreeMap, BTreeSet};

use ruage_fixed::Fixed;

use crate::{Entity, EntityId, EntityKind, Route, SimError, Task, Walk, World};
impl World {
    /// The tile a world position sits on.
    /// What stops a site of `unit` at `at` being raised for `player`, if anything.
    ///
    /// **A building owns its whole footprint**, so an overlap is refused by the footprint and not by the anchor tile (the
    /// owner's rule, 2026-11-03: "building may not overlap in any case (only expired farm can be)"). A house on a town
    /// centre's yard is the case that used to pass every check, because the yard is walkable ground. A building the
    /// player cannot see does not hold its ground (D58): the placement is allowed and the cancellation-on-sight rule
    /// resolves it later. **Units do not block a site** - the game drops a building on your own villagers and pushes them
    /// out - and a spent farm is the one thing a new building may cover.
    pub(crate) fn site_refusal(&self, player: u8, unit: u32, at: (u16, u16)) -> Option<SimError> {
        let walk = self.walk.as_ref()?;
        let mut held: BTreeMap<(u16, u16), EntityId> = BTreeMap::new();
        // Ground held by a building this player **cannot see**. The walk layer has it blocked - `insert` blocks a
        // building's tiles the moment it stands - so asking `walk.blocked` alone refused a blind placement with
        // `SiteBlocked`, and the rule this function documents (D58: a hidden building does not hold its ground, the
        // overbuild is cancelled on sight) could never fire. Nine foundation tests said so and were red.
        let mut hidden: BTreeSet<(u16, u16)> = BTreeSet::new();
        for entity in &self.entities {
            if entity.kind != EntityKind::Building || self.is_expired_farm(entity.id) {
                continue;
            }
            // **Seen** is `visible_to`, not the fog alone: an enemy foundation nobody has worked on is invisible even in
            // plain sight (D183 §2), so it does not hold its ground and the other side may place over it. Two bare
            // foundations on one tile are then decided by whoever puts a builder to work first (D58, `cancel_overbuilt`).
            let seen = entity.player == 0 || self.visible_to(player, entity.id);
            if seen {
                for tile in self.footprint_area(entity) {
                    held.entry(tile).or_insert(entity.id);
                }
            } else {
                hidden.extend(self.footprint_area(entity));
            }
        }
        let site = self.build_site_tiles(unit, at);
        for tile in &site {
            let tile = *tile;
            if !walk.contains(tile.0, tile.1) {
                return Some(SimError::SiteBlocked { tile });
            }
            // **A foundation may go in the fog, never in the black** (D183 addendum, the owner's rule: *"building can be
            // placed only on the scouted area (can be in fog but never in black)"*). The bitset has existed since the fog
            // landed, so this is a question on the order rather than new state - and it is asked before the ground checks
            // because "you have never been there" outranks "something is in the way there".
            if !self.explored_bit(player, tile) {
                return Some(SimError::SiteUnscouted { tile });
            }
            if held.contains_key(&tile) {
                return Some(SimError::SiteOccupied { tile });
            }
            // Terrain and resources are what is left: a resource node is not a player's to lose, so it blocks whoever is
            // looking. A **building** the player cannot see is not terrain - its tiles are blocked in the walk layer, and
            // they are the one kind of blocked ground a blind site may cover.
            if walk.blocked(tile.0, tile.1) && !hidden.contains(&tile) {
                return Some(SimError::SiteBlocked { tile });
            }
        }
        // **Nobody else's unit may be built over** (D183 addendum: *"you also cannot place any building over tile with
        // neutral/enemy in"*). Our own and allied units are the ones the site tells to step aside, so they do not refuse
        // it. A unit this player cannot see does not refuse either: a refusal would tell the player something they cannot
        // know, so that placement is allowed and the site-clear gate keeps construction from starting until he moves.
        for entity in &self.entities {
            if entity.kind != EntityKind::Unit
                || entity.hp <= 0
                || entity.player == 0
                || self.same_side(entity.player, player)
            {
                continue;
            }
            let tile = self.tile_of(entity.position);
            if site.contains(&tile) && self.visible_to(player, entity.id) {
                return Some(SimError::SiteOccupied { tile });
            }
        }
        None
    }

    /// Whether a site of `unit` at `at` leaves a **lane**: no building this player can see stands on the ring of tiles
    /// around its footprint. Two buildings placed edge to edge make a wall, and a base of walls is a base nobody can walk
    /// out of - measured the first time buildings blocked: the housing rule packed every house against the town centre
    /// and the villagers could not reach a single node ("no walkable route ... the target is walled off").
    ///
    /// A player is free to butt buildings together; this is what the **placement search** asks, so an AI's base keeps
    /// streets without anybody modelling a street.
    #[must_use]
    pub fn site_has_lane(&self, player: u8, unit: u32, at: (u16, u16)) -> bool {
        // The lane is asked of **every** building, farms included: the game's own manual says so - *"you should build four
        // or five farms around a granary, leaving some space between them to avoid 'peasant traffic jam'"* (AoE manual,
        // farms) - and the version that exempted a farm because it blocks nothing produced a floor of plots packed edge to
        // edge that villagers could not queue around. What made the exemption look necessary was the farm search's
        // four-tile radius, not the lane.
        let site: BTreeSet<(u16, u16)> = self.build_site_tiles(unit, at).into_iter().collect();
        let halo: BTreeSet<(u16, u16)> = site
            .iter()
            .flat_map(|(x, y)| {
                let (x, y) = (*x as i32, *y as i32);
                let mut ring = Vec::with_capacity(8);
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        if let (Ok(x), Ok(y)) = (u16::try_from(x + dx), u16::try_from(y + dy)) {
                            ring.push((x, y));
                        }
                    }
                }
                ring
            })
            .filter(|tile| !site.contains(tile))
            .collect();
        !self.entities.iter().any(|entity| {
            // A farm is a **floor** (`[footprint.farm]` is all walkable), so it cannot be part of a wall. The exemption
            // is what lets a spent plot inside a field be rebuilt; it is asked of every farm, live or spent, because the
            // question is "does this footprint block anyone" and for a farm the answer is no.
            if entity.kind != EntityKind::Building
                || self.is_expired_farm(entity.id)
                || self.is_farm_role(entity)
            {
                return false;
            }
            // A foundation nobody has started is invisible (D183 §2), so it cannot be part of a wall either - the
            // placement search would otherwise refuse a plot for a lane that is not there yet.
            let seen = entity.player == 0 || self.visible_to(player, entity.id);
            seen && self
                .footprint_area(entity)
                .iter()
                .any(|tile| halo.contains(tile))
        })
    }

    /// Whether a building of `unit` would fit at `at` **as a whole footprint** - what a placement search has to ask, or it
    /// chooses a tile whose neighbour the building already owns and the order is refused (measured: the acceptance
    /// match's housing rule asked for the town centre's own ground 450 times).
    #[must_use]
    pub fn site_fits(&self, player: u8, unit: u32, at: (u16, u16)) -> bool {
        self.site_refusal(player, unit, at).is_none()
    }

    /// Every tile a building's **footprint** covers, by id: the ground it is drawn on and the ground its site-clear gate
    /// asks about (D183 §4).
    pub(crate) fn footprint_area_of(&self, id: EntityId) -> Vec<(u16, u16)> {
        self.find(id)
            .map(|entity| self.footprint_area(entity))
            .unwrap_or_default()
    }

    /// **A unit standing on a site steps aside** (D183 §1): it is given a walk to the nearest free tile outside the
    /// footprint, and what it was doing is remembered so it can go back to it when it arrives.
    ///
    /// This replaces the teleport `push_units_off` performed. Two things make it a walk rather than a placement: the route
    /// is planned, so distance is covered at the unit's own speed, and the ground is checked the way a walk checks it - the
    /// accident behind D184 (a body thrown into an unreachable spot) cannot happen, because a route to a place that cannot
    /// be reached is not a route.
    pub(crate) fn step_aside(&mut self, id: EntityId, tiles: &[(u16, u16)]) {
        // **Already stepping aside.** The task is the walk and the interrupted order is in `resuming`; re-planning it every
        // tick would be a path search a tick, which is the shape that took an acceptance run from 16 s to 180 s.
        if self.resuming.contains_key(&id) {
            return;
        }
        let Some(entity) = self.find(id) else { return };
        let (from, task) = (entity.position, entity.task);
        let here = self.tile_of(from);
        let resumable = match task {
            // The two orders a step-aside can honestly put back; `resume_idle` reads this.
            task @ (Task::Gather { .. } | Task::Hunt { .. }) => task,
            _ => Task::Idle,
        };
        // **His place goes with him** (D186 addendum 5): a man moved off a foundation must not walk back onto it because the
        // ring's place he owned is still there. Without this the site's clear gate and the man's place fight each other, and the
        // site never starts - the ten-minute scenario's barracks stood at 0 of 1000 ticks with its builder at a gather task.
        self.release_gather_place(id);
        let Some(to) = self.exit_tile(tiles, here, from) else {
            // **Nowhere to go**: the site stalls, which D183 §4 accepts. The attempt is remembered anyway, so the search
            // is not repeated every tick - a unit that cannot leave would otherwise cost a ring of path searches a tick
            // (the shape D135 exists for). It is `Idle` for everything but a gather or a hunt, so arriving anywhere later
            // simply stands it down.
            self.resuming.insert(id, resumable);
            return;
        };
        let goal = self.tile_centre(to);
        let Ok(route) = self.plan_path(from, goal, 0) else {
            self.resuming.insert(id, resumable);
            return;
        };
        // The marker is also how this unit is known to be **already stepping aside**: `step_aside` returns early when it
        // is there, so a step-aside is one order and one route rather than a path search a tick.
        self.resuming.insert(id, resumable);
        self.paths.insert(
            id,
            Route {
                points: route,
                ..Route::default()
            },
        );
        if let Some(entity) = self.find_mut(id) {
            entity.task = Task::Move { to: goal };
        }
    }

    /// **The nearest tile outside `tiles` this unit can actually walk to**, in ring order, with the tile as the tie-break.
    /// Nearest free is not enough: a free tile behind a wall is no place to send a body - that is D184's lesson, and the
    /// route is what makes this function more than a placement.
    fn exit_tile(
        &self,
        tiles: &[(u16, u16)],
        from: (u16, u16),
        position: (Fixed, Fixed),
    ) -> Option<(u16, u16)> {
        let (wide, tall) = self.walk.as_ref().map_or((0, 0), Walk::size);
        for radius in 1..=wide.max(tall).max(1) {
            for at @ (x, y) in Self::ring(from, radius) {
                if tiles.contains(&at) || !self.is_free_ground(x, y) {
                    continue;
                }
                if self.plan_path(position, self.tile_centre(at), 0).is_ok() {
                    return Some(at);
                }
            }
        }
        None
    }

    /// **Tell our own and allied units standing on a new site to leave it** (D183 §1). The enemy is never told: he has no
    /// reason to go, and that is accepted. A farm is exempt, because it is a floor and traps nobody (D183 §7).
    pub(crate) fn evict_units_off(&mut self, site: EntityId, except: EntityId) {
        let Some((player, farm)) = self
            .find(site)
            .map(|entity| (entity.player, self.is_farm_role(entity)))
        else {
            return;
        };
        if farm {
            return;
        }
        let tiles = self.footprint_area_of(site);
        let movers: Vec<EntityId> = self
            .entities
            .iter()
            .filter(|entity| {
                entity.kind == EntityKind::Unit
                    && entity.id != except
                    && entity.hp > 0
                    && entity.player != 0
                    && !matches!(entity.task, Task::Garrisoned { .. })
                    && self.same_side(entity.player, player)
                    && tiles.contains(&self.tile_of(entity.position))
            })
            .map(|entity| entity.id)
            .collect();
        for id in movers {
            self.step_aside(id, &tiles);
        }
    }

    /// **A building's footprint as a rectangle**: anchor tile and size, which is what a builder's "am I on it?" question
    /// needs. `footprint_area` answers with the tiles themselves, which allocates; a builder asks this every tick.
    pub(crate) fn footprint_rect(&self, id: EntityId) -> Option<(u16, u16, u16, u16)> {
        let entity = self.find(id)?;
        let anchor = self.tile_of(entity.position);
        let unit = entity.unit?;
        let (width, height) = self
            .units
            .as_ref()
            .and_then(|units| units.footprint(unit))?;
        Some((
            anchor.0,
            anchor.1,
            u16::try_from(width).unwrap_or(1),
            u16::try_from(height).unwrap_or(1),
        ))
    }

    /// **Where a builder stands to work a site**: a free, reachable tile on the **outer ring of the footprint**, nearest
    /// to where the builder is now - never a tile of the footprint itself (D183 §3).
    ///
    /// `work_position` was enough while a site was solid from insertion, because its own tiles were blocked and the scan
    /// skipped them by accident. A walkable ghost makes `work_position` answer with a tile *inside* the site: the builder
    /// then stands on it, the empty-site gate waits for him, and the build never starts - measured as a house taking one
    /// builder-tick in five hundred (`a_house_is_paid_for_and_finishes_in_its_create_time`).
    pub(crate) fn approach_footprint(
        &self,
        from: (Fixed, Fixed),
        tiles: &[(u16, u16)],
    ) -> Option<(Fixed, Fixed)> {
        const RING: [(i32, i32); 8] = [
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ];
        let mut candidates: Vec<(u16, u16)> = Vec::new();
        for (x, y) in tiles {
            for (dx, dy) in RING {
                let (Ok(cx), Ok(cy)) = (
                    u16::try_from(i32::from(*x) + dx),
                    u16::try_from(i32::from(*y) + dy),
                ) else {
                    continue;
                };
                let tile = (cx, cy);
                if tiles.contains(&tile) || candidates.contains(&tile) {
                    continue;
                }
                candidates.push(tile);
            }
        }
        // Nearest to the builder first, then tile order: two builders sent to one site stand on the same tile otherwise.
        let (fx, fy) = (from.0.raw(), from.1.raw());
        candidates.sort_by_key(|(x, y)| {
            let dx = i64::from(*x) * self.scale - fx;
            let dy = i64::from(*y) * self.scale - fy;
            (dx * dx + dy * dy, *x, *y)
        });
        candidates.into_iter().find_map(|tile| {
            if !self.is_free_ground(tile.0, tile.1) {
                return None;
            }
            // **The middle of the tile**, not its origin: an origin is a corner shared with three other tiles, and a
            // body standing on it touches whichever of them is blocked - the clearance rule then refuses the last step
            // and the builder stalls one step short (measured: a farm that never started, 600 ticks of jitter at 4.78).
            let position = self.tile_centre(tile);
            // Free is not the same as reachable: a free tile inside a wall is no place to stand.
            self.plan_path(from, position, 0).ok().map(|_| position)
        })
    }

    /// The tiles a site of `unit` at `at` would cover: the footprint its `radius` gives it, which is what placement
    /// checks - not the mask, because a farm's floor is still the farm's ground.
    pub(crate) fn build_site_tiles(&self, unit: u32, at: (u16, u16)) -> Vec<(u16, u16)> {
        let Some((width, height)) = self.units.as_ref().and_then(|units| units.footprint(unit))
        else {
            return vec![at];
        };
        let mut tiles = Vec::with_capacity(width as usize * height as usize);
        for dy in 0..u16::try_from(height).unwrap_or(1) {
            for dx in 0..u16::try_from(width).unwrap_or(1) {
                tiles.push((at.0.saturating_add(dx), at.1.saturating_add(dy)));
            }
        }
        tiles
    }

    /// Every tile a building's **footprint** covers: the ground it is drawn on, from the same `radius` the renderer
    /// uses (D83). One rectangle, mask or not - `solid_tiles` says which of these cells block *movement*, this says which
    /// cells a building **owns**, and a building may not share any of them with another one.
    pub(crate) fn footprint_area(&self, entity: &Entity) -> Vec<(u16, u16)> {
        let anchor = self.tile_of(entity.position);
        let Some(unit) = entity.unit else {
            return vec![anchor];
        };
        let Some((width, height)) = self.units.as_ref().and_then(|units| units.footprint(unit))
        else {
            return vec![anchor];
        };
        let mut tiles = Vec::with_capacity(width as usize * height as usize);
        for dy in 0..u16::try_from(height).unwrap_or(1) {
            for dx in 0..u16::try_from(width).unwrap_or(1) {
                tiles.push((anchor.0.saturating_add(dx), anchor.1.saturating_add(dy)));
            }
        }
        tiles
    }

    /// The tiles a building occupies **solidly**, which is what blocks movement: its `[footprint.<role>]` `#` cells when
    /// content names a mask, otherwise its whole footprint (D76, and the owner's rule of 2026-11-03: "all other buildings
    /// block passage since start of construction"). A role in `[build] blocking_roles` blocks its whole footprint even
    /// where a mask says otherwise - the knob content has to override a floor.
    ///
    /// Two roles name a mask, and both are the exceptions the owner gave: a **farm** is all `.` (a floor, walked over while
    /// it is built and after) and a **town centre** is solid at its back core and open in its yard. Everything else builds
    /// its mask from its own footprint, which is why a house is no longer a picture a villager walks through.
    pub(crate) fn solid_tiles(&self, entity: &Entity) -> Vec<(u16, u16)> {
        let Some(unit) = entity.unit else {
            return Vec::new();
        };
        let root = self.units.as_ref().map_or(unit, |units| units.root(unit));
        if !self.blocking.contains(&root)
            && let Some(mask) = self.footprints.get(&root)
        {
            let anchor = self.tile_of(entity.position);
            return mask
                .blocked
                .iter()
                .enumerate()
                .filter(|(_, blocked)| **blocked)
                .map(|(index, _)| {
                    let column = (index % mask.width) as u16;
                    let row = (index / mask.width) as u16;
                    (
                        anchor.0.saturating_add(column),
                        anchor.1.saturating_add(row),
                    )
                })
                .collect();
        }
        self.footprint_area(entity)
    }

    /// The nearest building of `player`'s that the worker's own variant accepts as a drop site.
    /// Selection is by squared distance with ties broken by id, so two villagers sent home in the
    /// same tick agree on which building is closest.
    pub(crate) fn nearest_drop_site(
        &self,
        player: u8,
        from: (Fixed, Fixed),
        variant: Option<u32>,
    ) -> Option<EntityId> {
        let units = self.units.as_ref()?;
        let drop_sites = &units.unit(variant?)?.drop_sites;
        self.entities
            .iter()
            .filter(|entity| entity.kind == EntityKind::Building && entity.player == player)
            // By family, not by id: an upgraded town centre is unit 71, and the drop list names 109.
            .filter(|entity| {
                entity.unit.is_some_and(|unit| {
                    let root = units.root(unit);
                    drop_sites.contains(&unit) || drop_sites.contains(&root)
                })
            })
            .min_by_key(|entity| {
                let dx = entity.position.0.raw() - from.0.raw();
                let dy = entity.position.1.raw() - from.1.raw();
                (dx * dx + dy * dy, entity.id.0)
            })
            .map(|entity| entity.id)
    }

    /// Where a worker stands to work a building: the first neighbouring tile that is both free and
    /// reachable from `from`, in a fixed scan order, so the choice is a function of the map rather
    /// than of iteration order. `None` when there is none, and the caller keeps its old behaviour of
    /// walking at the building itself.
    /// A tile from which a blow on `target` can land, chosen **for this unit**: free ground, reachable from `from`,
    /// within `range` (adjacency when the range is melee) and outside `min_range`, in ring order around the target and
    /// skipping any tile another unit is already walking to - so a dozen attackers at one building spread over its edge
    /// instead of queueing behind whoever gets there first (E1b).
    ///
    /// This is the answer to "the route ends on a blocked tile": a building's own tile is not standable, and a route
    /// planned *to* it ends wherever collision first refuses, which measured as twelve attackers standing 1-4 tiles off
    /// with `cooldown_ticks: 0`, never striking.
    pub(crate) fn stand_near(
        &self,
        unit: EntityId,
        from: (Fixed, Fixed),
        target: (Fixed, Fixed),
        min_range: Fixed,
        range: Fixed,
    ) -> Option<(Fixed, Fixed)> {
        let scale = self.scale;
        let (tx, ty) = self.tile_of_raw(target);
        // Melee reaches the next tile; anything with reach may stand inside it, so the ring is the reach in tiles plus
        // one to cover the tile the target itself occupies.
        let tiles = if range.raw() > 0 {
            (range.raw() / scale).max(1) + 1
        } else {
            1
        };
        let mut candidates: Vec<(i64, (u16, u16))> = Vec::new();
        for radius in 1..=tiles {
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if dx.abs() != radius && dy.abs() != radius {
                        continue;
                    }
                    let (Ok(x), Ok(y)) = (u16::try_from(tx + dx), u16::try_from(ty + dy)) else {
                        continue;
                    };
                    if !self.is_free_ground(x, y) {
                        continue;
                    }
                    // A tile another unit is already heading for is that unit's: a crowd takes the edge, not one tile.
                    if self.entities.iter().any(|other| {
                        other.id != unit
                            && matches!(self.goal_of(other), Some((tile, _)) if tile == (x, y))
                    }) {
                        continue;
                    }
                    let position = self.tile((x, y));
                    let (ddx, ddy) = (
                        position.0.raw() - target.0.raw(),
                        position.1.raw() - target.1.raw(),
                    );
                    let squared = ddx * ddx + ddy * ddy;
                    let legal = if range.raw() > 0 {
                        squared <= range.raw() * range.raw()
                            && (min_range.raw() <= 0
                                || squared >= min_range.raw() * min_range.raw())
                    } else {
                        true
                    };
                    if !legal {
                        continue;
                    }
                    // **Nearest to this unit, then tile order.** Ring order alone hands every attacker the same tile
                    // when their orders land on one tick - none has a goal yet, so nothing looks claimed - and the crowd
                    // queues behind the first arrival exactly as before. Distance from where the unit stands spreads
                    // them over the edge it approached from, and the tile order keeps two runs identical.
                    let (fdx, fdy) = (
                        position.0.raw() - from.0.raw(),
                        position.1.raw() - from.1.raw(),
                    );
                    candidates.push((fdx * fdx + fdy * fdy, (x, y)));
                }
            }
        }
        candidates.sort_by_key(|(distance, tile)| (*distance, *tile));
        candidates.into_iter().find_map(|(_, tile)| {
            let position = self.tile(tile);
            self.plan_path(from, position, 0).ok().map(|_| position)
        })
    }

    /// Walk a unit towards a place, stopping as close to it as the map allows: the place itself first,
    /// then the nearest tile on each ring around it that a route exists to. A rally point inside a walled
    /// pocket is therefore an instruction to stand at the pocket's edge, not to stay home (D40).
    pub(crate) fn walk_towards(&mut self, unit: EntityId, to: (Fixed, Fixed)) {
        let Some(from) = self.find(unit).map(|entity| entity.position) else {
            return;
        };
        let mut target = None;
        if let Ok(route) = self.plan_path(from, to, 0) {
            self.paths.insert(
                unit,
                Route {
                    points: route,
                    ..Route::default()
                },
            );
            target = Some(to);
        } else {
            let scale = self.scale;
            let x = i32::from(u16::try_from(to.0.raw() / scale).unwrap_or(0));
            let y = i32::from(u16::try_from(to.1.raw() / scale).unwrap_or(0));
            'rings: for radius in 1..=10i32 {
                for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        // Only the ring itself: the interior has already been tried.
                        if dx.abs() != radius && dy.abs() != radius {
                            continue;
                        }
                        let (Some(cx), Some(cy)) =
                            (u16::try_from(x + dx).ok(), u16::try_from(y + dy).ok())
                        else {
                            continue;
                        };
                        if !self.is_free_ground(cx, cy) {
                            continue;
                        }
                        let candidate = self.tile_centre((cx, cy));
                        if let Ok(route) = self.plan_path(from, candidate, 0) {
                            self.paths.insert(
                                unit,
                                Route {
                                    points: route,
                                    ..Route::default()
                                },
                            );
                            target = Some(candidate);
                            break 'rings;
                        }
                    }
                }
            }
        }
        if let Some(target) = target
            && let Some(entity) = self.find_mut(unit)
        {
            entity.task = Task::Move { to: target };
        }
    }

    /// Start a walk: an ordinary move, an attack-move, or a patrol. `patrol` only decides whether the
    /// tile the unit is standing on is remembered as the other end.
    ///
    /// The order is stored as `Task::Hunt` for the two fighting orders and `Task::Move` for a plain
    /// one, so a plain move stays exactly what it was: a walk that ignores everything on the way.
    pub(crate) fn start_walk(
        &mut self,
        unit: EntityId,
        to: (Fixed, Fixed),
        fighting: bool,
        patrol: bool,
    ) -> Result<(), SimError> {
        // **Every click is accepted** (D134): a target on blocked ground becomes the nearest legal standing point before
        // anything is planned, slotted or stored, so the order's goal is always a tile a body can occupy.
        let from = self.find(unit).map_or(to, |entity| entity.position);
        let to = self.walk_goal(unit, from, to)?;
        // Crowd slots are **tiles** (a place to stand), so the point becomes a tile for the slot search and survives as
        // the goal when this unit takes the clicked tile itself (A).
        let wanted = self.tile_of(to);
        // A crowd ordered to one tile is a crowd ordered to a *place*: each unit takes a tile of its own
        // near the one that was clicked, in the order the orders arrive. Without this they all aim at one
        // tile, and with the last-step rule (D52) all but one stop a step short of it (D53).
        let already_heading = self
            .entities
            .iter()
            .filter(|entity| entity.id != unit)
            .filter(|entity| matches!(self.goal_of(entity), Some((tile, true)) if tile == wanted))
            .count();
        let slot = self.slot_for(wanted, already_heading);
        let goal = if slot == wanted {
            to
        } else {
            self.tile_centre(slot)
        };
        let goal = self.check_order_at(unit, goal)?;
        let entity = self.find(unit).ok_or(SimError::MissingEntity(unit))?;
        let route = self.plan_path_for(unit, entity.position, goal)?;
        let here = self.tile_of(entity.position);
        self.paths.insert(
            unit,
            Route {
                points: route,
                ..Route::default()
            },
        );
        // Any earlier order is superseded: a new order is not a second one running alongside.
        self.resuming.remove(&unit);
        if patrol {
            self.patrols.insert(unit, self.tile_centre(here));
        } else {
            self.patrols.remove(&unit);
        }
        if let Some(entity) = self.find_mut(unit) {
            entity.task = if fighting {
                Task::Hunt { to: goal }
            } else {
                Task::Move { to: goal }
            };
        }
        Ok(())
    }

    /// **The nearest farm of this player's that nobody is working and nothing has eaten** (D122), or `None`.
    ///
    /// "Free" is all three of: the right side's, finished, still holding food, and without a villager on it. Reachability is
    /// the gather order's job, not this search's: a plot behind a wall is offered here and refused by the route, which is the
    /// same answer the player would get clicking it himself.
    pub(crate) fn nearest_free_farm(
        &self,
        from: (Fixed, Fixed),
        player: u8,
        exclude: EntityId,
    ) -> Option<EntityId> {
        // **"Nearby", in the owner's word** - content's radius, in tiles. A plot across the map is not offered: the villager
        // stands idle instead, which is the answer the owner gave.
        let radius = self
            .economy
            .as_ref()
            .map_or(0, |economy| i64::from(economy.farm.free_farm_radius))
            * self.scale;
        let limit = radius.saturating_mul(radius);
        let mut best: Option<(i64, EntityId)> = None;
        for entity in &self.entities {
            if entity.id == exclude || entity.player != player || !self.is_farm_role(entity) {
                continue;
            }
            if entity.construction.is_some() || entity.amount.raw() <= 0 {
                continue;
            }
            if self.entities.iter().any(|other| {
                other.id != exclude
                    && matches!(other.task, Task::Gather { node, .. } | Task::Return { node, .. } if node == entity.id)
            }) {
                continue;
            }
            let dx = entity.position.0.raw() - from.0.raw();
            let dy = entity.position.1.raw() - from.1.raw();
            let distance = dx * dx + dy * dy;
            if distance > limit {
                continue;
            }
            if best.is_none_or(|(closest, _)| distance < closest) {
                best = Some((distance, entity.id));
            }
        }
        best.map(|(_, id)| id)
    }
}
