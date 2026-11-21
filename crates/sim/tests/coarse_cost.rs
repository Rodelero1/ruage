//! **What a step into a coarse cell costs** (P11, step 11c), read from `FUN_007ed280`.
//!
//! The opener's table, as the code branches (`docs/pathfinding-levels-evidence.md`):
//!
//! | situation | cost |
//! | --- | --- |
//! | an open cell whose four orthogonal neighbours hold no mine | **10** (plus a distance-from-the-edge term the caller adds) |
//! | an open cell beside a **gold or stone mine** (`FUN_007f11f0`: class 8 or 32) | **300** |
//! | (no row: the binary's 2000 keys on a direction-run grid, not on bodies - D328) |
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

/// **Open ground costs 10**, at every level - the row of the table that needs nothing but the level maps.
///
/// **The 2000 row is not here, and it is not about bodies** (D328). `FUN_007ed280` overrides any cost to 2000 when the per-tile grid at
/// `[0x4d]` is non-zero, and `FUN_007ef3f0` builds that grid as **eight-direction run masks** (`0x01 << n` per side, walked into maximal
/// straight runs) - not a list of standing bodies. The only body-aware test in the path system is `FUN_00801a20`, and it is in the **flat**
/// search. So our `body` parameter priced a rule the original does not have, and it was never passed `true` by any caller.
#[test]
fn clear_ground_is_ten() {
    let ground = open();
    let levels = Levels::of(&ground);
    for level in 0..Levels::DEPTH as u8 {
        let (cx, cy) = Levels::cell_of(level, (16, 16));
        assert_eq!(
            levels.step_cost(level, cx, cy),
            Some(10),
            "an open cell at level {level} costs ten"
        );
    }
}

/// **A wall beside a cell is the plain 10; a mine beside it is 300** (D342). The row used to be keyed on "any blocked
/// orthogonal neighbour", which made every wall the search passed expensive; `FUN_007f11f0` answers 1 only for an object
/// of class 8 (stone mine) or 32 (gold mine), and `FUN_007ed280` charges the plain 10 otherwise.
///
/// This fixture used to assert the 300 **beside a wall**, which is the reading the decomp denies - so the guard it
/// carried ("hugging an obstacle is expensive") is withdrawn here, and the row is pinned on the pack's own mark instead:
/// the same two ground states, one boolean apart.
#[test]
fn a_cell_beside_a_wall_is_ten_and_beside_a_mine_is_three_hundred() {
    let mut ground = open();
    ground.block(16, 15);
    let levels = Levels::of(&ground);
    // Level 0: the tile beside the wall is clear ground, and a wall is not a mine.
    assert_eq!(
        levels.step_cost(0, 16, 16),
        Some(10),
        "the tile below the wall is open, and only a mine makes a neighbour expensive"
    );
    assert_eq!(
        levels.step_cost(0, 17, 16),
        Some(10),
        "a tile two steps from the wall is clear on all four sides"
    );
    // The same tile, one mark apart: this is what a mine does.
    let mut with_mine = open();
    with_mine.block(16, 15);
    with_mine.mark_snug(16, 15);
    let levels = Levels::of(&with_mine);
    assert_eq!(
        levels.step_cost(0, 16, 16),
        Some(300),
        "a gold or stone mine beside the cell is the 300 row"
    );
    assert_eq!(
        levels.step_cost(0, 17, 16),
        Some(10),
        "and the row does not reach two steps out"
    );
    // Level 3: the 8x8 cell that holds the wall is blocked outright, so it is not enterable at all. It is the cell of the *wall*
    // (16, 15), not of the tile below it - my first version asked the wrong cell and got a correct 300.
    let (wx, wy) = Levels::cell_of(3, (16, 15));
    assert!(
        levels.blocked(3, wx, wy),
        "the level 3 cell holding the wall must be blocked for this test to mean anything"
    );
    assert_eq!(
        levels.step_cost(3, wx, wy),
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
    assert_eq!(levels.step_cost(0, 16, 16), None, "the wall tile itself");
    assert_eq!(
        levels.step_cost(3, 2, 2),
        None,
        "the 8x8 cell holding the wall"
    );
}
