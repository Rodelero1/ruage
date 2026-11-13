//! **Tile geometry: where a tile is, where its borders are, and where a circle of a given size can touch it** (D191).
//!
//! The owner, after seeing villagers stand at 1.156 tiles from a berry bush: *"the fixation on the tile center might be a problem.
//! Please implement also corner coordinates. Villagers building or gathering resources (except meat from animal) must be aligned
//! with borders of the tiles, not center of it."* And the rule behind it: *"center stays at least half the diameter of the circle
//! (however big that circle is)."*
//!
//! The numbers come from his saves: men at **0.70** from a resource tile's centre (`0.5 + 0.2`, a villager's circle touching - not
//! overlapping - the tile's edge) and at **0.99** where two edges meet (`0.707 + 0.2`). So the shape is the **square** in map
//! coordinates - the thing the renderer draws as a diamond - and the standoff is measured from its **border**, never its centre.
//!
//! Everything here is integer arithmetic in raw fixed-point units, so a tick stays deterministic and `no_float` keeps holding.

use ruage_fixed::Fixed;

/// The four corners of a tile, in raw map units, **anticlockwise from the low-x, low-y corner**: `(x, y)`, `(x+1, y)`,
/// `(x+1, y+1)`, `(x, y+1)` in tile coordinates. A renderer should draw the diamond between them; the simulation only ever needs the
/// square they bound.
#[must_use]
pub fn tile_corners(tile: (i64, i64), scale: i64) -> [(Fixed, Fixed); 4] {
    let (x, y) = (tile.0 * scale, tile.1 * scale);
    [
        (Fixed::from_raw(x), Fixed::from_raw(y)),
        (Fixed::from_raw(x + scale), Fixed::from_raw(y)),
        (Fixed::from_raw(x + scale), Fixed::from_raw(y + scale)),
        (Fixed::from_raw(x), Fixed::from_raw(y + scale)),
    ]
}

/// **How far a point is from a tile**, in raw units: zero inside it, and the distance to the nearest point of its square outside.
/// This is the one primitive a standoff, a clearance test and a placement check all want, and it is what makes "the circle may
/// never overlap the tile" a single comparison: `distance >= radius`.
#[must_use]
pub fn distance_to_tile(point: (Fixed, Fixed), tile: (i64, i64), scale: i64) -> i64 {
    let (px, py) = (point.0.raw(), point.1.raw());
    let (lo_x, lo_y) = (tile.0 * scale, tile.1 * scale);
    let (hi_x, hi_y) = (lo_x + scale, lo_y + scale);
    let dx = (lo_x - px).max(0).max(px - hi_x);
    let dy = (lo_y - py).max(0).max(py - hi_y);
    integer_sqrt(dx * dx + dy * dy)
}

