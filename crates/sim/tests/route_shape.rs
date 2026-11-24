//! **The shape of a route says which tier answered** (D302's coverage gap, item 2 of `docs/next-step.md`).
//!
//! D302's review flagged that the red-set diff could not see the change it made: blocked routes went from per-tile polylines to coarse chains,
//! and **no fixture asserted a shape at all**. Two claims close it:
//!
//! - in **cluttered ground** the descent lands below level 2, the hierarchy abstains (D303's gate), and the answer is a **tile-level**
//!   polyline - every waypoint but the last on a tile centre (`n + 0.5`);
//! - in **open ground** the hierarchy answers and the waypoints are **level-3 cell centres** (`n * 8 + 4`) - asserted in
//!   `coarse_route::a_blocked_walk_is_answered_by_the_hierarchy`, not repeated here.
//!
//! Written before running it (D7, SPEC §11).

use ruage_fixed::Fixed;
use ruage_sim::Ground;

const MAP: &str = r#"
schema = 1
[map]
width = 24
height = 24
terrain = [
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
]
[[map.town_center]]
player = 1
x = 1
y = 1
[[map.town_center]]
player = 2
x = 22
y = 22
"#;

const SCALE: i64 = 20_000;

/// **A maze of two-tile corridors**: walls in every 8-tile block, so the descent cannot reach level 2 and the flat search must answer.
fn cluttered() -> Ground {
    let resources = ruage_data::load_resources_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../config/resources.toml"
        ))
        .expect("config/resources.toml"),
        "resources.toml",
    )
    .expect("resources.toml");
    let map = ruage_data::load_map_str(MAP, "route_shape.toml", &resources)
        .expect("the fixture map loads");
    let mut ground = Ground::from_map(&map);
    // Comb teeth every four tiles, three tiles long: every 8-tile cell contains one, so the descent floors out and the hierarchy abstains.
    for band in 0..6u16 {
        for x in (2..22u16).step_by(4) {
            for dy in 0..3u16 {
                ground.block(x, band * 4 + dy);
            }
        }
    }
    ground
}

fn at(x: i64, y: i64) -> (Fixed, Fixed) {
    (
        Fixed::from_raw(x * SCALE + SCALE / 2),
        Fixed::from_raw(y * SCALE + SCALE / 2),
    )
}

/// **Cluttered ground is answered at tile level**, not as a coarse chain: every waypoint but the last sits on a tile centre.
#[test]
fn cluttered_ground_is_answered_at_tile_level() {
    let ground = cluttered();
    let (from, to) = (at(1, 3), at(21, 22));
    let route = ground
        .route(SCALE, from, to)
        .expect("a walk through the maze");
    assert!(
        route.len() > 1,
        "a maze walk must be a polyline, not {} waypoint(s)",
        route.len()
    );
    for point in &route[..route.len() - 1] {
        let on_tile_centre = |raw: i64| raw.rem_euclid(SCALE) == SCALE / 2;
        assert!(
            on_tile_centre(point.0.raw()) && on_tile_centre(point.1.raw()),
            "waypoint ({}, {}) is not a tile centre - the hierarchy answered in cluttered ground, where it should abstain",
            point.0.raw(),
            point.1.raw()
        );
    }
}
