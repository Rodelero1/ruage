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
use std::collections::{BTreeSet, BinaryHeap};

use ruage_data::Map;

// **The original's cost model** (D249; `docs/movement-in-the-binary.md`, R7): a cardinal step costs **16**, a diagonal the
// octile table's **23** (16 x sqrt2 rounded), and the estimate is Euclidean in tiles scaled by **25** - a weight of about
// 1.56, so the search is deliberately **not** an admissible A* and returns slightly longer routes than an optimal one.
const CARDINAL: u32 = 16;
const DIAGONAL: u32 = 23;
/// The heuristic's weight: `FUN_008007f0` scales Euclidean tile distance by `2500/100`, which is the same number as its
/// sibling branch's `*0x19`.
const ESTIMATE_WEIGHT: u32 = 25;

/// Which tiles a unit may stand on. Built once from the map, because terrain does not change
/// during a match yet.
#[derive(serde::Serialize, serde::Deserialize)]
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
        // **`Map` is public, so this is where a broken one stops** (second review, `path.rs:33`). The loader refuses a map
        // whose terrain array does not match its size, but tests and fixtures build `Map` directly, and indexing one by
        // `width`/`height` would then read one tile's ground for another - a walkability bug with no error and no symptom
        // except units crossing a wall. Debug builds say so; release builds cost nothing.
        debug_assert_eq!(
            map.terrain.len(),
            usize::from(map.width) * usize::from(map.height),
            "a map's terrain must be width * height: {} entries for {}x{}",
            map.terrain.len(),
            map.width,
            map.height
        );
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

/// The estimate: **Euclidean distance in tiles, scaled by the original's weight** (`FUN_007ffcd0` distance, `FUN_008007f0`
/// x25). Weighted rather than admissible, which is the game's trade - it explores fewer tiles and accepts slightly longer
/// routes (R7's reading).
fn estimate(from: (u16, u16), to: (u16, u16)) -> u32 {
    let dx = i64::from(from.0) - i64::from(to.0);
    let dy = i64::from(from.1) - i64::from(to.1);
    let distance = crate::integer_sqrt(dx * dx + dy * dy);
    (distance * i64::from(ESTIMATE_WEIGHT)) as u32
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
    line_clear_avoiding(walk, from, to, &BTreeSet::new())
}

