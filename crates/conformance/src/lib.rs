//! **The conformance set: rules of the original's pathing, asserted against this engine.**
//!
//! Every test in `tests/` states one rule, gives the place the rule was read from, and checks our side against it. The
//! source is `samuelcaldas/age-of-empires-2-decomp` (matching decompilation of AoC 1.0c / AoK 2.0a / the PS2 ELF, **no
//! licence**): read for behaviour and cited by module, never transcribed - and its addresses are AoC/PS2, so a number that
//! HD 5.7 carries differently is pinned to the HD binary's own site, which `docs/movement-in-the-binary.md` holds.
//!
//! **This set is not the gate.** `cargo test` at the workspace root does not run it; `make conformance` does, and a test
//! that fails here is a *finding* about our engine, not a broken build. Rules the engine deliberately does not implement are
//! `#[ignore]`d with the reason and the decision, so `cargo test -p ruage-conformance -- --ignored` prints exactly the list
//! of places where we stand apart from the original.
//!
//! The helpers below are the same shapes the engine's own fixtures use - a bare `Walk` for the tile layer, raw points for
//! the body layer - so a rule can be checked without building a world, a tick or a match.

use ruage_data::{Map, Terrain};
use ruage_fixed::Fixed;
use ruage_sim::Walk;

/// The project's scale: one tile is 20,000 raw units.
pub const SCALE: i64 = 20_000;
/// A villager's body - the dat's `radius[0]`, one fifth of a tile.
pub const RADIUS: i64 = 4_000;
/// The engine's tile-step costs (`crates/sim/src/path.rs`, D249), repeated here so a cost claim is checked against the
/// number it claims rather than against whatever the engine happens to do. **The original's model**: 16 a cardinal step,
/// 23 a diagonal (its octile table's `16 x sqrt2`, rounded).
pub const CARDINAL: u32 = 16;
pub const DIAGONAL: u32 = 23;

/// An empty map of the given size: grass, no resources, no town centres.
pub fn open_map(width: u16, height: u16) -> Map {
    Map {
        width,
        height,
        terrain: vec![Terrain::Grass; usize::from(width) * usize::from(height)],
        resources: Vec::new(),
        town_centers: Vec::new(),
    }
}

/// A walk layer with the named tiles occupied - `Walk::block`, which is what a tree or a wall does.
pub fn walk(width: u16, height: u16, blocked: &[(u16, u16)]) -> Walk {
    let mut walk = Walk::from_map(&open_map(width, height));
    for (x, y) in blocked {
        walk.block(*x, *y);
    }
    walk
}

/// A point in raw units, from tile coordinates.
pub fn at(x: i64, y: i64) -> (Fixed, Fixed) {
    (Fixed::from_raw(x * SCALE), Fixed::from_raw(y * SCALE))
}

/// The cost of a tile route under the engine's own step costs. The route excludes its start, so the first step is charged
/// by the caller's `from`.
pub fn route_cost(from: (u16, u16), route: &[(u16, u16)]) -> u32 {
    let mut total = 0;
    let mut here = from;
    for &next in route {
        let diagonal = next.0 != here.0 && next.1 != here.1;
        total += if diagonal { DIAGONAL } else { CARDINAL };
        here = next;
    }
    total
}

/// The octile lower bound between two tiles: what no route of king moves can beat, and what an admissible heuristic must
/// never exceed.
pub fn octile(from: (u16, u16), to: (u16, u16)) -> u32 {
    let dx = i64::from(from.0).abs_diff(i64::from(to.0));
    let dy = i64::from(from.1).abs_diff(i64::from(to.1));
    let diagonal = dx.min(dy);
    let straight = dx.max(dy) - diagonal;
    (straight * u64::from(CARDINAL) + diagonal * u64::from(DIAGONAL)) as u32
}

/// The squared distance from a point to a tile's square in raw units - zero when the point is inside the tile. Squared
/// because the body test is a comparison against `radius * radius` and a square root would only lose precision.
pub fn distance_sq_to_tile(point: (Fixed, Fixed), tile: (u16, u16)) -> i64 {
    let (x0, y0) = (i64::from(tile.0) * SCALE, i64::from(tile.1) * SCALE);
    let (x1, y1) = (x0 + SCALE, y0 + SCALE);
    let (px, py) = (point.0.raw(), point.1.raw());
    let dx = (x0 - px).max(0).max(px - x1);
    let dy = (y0 - py).max(0).max(py - y1);
    dx * dx + dy * dy
}

