//! **The ground and the router** (D279, New Way Task 5): the original's own model, ported bluntly,
//! routed geometrically.
//!
//! Two questions, and nothing else in this file:
//!
//! * **may a body stand on this tile** - terrain, plus however many things have been stamped on it;
//! * **which way does a walk go** - one call to the geometric solver over the blocked tiles as grown
//!   boxes, whose answer is handed back as world points.
//!
//! **What is deliberately absent.** The tile search and everything it carried: the **levels** and their refinement
//! (D296-D302, D322), the **expansion budget** (D304) and the **key** `g + octile` (D337), the **tier arm** with its
//! smoothing (D301, D338, D344-D347), and the **300 row** beside mines (D342). The cost of a route is not a gate anywhere in the
//! engine; the fixtures check shape (reached, never crossed, stopped beside) rather than an optimal length.
//! The step costs, the octile and the estimate stay: they are the original's cost table, pinned by the conformance set.
//!
//! **Determinism.** The solver walks slice and coordinate order, never hash order, so two runs agree on every waypoint.

use ruage_data::Map;
use ruage_fixed::Fixed;

use crate::geometry::{WayRoute, solve, verify_path};

/// An obstacle the plan must keep clear of (D282, D286): the object's **box**, and the mover's own reach.
///
/// The original puts a stopped object into the plan as a rectangle **grown by the mover's radius** (`FUN_007f9d80`) and
/// grows the collision query box the same way (`FUN_00840830`, `+0x268`), so the clearance is carried beside the grid:
/// the solver grows every box by the mover's radius before it walks.
///
/// **A box, not a disc** (D286). A disc of 0.7 forbids a one-tile gap that the game lets a 0.2-radius body through, and the
/// game's own test is an AABB against an AABB. `half` is the obstacle's own half-extents - a body's radius, a tile's half -
/// and `reach` is the mover's radius, so the grown box is `half + reach` on each axis. Touching the grown face is clear:
/// a body at exactly its radius from a wall is at the game's own contact point.
#[derive(Clone, Copy, Debug)]
pub struct Clear {
    /// The obstacle's centre, in raw units.
    pub at: (i64, i64),
    /// The obstacle's own half-extents, in raw units: a body's radius, or half a tile for a grid object.
    pub half: (i64, i64),
    /// The mover's own reach: how far its body stands from that box.
    pub reach: i64,
}