/// The same line with tiles the grid does not know about counted as impassable - **a body is not terrain**, so a caller
/// that plans around units (a repath, D98a) says which tiles to avoid rather than the `Walk` layer learning about bodies.
/// A pulled route has to obey the same avoid set the search did, or the pull would walk the line straight back through
/// the body the search went round.
#[must_use]
pub fn line_clear_avoiding(
    walk: &Walk,
    from: (u16, u16),
    to: (u16, u16),
    avoid: &BTreeSet<(u16, u16)>,
) -> bool {
    let free = |x: i64, y: i64| -> bool {
        let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y)) else {
            return false;
        };
        // **The start tile is not an obstruction to its own line** (second review, `path.rs:127`). `find_path` allows a
        // blocked start on purpose - *"a villager stands on the tile of the tree it is working, and must be able to walk
        // off it"* - and refusing the same line here made every such unit pay a full A* search for a line it could see.
        // `blocked` answers `true` for ground off the map as well, so the containment test that used to sit in front of it
        // was a second bounds check per tile walked.
        !walk.blocked(x, y) && !avoid.contains(&(x, y))
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
        // The first tile is where the body already is: it cannot obstruct its own line.
        if (x, y) != (x0, y0) && !free(x, y) {
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

/// **The search answers the closest point it can reach, and "no path" is not one of its answers** (the original's rule:
/// the Klingers deck, p. 96 - *"expand the search area once, otherwise give the closest point path"*; the PS2 low-level
/// search reserves "no low-level path" for a trapped *start*). The tiles to walk through exclude the start and include
/// whatever the search settled on: the goal when it is reachable, otherwise the reachable tile nearest to it. `None` only
/// when the start or the goal is off the map.
///
/// A caller that must insist on the exact tile asks [`find_path_strict`]; a walk order takes the closest point, which is
/// what "every click is accepted" means (D134) with the answer now coming from the search rather than a helper beside it.
/// A blocked goal is allowed: resource nodes stand on forest, and a villager has to be able to reach the tree it is working.
pub fn find_path(walk: &Walk, from: (u16, u16), to: (u16, u16)) -> Option<Vec<(u16, u16)>> {
    find_path_avoiding(walk, from, to, &BTreeSet::new())
}

/// [`find_path`] with the goal's own tile as the only acceptable answer: `None` when the search ended somewhere else.
///
/// This is the shape **thing** orders need - a tree behind a wall is a refusal, not a walk to the fence (D241 ruling) - and
/// it is a filter rather than a second search, so the two answers can never disagree about the map.
pub fn find_path_strict(walk: &Walk, from: (u16, u16), to: (u16, u16)) -> Option<Vec<(u16, u16)>> {
    find_path_avoiding_strict(walk, from, to, &BTreeSet::new())
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
    search(walk, from, to, avoid, false).map(|answer| answer.1)
}

/// [`find_path_avoiding`] that answers nothing unless the route ends **on the goal tile**.
pub fn find_path_avoiding_strict(
    walk: &Walk,
    from: (u16, u16),
    to: (u16, u16),
    avoid: &BTreeSet<(u16, u16)>,
) -> Option<Vec<(u16, u16)>> {
    search(walk, from, to, avoid, true).map(|answer| answer.1)
}

/// A settled tile and the route to it, as `search` reports them.
type Answer = ((u16, u16), Vec<(u16, u16)>);

/// A tile popped by the search, with its cost: `(estimate, y, x)`, paired with the tile it is.
type Closest = ((u32, u16, u16), (u16, u16));

/// The one search both entry points use: `answer.0` is the tile it settled on, `answer.1` the route to it.
///
/// `strict` decides what may be settled on. A strict search answers `None` when the goal was never reached; **the closest
/// point is still the closest point either way**, so the two answers describe the same search of the same map.
fn search(
    walk: &Walk,
    from: (u16, u16),
    to: (u16, u16),
    avoid: &BTreeSet<(u16, u16)>,
    strict: bool,
) -> Option<Answer> {
    if !walk.contains(to.0, to.1) || !walk.contains(from.0, from.1) {
        return None;
    }
    if from == to {
        return Some((from, Vec::new()));
    }
    // A blocked *start* is allowed and necessary: a villager stands on the tile of the tree it is working, and must be able
    // to walk off it. Only the route is searched, not the standing spot.
    //
    // **But a start boxed in on every side is re-seeded** - the original's low-level search offsets a start inside an
    // obstruction by `2 x radius` along a ring of directions and searches from the first free candidate
    // (`FUN_007fb220`, row 33). A soldier spawned inside his own crowd hits exactly this: the tile search cannot take a
    // first step, and refusing the order is not an answer the game has.

    let (width, height) = walk.size();
    let cells = usize::from(width) * usize::from(height);
    let at = |x: u16, y: u16| usize::from(y) * usize::from(width) + usize::from(x);

    let mut open: BinaryHeap<Reverse<(u32, u32, u16, u16)>> = BinaryHeap::new();
    // **Two flat arrays indexed by tile, not two trees** (D143). The maps allocated a node per visited tile and paid a
    // comparison per lookup; an array is one allocation and arithmetic, and `came` holds the parent's **index** rather
    // than its coordinates. The ordering of `open` is untouched - same tuples, same comparison - so the route is
    // identical, which the checksum in `examples/path_cost.rs` is there to prove rather than assume.
    let mut best: Vec<u32> = vec![u32::MAX; cells];
    let mut came: Vec<u32> = vec![u32::MAX; cells];
    // The closest point: the tile popped whose distance to the goal is smallest, excluding the start (a search that ends
    // where it began has not moved). Kept as a distance so the comparison is arithmetic, never iteration order.
    let mut closest: Option<Closest> = None;

    best[at(from.0, from.1)] = 0;
    open.push(Reverse((estimate(from, to), 0, from.1, from.0)));

    while let Some(Reverse((_, cost, y, x))) = open.pop() {
        let here = (x, y);
        let index = at(x, y);
        if here != from {
            let key = (estimate(here, to), y, x);
            if closest.is_none_or(|(found, _)| key < found) {
                closest = Some((key, here));
            }
        }
        if here == to {
            return Some((here, route_to(&came, width, at(from.0, from.1), index)));
        }
        if cost > best[index] {
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
                let next_index = at(nx, ny);
                if next_cost < best[next_index] {
                    best[next_index] = next_cost;
                    came[next_index] = index as u32;
                    open.push(Reverse((
                        next_cost + estimate((nx, ny), to),
                        next_cost,
                        ny,
                        nx,
                    )));
                }
            }
        }
    }
    // The goal was never reached. The closest point is still an answer - unless the caller asked for the goal itself, in
    // which case "somewhere short of it" must read as "no".
    let (_, tile) = closest?;
    if strict && tile != to {
        return None;
    }
    Some((
        tile,
        route_to(&came, width, at(from.0, from.1), at(tile.0, tile.1)),
    ))
}

/// Walk the parent chain back from `index`, **excluding the start** - a route beginning with the tile the body already
/// stands on costs a tick of standing still.
fn route_to(came: &[u32], width: u16, start: usize, index: usize) -> Vec<(u16, u16)> {
    let tile_of = |index: usize| {
        (
            (index % usize::from(width)) as u16,
            (index / usize::from(width)) as u16,
        )
    };
    let mut path = vec![tile_of(index)];
    let mut cursor = index;
    loop {
        let parent = came[cursor];
        if parent == u32::MAX {
            break;
        }
        let parent = parent as usize;
        if parent == start {
            break;
        }
        path.push(tile_of(parent));
        cursor = parent;
    }
    path.reverse();
    path
}
