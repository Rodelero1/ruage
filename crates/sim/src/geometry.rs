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