/// The original's cost of one cardinal step (`docs/archive/research/movement-in-the-binary.md`, `FUN_008007f0`).
pub const CARDINAL: u32 = 16;
/// The same table's diagonal: `16 x sqrt2`, rounded.
pub const DIAGONAL: u32 = 23;
/// **The estimate: Euclidean tile distance x 25** (`FUN_008007f0`): the original's number for "how far", kept
/// beside the cost table it belongs to and pinned by the conformance set.
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

    /// **How many times the pyramid has been built: zero, always.** The tile system is gone - no levels, no cache,
    /// no search - so there is nothing to build. Kept as a method (rather than deleted with the pyramid) because the
    /// geometry-movers witness pins the count at every reach: a route that built one would be the tile system creeping back.
    #[must_use]
    pub fn pyramid_builds(&self) -> u64 {
        0
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

    /// **Does an *object* stand on this tile** - a tree, a wall, a building - as opposed to impassable terrain?
    ///
    /// The distinction is the game's: terrain is a point test with no radius (`FUN_008aa480`), while every object has a box
    /// that the mover's body may not enter (`FUN_00840830`). So this is what the box rules read, and `walkable` keeps both.
    fn occupied(&self, x: u16, y: u16) -> bool {
        self.index(x, y)
            .is_some_and(|at| !self.terrain[at] && self.occupants[at] > 0)
    }

    /// **Which object tiles this body touches** - the game's own query (`FUN_00840830`: the box grown by the mover's radius,
    /// colliding on `<=`, so **edges count as collision**). Written into `out` so the step can ask twice a tick without
    /// allocating.
    fn objects_touched(
        &self,
        scale: i64,
        point: (i64, i64),
        reach: i64,
        out: &mut Vec<(u16, u16)>,
    ) {
        out.clear();
        // A grown box reaches `half + reach` from its tile's centre, so at the widest body the dat carries (a trebuchet at
        // half a tile) it touches the next tile's centre and no further: three by three covers every body there is.
        debug_assert!(
            reach <= scale / 2,
            "a body wider than half a tile cannot be tested against a three-by-three neighbourhood: reach {reach}, scale {scale}"
        );
        let grown = scale / 2 + reach;
        let (tx, ty) = (point.0.div_euclid(scale), point.1.div_euclid(scale));
        for dy in -1..=1i64 {
            for dx in -1..=1i64 {
                let (Ok(x), Ok(y)) = (u16::try_from(tx + dx), u16::try_from(ty + dy)) else {
                    continue;
                };
                if !self.occupied(x, y) {
                    continue;
                }
                let centre = (
                    i64::from(x) * scale + scale / 2,
                    i64::from(y) * scale + scale / 2,
                );
                if (point.0 - centre.0).abs() <= grown && (point.1 - centre.1).abs() <= grown {
                    out.push((x, y));
                }
            }
        }
    }

    /// **Does a body standing here touch an object?** The single-question form, for the step and the conformance set.
    pub fn touches_object(&self, scale: i64, point: (i64, i64), reach: i64) -> bool {
        let mut touched = Vec::new();
        self.objects_touched(scale, point, reach, &mut touched);
        !touched.is_empty()
    }

    /// **How deep is a body's box inside an object's, in raw units?** Zero when it only touches, which is the game's own
    /// collision point; positive is penetration, and the largest figure wins.
    ///
    /// The original's query is a yes/no - the boxes intersect - and its placement phase (`func_002699A0`) is what keeps a body
    /// out of an object, so a body that is *already* inside one never arises there. Ours can arise (a fixture or a spawn
    /// places a body on a tile corner, half inside the tree beside it), and a plain yes/no then refuses **every** step, so the
    /// body never moves at all: measured, a corridor pair spawned on tile corners overlapped the tree row and stood for
    /// 2,000 ticks. So the step asks the game's question of the *change* - a step may not take a body **deeper** into an object
    /// it already touches - which is identical wherever a body starts outside one, and lets a body that starts inside walk
    /// out.
    pub(crate) fn object_depth(&self, scale: i64, point: (i64, i64), reach: i64) -> i64 {
        debug_assert!(
            reach <= scale / 2,
            "a body wider than half a tile cannot be tested against a three-by-three neighbourhood: reach {reach}, scale {scale}"
        );
        let grown = scale / 2 + reach;
        let (tx, ty) = (point.0.div_euclid(scale), point.1.div_euclid(scale));
        let mut deepest = 0;
        for dy in -1..=1i64 {
            for dx in -1..=1i64 {
                let (Ok(x), Ok(y)) = (u16::try_from(tx + dx), u16::try_from(ty + dy)) else {
                    continue;
                };
                if !self.occupied(x, y) {
                    continue;
                }
                let centre = (
                    i64::from(x) * scale + scale / 2,
                    i64::from(y) * scale + scale / 2,
                );
                let (across, down) = (
                    grown - (point.0 - centre.0).abs(),
                    grown - (point.1 - centre.1).abs(),
                );
                if across > 0 && down > 0 {
                    deepest = deepest.max(across.min(down));
                }
            }
        }
        deepest
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

    /// **The original's octile distance between two tiles** (`FUN_007ffc80`): `23 x the diagonal share + 16 x the rest`,
    /// i.e. `16 x max + 7 x min`. Public so the conformance set can check the number rather than trust it.
    pub fn octile(from: (u16, u16), to: (u16, u16)) -> u32 {
        let dx = u32::from(from.0.abs_diff(to.0));
        let dy = u32::from(from.1.abs_diff(to.1));
        let (long, short) = (dx.max(dy), dx.min(dy));
        CARDINAL * long + DIAGONAL * short
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
    /// crossing the line takes must have both of its side tiles free, so a straight line and a route never disagree about
    /// a corner.
    ///
    /// Bresenham in integers, so the answer is exact and identical everywhere.
    pub fn line_clear(&self, from: (u16, u16), to: (u16, u16)) -> bool {
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
            if !last && !self.walkable(tx, ty) {
                return false;
            }
            // **A diagonal does not cut a corner.** Both tiles the step squeezes between must be free. This applies to
            // the **last** step too: arriving diagonally still squeezes through two tiles, and the goal's own exemption
            // is about its tile, not about the corner.
            if step_x && step_y {
                let (Ok(ax), Ok(ay)) = (u16::try_from(x - sx), u16::try_from(y)) else {
                    return false;
                };
                let (Ok(bx), Ok(by)) = (u16::try_from(x), u16::try_from(y - sy)) else {
                    return false;
                };
                if !self.walkable(ax, ay) || !self.walkable(bx, by) {
                    return false;
                }
            }
        }
    }

    /// **The route between two points**, as the polyline a body walks: the solver's answer, ending on the exact goal
    /// when the walk reaches it and stopping beside it when the goal is walled in - the original's "come as close as
    /// you can".
    ///
    /// `None` only when the start or the goal is off the map: there is no ground to stand on. A goal that is walled in
    /// is **not** such a case: the route ends beside it instead.
    pub fn route(
        &self,
        scale: i64,
        from: (Fixed, Fixed),
        to: (Fixed, Fixed),
    ) -> Option<Vec<(Fixed, Fixed)>> {
        self.route_with(scale, from, to, 0, &[], true)
    }

    /// **The same route, with the bodies a mover must keep clear of as grown boxes** (D281, D282). This is the original's
    /// answer to a collider that is not *past*: its epilogue counts the collision and **re-plans**
    /// (`"We have %d non-past collisions ... repathing"`), and the bodies its own standers hold are what the plan has to
    /// get round. The list is bodies only - terrain and objects come from the grid below.
    pub fn route_avoiding(
        &self,
        scale: i64,
        from: (Fixed, Fixed),
        to: (Fixed, Fixed),
        reach: i64,
        avoid: &[Clear],
        foray_near: bool,
    ) -> Option<Vec<(Fixed, Fixed)>> {
        self.route_with(scale, from, to, reach, avoid, foray_near)
    }

    /// **Every blocked tile near the walk, as a grown box, plus the map's own border as four walls**
    /// (New Way Task 5): the land mover's rect source. Terrain and objects share one grid, so one pass collects
    /// both - a tile the body may not stand on is a tile the walk goes round. The border is a wall the same way:
    /// the map edge stops every walk (the walkability rule), so the strips outside it are blocked ground and the
    /// solver can neither cut a corner off the map nor round an obstacle that touches the edge by leaving it.
    /// Each rect carries the mover's own reach, the same shape as the stander rects the caller hands over, so the
    /// solver's `max` growth is the identity and every box is grown exactly once.
    ///
    /// Only the walk's neighbourhood is collected: the solver never walks outside the from/to box plus its own search
    /// margin, so a tile further out cannot change the answer. Deterministic: row by row, left to right, then the
    /// four border strips west, east, north, south.
    fn solid_rects(
        &self,
        scale: i64,
        reach: i64,
        from_raw: (i64, i64),
        to_raw: (i64, i64),
    ) -> Vec<Clear> {
        let margin = scale
            .saturating_mul(8)
            .saturating_add(scale)
            .saturating_add(reach.max(0));
        let (lo_x, hi_x) = (
            from_raw.0.min(to_raw.0).saturating_sub(margin),
            from_raw.0.max(to_raw.0).saturating_add(margin),
        );
        let (lo_y, hi_y) = (
            from_raw.1.min(to_raw.1).saturating_sub(margin),
            from_raw.1.max(to_raw.1).saturating_add(margin),
        );
        let half = scale / 2;
        let mut rects = Vec::new();
        for y in lo_y.div_euclid(scale)..=hi_y.div_euclid(scale) {
            for x in lo_x.div_euclid(scale)..=hi_x.div_euclid(scale) {
                let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y)) else {
                    continue;
                };
                if !self.blocked(x, y) {
                    continue;
                }
                rects.push(Clear {
                    at: (i64::from(x) * scale + half, i64::from(y) * scale + half),
                    half: (half, half),
                    reach,
                });
            }
        }
        // The border strips, clipped to the walk's neighbourhood: everything past the map's edge is ground
        // nothing may enter. Each strip stops `reach` short of the edge, so that grown by the mover's radius
        // its face lands exactly on the map's border: a walk that starts on the border touches the grown face
        // instead of standing inside it, and the foray's corners on it are excluded as inside ground. A strip
        // is only pushed when the neighbourhood actually reaches past its edge - otherwise it would be a
        // degenerate box no walk can meet.
        let (edge_w, edge_h) = (
            i64::from(self.width) * scale,
            i64::from(self.height) * scale,
        );
        // `at`/`half` covering `[min, max]` exactly with integer division: `half` rounds up, so the box
        // always covers both ends.
        let covering = |min: i64, max: i64| ((min + max) / 2, (max - min + 1) / 2);
        let mut strip = |min_x: i64, max_x: i64, min_y: i64, max_y: i64| {
            if min_x < max_x && min_y < max_y {
                let (at_x, half_x) = covering(min_x, max_x);
                let (at_y, half_y) = covering(min_y, max_y);
                rects.push(Clear {
                    at: (at_x, at_y),
                    half: (half_x, half_y),
                    reach,
                });
            }
        };
        let inset = reach.max(0);
        strip(lo_x.min(-inset), -inset, lo_y, hi_y);
        strip(edge_w + inset, hi_x.max(edge_w + inset), lo_y, hi_y);
        strip(lo_x, hi_x, lo_y.min(-inset), -inset);
        strip(lo_x, hi_x, edge_h + inset, hi_y.max(edge_h + inset));
        rects
    }

    fn route_with(
        &self,
        scale: i64,
        from: (Fixed, Fixed),
        to: (Fixed, Fixed),
        reach: i64,
        avoid: &[Clear],
        foray_near: bool,
    ) -> Option<Vec<(Fixed, Fixed)>> {
        let _start = self.tile_of(scale, from)?;
        let _goal = self.tile_of(scale, to)?;
        // **Every mover routes through the geometric solver** (New Way Task 5): the blocked tiles near the walk join
        // the caller's stander rects, and `solve` answers for the whole set at once. `solve` grows each rect by
        // `max(clear.reach, reach)`, which is exact for every caller: `solid_rects` above and `standing_bodies`
        // stamp the mover's own reach into the rects they hand over, so `max` is the identity there, and a bare
        // rect with the reach carried separately is grown exactly once.
        // The verdict maps onto the plan's tail: `Reached` ends on the exact goal point, `Closest` settles on the
        // tile centre - the flat search's old best-settled rule. The snap is taken only when the centre and its leg
        // stay `verify_path`-clean against the grown boxes: the functional gates (never stuck, never through a blocked
        // area) outrank the tail shape, so a snap with no clean centre keeps the solver's certified endpoint instead.
        // The standing point is not a waypoint: the movement pass proposes a zero step when the next waypoint is where
        // the body already stands and never reaches the walk that would spend it, so a route whose first point is the
        // walk's own start drops it. A ring exit the solver stepped out to is ahead and stays.
        let from_raw = (from.0.raw(), from.1.raw());
        let to_raw = (to.0.raw(), to.1.raw());
        let terrain = self.solid_rects(scale, reach, from_raw, to_raw);
        let mut points = match solve(scale, from, to, reach, &terrain, avoid, foray_near) {
            WayRoute::Reached(mut points) => {
                if let Some(last) = points.last_mut() {
                    *last = to;
                }
                points
            }
            WayRoute::Closest(mut points) => {
                // The grown boxes the snap is certified against: the same `max` the solver grew by.
                let grown: Vec<Clear> = terrain
                    .iter()
                    .chain(avoid.iter())
                    .map(|clear| Clear {
                        at: clear.at,
                        half: clear.half,
                        reach: clear.reach.max(reach),
                    })
                    .collect();
                // The walked legs, starting where the body stands: the snap replaces the last one, so the
                // certification covers the leg the body will actually walk first as well as the snapped one.
                let walked: Vec<(i64, i64)> = std::iter::once(from_raw)
                    .chain(points.iter().map(|point| (point.0.raw(), point.1.raw())))
                    .collect();
                // A settled tile is standable ground, and the whole polyline with the snap in place
                // must certify - the centre and every leg, including the snapped one.
                let certifies = |centre: (Fixed, Fixed)| -> bool {
                    let mut candidate = walked.clone();
                    if let Some(end) = candidate.last_mut() {
                        *end = (centre.0.raw(), centre.1.raw());
                    }
                    verify_path(&candidate, &grown).is_none()
                };
                let centre_of = |tile: (u16, u16)| {
                    (
                        Fixed::from_raw(i64::from(tile.0) * scale + scale / 2),
                        Fixed::from_raw(i64::from(tile.1) * scale + scale / 2),
                    )
                };
                // The solver stops on a grown face, which a tile query reads as one of the tiles sharing it -
                // possibly a blocked one the body could never stand on. The settled tile is the walkable ground
                // beside the stop: its own tile first (the flat search's old best-settled rule), and only when
                // that tile itself is not standable, the walkable neighbours nearest the goal. A standable but
                // dirty centre keeps the solver's certified endpoint instead of settling somewhere farther away.
                let settled = points.last().copied().and_then(|last| {
                    let settled_tile = self.tile_of(scale, last)?;
                    // A stop on walkable ground within working reach of the goal stands where it is: the
                    // step stops the body at contact, and settling it back on a tile centre holds it a tile
                    // away - it paces at the centre, refused every push, instead of standing against the wall
                    // it was sent to. Only a stop on unstandable ground, or far from the goal, is settled.
                    let (dx, dy) = (last.0.raw() - to.0.raw(), last.1.raw() - to.1.raw());
                    let near = dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy))
                        <= (2 * scale).saturating_mul(2 * scale);
                    if near && self.walkable(settled_tile.0, settled_tile.1) {
                        return None;
                    }
                    let goal_raw = (to.0.raw(), to.1.raw());
                    let mut neighbours: Vec<(u16, u16)> = Vec::new();
                    for dy in -1..=1i64 {
                        for dx in -1..=1i64 {
                            if dx == 0 && dy == 0 {
                                continue;
                            }
                            let (Ok(nx), Ok(ny)) = (
                                u16::try_from(i64::from(settled_tile.0) + dx),
                                u16::try_from(i64::from(settled_tile.1) + dy),
                            ) else {
                                continue;
                            };
                            if self.index(nx, ny).is_some() {
                                neighbours.push((nx, ny));
                            }
                        }
                    }
                    neighbours.sort_by_key(|tile| {
                        let centre = centre_of(*tile);
                        let (dx, dy) = (centre.0.raw() - goal_raw.0, centre.1.raw() - goal_raw.1);
                        // `saturating_mul` keeps a whole-map diagonal inside `i64`: `dx * dx` alone can overflow it.
                        (
                            dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy)),
                            tile.0,
                            tile.1,
                        )
                    });
                    std::iter::once(settled_tile)
                        .chain(neighbours)
                        .map(|tile| (tile, centre_of(tile)))
                        .find(|(tile, centre)| {
                            // The settled tile settles the walk when it is standable and its leg is clean;
                            // a standable tile behind a wall is no stand to walk to, so a neighbour with a
                            // clean leg settles it instead - measured, a walk stalled on a pocket wall kept
                            // the grown-face point because its own tile's centre sat behind the wall, while
                            // the next tile over certified clean. Own tile first: a neighbour only wins when
                            // the settled tile cannot be stood on or reached.
                            self.walkable(tile.0, tile.1) && certifies(*centre)
                        })
                        .map(|(_, centre)| centre)
                });
                if let (Some(centre), Some(end)) = (settled, points.last_mut()) {
                    *end = centre;
                }
                points
            }
        };
        if points.len() > 1 && points.first() == Some(&from) {
            points.remove(0);
        }
        Some(points)
    }

    /// The tile a raw point stands in, when it is on the map.
    fn tile_of(&self, scale: i64, point: (Fixed, Fixed)) -> Option<(u16, u16)> {
        let x = u16::try_from(point.0.raw().div_euclid(scale)).ok()?;
        let y = u16::try_from(point.1.raw().div_euclid(scale)).ok()?;
        self.index(x, y).map(|_| (x, y))
    }
}
