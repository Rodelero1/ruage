//! **The pyramid is built once per change to the ground, not once per search** (D300 step 1).
//!
//! D300 recorded that a cache "would never hit", on the reading that `Ground::block`/`unblock` fire on every unit step. **That reading was
//! wrong**: `occupants` is touched only by **objects** - a tree seeded or felled, a building laid or razed, a gate opened - and never by a
//! unit, whose body the step pass handles through `objects_touched` instead. There are eight call sites and every one of them is an
//! object event, so a dirty flag on `Ground` fires rarely and a pyramid keyed to it is almost always warm.
//!
//! Written before the code (D7, SPEC §11).

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
    let map = ruage_data::load_map_str(MAP, "pyramid_cache.toml", &resources)
        .expect("the fixture map loads");
    Ground::from_map(&map)
}

/// **Two searches, one pyramid**: the second `coarse_route` on unchanged ground reuses the first build.
#[test]
fn a_second_search_reuses_the_pyramid() {
    let field = ground();
    assert_eq!(
        field.pyramid_builds(),
        0,
        "nothing is built until it is asked for"
    );
    let first = field.coarse_route((4, 4), (20, 20)).expect("an open walk");
    assert_eq!(
        field.pyramid_builds(),
        1,
        "the first search builds the pyramid"
    );
    let second = field.coarse_route((4, 4), (20, 20)).expect("the same walk");
    assert_eq!(field.pyramid_builds(), 1, "the second search reuses it");
    assert_eq!(first, second, "and answers the same route");
}

/// **A tree felled invalidates it, and the next search rebuilds**: `block`/`unblock` are the only mutators, so they are the only places a
/// stale pyramid can come from.
#[test]
fn a_change_to_the_ground_rebuilds_the_pyramid() {
    let mut field = ground();
    let _ = field.coarse_route((4, 4), (20, 20)).expect("an open walk");
    assert_eq!(field.pyramid_builds(), 1, "one build so far");

    field.block(12, 4);
    let after = field
        .coarse_route((4, 4), (20, 20))
        .expect("a walk round the new obstacle");
    assert_eq!(field.pyramid_builds(), 2, "the change forced one rebuild");
    assert!(
        after.cells.iter().all(|cell| *cell != (1u16, 0u16)),
        "the route should no longer run through the cell that now holds the obstacle: {:?}",
        after.cells
    );

    field.unblock(12, 4);
    let _ = field.coarse_route((4, 4), (20, 20)).expect("an open walk");
    assert_eq!(field.pyramid_builds(), 3, "clearing it forced another");
}
