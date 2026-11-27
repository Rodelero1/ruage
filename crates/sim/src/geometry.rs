//! **The geometric solver's verdict contract and self-check.**
//!
//! A route answers one of two questions, and the caller must know which: `Reached`
//! means the polyline arrives at the goal, `Closest` means the goal is unreachable
//! and the polyline stops beside it instead. A `Vec` alone cannot say that, so the
//! verdict rides with the points.
//!
//! `verify_path` certifies a polyline against the `avoid` boxes: every raw point
//! must stand outside every grown box, and no segment may pass inside one.
//! Touching a face is clear - a body at exactly its radius from a wall is at
//! contact, which is where a walk ends. `None` is valid; `Some` is a description
//! of the first overlap found, in path order, for debugging.
//!
//! The box test is the integer separating-axis test from `ground::Clear`
//! (`crossed_by`/`still_inside`), duplicated here so this module owns the
//! contract Tasks 2-3 build on without reaching into the tile search's privates.
//! Fixed-point only (D9): raw `i64`s, no float. Iteration is in path and slice
//! order, never hash order, so the first-overlap answer is deterministic.

use ruage_fixed::Fixed;

use crate::ground::Clear;

/// **What the geometric solver answered**: the goal, or the nearest it could stand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WayRoute {
    /// The polyline arrives at the goal; the points end on it.
    Reached(Vec<(Fixed, Fixed)>),
    /// The goal is unreachable; the points stop beside it instead of refusing.
    Closest(Vec<(Fixed, Fixed)>),
}

/// The obstacle grown by the mover's body on every side: `half + reach` per axis.
fn grown(clear: &Clear) -> (i64, i64) {
    (clear.half.0 + clear.reach, clear.half.1 + clear.reach)
}

/// Is this raw point inside the grown box? Touching is *not* inside.
fn point_inside(clear: &Clear, point: (i64, i64)) -> bool {
    let (hx, hy) = grown(clear);
    (point.0 - clear.at.0).abs() < hx && (point.1 - clear.at.1).abs() < hy
}

/// Is this raw point a corner where two grown boxes meet at exactly that point - a diagonal touch,
/// where two squares meet corner-to-corner? Such a point is no hold: stepping onto it threads the touch,
/// and the next step leaves through the other box's corner. The original's boxes collide on touching, the
/// passage rule forbids the diagonal gap, and a route through the point reads as entered to a tile query
/// even when no leg is strictly inside a box. Boxes that overlap, or share a face, meet along more than
/// the point - walking their seam or face is ordinary contact, so only the single-point meeting counts.
/// A corridor mouth's corners belong to one box each, so an exact-fit seam keeps its mouths.
fn shared_corner(rects: &[Clear], point: (i64, i64)) -> bool {
    // The boxes holding the point as a corner.
    let mut holders: Vec<(i64, i64, i64, i64)> = Vec::new();
    for clear in rects {
        let (hx, hy) = grown(clear);
        let (min_x, max_x) = (clear.at.0 - hx, clear.at.0 + hx);
        let (min_y, max_y) = (clear.at.1 - hy, clear.at.1 + hy);
        if (point.0 == min_x || point.0 == max_x) && (point.1 == min_y || point.1 == max_y) {
            holders.push((min_x, max_x, min_y, max_y));
        }
    }
    // A diagonal touch is a pair whose whole intersection is the point itself: overlapping boxes share
    // an area, and face-touching boxes a segment, and neither is a thread-through.
    for (index, a) in holders.iter().enumerate() {
        for b in &holders[index + 1..] {
            let (lo_x, hi_x) = (a.0.max(b.0), a.1.min(b.1));
            let (lo_y, hi_y) = (a.2.max(b.2), a.3.min(b.3));
            if lo_x == point.0 && hi_x == point.0 && lo_y == point.1 && hi_y == point.1 {
                return true;
            }
        }
    }
    false
}

