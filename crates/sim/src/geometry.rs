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
/// no multiply or divide - while the ray setup above is where the one exact
/// division lives. Every edge of the ring lies on a grown face, which is
/// exactly what `verify_path` calls clear, so the ring it returns certifies
/// clean.
#[must_use]
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
    // shared face, transitively, in slice order so the set is deterministic.
    let touches = |a: (i64, i64, i64, i64), b: (i64, i64, i64, i64)| {
        !(a.1 < b.0 || b.1 < a.0 || a.3 < b.2 || b.3 < a.2)
    };
    let mut member = vec![false; rects.len()];
    member[hit] = true;
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
