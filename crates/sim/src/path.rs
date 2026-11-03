//! A\* over the static tile grid.
//!
//! Costs are integers (10 for a cardinal step, 14 for a diagonal) rather than the f64 weights in
//! `movement.toml`: those values are still `-1.0` sentinels (U5/U7), and a pathing heuristic is an
//! algorithm constant, not a game value. When the weights are measured they replace `CARDINAL`
//! and `DIAGONAL` here and nowhere else.
//!
//! Determinism: the open set is a binary heap ordered by `(f, g, y, x)`, so two runs expand the
//! same nodes in the same order. No hash-map iteration order reaches the result.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

use ruage_data::Map;

const CARDINAL: u32 = 10;
const DIAGONAL: u32 = 14;

/// Which tiles a unit may stand on. Built once from the map, because terrain does not change
/// during a match yet.
pub struct Walk {
    width: u16,
    height: u16,
    /// Impassable **terrain**. Nothing in this content is impassable (D84), but water will be, and terrain never changes
    /// during a match, so it is a separate immutable layer.
    terrain: Vec<bool>,
    /// How many things stand on each tile - a wall, a tree, a building's solid part. **Counted, not flagged**: a tree and
    /// a wall can share a tile, and with one `bool` whichever was removed first freed the ground the other still held.
    occupants: Vec<u16>,
}

impl Walk {
    pub fn from_map(map: &Map) -> Self {
        let blocked = map
            .terrain
            .iter()
            // Terrain is **ground, not an obstacle** (the owner's rule, 2026-10-31): what blocks a tile is the tree or the
            // solid part of a building standing on it. Forest floor is walkable, so a felled wood leaves a clearing
            // instead of a permanent invisible wall.
            .map(|tile| !tile.walkable())
            .collect::<Vec<bool>>();
        Walk {
            width: map.width,
            height: map.height,
            occupants: vec![0; blocked.len()],
            terrain: blocked,
        }
    }

    fn index(&self, x: u16, y: u16) -> Option<usize> {
        self.contains(x, y)
            .then(|| usize::from(y) * usize::from(self.width) + usize::from(x))
    }

    /// Something now stands on this tile. Every occupant must be counted, or the first one to leave frees the ground.
    pub fn block(&mut self, x: u16, y: u16) {
        if let Some(at) = self.index(x, y)
            && let Some(count) = self.occupants.get_mut(at)
        {
            *count = count.saturating_add(1);
        }
    }

    /// Something that stood here has gone. Saturating at zero: an unbalanced call must not wrap a tile into "blocked by
    /// 65,535 things" - a stuck tile is worse than a missing one, and the counter is the only thing between the two.
    pub fn unblock(&mut self, x: u16, y: u16) {
        if let Some(at) = self.index(x, y)
            && let Some(count) = self.occupants.get_mut(at)
        {
            *count = count.saturating_sub(1);
        }
    }

    pub fn blocked(&self, x: u16, y: u16) -> bool {
        let Some(at) = self.index(x, y) else {
            return true;
        };
        self.terrain.get(at).copied().unwrap_or(false)
            || self.occupants.get(at).copied().unwrap_or(0) > 0
    }

    pub fn contains(&self, x: u16, y: u16) -> bool {
        x < self.width && y < self.height
    }

    /// The grid's size, for a search that has to stop at the edge of the world rather than at a guessed radius.
    #[must_use]
    pub fn size(&self) -> (u16, u16) {
        (self.width, self.height)
    }
}

fn octile(from: (u16, u16), to: (u16, u16)) -> u32 {
    let dx = i64::from(from.0) - i64::from(to.0);
    let dy = i64::from(from.1) - i64::from(to.1);
    let (dx, dy) = (dx.unsigned_abs(), dy.unsigned_abs());
    let diagonal = dx.min(dy);
    let straight = dx.max(dy) - diagonal;
    (straight * u64::from(CARDINAL) + diagonal * u64::from(DIAGONAL)) as u32
}

