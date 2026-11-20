//! **Which level a search starts at** (P11, step 11b), read from the binary's descent loop.
//!
//! Source: `FUN_007ee980_007ee980.c` (REA corpus, HD 5.7, read 2026-11-19):
//!
//! ```c
//! iVar7 = 3;
//! while (true) {
//!     iVar4 = (int)param_2 >> iVar7;                 // x >> level
//!     iVar6 = (int)param_3 >> iVar7;                 // y >> level
//!     if ((iVar7 == 0) ||
//!        ((*(byte *)(*(int *)(*(int *)(*in_ECX + iVar7 * 4) + iVar4 * 4) + 1 + iVar6 * 2) & 0x20) != 0))
//!         break;
//!     iVar7 = iVar7 - 1;
//! }
//! local_24 = FUN_007ee760(iVar7, iVar4, iVar6, (int)param_2, (int)param_3);
//! ```
//!
//! **The bit is read from the cell's second byte, and the per-search stamper writes the first** (`FUN_007f1100` writes `*pbVar1`,
//! the descent reads `+ 1 + cell*2`). So the second byte is the persistent passability of the cell, not the search stamp: its low
//! nibble is what `FUN_007ed8f0` tests for a usable child (`& 0xf == 0xf`, all four directions clear), and `0x20` is the same
//! "usable cell" flag at the top. Read that way the loop says: **descend while the cell is not usable, and stop at the coarsest level
//! whose cell is** - the fast coarse search in the open, fine cells in clutter, and level 0 (the tiles) as the floor.
//!
//! That reading is the only one that matches the rest of the machine, and it is recorded as an *interpretation* in
//! `docs/pathfinding-levels-evidence.md`, not as a certainty: the byte's producer has not been read.
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
    let map =
        ruage_data::load_map_str(OPEN, "descent.toml", &resources).expect("the fixture map loads");
    Ground::from_map(&map)
}

/// **Open ground starts at the coarsest level**: nothing is near enough to need detail, so a search over 8×8 cells is the answer.
#[test]
fn an_open_field_starts_at_level_three() {
    let ground = open();
    let levels = Levels::of(&ground);
    for tile in [(0u16, 0u16), (8, 8), (20, 5), (31, 31)] {
        assert_eq!(
            levels.descent(tile),
            3,
            "an open map should start the search at the coarsest level, at {tile:?}"
        );
    }
}

/// **A wall in the 8×8 cell drops the search one level**: the coarsest cell that is usable is the 4×4 one, and the search starts there.
#[test]
fn a_wall_in_the_coarse_cell_drops_the_start_one_level() {
    let mut ground = open();
    // The wall is in the start's 8x8 cell (x 8..16, y 0..8) but in a *different* 4x4 cell of it: x 12..16, y 4..8.
    ground.block(14, 6);
    let levels = Levels::of(&ground);
    assert!(
        levels.blocked(3, 1, 0),
        "the 8x8 cell x 8..16, y 0..8 must be blocked for this test to mean anything"
    );
    assert!(
        !levels.blocked(2, 2, 0),
        "the 4x4 cell x 8..12, y 0..4 - the one holding the start - has no wall in it and must be open"
    );
    assert_eq!(
        levels.descent((9, 2)),
        2,
        "the start drops to the 4x4 cell that is open"
    );
    assert_eq!(
        levels.descent((20, 20)),
        3,
        "a start far from the wall stays at level 3"
    );
}

/// **A one-tile corridor is a tile-level start**: every coarse cell that holds it has walls in it, so the descent reaches level 0.
#[test]
fn a_one_tile_corridor_starts_at_level_zero() {
    let mut ground = open();
    // Wall everything except column 5 and the tile the walk starts on.
    for y in 0..32u16 {
        for x in 0..32u16 {
            if x != 5 {
                ground.block(x, y);
            }
        }
    }
    let levels = Levels::of(&ground);
    for level in 1..Levels::DEPTH as u8 {
        let (cx, cy) = Levels::cell_of(level, (5, 5));
        assert!(
            levels.blocked(level, cx, cy),
            "level {level} cell ({cx}, {cy}) holds the corridor's walls and must be blocked"
        );
    }
    assert_eq!(
        levels.descent((5, 5)),
        0,
        "nothing coarser than the tile is usable in a corridor"
    );
}
