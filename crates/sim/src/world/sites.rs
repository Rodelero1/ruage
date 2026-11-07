//! **Where a building may stand**: footprints, lanes, approach tiles, and where a walk ends (D145).
//!
//! Split out of `lib.rs` under D145, verbatim.
use std::collections::{BTreeMap, BTreeSet};

use ruage_fixed::Fixed;

use crate::{Entity, EntityId, EntityKind, Route, SimError, Task, TileVision, World};
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
            let seen = entity.player == player
                || entity.player == 0
                || self.vision(player, self.tile_of(entity.position)) == TileVision::Visible;
            if seen {
                for tile in self.footprint_area(entity) {
                    held.entry(tile).or_insert(entity.id);
                }
            } else {
                hidden.extend(self.footprint_area(entity));
            }
        }
        for tile in self.build_site_tiles(unit, at) {
            if !walk.contains(tile.0, tile.1) {
                return Some(SimError::SiteBlocked { tile });
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
            let seen = entity.player == player
                || entity.player == 0
                || self.vision(player, self.tile_of(entity.position)) == TileVision::Visible;
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
