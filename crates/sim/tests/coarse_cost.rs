//! **What a step into a coarse cell costs** (P11, step 11c), read from `FUN_007ed280`.
//!
//! The opener's table, as the code branches (`docs/pathfinding-levels-evidence.md`):
//!
//! | situation | cost |
//! | --- | --- |
//! | an open cell whose four orthogonal neighbours are all clear | **10** (plus a distance-from-the-edge term the caller adds) |
//! | an open cell with a blocked orthogonal neighbour | **300** |
//! | a body standing on the cell | **2000** |
//! | blocked and not crossable | **600** - and refused outright when the cell is outside the search window |
//!
//! Two rows of the original's table - **100** ("blocked but crossable") and **600** ("not crossable but the search still opens it") - need
//! terrain the game has and this content model does not: our tiles are walkable or not, with no cost in between. They are named in
//! `Levels::step_cost` rather than invented.
//!
//! Written before the code (D7, SPEC §11).

use ruage_sim::{Ground, Levels};

const OPEN: &str = r#"
schema = 1
[map]
width = 32
height = 32
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
y = 30
"#;

fn open() -> Ground {
    let resources = ruage_data::load_resources_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../config/resources.toml"
        ))
        .expect("config/resources.toml"),
        "resources.toml",
    )
    .expect("resources.toml");
    let map = ruage_data::load_map_str(OPEN, "coarse_cost.toml", &resources)
        .expect("the fixture map loads");
    Ground::from_map(&map)
}

/// **Open ground costs 10**, at every level, and a body on the cell costs 2000 - the two rows of the table that need nothing but the
/// level maps.
#[test]
fn clear_ground_is_ten_and_a_body_is_two_thousand() {
    let ground = open();
    let levels = Levels::of(&ground);
    for level in 0..Levels::DEPTH as u8 {
        let (cx, cy) = Levels::cell_of(level, (16, 16));
        assert_eq!(
            levels.step_cost(level, cx, cy, false),
            Some(10),
            "an open cell at level {level} costs ten"
        );
        assert_eq!(
            levels.step_cost(level, cx, cy, true),
            Some(2000),
            "a cell with a body on it costs two thousand, at level {level}"
        );
    }
}

/// **An open cell with a blocked orthogonal neighbour costs 300**: the cell you are entering is clear, but hugging an obstacle is
/// expensive, which is the whole point of the row.
#[test]
fn a_cell_beside_an_obstacle_costs_three_hundred() {
    let mut ground = open();
    ground.block(16, 15);
    let levels = Levels::of(&ground);
    // Level 0: the tile beside the wall.
    assert_eq!(
        levels.step_cost(0, 16, 16, false),
        Some(300),
        "the tile below the wall is open but touches it"
    );
    assert_eq!(
        levels.step_cost(0, 17, 16, false),
        Some(10),
        "a tile two steps from the wall is clear on all four sides"
    );
    // Level 3: the 8x8 cell that holds the wall is blocked outright, so it is not enterable at all. It is the cell of the *wall*
    // (16, 15), not of the tile below it - my first version asked the wrong cell and got a correct 300.
    let (wx, wy) = Levels::cell_of(3, (16, 15));
    assert!(
        levels.blocked(3, wx, wy),
        "the level 3 cell holding the wall must be blocked for this test to mean anything"
    );
    assert_eq!(
        levels.step_cost(3, wx, wy, false),
        None,
        "a cell that contains the wall is blocked and cannot be entered"
    );
}

/// **A blocked cell is not enterable**, at any level: the answer is `None`, not a cost.
#[test]
fn a_blocked_cell_is_not_enterable() {
    let mut ground = open();
    ground.block(16, 16);
    let levels = Levels::of(&ground);
    assert_eq!(
        levels.step_cost(0, 16, 16, false),
        None,
        "the wall tile itself"
    );
    assert_eq!(
        levels.step_cost(3, 2, 2, false),
        None,
        "the 8x8 cell holding the wall"
    );
}
