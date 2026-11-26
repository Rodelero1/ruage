//! **Land routes answer from the geometry solver** (New Way Task 4): `route_with`
//! calls `geometry::solve` for the land path before the tier arm, mapping
//! `Reached` to the goal-exact tail and `Closest` to the stopped-beside tail.
//! Tier + flat code stays but is bypassed for the reach values Task 3 exercised.

use ruage_fixed::Fixed;
use ruage_sim::Ground;

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
        .route_avoiding(SCALE, from_pt(), to_pt(), REACH, &[])
        .expect("geometry answers");
    assert!(route.len() <= 2);
    assert_eq!(*route.last().unwrap(), to_pt());
    // The solver's polyline starts where the body stands; the tile tail never does.
    assert_eq!(route, vec![from_pt(), to_pt()]);
}
