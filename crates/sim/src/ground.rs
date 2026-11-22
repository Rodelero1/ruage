//! **The ground and the search** (D279): the original's own model, ported bluntly.
//!
//! Two questions, and nothing else in this file:
//!
//! * **may a body stand on this tile** - terrain, plus however many things have been stamped on it;
//! * **which tiles does a walk cross** - one A\* over that grid, at the original's cost model, whose answer is pulled to
//!   the corners an obstacle actually forces and handed back as world points.
//!
//! **What is deliberately absent.** The search window (`in_ECX[0x42..0x45]`, unread padding - D299) and the polygonal
//! short-range pathfinder (P11's third solver, a project). Everything else the original's search carries arrived with a
//! decision: the **levels** and their refinement (D296-D302, D322), the mover's **radius** in the plan (D306), the body list as
//! **`Clear` rectangles** (D282/D331), the **expansion budget** (D304) and the **key** `g + octile` (D337). The cost of a route is not a gate anywhere in the
//! engine; the fixtures check shape (reached, never crossed, stopped beside) rather than an optimal length.
//!
//! **Determinism.** The open set is a binary heap ordered by `(f, g, y, x)`, and the predecessor table is an array
//! indexed by tile, so two runs expand the same nodes in the same order. No hash-map iteration reaches the result.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use ruage_data::Map;
use ruage_fixed::Fixed;

use crate::integer_sqrt;

/// **An obstacle the plan must keep clear of** (D282, D286): the object's **box**, and the mover's own reach.
///
/// The original puts a stopped object into the search as a rectangle **grown by the mover's radius** (`FUN_007f9d80`) and
/// grows the collision query box the same way (`FUN_00840830`, `+0x268`). A tile grid cannot express a 0.4-tile clearance,
/// so the clearance is carried beside the grid: a tile whose centre is inside it is not planned through, and a straightening
/// jump that passes within it is refused.
///
/// **A box, not a disc** (D286). A disc of 0.7 forbids a one-tile gap that the game lets a 0.2-radius body through, and the
/// game's own test is an AABB against an AABB. `half` is the obstacle's own half-extents - a body's radius, a tile's half -
/// and `reach` is the mover's radius, so the grown box is `half + reach` on each axis.
#[derive(Clone, Copy, Debug)]
pub struct Clear {
    /// The obstacle's centre, in raw units.
    pub at: (i64, i64),
    /// The obstacle's own half-extents, in raw units: a body's radius, or half a tile for a grid object.
    pub half: (i64, i64),
    /// The mover's own reach: how far its body stands from that box.
    pub reach: i64,
}

/// **What a tier route costs against the flat search's answer for the same walk** (D344). Measurement only: nothing reads it,
/// it is never hashed, and it exists because "the tier answered" says nothing about whether the route it answered with is
/// better. Lengths are raw units along the polyline from the walk's start through the route's points, so they compare like for
/// like; `flat_missing` counts answers the flat search could not produce at all.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TierShadow {
    pub walks: u64,
    pub tier_points: u64,
    pub tier_length: u64,
    pub flat_points: u64,
    pub flat_length: u64,
    pub flat_missing: u64,
}

/// **Why the tier did not answer** (D344), counted where the tier stands: a long walk that never reached the arm, one the floor
/// refused, one whose `coarse_route` was `None`, one whose chain was a single cell, and one whose legs failed the avoid
/// certification. Without these, "the tier answered 0" is a number with no mechanism behind it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TierFunnel {
    pub long_walks: u64,
    pub floor_refused: u64,
    pub no_coarse: u64,
    pub one_cell: u64,
    /// The chain the refinement emitted, and what the game's smoothing pass left of it (D346): the census that says whether
    /// the pass is doing the job the `move_obj.cpp` log lines say it does.
    pub waypoints_raw: u64,
    pub waypoints_smoothed: u64,
}

/// **Which question refused a leg** (D344), so a refused chain names its reason instead of its count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LegVerdict {
    Clear,
    Body,
    Object,
    Tiles,
}

/// **One plan request**, bundled so the tier, its certification and the flat fallback all read the same seven values instead of
/// passing them one by one (D344).
#[derive(Clone, Copy)]
struct Walk<'a> {
    from: (Fixed, Fixed),
    to: (Fixed, Fixed),
    start: (u16, u16),
    goal: (u16, u16),
    scale: i64,
    reach: i64,
    avoid: &'a [Clear],
}

impl Ground {
    /// **The tier's answer beside the flat search's for the same walk** (D344): run it here, where the route, the two tiles and
    /// the plan's avoid list are all still in hand.
    fn record_shadow(&self, walk: Walk<'_>, tier: &[(Fixed, Fixed)]) {
        let from = walk.from;
        let mut shadow = self.tier_shadow.get();
        shadow.walks += 1;
        shadow.tier_points += u64::try_from(tier.len()).unwrap_or(u64::MAX);
        shadow.tier_length += route_length(from, tier);
        match self.flat_route(walk) {
            Some(flat) => {
                shadow.flat_points += u64::try_from(flat.len()).unwrap_or(u64::MAX);
                shadow.flat_length += route_length(from, &flat);
            }
            None => shadow.flat_missing += 1,
        }
        self.tier_shadow.set(shadow);
    }
}

