//! **The planner knows the body's width** (#7 of the crosscheck, D303).
//!
//! Source: the binary registers the mover's radius on the path object for the duration of a search (`FUN_008000d0` passes
//! `*(float *)(*(int *)(param_1 + 0xc) + 0x34) + local_58` into `FUN_007ee980`, and `FUN_007f0be0(...,1)` / `(...,0)` bracket the
//! search), so the passability tests inside it are the ones a body of that radius can make. The AoC decompilation says the same thing in
//! words - *"shrink the unit to a point, expand every other obstruction"* (`zone_spatial_pyramid_check_cell_step.cpp`).
//!
//! This engine planned for a **point** and let the step refuse the lane afterwards (D279's parked divergence). The witness below is the
//! visible consequence: a lane one tile wide is planned for a trebuchet that cannot fit down it.
//!
//! Written before the code (D7, SPEC §11).

use ruage_fixed::Fixed;
use ruage_sim::Ground;

const LANE: &str = r#"
schema = 1
[map]
width = 32
height = 20
terrain = [
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
]
[[map.town_center]]
player = 1
x = 1
y = 1
[[map.town_center]]
player = 2
x = 30
y = 18
"#;

const SCALE: i64 = 20_000;

fn lane() -> Ground {
    let resources = ruage_data::load_resources_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../config/resources.toml"
        ))
        .expect("config/resources.toml"),
        "resources.toml",
    )
    .expect("resources.toml");
    let map = ruage_data::load_map_str(LANE, "radius_route.toml", &resources)
        .expect("the fixture map loads");
    let mut ground = Ground::from_map(&map);
    // A wall down the middle of the map with a **one-tile gap** at y = 10: a villager (radius 0.20) fits, a packed trebuchet
    // (radius 0.50) does not.
    for y in 0..20u16 {
        if y != 10 {
            ground.block(16, y);
        }
    }
    // **And a stub that blocks the straight line**, so a search actually runs: without it both bodies answer from the line tier, which
    // is already radius-aware, and the fixture would never touch the searches at all.
    for x in 11..=14u16 {
        ground.block(x, 10);
    }
    ground
}

fn at(x: i64, y: i64) -> (Fixed, Fixed) {
    (
        Fixed::from_raw(x * SCALE + SCALE / 2),
        Fixed::from_raw(y * SCALE + SCALE / 2),
    )
}

/// **A villager walks the gap and a trebuchet does not plan it.** The path either goes round (which the wall prevents here, so the last
/// point is short of the goal) or is refused - what it must never do is run through the gap's tile.
/// **Both bodies pass the one-tile gap** - which is what the game does. The owner measured it in HD (2026-11-20): a **packed trebuchet**
/// walks a one-tile hole in a wall, and a one-tile corridor in bamboo, and the scenario editor even places a packed trebuchet in one. The
/// searches must agree with the game, so the fixture asserts the passage for both a 0.20 and a 0.50 radius body.
#[test]
fn both_bodies_take_the_one_tile_gap() {
    let ground = lane();
    let (from, to) = (at(6, 10), at(26, 10));

    let thin = ground
        .route_avoiding(SCALE, from, to, SCALE / 5, &[], true)
        .expect("a route for a body 0.20 wide");
    assert_eq!(
        thin.last(),
        Some(&to),
        "a radius-0.20 body must reach the goal through the one-tile gap; its route is {thin:?}"
    );

    let wide = ground
        .route_avoiding(SCALE, from, to, SCALE / 2, &[], true)
        .expect("a route for a body 0.50 wide");
    assert_eq!(
        wide.last(),
        Some(&to),
        "a radius-0.50 body must ALSO reach the goal through a one-tile gap - measured in HD, and the reason tangency is a passage: {wide:?}"
    );
}
