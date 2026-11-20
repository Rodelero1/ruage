//! **What the level pyramid costs to build and search** (the number 11d needs before the switch).
//!
//! 11d would put the levelled search in front of `Ground::route`, and every `route` call would then build a `Levels` - four `Vec<bool>`
//! maps covering the whole ground - unless the pyramid is cached. This probe measures that cost on the project map and on the largest
//! map the game allows, so the decision to wire it is made on a number instead of on a guess.
//!
//! Run with `cargo test -p ruage-sim --test levels_cost -- --nocapture`.

use std::time::Instant;

use ruage_fixed::Fixed;
use ruage_sim::{Ground, Levels};

const PROJECT: &str = r#"
schema = 1
[map]
width = 64
height = 65
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
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
]
[[map.town_center]]
player = 1
x = 1
y = 1
[[map.town_center]]
player = 2
x = 60
y = 60
"#;

fn ground_of(text: &str) -> Ground {
    let resources = ruage_data::load_resources_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../config/resources.toml"
        ))
        .expect("config/resources.toml"),
        "resources.toml",
    )
    .expect("resources.toml");
    let map = ruage_data::load_map_str(text, "levels_cost.toml", &resources)
        .expect("the fixture map loads");
    Ground::from_map(&map)
}

/// The pyramid build, the descent, the coarse search and the refinement, timed separately on the project map. The numbers printed here
/// are what 11d's switch has to pay per call if the pyramid is not cached.
#[test]
fn the_pyramid_costs_this_much() {
    let ground = ground_of(PROJECT);
    const RUNS: u32 = 200;

    let start = Instant::now();
    let mut built = 0u64;
    for _ in 0..RUNS {
        let levels = Levels::of(&ground);
        built += u64::from(levels.cells(3).0);
    }
    let build = start.elapsed() / RUNS;
    assert!(built > 0, "the build must do work");

    let levels = Levels::of(&ground);
    let start = Instant::now();
    for _ in 0..RUNS {
        let _ = levels.descent((4, 4));
    }
    let descent = start.elapsed() / RUNS;

    let mut cells = 0usize;
    let start = Instant::now();
    for _ in 0..RUNS {
        if let Some(route) = levels.coarse_route((4, 4), (60, 60)) {
            cells += route.cells.len();
        }
    }
    let search = start.elapsed() / RUNS;

    // **The comparison that matters**: a walk the straight line cannot take, so the flat path actually searches instead of
    // short-circuiting on `line_clear`. A wall across the middle of the map forces both to work.
    let mut blocked = ground_of(PROJECT);
    for y in 0..40u16 {
        blocked.block(32, y);
    }
    let _ = blocked.coarse_route((4, 4), (60, 60));
    let start = Instant::now();
    for _ in 0..RUNS {
        let _ = blocked.coarse_route((4, 4), (60, 60));
    }
    let coarse_walled = start.elapsed() / RUNS;
    let start = Instant::now();
    let mut searched = 0usize;
    for _ in 0..RUNS {
        let at = |x: i64, y: i64| {
            (
                Fixed::from_raw(x * 20_000 + 10_000),
                Fixed::from_raw(y * 20_000 + 10_000),
            )
        };
        if let Some(route) = blocked.route(20_000, at(4, 4), at(60, 60)) {
            searched += route.len();
        }
    }
    let flat_walled = start.elapsed() / RUNS;
    eprintln!(
        "ZZPROBE walled walk: coarse {coarse_walled:?}  flat {flat_walled:?}  ({searched} waypoints over {RUNS} runs)"
    );

    // The flat search on the same walk, for the comparison 11d turns on.
    let scale = 20_000i64;
    let centre = |x: i64, y: i64| {
        (
            Fixed::from_raw(x * scale + scale / 2),
            Fixed::from_raw(y * scale + scale / 2),
        )
    };
    let start = Instant::now();
    let mut flat = 0usize;
    for _ in 0..RUNS {
        if let Some(route) = ground.route(scale, centre(4, 4), centre(60, 60)) {
            flat += route.len();
        }
    }
    let flat_search = start.elapsed() / RUNS;

    // **The cached path** (D301): warm the pyramid, then time the search alone.
    let _ = ground.coarse_route((4, 4), (60, 60));
    let warm = ground.pyramid_builds();
    let start = Instant::now();
    for _ in 0..RUNS {
        let _ = ground.coarse_route((4, 4), (60, 60));
    }
    let cached = start.elapsed() / RUNS;

    eprintln!(
        "ZZPROBE cached-route {cached:?} (builds {warm})\nZZPROBE project 64x65: pyramid build {build:?}  descent {descent:?}  coarse search {search:?}  ({c} cells per route)  flat tile search {flat_search:?} ({f} waypoints)",
        c = cells / RUNS as usize,
        f = flat / RUNS as usize
    );
}