/// The polyline's length in raw units, from `from` through every point. Integer arithmetic, like everything the tick does.
fn route_length(from: (Fixed, Fixed), points: &[(Fixed, Fixed)]) -> u64 {
    let mut total = 0i64;
    let mut previous = (from.0.raw(), from.1.raw());
    for point in points {
        let (dx, dy) = (point.0.raw() - previous.0, point.1.raw() - previous.1);
        total += integer_sqrt(dx * dx + dy * dy);
        previous = (point.0.raw(), point.1.raw());
    }
    u64::try_from(total).unwrap_or(u64::MAX)
}

impl Clear {
    /// The obstacle grown by the mover's body on every side - the rectangle the search and the step both answer for.
    fn grown(&self) -> (i64, i64) {
        (self.half.0 + self.reach, self.half.1 + self.reach)
    }

    /// **Is this raw point inside the grown box?** Touching is *not* inside: a body at exactly its radius from a wall is at
    /// the game's own contact point, which is where a walk ends. A test that refused touching could never arrive.
    fn still_inside(&self, point: (i64, i64)) -> bool {
        let (hx, hy) = self.grown();
        (point.0 - self.at.0).abs() < hx && (point.1 - self.at.1).abs() < hy
    }

    /// **Does the straight segment between two raw points pass inside the grown box?** A segment against an axis-aligned
    /// rectangle, by separating axis - the two axis normals and the segment's own normal - in integers, so the answer is
    /// exact and identical everywhere. A segment that runs along a face touches it and does not cross.
    fn crossed_by(&self, from: (i64, i64), to: (i64, i64)) -> bool {
        let (hx, hy) = self.grown();
        let (min_x, max_x) = (self.at.0 - hx, self.at.0 + hx);
        let (min_y, max_y) = (self.at.1 - hy, self.at.1 + hy);
        // Axis x, then axis y: the segment's own span against the box's.
        let (sx0, sx1) = (from.0.min(to.0), from.0.max(to.0));
        let (sy0, sy1) = (from.1.min(to.1), from.1.max(to.1));
        if sx1 <= min_x || sx0 >= max_x || sy1 <= min_y || sy0 >= max_y {
            return false;
        }
        // The segment's normal: the box's four corners projected onto it, against the segment's own two ends.
        let (dx, dy) = (
            i128::from(to.0) - i128::from(from.0),
            i128::from(to.1) - i128::from(from.1),
        );
        let along = |x: i64, y: i64| -dy * i128::from(x) + dx * i128::from(y);
        let (ends_lo, ends_hi) = {
            let (a, b) = (along(from.0, from.1), along(to.0, to.1));
            (a.min(b), a.max(b))
        };
        let mut corners = [
            along(min_x, min_y),
            along(max_x, min_y),
            along(min_x, max_y),
            along(max_x, max_y),
        ];
        corners.sort_unstable();
        ends_hi > corners[0] && ends_lo < corners[3]
    }
}

