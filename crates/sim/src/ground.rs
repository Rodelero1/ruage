//! **The ground and the search** (D279): the original's own model, ported bluntly.
//!
//! Two questions, and nothing else in this file:
//!
//! * **may a body stand on this tile** - terrain, plus however many things have been stamped on it;
//! * **which tiles does a walk cross** - one A\* over that grid, at the original's cost model, whose answer is pulled to
//!   the corners an obstacle actually forces and handed back as world points.
//!
//! **What is deliberately absent.** This replaces `path.rs`'s weighted search and `short.rs`'s visibility graph, and it
//! carries none of what grew around them: no search window, no level hierarchy, no radius, no avoid set, no expansion
//! budget, no "closest reachable" flag. The original plans on the tile grid for *"the boundaries of where this unit's
//! radii will go"* and answers a goal it cannot reach by settling on the nearest cell - which is what this does, always.
//! The cost of a route is not a gate anywhere in the engine; the fixtures check shape (reached, never crossed, stopped
//! beside) rather than an optimal length.
//!
//! **Determinism.** The open set is a binary heap ordered by `(f, g, y, x)`, and the predecessor table is an array
//! indexed by tile, so two runs expand the same nodes in the same order. No hash-map iteration reaches the result.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use ruage_data::Map;
use ruage_fixed::Fixed;

/// **A body the plan must keep clear of** (D282): where it stands, and how far the mover's body plus its own reaches.
///
/// The original puts a stopped object into the search as a rectangle **grown by the mover's radius** (`FUN_007f9d80`) and
/// grows the collision query box the same way (`FUN_00840830`, `+0x268`). A tile grid cannot express a 0.4-tile clearance,
/// so the clearance is carried beside the grid: a tile whose centre is inside it is not planned through, and a straightening
/// jump that passes within it is refused.
#[derive(Clone, Copy, Debug)]
pub struct Clear {
    /// The body's centre, in raw units.
    pub at: (i64, i64),
    /// The distance the route must keep from it, in raw units: the mover's reach to that body.
    pub reach: i64,
}

impl Clear {
    /// How far one raw point is from the body's centre, squared - the cheap test the search runs per candidate tile.
    fn still_inside(&self, point: (i64, i64)) -> bool {
        let (dx, dy) = (point.0 - self.at.0, point.1 - self.at.1);
        dx * dx + dy * dy < self.reach * self.reach
    }

    /// **Does the straight segment between two raw points pass inside the clearance?** Point-to-segment distance, in
    /// integers, so the answer is exact and identical everywhere.
    fn crossed_by(&self, from: (i64, i64), to: (i64, i64)) -> bool {
        let (px, py) = (self.at.0 - from.0, self.at.1 - from.1);
        let (vx, vy) = (to.0 - from.0, to.1 - from.1);
        let length = vx * vx + vy * vy;
        // The nearest point on the segment, clamped to it: the projection may fall before the start or past the end.
        let (nx, ny) = if length == 0 {
            (0, 0)
        } else {
            let dot = px * vx + py * vy;
            let t = dot.clamp(0, length);
            (vx * t / length, vy * t / length)
        };
        let (dx, dy) = (px - nx, py - ny);
        dx * dx + dy * dy < self.reach * self.reach
    }
}

/// The original's cost of one cardinal step (`docs/movement-in-the-binary.md`, `FUN_008007f0`).
pub const CARDINAL: u32 = 16;
/// The same table's diagonal: `16 x sqrt2`, rounded.
pub const DIAGONAL: u32 = 23;
/// The estimate's weight: the original scales Euclidean tile distance by 25, so its search is deliberately not admissible
/// and returns routes slightly longer than the shortest. Kept because it is what the game does, not because it is better.
pub const ESTIMATE: u32 = 25;

