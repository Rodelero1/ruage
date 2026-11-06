//! **The cheap line check and the grid it lives on** (second review, `path.rs:127` and `path.rs:33`).
//!
//! Two claims, one file: `line_clear` must agree with `find_path` about the tile a body **already stands on**, and a
//! `Map` whose terrain array does not match its declared size must fail loudly instead of misindexing silently.
//!
//! Written before the changes (D7, SPEC §11).

use ruage_sim::{Walk, line_clear};

const MAP: &str = r#"
schema = 1
[map]
width = 10
height = 3
terrain = [
  "GGGGGGGGGG",
  "GGGGGGGGGG",
  "GGGGGGGGGG",
]
[[map.town_center]]
player = 1
x = 1
y = 1
[[map.town_center]]
player = 2
x = 8
y = 1
"#;

fn map() -> ruage_data::Map {
    let resources = ruage_data::load_resources_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../config/resources.toml"
        ))
        .expect("config/resources.toml"),
        "resources.toml",
    )
    .expect("resources.toml");
    ruage_data::load_map_str(MAP, "walk.toml", &resources).expect("the fixture map loads")
}

/// **A body standing on an occupied tile can still see a clear line** (second review, `path.rs:127`).
///
/// `find_path` allows a blocked **start** on purpose - *"a villager stands on the tile of the tree it is working, and must
/// be able to walk off it"* - and the cheap straight-line check refused it, so every unit standing beside or on a node paid
/// a full A* search for a line it could see. The two have to agree about the tile you are already on.
#[test]
fn a_line_from_an_occupied_start_is_clear_when_the_rest_of_it_is() {
    let mut walk = Walk::from_map(&map());
    let (from, to) = ((2u16, 2u16), (6u16, 2u16));
    walk.block(from.0, from.1);
    assert!(
        walk.blocked(from.0, from.1),
        "the start has to be occupied for this test to mean anything"
    );
    assert!(
        line_clear(&walk, from, to),
        "the start tile is where the body already is: it cannot be an obstruction to its own line"
    );
    // The rest of the line still counts: an obstruction one tile along refuses it.
    walk.block(4, 2);
    assert!(
        !line_clear(&walk, from, to),
        "a body in the middle of the line is still a body in the way"
    );
}

/// **A map whose terrain array does not match its size is a bug, not a grid** (second review, `path.rs:33`).
///
/// The loader refuses one, but `Map` is public and tests and fixtures build it directly; `Walk::from_map` indexed it by
/// `width`/`height` and would read one tile's ground for another. It asserts instead, in debug builds, where a fixture bug
/// belongs.
#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "terrain")]
fn a_map_whose_terrain_does_not_match_its_size_is_refused() {
    let mut map = map();
    map.terrain.pop();
    let _ = Walk::from_map(&map);
}
