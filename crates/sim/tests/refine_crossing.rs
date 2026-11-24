//! **D343 (sequence step 8): the refined waypoint is the crossing NEAREST the interpolated point, not the first from the low
//! end of the span.**
//!
//! `FUN_007eeda0` is handed the mover's position and a far point; for a boundary crossed along x it computes
//! `iVar7 = (iVar4 * param_5) / param_4 + param_7` - where the straight line itself crosses the edge - and only then scans the
//! span: **forward** from that point to the span's end (first crossing kept, with its distance), then **backward** from the
//! same point, stopping when it has walked back as far as the forward crossing was ahead. Ours walks the span from
//! `first + 0` and takes whatever it meets first.
//!
//! Written before the implementation (D7, SPEC §11).

use ruage_fixed::Fixed;
use ruage_sim::{CoarseRoute, Ground, Levels};

const PLATE: &str = r#"
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
x = 0
y = 0
[[map.town_center]]
player = 2
x = 15
y = 15
"#;

fn ground() -> Ground {
    let resources = ruage_data::load_resources_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../config/resources.toml"
        ))
        .expect("config/resources.toml"),
        "resources.toml",
    )
    .expect("resources.toml");
    let map = ruage_data::load_map_str(PLATE, "refine_crossing.toml", &resources)
        .expect("the fixture map loads");
    Ground::from_map(&map)
}

/// **Two crossings on one edge: the waypoint goes where the line crosses, not where the scan starts.**
///
/// The edge between the cells `(0,0)` and `(1,0)` at level 2 is the column `x = 4`, rows 0-3. Rows 1 and 3 are walled, so the
/// only crossings are rows 0 and 2. The walk is a straight line along row 2, so the interpolated crossing is row 2 - and the
/// low-end scan takes row 0.
#[test]
fn the_refined_point_is_the_crossing_the_line_makes() {
    let scale = 20_000i64;
    let mut ground = ground();
    ground.block(4, 1);
    ground.block(4, 3);
    let levels = Levels::of(&ground);
    let route = CoarseRoute {
        level: 2,
        cells: vec![(0, 0), (1, 0)],
    };
    // A line along row 2: from the middle of tile (2,2) to the middle of tile (7,2), in raw units. The line is what the
    // binary interpolates on, so the fixture states it even before the signature takes it.
    let from = (2 * scale + scale / 2, 2 * scale + scale / 2);
    let to = (7 * scale + scale / 2, 2 * scale + scale / 2);
    let points = levels.refine(&route, scale, from, to);
    // The first point is the start cell's centre, which the caller discards; the crossing is the second.
    let crossing = points
        .get(1)
        .copied()
        .expect("a crossing on the shared edge");
    assert_eq!(
        crossing,
        (
            Fixed::from_raw(4 * scale),
            Fixed::from_raw(2 * scale + scale / 2)
        ),
        "the waypoint must sit on the boundary at the row the line crosses (row 2), not at row 0: got {:?}",
        (crossing.0.raw(), crossing.1.raw())
    );
}
