//! **Land routes answer from the geometry solver** (New Way Task 4): `route_with`
//! calls `geometry::solve` for the land path before the tier arm, mapping
//! `Reached` to the goal-exact tail and `Closest` to the stopped-beside tail.
//! Tier + flat code stays but is bypassed for the reach values Task 3 exercised.

use ruage_fixed::Fixed;
use ruage_sim::{Clear, Ground, verify_path};

const SCALE: i64 = 20_000;
/// The only reach the solver has been exercised with (Task 3 pins `reach = 0`):
/// the land arm answers these walks from geometry.
const REACH: i64 = 0;

const MAP: &str = r#"
schema = 1
[map]
width = 16
height = 16
terrain = [
  "GGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGG",
]
[[map.town_center]]
player = 1
x = 1
y = 1
[[map.town_center]]
player = 2
x = 14
y = 14
"#;

fn open_ground_16() -> Ground {
    let resources = ruage_data::load_resources_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../config/resources.toml"
        ))
        .expect("config/resources.toml"),
        "resources.toml",
    )
    .expect("resources.toml");
    let map = ruage_data::load_map_str(MAP, "geometry_land.toml", &resources)
        .expect("the fixture map loads");
    Ground::from_map(&map)
}

fn at(x: i64, y: i64) -> (Fixed, Fixed) {
    (
        Fixed::from_raw(x * SCALE + SCALE / 2),
        Fixed::from_raw(y * SCALE + SCALE / 2),
    )
}

fn from_pt() -> (Fixed, Fixed) {
    at(1, 1)
}

fn to_pt() -> (Fixed, Fixed) {
    at(14, 14)
}

#[test]
fn land_uses_geometry_not_tiles() {
    let ground = open_ground_16();
    let route = ground
        .route_avoiding(SCALE, from_pt(), to_pt(), REACH, &[], true)
        .expect("geometry answers");
    assert!(route.len() <= 2);
    assert_eq!(*route.last().unwrap(), to_pt());
    // The standing point is dropped: the route holds only the waypoints ahead of the body.
    assert_eq!(route, vec![to_pt()]);
}

fn raw(route: &[(Fixed, Fixed)]) -> Vec<(i64, i64)> {
    route
        .iter()
        .map(|point| (point.0.raw(), point.1.raw()))
        .collect()
}

fn is_tile_centre(point: (Fixed, Fixed)) -> bool {
    point.0.raw() % SCALE == SCALE / 2 && point.1.raw() % SCALE == SCALE / 2
}

/// **Closest settles on the tile centre** (the brief's tail rule, same as the flat search's
/// best-settled): a start walled inside a pocket of standers cannot reach the goal, so the route
/// stops beside them - on the settled tile's centre, not on a grown face.
#[test]
fn closest_settles_on_the_tile_centre() {
    let ground = open_ground_16();
    // Off-centre inside the pocket, so the settled centre differs from where the body stands.
    let from = (Fixed::from_raw(35_000), Fixed::from_raw(35_000));
    let (fx, fy) = (35_000i64, 35_000i64);
    let pocket = vec![
        Clear {
            at: (fx, fy + 30_000),
            half: (40_000, 5_000),
            reach: 0,
        },
        Clear {
            at: (fx, fy - 30_000),
            half: (40_000, 5_000),
            reach: 0,
        },
        Clear {
            at: (fx + 30_000, fy),
            half: (5_000, 40_000),
            reach: 0,
        },
        Clear {
            at: (fx - 30_000, fy),
            half: (5_000, 40_000),
            reach: 0,
        },
    ];
    let route = ground
        .route_avoiding(SCALE, from, to_pt(), REACH, &pocket, true)
        .expect("Closest still answers");
    let last = *route.last().unwrap();
    assert_ne!(last, to_pt(), "the goal is walled off, not reached");
    assert_ne!(
        last, from,
        "the endpoint settles, it does not stay a grown-face point"
    );
    // The settled tile is the goal-nearest clean standable centre: the stall's own tile (1, 3) is
    // standable, but its centre sits behind the pocket's north wall, so its leg is dirty and the
    // settle falls through to the next tile over. Rebased from (1, 1) when the solver's hit-box
    // corners joined the advance vertices and the stall moved onto the wall.
    assert_eq!(last, at(2, 2), "the stop is the settled tile's centre");
    assert!(is_tile_centre(last));
    assert!(
        verify_path(&raw(&route), &pocket).is_none(),
        "the snapped point and its leg stay clear of the grown boxes: {route:?}"
    );
}

/// **No clean snap keeps the solver endpoint**: the functional gates (never stuck, never through a
/// blocked area) outrank the tail shape, so when the settled centre sits inside a grown box the
/// certified endpoint stands instead of a dirty snap.
#[test]
fn closest_without_a_clean_centre_keeps_the_solver_endpoint() {
    let ground = open_ground_16();
    let (gx, gy) = (14 * SCALE + SCALE / 2, 14 * SCALE + SCALE / 2);
    let boxed = vec![Clear {
        at: (gx, gy),
        half: (10_000, 10_000),
        reach: 0,
    }];
    let route = ground
        .route_avoiding(SCALE, from_pt(), to_pt(), REACH, &boxed, true)
        .expect("Closest still answers");
    let last = *route.last().unwrap();
    assert_ne!(last, to_pt(), "the goal is walled off, not reached");
    assert!(
        !is_tile_centre(last),
        "no centre here is clean, so the grown-face endpoint stands: {last:?}"
    );
    // The stall stops on the arrival face's middle, not the grown corner: face middles joined the
    // advance vertices, and the west middle is goal-nearest. Still unsnapped - no settled centre is
    // clean, so the certified endpoint stands.
    assert_eq!(
        last,
        (Fixed::from_raw(280_000), Fixed::from_raw(290_000)),
        "the solver's certified ring exit, unsnapped"
    );
    assert!(
        verify_path(&raw(&route), &boxed).is_none(),
        "the kept endpoint and its leg stay clear of the grown box: {route:?}"
    );
}