/// Is the **straight line** between two tiles walkable, ignoring the goal's own tile?
///
/// This is the original's first attempt at every movement (`doIntelligentMovementUpdate`, move_obj.cpp 4,079-4,245:
/// "no non-target collisions there, so we can do the straight line path"), and it is cheap where a search is not: one
/// Bresenham walk instead of an A* over the map. The goal tile is exempt for the same reason `find_path` allows a blocked
/// goal - a node's own tile holds the thing being worked, and a body you were ordered onto is a target, not an obstacle.
///
/// A diagonal step also requires both of its corners free, which is the rule the search uses: a line that would clip the
/// corner of a wall is not a line a unit may walk.
#[must_use]
pub fn line_clear(walk: &Walk, from: (u16, u16), to: (u16, u16)) -> bool {
    let free = |x: i64, y: i64| -> bool {
        let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y)) else {
            return false;
        };
        walk.contains(x, y) && !walk.blocked(x, y)
    };
    let (x0, y0) = (i64::from(from.0), i64::from(from.1));
    let (x1, y1) = (i64::from(to.0), i64::from(to.1));
    let (dx, dy) = ((x1 - x0).abs(), (y1 - y0).abs());
    let (sx, sy) = ((x1 - x0).signum(), (y1 - y0).signum());
    let (mut x, mut y) = (x0, y0);
    let mut err = dx - dy;
    loop {
        if (x, y) == (x1, y1) {
            return true;
        }
        if !free(x, y) {
            return false;
        }
        let e2 = 2 * err;
        let mut stepped_x = false;
        let mut stepped_y = false;
        if e2 > -dy {
            err -= dy;
            x += sx;
            stepped_x = true;
        }
        if e2 < dx {
            err += dx;
            y += sy;
            stepped_y = true;
        }
        if stepped_x && stepped_y && (!free(x - sx, y) || !free(x, y - sy)) {
            return false;
        }
    }
}

/// The tiles to walk through, excluding the start and including the goal. `None` when the goal is
/// unreachable. A blocked goal is allowed: resource nodes stand on forest, and a villager has to
/// be able to reach the tree it is working.
pub fn find_path(walk: &Walk, from: (u16, u16), to: (u16, u16)) -> Option<Vec<(u16, u16)>> {
    find_path_avoiding(walk, from, to, &BTreeSet::new())
}

/// The same search with tiles the grid does not know about treated as impassable - **a unit is not terrain**, so the
/// caller says which tiles to route around rather than the `Walk` layer learning about bodies.
///
/// This is what the original does when a collision is *non-past*: it repaths, planning around the objects it just met
/// (D98a, move_obj.cpp 4,600-4,760). The set is per-search and temporary, so two units crossing do not poison each
/// other's plans. The goal is allowed to be in the set, for the same reason a blocked goal is allowed: a node's own tile
/// is usually occupied by the thing being worked.
pub fn find_path_avoiding(
    walk: &Walk,
    from: (u16, u16),
    to: (u16, u16),
    avoid: &BTreeSet<(u16, u16)>,
) -> Option<Vec<(u16, u16)>> {
    if !walk.contains(to.0, to.1) || !walk.contains(from.0, from.1) {
        return None;
    }
    if from == to {
        return Some(Vec::new());
    }
    // A blocked *start* is allowed and necessary: a villager stands on the tile of the tree it is
    // working, and must be able to walk off it. Only the route is searched, not the standing spot.

    let mut open: BinaryHeap<Reverse<(u32, u32, u16, u16)>> = BinaryHeap::new();
    let mut best: BTreeMap<(u16, u16), u32> = BTreeMap::new();
    let mut came: BTreeMap<(u16, u16), (u16, u16)> = BTreeMap::new();

    best.insert(from, 0);
    open.push(Reverse((octile(from, to), 0, from.1, from.0)));

    while let Some(Reverse((_, cost, y, x))) = open.pop() {
        let here = (x, y);
        if here == to {
            let mut path = vec![here];
            let mut cursor = here;
            while let Some(&previous) = came.get(&cursor) {
                if previous == from {
                    break;
                }
                path.push(previous);
                cursor = previous;
            }
            path.reverse();
            return Some(path);
        }
        if cost > *best.get(&here).unwrap_or(&u32::MAX) {
            continue;
        }

        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let (Some(nx), Some(ny)) = (
                    u16::try_from(i32::from(x) + dx).ok(),
                    u16::try_from(i32::from(y) + dy).ok(),
                ) else {
                    continue;
                };
                let diagonal = dx != 0 && dy != 0;
                let is_goal = (nx, ny) == to;
                if !walk.contains(nx, ny) {
                    continue;
                }
                // Walls may not be cut across: a diagonal step needs both of its corners open. A unit is not a wall, but
                // it is in the way, so the same rule keeps a route from clipping its corner.
                if diagonal
                    && (walk.blocked(nx, y)
                        || walk.blocked(x, ny)
                        || avoid.contains(&(nx, y))
                        || avoid.contains(&(x, ny)))
                {
                    continue;
                }
                if !is_goal && (walk.blocked(nx, ny) || avoid.contains(&(nx, ny))) {
                    continue;
                }
                let step = if diagonal { DIAGONAL } else { CARDINAL };
                let next_cost = cost + step;
                let entry = best.entry((nx, ny)).or_insert(u32::MAX);
                if next_cost < *entry {
                    *entry = next_cost;
                    came.insert((nx, ny), here);
                    open.push(Reverse((
                        next_cost + octile((nx, ny), to),
                        next_cost,
                        ny,
                        nx,
                    )));
                }
            }
        }
    }
    None
}