/// The squared distance from a segment to a point, in raw units, by projection with integer arithmetic.
///
/// The intermediate product is `i128`: raw units run to hundreds of thousands per tile, so a cross product squares past
/// `i64` on any walk longer than a few tiles. The answer is clamped back to `i64` because every caller compares it against a
/// radius squared, which is small.
pub fn segment_distance_sq(a: (Fixed, Fixed), b: (Fixed, Fixed), p: (Fixed, Fixed)) -> i64 {
    let (ax, ay) = (i128::from(a.0.raw()), i128::from(a.1.raw()));
    let (bx, by) = (i128::from(b.0.raw()), i128::from(b.1.raw()));
    let (px, py) = (i128::from(p.0.raw()), i128::from(p.1.raw()));
    let (dx, dy) = (bx - ax, by - ay);
    let (ex, ey) = (px - ax, py - ay);
    let dot = ex * dx + ey * dy;
    let len = dx * dx + dy * dy;
    let squared = if len == 0 || dot <= 0 {
        ex * ex + ey * ey
    } else if dot >= len {
        (px - bx) * (px - bx) + (py - by) * (py - by)
    } else {
        // |e - dot/len * d|^2, kept integral: the perpendicular distance squared is cross^2 / len.
        let cross = ex * dy - ey * dx;
        cross * cross / len
    };
    i64::try_from(squared).unwrap_or(i64::MAX)
}

/// The closest squared distance a polyline comes to a tile, over its segments.
pub fn polyline_distance_sq_to_tile(points: &[(Fixed, Fixed)], tile: (u16, u16)) -> i64 {
    // The tile's corners and the four edges, checked by sampling the segment against the square analytically: exact for an
    // axis-aligned box is `distance_sq_to_tile` at the nearest point, so walk the segment and project on each edge.
    let (x0, y0) = (i64::from(tile.0) * SCALE, i64::from(tile.1) * SCALE);
    let corners = [
        at(tile.0.into(), tile.1.into()),
        at(i64::from(tile.0) + 1, i64::from(tile.1)),
        at(i64::from(tile.0) + 1, i64::from(tile.1) + 1),
        at(i64::from(tile.0), i64::from(tile.1) + 1),
    ];
    let mut best = i64::MAX;
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        // Ends inside the tile are zero distance.
        for point in [a, b] {
            let (px, py) = (point.0.raw(), point.1.raw());
            if px >= x0 && px <= x0 + SCALE && py >= y0 && py <= y0 + SCALE {
                return 0;
            }
        }
        // Otherwise the minimum over the box's four edges.
        for edge in 0..4 {
            let (c, d) = (corners[edge], corners[(edge + 1) % 4]);
            let distance = segment_to_segment_sq(a, b, c, d);
            best = best.min(distance);
        }
    }
    best
}

/// Squared distance between two segments, by candidates: each endpoint against the other segment, plus a proper-crossing
/// test that answers zero.
fn segment_to_segment_sq(
    a: (Fixed, Fixed),
    b: (Fixed, Fixed),
    c: (Fixed, Fixed),
    d: (Fixed, Fixed),
) -> i64 {
    let mut best = segment_distance_sq(a, b, c)
        .min(segment_distance_sq(a, b, d))
        .min(segment_distance_sq(c, d, a))
        .min(segment_distance_sq(c, d, b));
    let cross = |p: (Fixed, Fixed), q: (Fixed, Fixed), r: (Fixed, Fixed)| -> i128 {
        let (ax, ay) = (q.0.raw() - p.0.raw(), q.1.raw() - p.1.raw());
        let (bx, by) = (r.0.raw() - p.0.raw(), r.1.raw() - p.1.raw());
        i128::from(ax) * i128::from(by) - i128::from(ay) * i128::from(bx)
    };
    let (d1, d2) = (cross(c, d, a), cross(c, d, b));
    let (d3, d4) = (cross(a, b, c), cross(a, b, d));
    if ((d1 > 0) != (d2 > 0)) && ((d3 > 0) != (d4 > 0)) {
        best = 0;
    }
    best
}

/// The closest squared distance a polyline comes to a raw point.
pub fn polyline_distance_sq_to_point(points: &[(Fixed, Fixed)], p: (Fixed, Fixed)) -> i64 {
    points
        .windows(2)
        .map(|pair| segment_distance_sq(pair[0], pair[1], p))
        .min()
        .unwrap_or(i64::MAX)
}
