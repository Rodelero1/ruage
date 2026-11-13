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
/// `4 + 2 * pi * 0.2 = 5.26` tiles of outline and `0.4` of body: **thirteen places**, and a bigger circle fits fewer.
///
/// The points are found by building candidates along the outline and keeping those at least `spacing` from every point kept
/// before, in a fixed order - so the list is **deterministic**, which the hashed simulation requires.
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
    let (lo_x, lo_y) = (tile.0 * scale, tile.1 * scale);
    let (hi_x, hi_y) = (lo_x + scale, lo_y + scale);
    let mut candidates: Vec<(i64, i64)> = Vec::new();

    // **The straight sides**, grown by the radius, walked every `spacing / 2` so the greedy pass has enough to choose from.
    let step = (spacing / 2).max(1);
    let mut along = 0;
    while along <= scale {
        candidates.push((lo_x + along, lo_y - radius));
        candidates.push((lo_x + along, hi_y + radius));
        candidates.push((lo_x - radius, lo_y + along));
        candidates.push((hi_x + radius, lo_y + along));
        along += step;
    }
    // **The rounded corners**: the quarter circle of `radius` round each corner point, every two degrees.
    for (corner_x, corner_y) in [(lo_x, lo_y), (hi_x, lo_y), (hi_x, hi_y), (lo_x, hi_y)] {
        for degree in 0..=45usize {
            let (sin, cos) = (SIN[degree], SIN[45 - degree]);
            candidates.push((
                corner_x + radius * cos / 1_000_000 * if corner_x == hi_x { 1 } else { -1 },
                corner_y + radius * sin / 1_000_000 * if corner_y == hi_y { 1 } else { -1 },
            ));
        }
    }

    // **Keep only what fits**: a place is a place while it is `spacing` from every place already taken.
    let mut kept: Vec<(i64, i64)> = Vec::new();
    for (x, y) in candidates {
        if distance_to_tile((Fixed::from_raw(x), Fixed::from_raw(y)), tile, scale) < radius {
            continue;
        }
        if kept
            .iter()
            .any(|(kx, ky)| (kx - x).pow(2) + (ky - y).pow(2) < spacing.pow(2))
        {
            continue;
        }
        kept.push((x, y));
    }
    kept.into_iter()
        .map(|(x, y)| (Fixed::from_raw(x), Fixed::from_raw(y)))
        .collect()
}