/// The integer square root, rounded down. Kept here rather than shared with `lib.rs` because geometry is the only caller that needs
/// it without a `World`.
fn integer_sqrt(value: i64) -> i64 {
    if value <= 0 {
        return 0;
    }
    let mut low = 0i64;
    let mut high = value.min(1 << 31);
    while low < high {
        let middle = (low + high + 1) / 2;
        if middle * middle <= value {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    low
}

/// Sin of 0, 2, 4 ... 90 degrees in millionths, the same quarter table the gather ring uses.
const SIN: [i64; 46] = [
    0, 34899, 69756, 104528, 139173, 173648, 207912, 241922, 275637, 309017, 342020, 374607,
    406737, 438371, 469472, 500000, 529919, 559193, 587785, 615661, 642788, 669131, 694658, 719340,
    743145, 766044, 788011, 809017, 829038, 848048, 866025, 882948, 898794, 913545, 927184, 939693,
    951057, 961262, 970296, 978148, 984808, 990268, 994522, 997564, 999391, 1000000,
];

/// **The places where a circle of `radius` touches the tile without overlapping it**, at least `spacing` apart: the outline of the
/// square grown by the radius (straight sides, quarter-circle corners), walked once in a fixed order.
///
/// This is the whole standoff rule in one function. The owner's rule is that a villager works where **his circle fits**, so the
/// number of places is not chosen - it falls out of `perimeter / spacing`, which for a villager on one tile is
/// `4 + 2 * pi * 0.2 = 5.257` tiles of outline: **thirteen** at a body's width (0.4) and **seven** at the 0.66 the owner's
/// saves measure between working men (D199/D202).
///
/// **The places are spread evenly, not fitted greedily.** The first version walked the outline and kept any candidate at least
/// `spacing` from the ones already kept, which loses a place at every seam: it reported 11 where the arithmetic allows 13, and
/// only **5** at 0.66 where it allows 7 - and with friends blocking (D200) a missing place is a man who cannot work. The walk
/// below emits a place every `perimeter / count` of **arc length**, so the count is the arithmetic one.
#[must_use]
pub fn touching_places(
    tile: (i64, i64),
    radius: i64,
    spacing: i64,
    scale: i64,
) -> Vec<(Fixed, Fixed)> {
    if spacing <= 0 || radius < 0 || scale <= 0 {
        return Vec::new();
    }
    let outline = grown_outline(tile, radius, scale);
    let Some(&first) = outline.first() else {
        return Vec::new();
    };
    // Four sides plus four quarter circles: `4 * scale + 2 * pi * radius`, with pi as 3_141_593 millionths.
    let quarter = radius * 1_571 / 1_000;
    let perimeter = 4 * scale + 4 * quarter;
    let mut count = (perimeter / spacing).max(1);
    // **Round a corner, the chord is shorter than the arc.** Spreading `perimeter / spacing` places evenly gives the
    // arithmetic count, but two places straddling a corner end up closer in a straight line than along the outline. The
    // physical rule is what must hold: two bodies of this radius may never be inside one another, so the count steps down
    // until the shortest *chord* between neighbours is at least `2 * radius`. `spacing` is the comfort distance (0.66 for
    // gatherers, the owner's measurement); the overlap distance is the hard one. A villager gets **12** places at 0.4 and
    // **7** at 0.66 - the same seven the owner's saves show.
    let physical = (2 * radius).max(1);
    loop {
        let points = spread(&outline, perimeter, count);
        if count <= 1 || neighbours_hold(&points, physical) {
            return points;
        }
        count -= 1;
    }
}

/// Emit `count` places from the outline, one every `perimeter / count` of **arc length**, starting at the first point.
fn spread(outline: &[(i64, i64)], perimeter: i64, count: i64) -> Vec<(Fixed, Fixed)> {
    let Some(&first) = outline.first() else {
        return Vec::new();
    };
    let want = perimeter / count.max(1);
    let mut kept: Vec<(Fixed, Fixed)> = Vec::with_capacity(usize::try_from(count).unwrap_or(16));
    kept.push((Fixed::from_raw(first.0), Fixed::from_raw(first.1)));
    let mut since = 0i64;
    let mut last = first;
    for &point in outline.iter().skip(1) {
        let (dx, dy) = (point.0 - last.0, point.1 - last.1);
        since += integer_sqrt(dx * dx + dy * dy);
        if since >= want {
            since -= want;
            kept.push((Fixed::from_raw(point.0), Fixed::from_raw(point.1)));
        }
        last = point;
    }
    kept
}

/// Whether every pair of **neighbours** in the walk is at least `spacing` apart in a straight line, including the pair that
/// closes the loop. The outline is convex and walked once, so a pair that is not adjacent is further apart than the pair next
/// to it - checking neighbours is enough.
fn neighbours_hold(points: &[(Fixed, Fixed)], spacing: i64) -> bool {
    let near = |a: (Fixed, Fixed), b: (Fixed, Fixed)| {
        let (dx, dy) = (a.0.raw() - b.0.raw(), a.1.raw() - b.1.raw());
        dx * dx + dy * dy >= spacing * spacing
    };
    points.windows(2).all(|pair| near(pair[0], pair[1]))
        && match (points.first(), points.last()) {
            (Some(first), Some(last)) => near(*first, *last),
            _ => true,
        }
}

/// The grown square's outline in raw units, **in walk order** (bottom, bottom-right corner, right, top-right, top, top-left,
/// left, bottom-left), sampled finely enough that arc length can be accumulated by walking it. Corners are 2-degree steps of
/// the same sine table the ring uses; sides are `scale / 32` apart.
fn grown_outline(tile: (i64, i64), radius: i64, scale: i64) -> Vec<(i64, i64)> {
    let (lo_x, lo_y) = (tile.0 * scale, tile.1 * scale);
    let (hi_x, hi_y) = (lo_x + scale, lo_y + scale);
    let step = (scale / 32).max(1);
    let mut out: Vec<(i64, i64)> = Vec::new();
    let mut push = |x: i64, y: i64| out.push((x, y));

    let mut along = 0;
    while along <= scale {
        push(lo_x + along, lo_y - radius);
        along += step;
    }
    // Bottom-right: sin/cos of 0..90 degrees, from `(hi_x, lo_y - r)` to `(hi_x + r, lo_y)`.
    for degree in 0..=45usize {
        let (sin, cos) = (SIN[degree], SIN[45 - degree]);
        push(hi_x + radius * sin / 1_000_000, lo_y - radius * cos / 1_000_000);
    }
    let mut along = step;
    while along <= scale {
        push(hi_x + radius, lo_y + along);
        along += step;
    }
    // Top-right: from `(hi_x + r, hi_y)` to `(hi_x, hi_y + r)`.
    for degree in 0..=45usize {
        let (sin, cos) = (SIN[degree], SIN[45 - degree]);
        push(hi_x + radius * cos / 1_000_000, hi_y + radius * sin / 1_000_000);
    }
    let mut along = step;
    while along <= scale {
        push(hi_x - along, hi_y + radius);
        along += step;
    }
    // Top-left: from `(lo_x, hi_y + r)` to `(lo_x - r, hi_y)`.
    for degree in 0..=45usize {
        let (sin, cos) = (SIN[degree], SIN[45 - degree]);
        push(lo_x - radius * sin / 1_000_000, hi_y + radius * cos / 1_000_000);
    }
    let mut along = step;
    while along <= scale {
        push(lo_x - radius, hi_y - along);
        along += step;
    }
    // Bottom-left: from `(lo_x - r, lo_y)` back to `(lo_x, lo_y - r)`.
    for degree in 0..=45usize {
        let (sin, cos) = (SIN[degree], SIN[45 - degree]);
        push(lo_x - radius * cos / 1_000_000, lo_y - radius * sin / 1_000_000);
    }
    out
}