/// Every tile a body may stand on, and what stands on it.
///
/// `terrain` is immutable during a match (a wood is where it was); `occupants` is **counted**, not flagged, because a tree
/// and a wall can share a tile and a flag would free the ground when the first of them went.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Ground {
    width: u16,
    height: u16,
    terrain: Vec<bool>,
    occupants: Vec<u16>,
}

impl Ground {
    /// The ground a map describes: impassable terrain blocked, no occupants yet.
    pub fn from_map(map: &Map) -> Self {
        debug_assert_eq!(
            map.terrain.len(),
            usize::from(map.width) * usize::from(map.height),
            "a map's terrain must be width * height: {} entries for {}x{}",
            map.terrain.len(),
            map.width,
            map.height
        );
        // Terrain is ground, not an obstacle (the owner's rule, 2026-10-31): forest *floor* is walkable, and what blocks a
        // tile is the tree or the solid part of a building stamped on it, so a felled wood leaves a clearing.
        let terrain: Vec<bool> = map.terrain.iter().map(|tile| !tile.walkable()).collect();
        Ground {
            width: map.width,
            height: map.height,
            occupants: vec![0; terrain.len()],
            terrain,
        }
    }

    pub fn size(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    /// Is this tile on the map at all?
    pub fn contains(&self, x: u16, y: u16) -> bool {
        self.index(x, y).is_some()
    }

    /// Is something standing on this tile? A caller that has already asked `contains` wants the obstacle half alone.
    pub fn blocked(&self, x: u16, y: u16) -> bool {
        !self.walkable(x, y)
    }

    fn index(&self, x: u16, y: u16) -> Option<usize> {
        (x < self.width && y < self.height)
            .then(|| usize::from(y) * usize::from(self.width) + usize::from(x))
    }

    /// Something now stands on this tile.
    pub fn block(&mut self, x: u16, y: u16) {
        if let Some(at) = self.index(x, y)
            && let Some(count) = self.occupants.get_mut(at)
        {
            *count = count.saturating_add(1);
        }
    }

    /// Something left it. The count is what keeps a tile held by two things from freeing on the first one's removal.
    pub fn unblock(&mut self, x: u16, y: u16) {
        if let Some(at) = self.index(x, y)
            && let Some(count) = self.occupants.get_mut(at)
        {
            *count = count.saturating_sub(1);
        }
    }

    /// May a body stand on this tile? Off the map is not ground.
    pub fn walkable(&self, x: u16, y: u16) -> bool {
        self.index(x, y)
            .is_some_and(|at| !self.terrain[at] && self.occupants[at] == 0)
    }

    /// The original's cost of one step (`FUN_008007f0`): 16 along an axis, 23 diagonally.
    pub fn step_cost(from: (u16, u16), to: (u16, u16)) -> u32 {
        if from == to {
            0
        } else if from.0 != to.0 && from.1 != to.1 {
            DIAGONAL
        } else {
            CARDINAL
        }
    }

    /// **The original's estimate from one tile to another**: Euclidean tile distance scaled by 25 (`FUN_008007f0`). Public
    /// so the conformance set can check the number rather than trust it.
    pub fn estimate(from: (u16, u16), to: (u16, u16)) -> u32 {
        let dx = i64::from(from.0) - i64::from(to.0);
        let dy = i64::from(from.1) - i64::from(to.1);
        u32::try_from(crate::integer_sqrt(dx * dx + dy * dy).saturating_mul(i64::from(ESTIMATE)))
            .unwrap_or(u32::MAX)
    }

    /// **Whether a straight line between two tiles is clear.** Every tile the line crosses must be walkable, except the
    /// last one: a line may *end* on the tile it was sent to, because a node's tile holds the thing being worked. A diagonal
    /// crossing the line takes must have both of its side tiles free - the same rule the search applies, so a straight line
    /// and a searched route never disagree about a corner.
    ///
    /// Bresenham in integers, so the answer is exact and identical everywhere.
    pub fn line_clear(&self, from: (u16, u16), to: (u16, u16)) -> bool {
        self.line_clear_with(from, to, 1, &[])
    }

    /// **The same line, with tiles a body holds counting as solid** (D281). The smoothing pass uses this: a straight jump
    /// across a tile the plan was told to get round would undo the plan, which is what the first version did - the route
    /// bent in the search and was pulled straight again through the man.
    fn line_clear_with(
        &self,
        from: (u16, u16),
        to: (u16, u16),
        scale: i64,
        avoid: &[Clear],
    ) -> bool {
        // **The clearance first**: a straight line between two tile centres that passes inside a body's clearance would be
        // pulled back through the man the plan went round, which is how the first version of this rule failed.
        let (from_raw, to_raw) = (
            (
                i64::from(from.0) * scale + scale / 2,
                i64::from(from.1) * scale + scale / 2,
            ),
            (
                i64::from(to.0) * scale + scale / 2,
                i64::from(to.1) * scale + scale / 2,
            ),
        );
        if avoid.iter().any(|clear| clear.crossed_by(from_raw, to_raw)) {
            return false;
        }
        let (mut x, mut y) = (i64::from(from.0), i64::from(from.1));
        let (x1, y1) = (i64::from(to.0), i64::from(to.1));
        let (dx, dy) = ((x1 - x).abs(), -(y1 - y).abs());
        let (sx, sy) = (if x < x1 { 1 } else { -1 }, if y < y1 { 1 } else { -1 });
        let mut err = dx + dy;
        loop {
            if (x, y) == (x1, y1) {
                return true;
            }
            let twice = 2 * err;
            let step_x = twice >= dy;
            let step_y = twice <= dx;
            if step_x {
                err += dy;
                x += sx;
            }
            if step_y {
                err += dx;
                y += sy;
            }
            // The tile the line has arrived at.
            let last = x == x1 && y == y1;
            let (Ok(tx), Ok(ty)) = (u16::try_from(x), u16::try_from(y)) else {
                return false;
            };
            if !last && (!self.walkable(tx, ty) || self.inside_a_body(scale, (tx, ty), avoid)) {
                return false;
            }
            // **A diagonal does not cut a corner.** Both tiles the step squeezes between must be free, exactly as in the
            // search - otherwise a "clear line" certifies the squeeze the search refuses. This applies to the **last** step
            // too: arriving diagonally still squeezes through two tiles, and the goal's own exemption is about its tile, not
            // about the corner.
            if step_x && step_y {
                let (Ok(ax), Ok(ay)) = (u16::try_from(x - sx), u16::try_from(y)) else {
                    return false;
                };
                let (Ok(bx), Ok(by)) = (u16::try_from(x), u16::try_from(y - sy)) else {
                    return false;
                };
                if !self.walkable(ax, ay)
                    || !self.walkable(bx, by)
                    || self.inside_a_body(scale, (ax, ay), avoid)
                    || self.inside_a_body(scale, (bx, by), avoid)
                {
                    return false;
                }
            }
        }
    }

    /// **The route between two points**, as the polyline a body walks: the goal itself when a straight line reaches it,
    /// and otherwise the corners of the tile search, with the last point the exact place that was asked for.
    ///
    /// `None` only when the start is off the map or no tile at all can be reached - a world with no ground for the body to
    /// stand on. A goal that is walled in is **not** such a case: the search settles on the nearest cell it can reach and
    /// the route ends there, which is the original's "come as close as you can".
    pub fn route(
        &self,
        scale: i64,
        from: (Fixed, Fixed),
        to: (Fixed, Fixed),
    ) -> Option<Vec<(Fixed, Fixed)>> {
        self.route_with(scale, from, to, &[])
    }

    /// **The same route, with tiles a body is standing on treated as blocked** (D281). This is the original's answer to a
    /// collider that is not *past*: its epilogue counts the collision and **re-plans** (`"We have %d non-past collisions ...
    /// repathing"`), and the tiles its own bodies hold are what the plan has to get round. The list is bodies only - terrain
    /// is already in the grid.
    pub fn route_avoiding(
        &self,
        scale: i64,
        from: (Fixed, Fixed),
        to: (Fixed, Fixed),
        avoid: &[Clear],
    ) -> Option<Vec<(Fixed, Fixed)>> {
        self.route_with(scale, from, to, avoid)
    }

    fn route_with(
        &self,
        scale: i64,
        from: (Fixed, Fixed),
        to: (Fixed, Fixed),
        avoid: &[Clear],
    ) -> Option<Vec<(Fixed, Fixed)>> {
        let start = self.tile_of(scale, from)?;
        let goal = self.tile_of(scale, to)?;
        // A straight walk needs no chain: the search's own answer would be pulled back to this one point anyway. Not when
        // tiles are to be avoided: the line test knows nothing about bodies, and a straight line through a collider is
        // exactly the step the caller is giving up.
        if avoid.is_empty() && self.walkable(goal.0, goal.1) && self.line_clear(start, goal) {
            return Some(vec![to]);
        }
        let steps = self.search(start, goal, scale, avoid)?;
        let end = steps.last().copied().unwrap_or(start);
        let pulled = self.pull(start, &steps, scale, avoid);
        let half = scale / 2;
        let mut points: Vec<(Fixed, Fixed)> = pulled
            .iter()
            .map(|(x, y)| {
                (
                    Fixed::from_raw(i64::from(*x) * scale + half),
                    Fixed::from_raw(i64::from(*y) * scale + half),
                )
            })
            .collect();
        // **Where the body will actually stand**: the clicked point when the route ends on its tile, and the settled
        // tile's centre when the ground stopped it short.
        if let Some(last) = points.last_mut() {
            *last = if end == goal { to } else { *last };
        }
        Some(points)
    }

    /// **Is this tile's centre inside a body's clearance?** A tile the mover cannot stand on without touching the body it is
    /// going round, expressed on the grid the search actually walks.
    fn inside_a_body(&self, scale: i64, tile: (u16, u16), avoid: &[Clear]) -> bool {
        if avoid.is_empty() {
            return false;
        }
        let centre = (
            i64::from(tile.0) * scale + scale / 2,
            i64::from(tile.1) * scale + scale / 2,
        );
        avoid.iter().any(|clear| clear.still_inside(centre))
    }

    /// The tile a raw point stands in, when it is on the map.
    fn tile_of(&self, scale: i64, point: (Fixed, Fixed)) -> Option<(u16, u16)> {
        let x = u16::try_from(point.0.raw().div_euclid(scale)).ok()?;
        let y = u16::try_from(point.1.raw().div_euclid(scale)).ok()?;
        self.index(x, y).map(|_| (x, y))
    }

    /// Spend the waypoints a clear line lets the body skip, so what is left is the corners the terrain forces. The
    /// original smooths its waypoint list for the same reason.
    fn pull(
        &self,
        start: (u16, u16),
        steps: &[(u16, u16)],
        scale: i64,
        avoid: &[Clear],
    ) -> Vec<(u16, u16)> {
        if steps.len() < 2 {
            return steps.to_vec();
        }
        let mut nodes = Vec::with_capacity(steps.len() + 1);
        nodes.push(start);
        nodes.extend_from_slice(steps);
        let last = nodes.len() - 1;
        let mut pulled = Vec::with_capacity(nodes.len());
        let mut at = 0;
        while at < last {
            let mut jump = last;
            while jump > at + 1 && !self.line_clear_with(nodes[at], nodes[jump], scale, avoid) {
                jump -= 1;
            }
            pulled.push(nodes[jump]);
            at = jump;
        }
        pulled
    }

    /// **A\* over the tiles.** Returns the tiles after `start`, ending on the goal when it can be reached and on the
    /// nearest cell that can be otherwise.
    ///
    /// The original's costs and estimate; a diagonal step is not allowed to cut a corner (both orthogonal neighbours must
    /// be walkable), which is the same rule the line test and the search have always shared.
    pub fn search(
        &self,
        start: (u16, u16),
        goal: (u16, u16),
        scale: i64,
        avoid: &[Clear],
    ) -> Option<Vec<(u16, u16)>> {
        let width = usize::from(self.width);
        let count = width * usize::from(self.height);
        let index = |tile: (u16, u16)| usize::from(tile.1) * width + usize::from(tile.0);
        let tile = |at: usize| {
            (
                u16::try_from(at % width).unwrap_or(0),
                u16::try_from(at / width).unwrap_or(0),
            )
        };
        let estimate = |tile: (u16, u16)| Self::estimate(tile, goal);
        let start_at = index(start);
        let mut best = vec![u32::MAX; count];
        let mut from: Vec<Option<usize>> = vec![None; count];
        let mut open = BinaryHeap::new();
        best[start_at] = 0;
        open.push(Reverse((estimate(start), 0u32, start.1, start.0)));
        // The closest cell the search ever stands on: the answer when the goal cannot be reached.
        let mut settled_best: Option<(u32, u32, usize)> = None;
        while let Some(Reverse((_, g, y, x))) = open.pop() {
            let here = (x, y);
            let here_at = index(here);
            // A stale entry: the node has already been reached more cheaply.
            if g > best[here_at] {
                continue;
            }
            let h = estimate(here);
            // **The closest cell the search stands on**, which is the original's answer to a goal it cannot reach. The start
            // is not a candidate: "come as close as you can" must move the body, not leave it where it stands. Ties go to
            // the cheaper cell, so the settled point is on the shortest path the search found, not merely near the goal.
            if here_at != start_at
                && settled_best.is_none_or(|(old_h, old_g, _)| (h, g) < (old_h, old_g))
            {
                settled_best = Some((h, g, here_at));
            }
            if here == goal {
                settled_best = Some((0, g, here_at));
                break;
            }
            for (dx, dy) in NEIGHBOURS {
                let (Some(nx), Some(ny)) =
                    (i64::from(x).checked_add(dx), i64::from(y).checked_add(dy))
                else {
                    continue;
                };
                let (Ok(nx), Ok(ny)) = (u16::try_from(nx), u16::try_from(ny)) else {
                    continue;
                };
                if !self.walkable(nx, ny) || self.inside_a_body(scale, (nx, ny), avoid) {
                    continue;
                }
                // **No corner cutting.** A diagonal enters two tiles; both must be free, or the body clips the corner of
                // the wall the route was planned round.
                if dx != 0 && dy != 0 {
                    let (Ok(sx), Ok(sy)) = (
                        u16::try_from(i64::from(x) + dx),
                        u16::try_from(i64::from(y) + dy),
                    ) else {
                        continue;
                    };
                    if !self.walkable(sx, y)
                        || !self.walkable(x, sy)
                        || self.inside_a_body(scale, (sx, y), avoid)
                        || self.inside_a_body(scale, (x, sy), avoid)
                    {
                        continue;
                    }
                }
                let next_at = index((nx, ny));
                let step = g + Self::step_cost(here, (nx, ny));
                if step < best[next_at] {
                    best[next_at] = step;
                    from[next_at] = Some(here_at);
                    open.push(Reverse((step + estimate((nx, ny)), step, ny, nx)));
                }
            }
        }
        let (_, _, end) = settled_best?;
        if end == start_at {
            return None;
        }
        let mut path = vec![tile(end)];
        let mut at = end;
        while let Some(previous) = from[at] {
            if previous == start_at {
                break;
            }
            path.push(tile(previous));
            at = previous;
        }
        path.reverse();
        Some(path)
    }
}

/// The eight steps a body may take, in a fixed order so a tie is broken the same way on every run.
const NEIGHBOURS: [(i64, i64); 8] = [
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];
