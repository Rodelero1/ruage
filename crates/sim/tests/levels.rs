//! **The four level maps the levelled pathfinder searches over** (P11, step 11a).
//!
//! Source: `FUN_007ee980_007ee980.c`, `FUN_007ede40_007ede40.c` and `FUN_007f1100_007f1100.c` (the REA corpus, HD 5.7, read
//! 2026-11-19), written up in `docs/archive/research/pathfinding-levels-evidence.md`:
//!
//! ```c
//! // FUN_007ede40: stamp an obstacle rectangle into the top level only
//! for (iVar2 = iVar2 >> 3; iVar2 <= param_1 >> 3; iVar2 = iVar2 + 1)
//!     for (; iVar3 <= param_2 >> 3; iVar3 = iVar3 + 1)
//!         FUN_007f1100(3, iVar2, iVar3, param_5);          // level 3 = 8x8-tile cells
//!
//! // FUN_007f1100: recurse DOWN the pyramid while the cell being stamped is unmarked
//! while ((bVar4 & 0x20) == 0) { ... FUN_007f1100(param_1 - 1, ...four children...) ... }
//! ```
//!
//! A cell is stamped when an obstacle touches it, so a coarse cell is **blocked when any of its tiles is blocked** - a coarse
//! node that spans a wall is not usable, whatever the rest of its tiles look like.
//!
//! Written before the code (D7, SPEC §11).

use ruage_sim::{Ground, Levels};

const OPEN: &str = r#"
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

fn open() -> LevelFixture {
    let resources = ruage_data::load_resources_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../config/resources.toml"
        ))
        .expect("config/resources.toml"),
        "resources.toml",
    )
    .expect("resources.toml");
    let map =
        ruage_data::load_map_str(OPEN, "levels.toml", &resources).expect("the fixture map loads");
    LevelFixture {
        ground: Ground::from_map(&map),
    }
}

struct LevelFixture {
    ground: Ground,
}

impl LevelFixture {
    /// The level map the search would build for this ground, rebuilt per search exactly as the binary does.
    fn levels(&self) -> Levels {
        Levels::of(&self.ground)
    }
}

/// **Every tile of an open map is open at every level**: nothing is stamped where nothing stands.
#[test]
fn an_open_map_is_open_at_every_level() {
    let fixture = open();
    let levels = fixture.levels();
    for level in 0..Levels::DEPTH as u8 {
        for cy in 0..levels.cells(level).1 {
            for cx in 0..levels.cells(level).0 {
                assert!(
                    !levels.blocked(level, cx, cy),
                    "level {level} cell ({cx}, {cy}) is stamped on an empty map"
                );
            }
        }
    }
}

/// **A coarse cell is blocked when any of its tiles is** (the stamping rule): one wall tile closes the 8×8 cell that holds it and
/// leaves every other 8×8 cell of the map open.
#[test]
fn one_wall_tile_closes_the_eight_by_eight_cell_that_holds_it() {
    let mut fixture = open();
    fixture.ground.block(9, 3);
    let levels = fixture.levels();

    assert!(
        levels.blocked(3, 1, 0),
        "the 8x8 cell covering x 8..16, y 0..8 holds the wall at (9, 3) and must be blocked"
    );
    for cy in 0..levels.cells(3).1 {
        for cx in 0..levels.cells(3).0 {
            if (cx, cy) == (1, 0) {
                continue;
            }
            assert!(
                !levels.blocked(3, cx, cy),
                "level 3 cell ({cx}, {cy}) is far from the wall and must stay open"
            );
        }
    }
    // The finest level is the tiles themselves.
    assert!(
        levels.blocked(0, 9, 3),
        "level 0 is the tile that holds the wall"
    );
    assert!(!levels.blocked(0, 8, 3), "the tile beside the wall is open");
}

/// **A 4×4 hole is open at level 2 and closed at level 3**: the level must be fine enough to hold a route through the hole, which
/// is what "a coarse node spans `<< level` tiles" buys.
#[test]
fn a_four_by_four_hole_is_open_at_level_two_and_closed_at_level_three() {
    let mut fixture = open();
    // Wall the whole map except the 4x4 block x 4..8, y 4..8.
    for y in 0..16u16 {
        for x in 0..16u16 {
            if !(4..8).contains(&x) || !(4..8).contains(&y) {
                fixture.ground.block(x, y);
            }
        }
    }
    let levels = fixture.levels();

    assert!(
        levels.blocked(3, 0, 0),
        "the level 3 cell x 0..8, y 0..8 contains walls, so it is blocked"
    );
    assert!(
        !levels.blocked(2, 1, 1),
        "the level 2 cell covering exactly x 4..8, y 4..8 is the hole and must be open"
    );
    assert!(
        levels.blocked(2, 0, 1),
        "the level 2 cell beside the hole is walled and must be blocked"
    );
}

/// **The maps are the size the levels say**: level L has `ceil(width / 2^L)` cells per side, so an odd-sized map does not lose its
/// last column or row, and no lookup is out of bounds.
#[test]
fn each_level_is_a_quarter_of_the_one_below_it() {
    let fixture = open();
    let levels = fixture.levels();
    let expected = [(16u16, 16u16), (8, 8), (4, 4), (2, 2)];
    for (level, want) in expected.iter().enumerate() {
        assert_eq!(
            levels.cells(u8::try_from(level).expect("a level")),
            *want,
            "level {level} cell count"
        );
    }
}
