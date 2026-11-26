//! **All movers route through the geometric solver; nothing builds a pyramid** (New Way Task 5).
//!
//! Land, ships, flying things and projectiles share one router: `geometry::solve`.
//! Each mover's rect source differs - land feeds its blocked tiles as grown `Clear`
//! (the same shape as the occupant rects), ships feed the water-layer tiles, and
//! flying things and projectiles pass an empty avoid list (direct) - but the
//! solver and its contract are the same. What this file pins:
//!
//! - a wall of water rects is avoided by a ship and ignored by a direct flight;
//! - no route, at any reach, builds the level pyramid (it is deleted with the tile system).

use ruage_fixed::Fixed;
use ruage_sim::{Clear, Ground, WayRoute, solve, verify_path};

const SCALE: i64 = 20_000;
/// A ship's own reach: half a tile, so its grown boxes are a full tile wider than the water tiles.
const SHIP_REACH: i64 = 10_000;

const MAP_16: &str = r#"
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

const MAP_64: &str = r#"
schema = 1
[map]
width = 64
height = 16
terrain = [
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
]
[[map.town_center]]
player = 1
x = 1
y = 1
[[map.town_center]]
player = 2
x = 62
y = 14
"#;

fn ground_of(map: &str, name: &str) -> Ground {
    let resources = ruage_data::load_resources_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../config/resources.toml"
        ))
        .expect("config/resources.toml"),
        "resources.toml",
    )
    .expect("resources.toml");
    let map = ruage_data::load_map_str(map, name, &resources).expect("the fixture map loads");
    Ground::from_map(&map)
}

fn open_ground_16() -> Ground {
    ground_of(MAP_16, "geometry_movers_16.toml")
}

fn open_ground_64() -> Ground {
    ground_of(MAP_64, "geometry_movers_64.toml")
}

/// One blocked tile's rect, the shape the water layer feeds the ship: the tile's own
/// box, grown by the mover at solve time.
fn tile_box(x: i64, y: i64) -> Clear {
    Clear {
        at: (x * SCALE + SCALE / 2, y * SCALE + SCALE / 2),
        half: (SCALE / 2, SCALE / 2),
        reach: 0,
    }
}

fn water_rects_for_test_map() -> Vec<Clear> {
    // A wall of water down x = 8, through the walk's own row band: a ship must go round it.
    (0..=10).map(|y| tile_box(8, y)).collect()
}

fn centre(x: i64, y: i64) -> (Fixed, Fixed) {
    (
        Fixed::from_raw(x * SCALE + SCALE / 2),
        Fixed::from_raw(y * SCALE + SCALE / 2),
    )
}

fn from_pt() -> (Fixed, Fixed) {
    centre(4, 5)
}

fn to_pt() -> (Fixed, Fixed) {
    centre(12, 5)
}

fn raw_points(route: &WayRoute) -> Vec<(i64, i64)> {
    let points = match route {
        WayRoute::Reached(points) | WayRoute::Closest(points) => points,
    };
    points
        .iter()
        .map(|point| (point.0.raw(), point.1.raw()))
        .collect()
}

#[test]
fn ships_avoid_water_rects_flying_goes_direct() {
    let water_rects = water_rects_for_test_map();
    assert!(!water_rects.is_empty());
    let air = solve(SCALE, from_pt(), to_pt(), 0, &[], &[], true);
    assert!(matches!(air, WayRoute::Reached(_)));
    let sea = solve(
        SCALE,
        from_pt(),
        to_pt(),
        SHIP_REACH,
        &water_rects,
        &[],
        true,
    );
    assert!(verify_path(&raw_points(&sea), &water_rects).is_none());
}

#[test]
fn no_pyramid_builds() {
    let ground = open_ground_16();
    let _ = ground.route(SCALE, from_pt(), to_pt());
    assert_eq!(ground.pyramid_builds(), 0);
}

/// **A nonzero-reach walk past a wall builds no pyramid either** (implementer-added:
/// the brief's pin only routes at reach 0, which already bypassed the tile system
/// in Task 4 - this is the walk that still ran the tier, and the one Task 5 deletes
/// it for). The walk is long (56 tiles, past the tier's 16-tile gate) and the straight
/// leg is walled, so the old arm built the pyramid before searching.
#[test]
fn a_nonzero_reach_walk_builds_no_pyramid() {
    let ground = open_ground_64();
    let wall = water_rects_for_test_map();
    let route = ground
        .route_avoiding(SCALE, centre(4, 5), centre(60, 5), SHIP_REACH, &wall, true)
        .expect("the solver always answers");
    assert_eq!(ground.pyramid_builds(), 0);
    let grown: Vec<Clear> = wall
        .iter()
        .map(|clear| Clear {
            at: clear.at,
            half: clear.half,
            reach: clear.reach.max(SHIP_REACH),
        })
        .collect();
    let raw: Vec<(i64, i64)> = route
        .iter()
        .map(|point| (point.0.raw(), point.1.raw()))
        .collect();
    assert!(
        verify_path(&raw, &grown).is_none(),
        "the route stays clear of the grown water boxes: {route:?}"
    );
}