/// Does the straight segment between two raw points pass inside the grown box?
/// A segment against an axis-aligned rectangle, by separating axis - the two
/// axis normals and the segment's own normal - in integers. A segment that runs
/// along a face touches it and does not cross.
fn segment_crosses(clear: &Clear, from: (i64, i64), to: (i64, i64)) -> bool {
    let (hx, hy) = grown(clear);
    let (min_x, max_x) = (clear.at.0 - hx, clear.at.0 + hx);
    let (min_y, max_y) = (clear.at.1 - hy, clear.at.1 + hy);
    // Axis x, then axis y: the segment's own span against the box's.
    let (sx0, sx1) = (from.0.min(to.0), from.0.max(to.0));
    let (sy0, sy1) = (from.1.min(to.1), from.1.max(to.1));
    if sx1 <= min_x || sx0 >= max_x || sy1 <= min_y || sy0 >= max_y {
        return false;
    }
    // The segment's normal: the box's four corners projected onto it, against
    // the segment's own two ends.
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

/// The grown rectangle's sides, in raw units: left, right, bottom, top.
fn sides(clear: &Clear) -> (i64, i64, i64, i64) {
    let (hx, hy) = grown(clear);
    (
        clear.at.0 - hx,
        clear.at.0 + hx,
        clear.at.1 - hy,
        clear.at.1 + hy,
    )
}

/// One axis of the slab test: `None` when the ray runs parallel outside the
/// box (touching counts as outside), otherwise the entry/exit bounds, each an
/// exact fraction with a positive denominator. A parallel axis strictly inside
/// the box bounds nothing, written as `(None, None)`.
type AxisBound = Option<(Option<(i128, i128)>, Option<(i128, i128)>)>;

/// First grown rect the ray from `from` to `to` passes inside, in slice
/// order ties: `None` when the ray stays clear.
///
/// The hit test is the same box test `verify_path` certifies against, read
/// along the ray: touching a face is clear, so a ray that only grazes a
/// corner or runs along a face is not a hit, and one that ends exactly on a
/// face counts only when it passed inside first. Entry and exit are exact
/// rational comparisons (`i128` cross products), so no float enters and two
/// runs agree about which rect is first.
#[must_use]
pub fn ray_hit(from: (i64, i64), to: (i64, i64), avoid: &[Clear]) -> Option<usize> {
    if from == to {
        return None;
    }
    // `None` is minus infinity on entry, plus infinity on exit.
    fn axis(from: i64, to: i64, lo: i64, hi: i64) -> AxisBound {
        let step = i128::from(to) - i128::from(from);
        if step == 0 {
            if lo < from && from < hi {
                return Some((None, None));
            }
            return None;
        }
        if step > 0 {
            Some((
                Some((i128::from(lo) - i128::from(from), step)),
                Some((i128::from(hi) - i128::from(from), step)),
            ))
        } else {
            let span = -step;
            Some((
                Some((i128::from(from) - i128::from(hi), span)),
                Some((i128::from(from) - i128::from(lo), span)),
            ))
        }
    }
    /// `a/ad < b/bd`, denominators positive: the exact entry/exit ordering.
    fn less(a: i128, ad: i128, b: i128, bd: i128) -> bool {
        a * bd < b * ad
    }
    let mut best: Option<(usize, Option<(i128, i128)>)> = None;
    for (index, clear) in avoid.iter().enumerate() {
        let (min_x, max_x, min_y, max_y) = sides(clear);
        let (Some((enter_x, exit_x)), Some((enter_y, exit_y))) = (
            axis(from.0, to.0, min_x, max_x),
            axis(from.1, to.1, min_y, max_y),
        ) else {
            continue;
        };
        // The entry is the later of the two axes' entries (`None` is
        // minus infinity and loses); the exit is the earlier of the exits.
        let entry = match (enter_x, enter_y) {
            (Some(a), Some(b)) => Some(if less(a.0, a.1, b.0, b.1) { b } else { a }),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
        let exit = match (exit_x, exit_y) {
            (Some(a), Some(b)) => Some(if less(a.0, a.1, b.0, b.1) { a } else { b }),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
        // Strict on every side: grazing a corner (`entry == exit`) is a
        // touch, and ending exactly on the entry face (`entry == 1`) or
        // starting on the exit face (`exit == 0`) never went inside.
        let inside = match (entry, exit) {
            (Some(en), Some(ex)) => less(en.0, en.1, ex.0, ex.1),
            _ => true,
        };
        let reaches = match entry {
            Some((n, d)) => less(n, d, 1, 1),
            None => true,
        };
        let leaves = match exit {
            Some((n, d)) => less(0, 1, n, d),
            None => true,
        };
        if inside && reaches && leaves {
            // Minus infinity (the ray starts inside) beats every entry;
            // equal entries keep the earlier slice index, so the answer is
            // deterministic.
            let take = match (&best, &entry) {
                (None, _) => true,
                (Some(_), None) => true,
                (Some((_, None)), _) => false,
                (Some((_, Some((bn, bd)))), Some((n, d))) => less(*n, *d, *bn, *bd),
            };
            if take {
                best = Some((index, entry));
            }
        }
    }
    best.map(|(index, _)| index)
}

/// Closed ring of the merged shape the hit rect belongs to: the outer
/// boundary of every grown rect joined to `hit` by overlap or touch, walked
/// edge to edge until it loops. The last point repeats the first, closing
/// the ring.
///
/// The walk is the deck's edge rule: start on a boundary edge, and at each
/// vertex turn right at the rect's own corner and left onto the intersected
/// rect instead of cutting inside it, bridging straight across perfectly
/// aligned overlaps. Overlapping rects' intersection points split the edges
/// they cross, so every vertex of the merged boundary is walked. The core is
/// only comparisons and index steps on the raw coordinates - no float, and
/// no multiply or divide on the coordinates themselves (the grid bookkeeping
/// still multiplies cell indices) - while the ray setup above is where the
/// one exact division lives. Every edge of the ring lies on a grown face, which is
/// exactly what `verify_path` calls clear, so the ring it returns certifies
/// clean.
#[must_use]
/// Which rects belong to the hit rect's merged shape: everything joined to it by
/// overlap or a shared face, transitively, in slice order so the set is
/// deterministic.
fn shape_members(hit: usize, rects: &[(i64, i64, i64, i64)]) -> Vec<bool> {
    let touches = |a: (i64, i64, i64, i64), b: (i64, i64, i64, i64)| {
        !(a.1 < b.0 || b.1 < a.0 || a.3 < b.2 || b.3 < a.2)
    };
    let mut member = vec![false; rects.len()];
    if hit < rects.len() {
        member[hit] = true;
    }
    let mut grew = true;
    while grew {
        grew = false;
        for (index, rect) in rects.iter().enumerate() {
            if member[index] {
                continue;
            }
            let mut joins = false;
            for (other, held) in rects.iter().zip(member.iter()) {
                if *held && touches(*rect, *other) {
                    joins = true;
                    break;
                }
            }
            if joins {
                member[index] = true;
                grew = true;
            }
        }
    }
    member
}

pub fn walk_shape(hit: usize, avoid: &[Clear]) -> Vec<(i64, i64)> {
    // Sides inflated so a degenerate grown rect (a line or a point: half and
    // reach both zero on an axis) still owns grid cells. The walk's ring is
    // certified against the true boxes, and every edge it returns lies on an
    // inflated face - one raw unit off a line, on it otherwise - which the
    // box test still calls clear, so the inflation never leaks into the
    // verdict. One raw unit is below every body the engine moves.
    let rects: Vec<(i64, i64, i64, i64)> = avoid
        .iter()
        .map(sides)
        .map(|(min_x, max_x, min_y, max_y)| {
            (
                min_x,
                if min_x == max_x { max_x + 1 } else { max_x },
                min_y,
                if min_y == max_y { max_y + 1 } else { max_y },
            )
        })
        .collect();
    if hit >= rects.len() {
        return Vec::new();
    }
    // The merged shape: everything joined to the hit rect by overlap or a
    // shared face, transitively.
    let member = shape_members(hit, &rects);
    // Split coordinates: every member rect's sides, sorted, so intersection
    // points of overlapping rects become the vertices the walk turns on.
    let mut xs: Vec<i64> = Vec::new();
    let mut ys: Vec<i64> = Vec::new();
    for (rect, held) in rects.iter().zip(member.iter()) {
        if *held {
            xs.push(rect.0);
            xs.push(rect.1);
            ys.push(rect.2);
            ys.push(rect.3);
        }
    }
    xs.sort_unstable();
    xs.dedup();
    ys.sort_unstable();
    ys.dedup();
    if xs.len() < 2 || ys.len() < 2 {
        let (min_x, max_x, min_y, max_y) = rects[hit];
        return vec![
            (min_x, min_y),
            (max_x, min_y),
            (max_x, max_y),
            (min_x, max_y),
            (min_x, min_y),
        ];
    }
    // One cell per coordinate span: solid when strictly inside a member rect.
    let mut solid = vec![false; (xs.len() - 1) * (ys.len() - 1)];
    for jx in 0..xs.len() - 1 {
        for jy in 0..ys.len() - 1 {
            let mut inside = false;
            for (rect, held) in rects.iter().zip(member.iter()) {
                if *held
                    && rect.0 <= xs[jx]
                    && xs[jx + 1] <= rect.1
                    && rect.2 <= ys[jy]
                    && ys[jy + 1] <= rect.3
                {
                    inside = true;
                    break;
                }
            }
            solid[jy * (xs.len() - 1) + jx] = inside;
        }
    }
    let (nx, ny) = (xs.len() - 1, ys.len() - 1);
    let cell = |ix: usize, iy: usize| -> bool { ix < nx && iy < ny && solid[iy * nx + ix] };
    // A grid-line step is a boundary edge when the solid side is exactly on
    // the right of travel (clockwise walk) and the left side is empty.
    // Headings: 0 east, 1 south, 2 west, 3 north. Only comparisons and
    // index steps; the coordinates themselves are only read, never scaled.
    let at = |cx: usize, cy: usize| -> bool { cell(cx, cy) };
    let below = |cx: usize, cy: usize| -> bool { cy.checked_sub(1).is_some_and(|py| cell(cx, py)) };
    let west_of =
        |cx: usize, cy: usize| -> bool { cx.checked_sub(1).is_some_and(|px| cell(px, cy)) };
    let edge = |ix: usize, iy: usize, heading: usize| -> Option<(usize, usize)> {
        match heading {
            // East along the line: right is south.
            0 => {
                if ix < nx && below(ix, iy) && !at(ix, iy) {
                    Some((ix + 1, iy))
                } else {
                    None
                }
            }
            // South along the line: right is west.
            1 => {
                if iy > 0 && west_of(ix, iy - 1) && !at(ix, iy - 1) {
                    Some((ix, iy - 1))
                } else {
                    None
                }
            }
            // West along the line: right is north.
            2 => {
                if ix > 0 && at(ix - 1, iy) && !below(ix - 1, iy) {
                    Some((ix - 1, iy))
                } else {
                    None
                }
            }
            // North along the line: right is east.
            _ => {
                if iy < ny && at(ix, iy) && !west_of(ix, iy) {
                    Some((ix, iy + 1))
                } else {
                    None
                }
            }
        }
    };
    // Start on the westmost boundary vertex (nothing solid west of it, so it
    // is on the outer boundary), heading along the first boundary edge in
    // fixed order. Both choices are index order, never hash order.
    let mut start: Option<(usize, usize, usize)> = None;
    for ix in 0..xs.len() {
        for iy in 0..ys.len() {
            for heading in 0..4 {
                if edge(ix, iy, heading).is_some() {
                    start = Some((ix, iy, heading));
                    break;
                }
            }
            if start.is_some() {
                break;
            }
        }
        if start.is_some() {
            break;
        }
    }
    let Some((sx, sy, sh)) = start else {
        let (min_x, max_x, min_y, max_y) = rects[hit];
        return vec![
            (min_x, min_y),
            (max_x, min_y),
            (max_x, max_y),
            (min_x, max_y),
            (min_x, min_y),
        ];
    };
    // Right hand on the wall: right, straight, left, back. Right turns the
    // rect's own corners; left turns onto the intersected rect at a notch;
    // straight bridges an aligned overlap past a point that is a corner of
    // one rect and mid-edge of the other.
    let mut ring: Vec<(i64, i64)> = vec![(xs[sx], ys[sy])];
    let (mut ix, mut iy, mut heading) = (sx, sy, sh);
    let fuel = 4 * (xs.len() + 1) * (ys.len() + 1) + 4;
    for _ in 0..fuel {
        let mut stepped = false;
        for turn in 0..4 {
            let next = (heading + 3 + turn) % 4;
            if let Some((jx, jy)) = edge(ix, iy, next) {
                (ix, iy, heading) = (jx, jy, next);
                ring.push((xs[ix], ys[iy]));
                stepped = true;
                break;
            }
        }
        if !stepped || (ix == sx && iy == sy) {
            break;
        }
    }
    ring
}

/// The exact squared distance between two raw points, in `i128` so a
/// whole-map diagonal cannot overflow (`2 * span^2` is what D128 budgets the
/// pack for). Only the progress comparison and the nearest-vertex pick read
/// it; both are exact.
fn dist_sq(from: (i64, i64), to: (i64, i64)) -> i128 {
    let (dx, dy) = (
        i128::from(from.0) - i128::from(to.0),
        i128::from(from.1) - i128::from(to.1),
    );
    dx * dx + dy * dy
}

/// Right turns minus left turns around one edge-walk ring: the bound the
/// solver watches while it walks. An outer boundary walked clockwise counts
/// +4 (open ground, keep walking); a walk that ever counts 4 more lefts is
/// tracing the inside of an enclosure, which is where the loop stops and
/// answers `Closest` instead of circling the hole. Exact integer cross
/// products, no float.
fn ring_turns(ring: &[(i64, i64)]) -> (i64, i64) {
    let mut distinct: Vec<(i64, i64)> = ring.to_vec();
    if distinct.len() > 1 && distinct.first() == distinct.last() {
        distinct.pop();
    }
    let mut rights = 0i64;
    let mut lefts = 0i64;
    if distinct.len() < 3 {
        return (rights, lefts);
    }
    for i in 0..distinct.len() {
        let (ax, ay) = distinct[i];
        let (bx, by) = distinct[(i + 1) % distinct.len()];
        let (cx, cy) = distinct[(i + 2) % distinct.len()];
        let cross = (i128::from(bx) - i128::from(ax)) * (i128::from(cy) - i128::from(by))
            - (i128::from(by) - i128::from(ay)) * (i128::from(cx) - i128::from(bx));
        if cross > 0 {
            lefts += 1;
        } else if cross < 0 {
            rights += 1;
        }
    }
    (rights, lefts)
}

/// One geometric walk from `from` to `to`, and it always answers: `Reached`
/// when the polyline arrives at the goal, `Closest` when the goal is walled
/// in or the walk runs out of search area, stopping beside it instead of
/// refusing. Never a failure, never a crossed wall.
///
/// The mover shrinks to a point and every `avoid` box grows by the mover's
/// own radius - the existing `Clear` semantics, so a rect that already
/// carries the mover's reach keeps it (`max`, never a doubled growth) and a
/// bare rect is grown by `reach`. The loop shoots a ray at the goal, walks
/// the hit shape's ring, and re-rays from each vertex: the first vertex in
/// coordinate order that sees both the walk and the goal wins outright,
/// otherwise the nearest certified vertex that is strictly closer to the
/// goal. What survives is brute-force smoothed - every waypoint the
/// grown-segment test certifies as skippable is skipped - so the answer holds
/// no detour the test would have allowed cutting.
///
/// Three bounds keep the loop finite, in the order they bite: the
/// search-area rect (the from/to box, then once more after one expansion),
/// the turn count (4 more lefts than rights is the inside of a hole), and a
/// fuel cap of `8 * (boxes + 1) + 8` re-rays. When the greedy walk stalls on a
/// grown face, the loop spends its remaining fuel on one foray - certified box
/// corners nearest the goal first, none repeated - because an exact-fit seam
/// hides its mouth inside the merged shape, where no ring vertex stands. A
/// foray that sees the goal reaches it; one that never does keeps the nearest
/// approach when the goal sits inside a grown box (a worker's tree), and
/// otherwise rewinds to the stall point, so a walk that cannot progress ends
/// exactly where the greedy walk stopped. Vertex order is sorted raw
/// coordinates and rect order is slice order, never hash order, so two runs
/// agree on every waypoint.
#[must_use]
/// `foray_near` is the order-time search: an initial plan walks round a stander even when the
/// stall is beside the goal, while a re-plan on the same stall presses straight through (the crowd
/// would otherwise hold at arm's length on every tail tick, and every one of those ticks would buy
/// a foray). Re-plans still foray at a goal inside a box, where the answer is kept, not rewound.
pub fn solve(
    scale: i64,
    from: (Fixed, Fixed),
    to: (Fixed, Fixed),
    reach: i64,
    terrain: &[Clear],
    bodies: &[Clear],
    foray_near: bool,
) -> WayRoute {
    let start = (from.0.raw(), from.1.raw());
    let goal = (to.0.raw(), to.1.raw());
    // The mover's own radius on top of each rect's own reach; `max` because
    // the callers that pass `reach` also stamp it into the rects they hand
    // over, and growing twice would close gaps the game walks through.
    //
    // Terrain first, bodies after: a stall on a terrain box forays even near the goal (a wall needs
    // walking round), while a stall on a body that will have moved by next tick skips the search - the
    // foray would wander the neighbourhood and rewind to where it stands.
    let grown: Vec<Clear> = terrain
        .iter()
        .chain(bodies.iter())
        .map(|clear| Clear {
            at: clear.at,
            half: clear.half,
            reach: clear.reach.max(reach),
        })
        .collect();
    let terrain_len = terrain.len();
    let to_fixed = |point: (i64, i64)| (Fixed::from_raw(point.0), Fixed::from_raw(point.1));
    if start == goal {
        return WayRoute::Reached(vec![to_fixed(start)]);
    }
    // A start standing inside a grown box steps out before the walk begins:
    // the step is to the nearest corner of the box it stands in that is itself
    // a hold - never strictly inside another box. A worker with a route still on
    // his books stands on the blocked tile he is working, and his box merges
    // with every touching neighbour into a mass whose ring can sit a dozen tiles
    // away; stepping to that ring trades one inside for a walk that starts across
    // the map. The box's own corners are beside the body, which is where a walk
    // out begins. When every own corner sits inside some other box, the merged
    // ring is the fallback, and when that is inside too there is nowhere to go.
    let mut current = start;
    if let Some(hit) = grown.iter().position(|clear| point_inside(clear, current)) {
        let (min_x, max_x, min_y, max_y) = sides(&grown[hit]);
        let mut exit = current;
        let mut best: Option<(i128, i64, i64)> = None;
        for corner in [
            (min_x, min_y),
            (max_x, min_y),
            (min_x, max_y),
            (max_x, max_y),
        ] {
            if grown.iter().any(|clear| point_inside(clear, corner)) {
                continue;
            }
            let key = (dist_sq(corner, current), corner.0, corner.1);
            if best.is_none_or(|held| key < held) {
                best = Some(key);
                exit = corner;
            }
        }
        if best.is_none() {
            let ring = walk_shape(hit, &grown);
            for vertex in &ring {
                if grown.iter().any(|clear| point_inside(clear, *vertex)) {
                    continue;
                }
                let key = (dist_sq(*vertex, current), vertex.0, vertex.1);
                if best.is_none_or(|held| key < held) {
                    best = Some(key);
                    exit = *vertex;
                }
            }
            if best.is_none() {
                for vertex in &ring {
                    let key = (dist_sq(*vertex, current), vertex.0, vertex.1);
                    if best.is_none_or(|held| key < held) {
                        best = Some(key);
                        exit = *vertex;
                    }
                }
            }
        }
        current = exit;
        if grown.iter().any(|clear| point_inside(clear, current)) {
            return WayRoute::Closest(vec![to_fixed(current)]);
        }
    }
    let mut path = vec![current];
    // The search-area rect: the from/to box, then once more after one
    // expansion. A vertex outside it is not walked to.
    let margin = |expanded: bool| {
        let step = scale
            .max(0)
            .saturating_mul(i64::from(if expanded { 8 } else { 4 }));
        (
            start.0.min(goal.0).saturating_sub(step),
            start.0.max(goal.0).saturating_add(step),
            start.1.min(goal.1).saturating_sub(step),
            start.1.max(goal.1).saturating_add(step),
        )
    };
    let inside = |bounds: (i64, i64, i64, i64), point: (i64, i64)| {
        bounds.0 <= point.0 && point.0 <= bounds.1 && bounds.2 <= point.1 && point.1 <= bounds.3
    };
    let mut expanded = false;
    let mut rights = 0i64;
    let mut lefts = 0i64;
    // Three bounds keep the loop finite, in the order they bite: the
    // search-area rect (the from/to box, then once more after one expansion),
    // the turn count (4 more lefts than rights is the inside of a hole), and a
    // fuel cap of `8 * (boxes + 1) + 8` re-rays. The foray below spends the same
    // fuel, so a hopeless walk costs no more than the greedy walk already did.
    let mut fuel = 8 * (grown.len() + 1) + 8;
    while fuel > 0 {
        fuel -= 1;
        if current == goal {
            break;
        }
        let Some(hit) = ray_hit(current, goal, &grown) else {
            current = goal;
            path.push(current);
            break;
        };
        let ring = walk_shape(hit, &grown);
        let (round_rights, round_lefts) = ring_turns(&ring);
        rights += round_rights;
        lefts += round_lefts;
        if lefts >= rights + 4 {
            // The walk is tracing the inside of an enclosure: stop beside
            // it instead of circling the hole.
            break;
        }
        let bounds = margin(expanded);
        let mut vertices: Vec<(i64, i64)> = ring;
        // The hit box's face middles walk beside it too: a corner is the nearest a ring vertex gets to a goal
        // past the face, but contact happens *on* the face - a gatherer works from touching distance, and the
        // tile test measures from the tile, not its centre. A middle that sits inside another box is no hold;
        // the certification below drops it the same way it drops a bad vertex.
        {
            let (min_x, max_x, min_y, max_y) = sides(&grown[hit]);
            // The box's own corners as well as its face middles: the box merges with every neighbour into
            // one mass, and the union ring keeps only the mass's outer boundary - measured, a villager's own
            // box merged with the map-edge strips, the ring kept no vertex within two tiles, the walk advanced
            // onto the one remaining hold (a shared face middle) and stalled there. A corner inside another
            // box is no hold; the certification below drops it the same way it drops a bad middle.
            vertices.push((min_x, min_y));
            vertices.push((max_x, min_y));
            vertices.push((min_x, max_y));
            vertices.push((max_x, max_y));
            vertices.push(((min_x + max_x) / 2, min_y));
            vertices.push(((min_x + max_x) / 2, max_y));
            vertices.push((min_x, (min_y + max_y) / 2));
            vertices.push((max_x, (min_y + max_y) / 2));
        }
        vertices.sort_unstable();
        vertices.dedup();
        // The first vertex in coordinate order that sees both the walk and
        // the goal wins outright; otherwise the nearest certified vertex
        // that is strictly closer to the goal.
        let mut direct: Option<(i64, i64)> = None;
        let mut advance: Option<(i64, i64)> = None;
        let mut advance_key: Option<(i128, i64, i64)> = None;
        for vertex in vertices {
            if vertex == current || !inside(bounds, vertex) {
                continue;
            }
            // A vertex another box stands inside is no hold: two bodies overlap, and the ring's corners and
            // the hit box's own face middles can both sit strictly inside the neighbour's grown box - measured,
            // a villager routed round his fellow stepped onto the shared face middle and stalled there, and the
            // snap settled the stall back on his own tile centre three tiles off the tree. The leg there reads
            // clean (it ends on the face), so only the point test catches it. A corner two boxes share is no
            // hold either: stepping onto a diagonal touch threads it, and the next step leaves through the other
            // box's corner while no leg is ever strictly inside one.
            if grown.iter().any(|clear| point_inside(clear, vertex))
                || shared_corner(&grown, vertex)
            {
                continue;
            }
            if ray_hit(current, vertex, &grown).is_some() {
                continue;
            }
            if ray_hit(vertex, goal, &grown).is_none() {
                direct = Some(vertex);
                break;
            }
            if dist_sq(vertex, goal) < dist_sq(current, goal) {
                let key = (dist_sq(vertex, goal), vertex.0, vertex.1);
                if advance_key.is_none_or(|held| key < held) {
                    advance_key = Some(key);
                    advance = Some(vertex);
                }
            }
        }
        if let Some(vertex) = direct {
            current = vertex;
            path.push(current);
            if current != goal {
                current = goal;
                path.push(current);
            }
            break;
        }
        if let Some(vertex) = advance {
            current = vertex;
            path.push(current);
            continue;
        }
        if !expanded {
            // Once more over a wider search area before stopping beside the
            // walk's nearest point.
            expanded = true;
            continue;
        }
        // No direct vertex and no advance: the greedy walk stalls where it stands -
        // on a grown face, with the way round past a corner it may not turn toward
        // because every corner is farther from the goal than it is. Foray instead:
        // step to the certified box corner nearest the goal, closer or not, each
        // corner once, until a corner sees the goal or the fuel runs out. Corners,
        // not ring vertices, because an exact-fit seam - a one-tile gap at zero
        // clearance - hides its mouth inside the merged shape, where no ring vertex
        // stands, while its mouths are still some box's corners.
        //
        // A foray that never sees the goal keeps the walk's nearest approach when
        // the goal sits inside a grown box (a worker's tree: the stop beside it is
        // the whole answer), and otherwise rewinds to the stall point, so a walk
        // that cannot progress ends exactly where the greedy walk stopped - and a
        // walled-in goal's certified endpoint never moves.
        //
        // No foray for a stall on a *body* within two tiles of the goal when the goal stands outside
        // the boxes: the stop is already beside it, the snap settles it, and a foray here would wander the
        // neighbourhood and rewind to where it stands. (Crowded standoffs stall constantly; without
        // this every gather re-plan burns a foray.) A stall on *terrain* always forays: a wall needs
        // walking round, and the foray is the walk that does it - measured, a villager sent through the
        // corner two trees touch stalled a tile off his goal on a grown face, skipped the foray, and never
        // arrived. A stall at a goal that sits *inside* a grown box always forays too: its answer is the
        // nearest approach, kept without rewinding, so nothing is wasted - measured, a gatherer held two
        // tiles off his bush by a friend in the lane never reached it, because the skip stopped the walk
        // on the friend's face and the snap settled it three tiles back.
        let goal_inside = grown.iter().any(|clear| point_inside(clear, goal));
        let stalled_on_terrain = ray_hit(current, goal, &grown).is_none_or(|hit| hit < terrain_len);
        if !foray_near
            && !goal_inside
            && !stalled_on_terrain
            && dist_sq(current, goal) <= (2 * scale).saturating_mul(2 * scale) as i128
        {
            break;
        }
        let stall = current;
        let rewind_to = path.len();
        let mut visited = std::collections::BTreeSet::from([current]);
        let mut best_at = path.len();
        let mut best_key = dist_sq(current, goal);
        let mut escaped = false;
        // A foray at a goal that sits inside a grown box can never see it: every
        // ray to the goal ends inside. Its answer is the nearest approach, so the
        // foray stops when eight straight steps bring nothing nearer - the rest
        // would be wandering. A foray at a goal outside the boxes keeps going: it
        // is looking for a direct sighting, and only the fuel bounds it.
        let mut since_best = 0u32;
        // True sides, not the walk's inflated grid: a corner strictly inside
        // another box is no hold. Corner sets are cached by hit shape: the foray
        // usually crawls one shape for many steps, and rebuilding its flood fill
        // and corner list every step dominates the cost.
        let sides_list: Vec<(i64, i64, i64, i64)> = grown.iter().map(sides).collect();
        let mut cached_hit: Option<usize> = None;
        let mut cached_corners: Vec<(i64, i64)> = Vec::new();
        while fuel > 0 {
            fuel -= 1;
            let Some(hit) = ray_hit(current, goal, &grown) else {
                current = goal;
                path.push(current);
                escaped = true;
                break;
            };
            let bounds = margin(true);
            // The foray walks the hit shape's own corners: an exact-fit seam hides
            // its mouth inside the merged shape, where no ring vertex stands, while
            // the mouths are still some member box's corners.
            if cached_hit != Some(hit) {
                let member = shape_members(hit, &sides_list);
                let mut corners: Vec<(i64, i64)> = Vec::new();
                for (clear, held) in grown.iter().zip(member.iter()) {
                    if !held {
                        continue;
                    }
                    let (min_x, max_x, min_y, max_y) = sides(clear);
                    corners.push((min_x, min_y));
                    corners.push((max_x, min_y));
                    corners.push((min_x, max_y));
                    corners.push((max_x, max_y));
                }
                corners.sort_unstable();
                corners.dedup();
                cached_corners = corners;
                cached_hit = Some(hit);
            }
            let mut candidates: Vec<(i128, i64, i64)> = Vec::new();
            for corner in &cached_corners {
                if *corner == current || visited.contains(corner) || !inside(bounds, *corner) {
                    continue;
                }
                if grown.iter().any(|clear| point_inside(clear, *corner))
                    || shared_corner(&grown, *corner)
                {
                    continue;
                }
                if ray_hit(current, *corner, &grown).is_some() {
                    continue;
                }
                candidates.push((dist_sq(*corner, goal), corner.0, corner.1));
            }
            candidates.sort_unstable();
            let Some((_, vx, vy)) = candidates.first().copied() else {
                break;
            };
            let vertex = (vx, vy);
            visited.insert(vertex);
            current = vertex;
            path.push(current);
            let key = dist_sq(current, goal);
            if key < best_key {
                best_at = path.len();
                best_key = key;
                since_best = 0;
            } else {
                since_best += 1;
                if goal_inside && since_best > 8 {
                    break;
                }
            }
        }
        if escaped {
            // `current` is the goal: the smoothing below answers `Reached`.
        } else if goal_inside {
            // Stop at the nearest approach, keeping the certified stepping-stones
            // the foray walked to get there: the worker's tree is the whole answer.
            path.truncate(best_at);
            current = *path.last().unwrap_or(&stall);
        } else {
            current = stall;
            path.truncate(rewind_to);
        }
        break;
    }
    // Brute-force smooth: from each anchor the furthest later waypoint the
    // grown-segment test still certifies. Every skipped leg is one
    // `verify_path` would have passed, so the smoothed answer certifies.
    let mut smooth = vec![path[0]];
    let mut anchor = 0;
    while anchor + 1 < path.len() {
        let mut jump = anchor + 1;
        for next in anchor + 1..path.len() {
            if ray_hit(path[anchor], path[next], &grown).is_none() {
                jump = next;
            }
        }
        smooth.push(path[jump]);
        anchor = jump;
    }
    let points: Vec<(Fixed, Fixed)> = smooth.iter().map(|&point| to_fixed(point)).collect();
    if current == goal {
        WayRoute::Reached(points)
    } else {
        WayRoute::Closest(points)
    }
}

fn describe(clear: &Clear) -> String {
    format!(
        "box at {:?} half {:?} reach {}",
        clear.at, clear.half, clear.reach
    )
}

/// Certify a raw polyline against the grown `avoid` boxes: `None` when every
/// point stands clear and no segment passes inside a box, otherwise a
/// description of the first overlap in path order.
#[must_use]
pub fn verify_path(points_raw: &[(i64, i64)], avoid: &[Clear]) -> Option<String> {
    for (index, point) in points_raw.iter().enumerate() {
        if let Some(clear) = avoid.iter().find(|clear| point_inside(clear, *point)) {
            return Some(format!(
                "point {index} ({}, {}) inside {}",
                point.0,
                point.1,
                describe(clear)
            ));
        }
    }
    for (index, leg) in points_raw.windows(2).enumerate() {
        let (from, to) = (leg[0], leg[1]);
        if let Some(clear) = avoid.iter().find(|clear| segment_crosses(clear, from, to)) {
            return Some(format!(
                "segment {index} [({}, {}) -> ({}, {})] crosses {}",
                from.0,
                from.1,
                to.0,
                to.1,
                describe(clear)
            ));
        }
    }
    None
}