/// The original's cost of one cardinal step (`docs/movement-in-the-binary.md`, `FUN_008007f0`).
pub const CARDINAL: u32 = 16;
/// The same table's diagonal: `16 x sqrt2`, rounded.
pub const DIAGONAL: u32 = 23;
/// **The estimate: Euclidean tile distance x 25** - the original's **expansion budget** (`FUN_008007f0` line 198 compares it
/// with the expansion counter). It is **not** the frontier's key: that is `g + octile` (line 316) since D337, and using this
/// value as the key is what made every search greedy (28.9 expansions a search and 54% exhausting the budget, against 11.8
/// and 5.8% - D334/D337).
pub const ESTIMATE: u32 = 25;
/// **The hard ceiling on expansions** (`20000 < local_46c` at `0x800d57`): the estimate is the budget, and this is the most the
/// binary will ever spend on one search, whatever the distance asks for.
pub const EXPANSION_CAP: u32 = 20_000;

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
    /// **The tiles whose neighbour the search charges 300 for** (D342): a gold or stone mine, named by the pack. Kept
    /// apart from `occupants` because it is not "blocked" - a felled mine unblocks and this goes with it - and because
    /// the binary's row is the object's *class*, which the engine may not know (D17).
    /// `serde(default)` so a `Ground` serialised before D342 still loads, with every tile at the plain 10.
    #[serde(default)]
    snug: Vec<bool>,
    /// **The level pyramid, built on demand and dropped when the ground changes** (D301). `occupants` is touched only by *objects* -
    /// a tree, a building, a gate - never by a unit, whose body the step pass handles through `objects_touched`. So this cache is warm
    /// across every tick a unit walks and is rebuilt only when something is placed or removed.
    #[serde(skip)]
    pyramid: std::cell::RefCell<std::collections::BTreeMap<i64, std::rc::Rc<crate::Levels>>>,
    /// How many times the pyramid has been built: a diagnostic, the way `plan_count` is.
    #[serde(skip)]
    pyramid_builds: std::cell::Cell<u64>,
    /// **How many routes the levelled tier answered** (D338): instrumentation like `plan_count`, never hashed, so a slice that
    /// changes the tier's gate can say how often it fired instead of arguing that it must have. Zero on a map with no obstacles.
    #[serde(skip)]
    tier_answers: std::cell::Cell<u64>,
    /// **Measurement only** (D344): when a probe turns this on, every tier answer is priced beside the flat search's answer
    /// for *the same walk* - same ground, same start, same goal, same avoid list. That comparison is the only way to say
    /// whether the tier's route is better, and it cannot be reconstructed afterwards because the avoid list is a snapshot of
    /// the bodies standing there at plan time. Off by default; the normal path pays one `Cell` read per tier answer.
    #[serde(skip)]
    compare_tier: std::cell::Cell<bool>,
    #[serde(skip)]
    tier_shadow: std::cell::Cell<TierShadow>,
    #[serde(skip)]
    tier_funnel: std::cell::Cell<TierFunnel>,
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
            snug: vec![false; terrain.len()],
            terrain,
            pyramid: std::cell::RefCell::new(std::collections::BTreeMap::new()),
            pyramid_builds: std::cell::Cell::new(0),
            tier_answers: std::cell::Cell::new(0),
            compare_tier: std::cell::Cell::new(false),
            tier_shadow: std::cell::Cell::new(TierShadow::default()),
            tier_funnel: std::cell::Cell::new(TierFunnel::default()),
        }
    }

    pub fn size(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    /// **How many routes the levelled tier has answered** (D338). A diagnostic: the tier's gate is judged by this count, not by
    /// an argument that it must have fired.
    #[must_use]
    pub fn tier_answers(&self) -> u64 {
        self.tier_answers.get()
    }

    /// **Turn on the tier-vs-flat comparison** (D344). A probe's switch: it costs a second search per tier answer, so it is off
    /// unless a measurement asks for it.
    pub fn compare_tier_routes(&self, on: bool) {
        self.compare_tier.set(on);
    }

    /// What the comparison has seen so far, summed over every tier answer since it was switched on.
    #[must_use]
    pub fn tier_shadow(&self) -> TierShadow {
        self.tier_shadow.get()
    }

    /// **Why the tier did not answer**: the funnel, counted only while the comparison is on.
    #[must_use]
    pub fn tier_funnel(&self) -> TierFunnel {
        self.tier_funnel.get()
    }

    /// Add one to a funnel counter. Reads and writes the whole struct, because `Cell` cannot hand out a reference.
    fn bump_funnel(&self, change: impl FnOnce(&mut TierFunnel)) {
        let mut funnel = self.tier_funnel.get();
        change(&mut funnel);
        self.tier_funnel.set(funnel);
    }

    /// Is this tile on the map at all?
    pub fn contains(&self, x: u16, y: u16) -> bool {
        self.index(x, y).is_some()
    }

    /// **The level pyramid, built once and reused until the ground changes** (D301). `RefCell` because a search takes `&self` and the
    /// build is a cache, not a mutation of the ground's meaning.
    #[must_use]
    pub fn pyramid(&self) -> std::rc::Rc<crate::Levels> {
        self.pyramid_for(0)
    }

    /// **The pyramid for a body of this reach** (D306): the search's own question, since the original grows every obstruction by the mover's
    /// radius. Keyed by reach, so the few radii the dat carries cost one build each per ground change and nothing per search.
    #[must_use]
    pub fn pyramid_for(&self, reach: i64) -> std::rc::Rc<crate::Levels> {
        if let Some(levels) = self.pyramid.borrow().get(&reach) {
            return std::rc::Rc::clone(levels);
        }
        let built = std::rc::Rc::new(crate::Levels::for_radius(self, reach));
        self.pyramid_builds.set(self.pyramid_builds.get() + 1);
        self.pyramid
            .borrow_mut()
            .insert(reach, std::rc::Rc::clone(&built));
        built
    }

    /// **How many times the pyramid has been built**, for a probe or a test.
    #[must_use]
    pub fn pyramid_builds(&self) -> u64 {
        self.pyramid_builds.get()
    }

    /// **The coarse route for a long walk**, over the cached pyramid.
    #[must_use]
    pub fn coarse_route(&self, from: (u16, u16), to: (u16, u16)) -> Option<crate::CoarseRoute> {
        self.pyramid().coarse_route(from, to)
    }

    /// **May a body of this reach stand centred on this tile?** The original grows every obstruction by the mover's radius inside the
    /// search (`FUN_008000d0` registers the radius, `FUN_007f0be0` stamps with it, and the AoC note states the rule as *"shrink the unit to
    /// a point, expand every obstruction"*).
    ///
    /// **Tangency is a passage**, not a collision: measured in HD by the owner (2026-11-20), a **packed trebuchet walks a one-tile gap** in
    /// a wall and through a one-tile corridor in bamboo, and the scenario editor even places a packed trebuchet in one. A body 1.0 tile wide
    /// in a 1.0-tile gap has *zero* clearance and still passes, which is consistent with the step, where the depenetration is computed and
    /// never applied (D290). So the test is `>= 0`, and `reach == 0` is the plain tile question.
    fn fits(&self, x: u16, y: u16, scale: i64, reach: i64) -> bool {
        if !self.walkable(x, y) {
            return false;
        }
        if reach == 0 {
            return true;
        }
        let half = scale / 2;
        let (px, py) = (i64::from(x) * scale + half, i64::from(y) * scale + half);
        let mut nearest = i64::MAX;
        for dy in -1i64..=1 {
            for dx in -1i64..=1 {
                let (nx, ny) = (i64::from(x) + dx, i64::from(y) + dy);
                let (Ok(nx), Ok(ny)) = (u16::try_from(nx), u16::try_from(ny)) else {
                    continue;
                };
                if !self.blocked(nx, ny) {
                    continue;
                }
                let (left, top) = (i64::from(nx) * scale, i64::from(ny) * scale);
                let gap_x = (left - px).max(px - (left + scale)).max(0);
                let gap_y = (top - py).max(py - (top + scale)).max(0);
                nearest = nearest.min(crate::integer_sqrt(gap_x * gap_x + gap_y * gap_y));
            }
        }
        nearest == i64::MAX || nearest - reach >= 0
    }

    /// **`fits` for the pyramid's builder**, which works in tiles and has no scale of its own.
    pub(crate) fn fits_for(&self, x: u16, y: u16, scale: i64, reach: i64) -> bool {
        self.fits(x, y, scale, reach)
    }

    /// Is something standing on this tile? A caller that has already asked `contains` wants the obstacle half alone.
    pub fn blocked(&self, x: u16, y: u16) -> bool {
        !self.walkable(x, y)
    }

    fn index(&self, x: u16, y: u16) -> Option<usize> {
        (x < self.width && y < self.height)
            .then(|| usize::from(y) * usize::from(self.width) + usize::from(x))
    }

    /// **Something worth walking round now stands on this tile** (D342): the pack's own mark for the 300 row. It does
    /// not block - `block` does that - and it is taken back with `unmark_snug` when the node is spent.
    pub fn mark_snug(&mut self, x: u16, y: u16) {
        if let Some(at) = self.index(x, y)
            && let Some(flag) = self.snug.get_mut(at)
        {
            *flag = true;
            self.pyramid.borrow_mut().clear();
        }
    }

    /// The node that made this tile expensive is gone.
    pub fn unmark_snug(&mut self, x: u16, y: u16) {
        if let Some(at) = self.index(x, y)
            && let Some(flag) = self.snug.get_mut(at)
        {
            *flag = false;
            self.pyramid.borrow_mut().clear();
        }
    }

    /// **Is this tile one the 300 row is about** - a mine, as the pack named it (D342).
    #[must_use]
    pub fn is_snug(&self, x: u16, y: u16) -> bool {
        self.index(x, y).is_some_and(|at| self.snug[at])
    }

    /// Something now stands on this tile.
    pub fn block(&mut self, x: u16, y: u16) {
        if let Some(at) = self.index(x, y)
            && let Some(count) = self.occupants.get_mut(at)
        {
            *count = count.saturating_add(1);
            // The pyramid describes where objects stand, so it is stale from this moment (D301).
            self.pyramid.borrow_mut().clear();
        }
    }

    /// Something left it. The count is what keeps a tile held by two things from freeing on the first one's removal.
    pub fn unblock(&mut self, x: u16, y: u16) {
        if let Some(at) = self.index(x, y)
            && let Some(count) = self.occupants.get_mut(at)
        {
            *count = count.saturating_sub(1);
            self.pyramid.borrow_mut().clear();
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

    /// **Does a straight leg pass inside any object's grown box?** The plan-side half of `in_an_object`: a leg between two
    /// points can clip a footprint's corner even when both of its ends are clear, which is what the owner saw as a body
    /// walking through a house's corner. Asked per tile the leg crosses, over that tile's own three-by-three neighbourhood.
    fn crosses_an_object(&self, scale: i64, from: (i64, i64), to: (i64, i64), reach: i64) -> bool {
        let (tx, ty) = (
            from.0.div_euclid(scale).min(to.0.div_euclid(scale)),
            from.1.div_euclid(scale).min(to.1.div_euclid(scale)),
        );
        let (ex, ey) = (
            from.0.div_euclid(scale).max(to.0.div_euclid(scale)),
            from.1.div_euclid(scale).max(to.1.div_euclid(scale)),
        );
        // Every tile the leg's own bounding box reaches, plus one ring: a box grown by less than a tile spills only into
        // the neighbours of the tiles the leg crosses, and the ring costs a handful of grid reads.
        for y in ty - 1..=ey + 1 {
            for x in tx - 1..=ex + 1 {
                let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y)) else {
                    continue;
                };
                if !self.occupied(x, y) {
                    continue;
                }
                let at = (
                    i64::from(x) * scale + scale / 2,
                    i64::from(y) * scale + scale / 2,
                );
                let grown = Clear {
                    at,
                    half: (scale / 2, scale / 2),
                    reach,
                };
                if grown.crossed_by(from, to) {
                    return true;
                }
            }
        }
        false
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
    /// i.e. `16 x max + 7 x min`. It is the search's "how close am I really" measure - the best-settled node is the one with
    /// the **smallest octile distance to the goal**, not the smallest weighted estimate - and it is public so the conformance
    /// set can check it rather than trust it.
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
    /// crossing the line takes must have both of its side tiles free - the same rule the search applies, so a straight line
    /// and a searched route never disagree about a corner.
    ///
    /// Bresenham in integers, so the answer is exact and identical everywhere.
    pub fn line_clear(&self, from: (u16, u16), to: (u16, u16)) -> bool {
        self.line_clear_with(from, to, 1, 0, &[])
    }

    /// **The same line, with tiles a body holds counting as solid** (D281). The smoothing pass uses this: a straight jump
    /// across a tile the plan was told to get round would undo the plan, which is what the first version did - the route
    /// bent in the search and was pulled straight again through the man.
    fn line_clear_with(
        &self,
        from: (u16, u16),
        to: (u16, u16),
        scale: i64,
        reach: i64,
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
        // **And the objects the leg would clip** (D286): a building's footprint is a box, and a leg between two clear tile
        // centres can still pass inside it by a rounding - which is the corner a body was seen cutting.
        if reach > 0 && self.crosses_an_object(scale, from_raw, to_raw, reach) {
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
        self.route_with(scale, from, to, 0, &[])
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
        reach: i64,
        avoid: &[Clear],
    ) -> Option<Vec<(Fixed, Fixed)>> {
        self.route_with(scale, from, to, reach, avoid)
    }

    fn route_with(
        &self,
        scale: i64,
        from: (Fixed, Fixed),
        to: (Fixed, Fixed),
        reach: i64,
        avoid: &[Clear],
    ) -> Option<Vec<(Fixed, Fixed)>> {
        let start = self.tile_of(scale, from)?;
        let goal = self.tile_of(scale, to)?;
        // A straight walk needs no chain: the search's own answer would be pulled back to this one point anyway. The test is
        // of the **leg the body will actually walk**, not of the line between two tile centres (see `leg_is_clear`).
        if self.walkable(goal.0, goal.1)
            && self.leg_is_clear(
                scale,
                (from.0.raw(), from.1.raw()),
                (to.0.raw(), to.1.raw()),
                reach,
                avoid,
            )
        {
            return Some(vec![to]);
        }
        // **Tier 2: the levelled search** (D301, D299). The binary's land-unit default is the coarse search over 8-tile cells, and
        // it **ignores moving bodies**: `FUN_007ee980` calls no stander inserter, and openage's note says the mip-map "ignores
        // moving objects, e.g. units or wildlife". So the tier is asked on every long walk, bodies or not (D338) - it used to be
        // gated on `avoid.is_empty()`, which is the opposite of the game's rule and, measured, left it answering **0 of 121** long
        // walks in one acceptance match (D334: every one of them carried a stander). What the coarse tier cannot see, the step pass
        // answers the original's way: the mover bumps, the collider joins the plan's avoid list, and the route is re-made.
        //
        // The chain's legality is by construction: every cell in it is open, and two adjacent open cells share a boundary whose two
        // centres lie inside them. What is *not* by construction is the first and last leg - a body may sit in a corner of its cell with
        // an object between it and the cell's centre - so both are certified here and the whole tier is abandoned if either fails.
        // **Long walks only** (D306): the hierarchy's answer is a chain at `2^level` tiles, and until the refinement matches the binary's
        // (a tile on the boundary, #5 of the crosscheck) a coarse chain is the right answer for a long walk and a staircase for a short one -
        // measured, a walk past a single tree became 31 points. Sixteen tiles is two level-3 cells, below which the flat search's corners are
        // cheaper and finer.
        let long_walk = (i64::from(start.0) - i64::from(goal.0)).abs()
            + (i64::from(start.1) - i64::from(goal.1)).abs()
            >= 16;
        if long_walk {
            // **The descent first, the search second** (D338): `coarse_route` derives its level as the lower of the two ends'
            // descents (`levels.rs:200`), so the level is knowable without searching. Both live inside the `long_walk` arm:
            // `pyramid_for` builds the pyramid on a cold cache, and a short walk must not pay for it.
            //
            // **The floor is the game's again** (D346, after the owner's *"follow original game"*). `FUN_007ee980` walks the
            // levels down to 0 and a mid-level attempt that fails *falls through* to the low-level path (`0xf98`), so nothing
            // is refused here for being fine-grained. D345 put a floor of 2 back on a *cost* measurement - the raw chain was
            // worse than the flat search's - and the cost had a cause: the polyline was handed over unsmoothed, which the
            // original never does (`move_obj.cpp:0xf56` -> `FUN_008acf20` -> `0xf69`, *"After smoothing"*). With `smooth` in
            // place the chain is the game's route, and its cost is no longer a reason to refuse the game's own mechanism.
            let levels = self.pyramid_for(reach);
            let watched = self.compare_tier.get();
            if watched {
                self.bump_funnel(|f| f.long_walks += 1);
            }
            let floor = levels.descent(start).min(levels.descent(goal));
            if floor == 0 {
                if watched {
                    self.bump_funnel(|f| f.floor_refused += 1);
                }
            } else if let Some(coarse) = levels.coarse_route(start, goal) {
                // The refinement interpolates the walk's own line across each shared edge (D343), so it needs the two
                // points the plan was asked for, not just the cells.
                let centres = levels.refine(
                    &coarse,
                    scale,
                    (from.0.raw(), from.1.raw()),
                    (to.0.raw(), to.1.raw()),
                );
                if centres.len() > 1 {
                    // The chain, then **the game's smoothing pass over it** (D346). The refinement emits a crossing per cell
                    // edge; `FUN_008acf20` keeps only the ones the straight segment test cannot skip, so what the mover is
                    // handed is the polyline's corners.
                    let mut raw: Vec<(Fixed, Fixed)> = centres.iter().skip(1).copied().collect();
                    raw.push(to);
                    let points = self.smooth(from, &raw, scale, reach);
                    self.tier_answers.set(self.tier_answers.get() + 1);
                    if watched {
                        self.bump_funnel(|f| {
                            f.waypoints_raw += u64::try_from(raw.len()).unwrap_or(u64::MAX);
                            f.waypoints_smoothed += u64::try_from(points.len()).unwrap_or(u64::MAX);
                        });
                        self.record_shadow(
                            Walk {
                                from,
                                to,
                                start,
                                goal,
                                scale,
                                reach,
                                avoid,
                            },
                            &points,
                        );
                    }
                    return Some(points);
                } else if watched {
                    self.bump_funnel(|f| f.one_cell += 1);
                }
            } else if watched {
                self.bump_funnel(|f| f.no_coarse += 1);
            }
        }
        self.flat_route(Walk {
            from,
            to,
            start,
            goal,
            scale,
            reach,
            avoid,
        })
    }

    /// **The flat search's answer, as a plan's points** (the tail of `route_with`, extracted so the tier-vs-flat comparison can
    /// ask it for the same walk it just answered - D344). Behaviour unchanged: the search, the pull, tile centres for every
    /// interior waypoint, and the exact clicked point when the walk ends on the goal's tile.
    fn flat_route(&self, walk: Walk<'_>) -> Option<Vec<(Fixed, Fixed)>> {
        let Walk {
            from,
            to,
            start,
            goal,
            scale,
            reach,
            avoid,
        } = walk;
        let steps = self.search_reporting(start, goal, scale, reach, avoid).0?;
        let end = steps.last().copied().unwrap_or(start);
        let pulled = self.pull(
            (from.0.raw(), from.1.raw()),
            (to.0.raw(), to.1.raw()),
            end == goal,
            start,
            &steps,
            scale,
            reach,
            avoid,
        );
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

    /// **Is the leg the body will actually walk clear?** A route is certified for the line the body walks, not for the line
    /// between two tile centres: a body that starts off-centre - a fixture's spawn on a tile corner, a man stopped by a body
    /// on the way - walks a line the tile test never looked at. Measured on a trebuchet of radius half a tile, spawned on a
    /// corner beside a one-tile gap: the tile-centre line is exactly down the gap, the line he walks clips the house beside
    /// it, and every step into the gap was refused.
    ///
    /// So the same three questions are asked of the raw segment: the tiles it crosses are walkable (the grid's own point
    /// test), no object's grown box is crossed, and no body's clearance is.
    fn leg_is_clear(
        &self,
        scale: i64,
        from: (i64, i64),
        to: (i64, i64),
        reach: i64,
        avoid: &[Clear],
    ) -> bool {
        self.leg_verdict(scale, from, to, reach, avoid) == LegVerdict::Clear
    }

    /// **Which question refused the leg** (D344): the certification is a measurement as much as a guard, and "18 legs
    /// refused" says nothing until it says whether a body, an object's box or a blocked tile refused them.
    fn leg_verdict(
        &self,
        scale: i64,
        from: (i64, i64),
        to: (i64, i64),
        reach: i64,
        avoid: &[Clear],
    ) -> LegVerdict {
        if avoid.iter().any(|clear| clear.crossed_by(from, to)) {
            return LegVerdict::Body;
        }
        self.chain_leg(scale, from, to, reach)
    }

    /// **What the chain is certified against** (D338, kept by D345): an **object's** grown box on the segment the body
    /// walks, and the tiles the segment crosses. **Bodies are not asked here** - the caller asks them first, in
    /// `leg_verdict`. Dropping that half was measured (17 more chains answered, D345) and withdrawn with the floor: the
    /// routes those chains produced were worse than the flat search's on every count.
    fn chain_leg(&self, scale: i64, from: (i64, i64), to: (i64, i64), reach: i64) -> LegVerdict {
        // **The objects first, on the segment the body walks** (D306): a leg between two clear tile centres can still pass
        // inside a building's box, which is the corner a body was seen cutting.
        if reach > 0 && self.crosses_an_object(scale, from, to, reach) {
            return LegVerdict::Object;
        }
        let (Ok(fx), Ok(fy)) = (
            u16::try_from(from.0.div_euclid(scale)),
            u16::try_from(from.1.div_euclid(scale)),
        ) else {
            return LegVerdict::Tiles;
        };
        let (Ok(tx), Ok(ty)) = (
            u16::try_from(to.0.div_euclid(scale)),
            u16::try_from(to.1.div_euclid(scale)),
        ) else {
            return LegVerdict::Tiles;
        };
        if self.line_clear_with((fx, fy), (tx, ty), scale, 0, &[]) {
            LegVerdict::Clear
        } else {
            LegVerdict::Tiles
        }
    }

    /// **The original's smoothing pass on the levelled route** (D346): `FUN_008acf20` walks the polyline with two indices -
    /// the last **kept** waypoint and a candidate further along - and asks the radius-grown segment test `FUN_00840c60`
    /// whether the straight line between them is walkable. Clear: the waypoint in between is deleted and the same candidate is
    /// tried again. Blocked: the in-between waypoint is kept and both indices advance. The `move_obj.cpp` log says what it is
    /// for - `"        Found a valid mid-level path with %d waypoints"` (`0xf56`) becomes
    /// `"          After smoothing, path has %d waypoints"` (`0xf69`).
    ///
    /// **What is ported and what is ours.** The order, the anchor and the delete-then-retry are the binary's; the segment
    /// test is `chain_leg`, so the geometry is the engine's own (objects' boxes and the grid, at the mover's reach). Bodies
    /// are not in the test, which matches the coarse search being blind to them (D338) and leaves them to the step's bump and
    /// re-plan (D320). The pass never refuses a route - a blocked segment simply keeps its waypoint - so it cannot turn an
    /// answer into a fall-through the binary would not have had.
    fn smooth(
        &self,
        from: (Fixed, Fixed),
        route: &[(Fixed, Fixed)],
        scale: i64,
        reach: i64,
    ) -> Vec<(Fixed, Fixed)> {
        // The binary's path starts with the unit's own position - `FUN_007ee980` pushes it last, and the refinement walks the
        // chain outward from there - so the first anchor is the mover, not the first crossing.
        let mut points: Vec<(Fixed, Fixed)> = Vec::with_capacity(route.len() + 1);
        points.push(from);
        points.extend_from_slice(route);
        if points.len() < 3 {
            points.remove(0);
            return points;
        }
        let mut anchor = 0usize;
        let mut candidate = 2usize;
        while candidate < points.len() {
            let (ax, ay) = (points[anchor].0.raw(), points[anchor].1.raw());
            let (cx, cy) = (points[candidate].0.raw(), points[candidate].1.raw());
            if self.chain_leg(scale, (ax, ay), (cx, cy), reach) == LegVerdict::Clear {
                // The waypoint in between is one the straight line does not need. The array shifts, so the same candidate
                // index is now the next original point - which is the binary's `do` loop, unchanged.
                points.remove(anchor + 1);
            } else {
                anchor += 1;
                candidate += 1;
            }
        }
        points.remove(0);
        points
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
    ///
    /// **The first leg starts where the body stands, not on a tile centre.** A body that spawns off-centre walks from its own
    /// point to the route's first corner, and that leg is the one that clipped a house beside a gap: the tile-centre line was
    /// exactly down the gap, the line the body walked was not. Every leg is certified with `leg_is_clear` for that reason -
    /// the same three questions asked of the actual segment - and the last one ends on the goal **point** when the goal was
    /// reached, which is where the body will stand (*"Setting position to exact waypoint"*).
    #[allow(clippy::too_many_arguments)]
    fn pull(
        &self,
        from_raw: (i64, i64),
        to_raw: (i64, i64),
        ends_on_goal: bool,
        start: (u16, u16),
        steps: &[(u16, u16)],
        scale: i64,
        reach: i64,
        avoid: &[Clear],
    ) -> Vec<(u16, u16)> {
        if steps.len() < 2 {
            return steps.to_vec();
        }
        let mut nodes = Vec::with_capacity(steps.len() + 1);
        nodes.push(start);
        nodes.extend_from_slice(steps);
        // Where the body actually walks for each node: its own point first, a tile centre after that, and the goal point last.
        let half = scale / 2;
        let centre = |tile: (u16, u16)| {
            (
                i64::from(tile.0) * scale + half,
                i64::from(tile.1) * scale + half,
            )
        };
        let last = nodes.len() - 1;
        let mut raw: Vec<(i64, i64)> = nodes.iter().map(|tile| centre(*tile)).collect();
        raw[0] = from_raw;
        if ends_on_goal {
            raw[last] = to_raw;
        }
        let mut pulled = Vec::with_capacity(nodes.len());
        let mut at = 0;
        while at < last {
            let mut jump = last;
            while jump > at + 1 && !self.leg_is_clear(scale, raw[at], raw[jump], reach, avoid) {
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
        self.search_reporting(start, goal, scale, 0, avoid).0
    }

    /// **The same search, reporting what it spent.**
    ///
    /// The original's budget is **its own estimate**: `expansions > h`, where `h = 25 x the Euclidean tile distance`, and
    /// never more than **20,000** (`FUN_008007f0`, the guard at `0x800d4f` and the second clause beside it). So a farther goal
    /// is allowed more expansions and a hopeless one costs a bounded number of cells instead of every reachable tile. When the
    /// budget runs out the search answers the **best node it settled** - the binary pushes that node with cost 0 and its caller
    /// walks the parents from it - which is the same answer it gives a goal it cannot reach.
    ///
    /// "Best" is the **smallest octile distance to the goal**, first found wins: `if (iVar21 < local_468)` in the binary, with
    /// `iVar21 = FUN_007ffc80(node, goal)`, a strict comparison, so a tie keeps the earlier node.
    ///
    /// **The key is the game's since D337** (`g + octile`); the cap that used to have no witness is now pinned by a 1,200-tile
    /// scene (D336).
    pub fn search_reporting(
        &self,
        start: (u16, u16),
        goal: (u16, u16),
        scale: i64,
        reach: i64,
        avoid: &[Clear],
    ) -> (Option<Vec<(u16, u16)>>, u32) {
        let width = usize::from(self.width);
        let count = width * usize::from(self.height);
        let index = |tile: (u16, u16)| usize::from(tile.1) * width + usize::from(tile.0);
        let tile = |at: usize| {
            (
                u16::try_from(at % width).unwrap_or(0),
                u16::try_from(at / width).unwrap_or(0),
            )
        };
        // **The frontier's key is `g + octile`, weight 1** (D337): `FUN_008007f0` stores the octile as the start's value
        // (line 177) and adjusts it node by node (line 316), so the priority is the octile distance. The `25 x euclid` in
        // this function is the *budget* (line 198), not the key - reading it as the weight is what made our search greedy.
        let key = |tile: (u16, u16)| Self::octile(tile, goal);
        let start_at = index(start);
        let mut best = vec![u32::MAX; count];
        let mut from: Vec<Option<usize>> = vec![None; count];
        let mut open = BinaryHeap::new();
        best[start_at] = 0;
        open.push(Reverse((key(start), 0u32, start.1, start.0)));
        // **The budget is the estimate**: the binary computes it once, before the loop, from Euclidean tile distance x 25, and
        // caps it at 20,000. It is spent one node at a time, and the loop stops *before* expanding a node it cannot afford.
        let budget = Self::estimate(start, goal).min(EXPANSION_CAP);
        let mut expanded = 0u32;
        // The closest cell the search ever stands on: the answer when the goal cannot be reached, or when the budget runs out.
        let mut settled_best: Option<(u32, usize)> = None;
        while let Some(Reverse((_, g, y, x))) = open.pop() {
            let here = (x, y);
            let here_at = index(here);
            // A stale entry: the node has already been reached more cheaply.
            if g > best[here_at] {
                continue;
            }
            if expanded >= budget {
                break;
            }
            expanded += 1;
            // **The closest cell the search stands on**, which is the original's answer to a goal it cannot reach. The start
            // is not a candidate: "come as close as you can" must move the body, not leave it where it stands. Ties go to
            // the cheaper cell, so the settled point is on the shortest path the search found, not merely near the goal.
            let near = Self::octile(here, goal);
            if here_at != start_at && settled_best.is_none_or(|(old, _)| near < old) {
                settled_best = Some((near, here_at));
            }
            if here == goal {
                settled_best = Some((0, here_at));
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
                if !self.fits(nx, ny, scale, reach) || self.inside_a_body(scale, (nx, ny), avoid) {
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
                    if !self.fits(sx, y, scale, reach)
                        || !self.fits(x, sy, scale, reach)
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
                    open.push(Reverse((step + key((nx, ny)), step, ny, nx)));
                }
            }
        }
        let Some((_, end)) = settled_best else {
            return (None, expanded);
        };
        if end == start_at {
            return (None, expanded);
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
        (Some(path), expanded)
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
