//! **A drag becomes a line** (D155): the geometry of wall placement, with no world and no state.
//!
//! The owner's rules: walls go *vertically, horizontally* and *at 45 degrees*; *first left click starts placement, second click
//! ends it*; the line may have *one corner*, and when the drag is neither straight nor diagonal the engine goes **45 degrees
//! first, then straight** (his answer A). Ineligible segments are skipped later, by the command - this module only says which
//! tiles the drag covers.
//!
//! It is a plain function rather than a method on the world for one reason: the **view** needs the same line to draw its
//! semi-transparent preview and red ineligible tiles, and two implementations of "where does the wall go" would disagree.

/// The tiles a drag from `from` to `to` covers, both ends included, in the order the drag was made.
///
/// One bend at most: a **45 degree leg first** (both axes at once, corner to corner - which blocks passage, because a diagonal
/// step in the search requires both of its corners free), then the straight leg along whichever axis is longer.
///
/// # Examples
///
/// A drag from `(5, 5)` to `(9, 7)` is `(5,5), (6,6), (7,7), (8,7), (9,7)` - the diagonal to the shorter axis, then along it.
#[must_use]
pub fn wall_line(from: (u16, u16), to: (u16, u16)) -> Vec<(u16, u16)> {
    let dx = i64::from(to.0) - i64::from(from.0);
    let dy = i64::from(to.1) - i64::from(from.1);
    let diagonal = dx.abs().min(dy.abs());
    let mut out = Vec::with_capacity(usize::try_from(dx.abs().max(dy.abs())).unwrap_or(0) + 1);
    let (mut x, mut y) = (i64::from(from.0), i64::from(from.1));
    out.push(from);
    if dx == 0 && dy == 0 {
        return out;
    }
    let (sx, sy) = (dx.signum(), dy.signum());
    for _ in 0..diagonal {
        x += sx;
        y += sy;
        out.push(as_tile(x, y));
    }
    while x != i64::from(to.0) || y != i64::from(to.1) {
        if x != i64::from(to.0) {
            x += sx;
        } else {
            y += sy;
        }
        out.push(as_tile(x, y));
    }
    out
}

/// Tiles are `u16` and every step stays inside the map the drag was made on; a caller that hands in a line running off the map
/// gets the edge tile repeated, which the command then refuses as a site rather than silently wrapping.
fn as_tile(x: i64, y: i64) -> (u16, u16) {
    (
        u16::try_from(x.clamp(0, i64::from(u16::MAX))).unwrap_or(u16::MAX),
        u16::try_from(y.clamp(0, i64::from(u16::MAX))).unwrap_or(u16::MAX),
    )
}
