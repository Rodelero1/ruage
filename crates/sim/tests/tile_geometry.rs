//! **Tile geometry: corners, borders and the places where a circle touches a tile** (D191). The owner:
//!
//! > *"Do we know coordinates of the tiles and their corners? If not, it should be added. Because all work of villagers depends on
//! > that ... Villagers building or gathering resources (except meat from animal) must be aligned with borders of the tiles, not
//! > center of it."*
//!
//! And the distances his saves measure: men standing **0.70** from a resource tile's centre, which is `0.5 + 0.2` — a villager's
//! circle touching the tile's edge — and **0.99** where the circle touches a corner. Everything below is integer arithmetic, so the
//! sim stays deterministic and `no_float` keeps holding.
//!
//! Written before the implementation (D7, SPEC §11).

use ruage_fixed::Fixed;
use ruage_sim::tiles::{distance_to_tile, touching_places};

const SCALE: i64 = 20_000;

fn at(tile: (i64, i64)) -> (Fixed, Fixed) {
    (
        Fixed::from_raw(tile.0 * SCALE + SCALE / 2),
        Fixed::from_raw(tile.1 * SCALE + SCALE / 2),
    )
}

fn raw(point: (Fixed, Fixed)) -> (i64, i64) {
    (point.0.raw(), point.1.raw())
}

/// **A tile's own centre is inside it, and the distance grows by the tile's half-width at the edge.** The four side midpoints of a
/// 1x1 tile are 0.5 tiles from its centre, and a corner is `0.5 * sqrt(2) = 0.707`.
#[test]
fn the_distance_to_a_tile_is_zero_inside_and_half_at_the_edge() {
    let tile = (10i64, 10i64);
    assert_eq!(
        distance_to_tile(at(tile), tile, SCALE),
        0,
        "the centre is inside"
    );
    for (dx, dy) in [(1i64, 0i64), (-1, 0), (0, 1), (0, -1)] {
        let point = (
            Fixed::from_raw((tile.0 + dx) * SCALE + SCALE / 2),
            Fixed::from_raw((tile.1 + dy) * SCALE + SCALE / 2),
        );
        assert_eq!(
            distance_to_tile(point, tile, SCALE),
            SCALE / 2,
            "a neighbouring tile's centre is half a tile from this one's edge"
        );
    }
    // Diagonally: 0.7071 tiles, so 14,142 raw units within one.
    let corner = (
        Fixed::from_raw((tile.0 + 1) * SCALE + SCALE / 2),
        Fixed::from_raw((tile.1 + 1) * SCALE + SCALE / 2),
    );
    let diagonal = distance_to_tile(corner, tile, SCALE);
    assert!(
        (diagonal - 14_142).abs() <= 2,
        "MEASURED {diagonal} raw units to the corner; 0.7071 tiles is 14,142"
    );
}

/// **The places where a villager's circle touches the tile are exactly `radius` from it** - never inside - and they are one
/// diameter apart, which is what "either the villager fits in or he does not" means. This is the number the whole standoff rests
/// on: `0.5 + 0.2 = 0.70` at a side, `0.707 + 0.2 = 0.907` at a corner.
#[test]
fn the_places_touch_the_tile_and_are_a_diameter_apart() {
    let tile = (10i64, 10i64);
    let radius = SCALE / 5; // a villager's 0.2
    let places = touching_places(tile, radius, 2 * radius, SCALE);
    assert!(
        places.len() >= 8,
        "MEASURED {} places round one tile; the perimeter is 4 + 2*pi*0.2 = 5.26 tiles and a body needs 0.4",
        places.len()
    );
    for (index, place) in places.iter().enumerate() {
        let distance = distance_to_tile(*place, tile, SCALE);
        assert!(
            distance >= radius - 2,
            "MEASURED place {index} only {distance} raw units from the tile; the circle is {radius} wide"
        );
        assert!(
            distance <= radius + 2,
            "MEASURED place {index} {distance} raw units out; a touching place is {radius}"
        );
    }
    // **No two places closer than a diameter**, which is the packing rule itself.
    let mut closest = i64::MAX;
    for a in 0..places.len() {
        for b in (a + 1)..places.len() {
            let (ax, ay) = raw(places[a]);
            let (bx, by) = raw(places[b]);
            let apart = ((ax - bx).pow(2) + (ay - by).pow(2)) as f64;
            closest = closest.min(apart.sqrt() as i64);
        }
    }
    assert!(
        closest >= 2 * radius - 40,
        "MEASURED the closest pair of places {closest} raw units apart; a diameter is {}",
        2 * radius
    );
}

/// **The same places come out of the same tile every time, in the same order** - the sim is hashed, and a geometry that reorders
/// itself between runs is a desync waiting to happen.
#[test]
fn the_places_are_the_same_list_every_time() {
    let tile = (4i64, 9i64);
    let radius = SCALE / 5;
    let first = touching_places(tile, radius, 2 * radius, SCALE);
    let second = touching_places(tile, radius, 2 * radius, SCALE);
    assert_eq!(first, second, "the same tile must give the same places");
}

/// **A bigger circle stands further out and fits fewer times** — the owner: *"some units have different circle sizes - center stays
/// at least half the diameter of the circle (however big that circle is)"*.
#[test]
fn a_bigger_circle_stands_further_out_and_fits_fewer_times() {
    let tile = (7i64, 7i64);
    let small = touching_places(tile, SCALE / 5, 2 * SCALE / 5, SCALE);
    let big_radius = SCALE * 2 / 5; // 0.4, twice a villager
    let big = touching_places(tile, big_radius, 2 * big_radius, SCALE);
    assert!(
        big.len() < small.len(),
        "MEASURED {} places for the big circle against {} for the small one",
        big.len(),
        small.len()
    );
    for place in &big {
        let distance = distance_to_tile(*place, tile, SCALE);
        assert!(
            distance >= big_radius - 2,
            "a big circle's place is {distance} out, and its radius is {big_radius}"
        );
    }
}

