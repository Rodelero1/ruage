//! **The level maps the levelled pathfinder searches over** (P11, step 11a; D296).
//!
//! Source, read 2026-11-19 and written up in `docs/pathfinding-levels-evidence.md`:
//!
//! ```c
//! // FUN_007ee980: the search descends from level 3, a cell is 2^level tiles per side
//! iVar7 = 3;
//! iVar4 = (int)param_2 >> iVar7;                 // x >> level
//! iVar6 = (int)param_3 >> iVar7;                 // y >> level
//!
//! // FUN_007ede40: an obstacle rectangle is stamped at the top level only...
//! for (iVar2 = iVar2 >> 3; iVar2 <= param_1 >> 3; iVar2 = iVar2 + 1)
//!     FUN_007f1100(3, iVar2, iVar3, param_5);
//!
//! // ...and FUN_007f1100 recurses DOWN while the cell it is at is unmarked, so a coarse
//! // cell carries the stamp of every obstacle that touches any of its tiles.
//! while ((bVar4 & 0x20) == 0) { FUN_007f1100(param_1 - 1, ...its four children...); }
//! ```
//!
//! So the rule this module keeps is the stamping rule: **a cell is blocked when any tile under it is blocked**. A coarse node that
//! spans a wall is not a node a route may use, whatever the rest of its tiles look like.
//!
//! The maps are rebuilt per search, as the binary rebuilds them (`FUN_007f1100` clears with `& 0xf` at the end of `FUN_007ee980`),
//! because ground changes when a building is stamped or a tree is felled.

use crate::Ground;

/// Four levels, as the binary's `iVar7 = 3 .. 0`: level 3 cells are 8×8 tiles, level 0 cells are the tiles themselves.
pub const DEPTH: usize = 4;

/// **One blocked-cell map per level**, built from a ground in one pass.
#[derive(Debug, Clone)]
pub struct Levels {
    /// `maps[level][cy * cells_wide + cx]`, `true` = a blocked cell.
    maps: Vec<Vec<bool>>,
    /// The tile size of the map the levels were built from.
    width: u16,
    height: u16,
}

impl Levels {
    /// How many levels there are: 3, 2, 1, 0 in the binary's numbering.
    pub const DEPTH: usize = DEPTH;

    /// **Build the pyramid**: level 0 from the tiles, and each coarser level from the one below by OR-reducing 2×2 blocks - the
    /// same information the binary's recursive stamp carries, without the stamping.
    #[must_use]
    pub fn of(ground: &Ground) -> Self {
        let (width, height) = ground.size();

        let mut maps = Vec::with_capacity(DEPTH);
        let mut base = vec![false; usize::from(width) * usize::from(height)];
        for y in 0..height {
            for x in 0..width {
                base[usize::from(y) * usize::from(width) + usize::from(x)] = ground.blocked(x, y);
            }
        }
        maps.push(base);

        for level in 1..DEPTH {
            let below_level = u8::try_from(level - 1).expect("a level fits a u8");
            let this_level = u8::try_from(level).expect("a level fits a u8");
            let (below_w, below_h) = cells_of(width, height, below_level);
            let (wide, tall) = cells_of(width, height, this_level);
            let below = &maps[level - 1];
            let mut map = vec![false; usize::from(wide) * usize::from(tall)];
            for cy in 0..tall {
                for cx in 0..wide {
                    // The four (or fewer, at the map's edge) children of this cell.
                    let mut blocked = false;
                    for dy in 0..2u16 {
                        for dx in 0..2u16 {
                            let child_x = cx * 2 + dx;
                            let child_y = cy * 2 + dy;
                            if child_x >= below_w || child_y >= below_h {
                                continue;
                            }
                            if below
                                [usize::from(child_y) * usize::from(below_w) + usize::from(child_x)]
                            {
                                blocked = true;
                            }
                        }
                    }
                    map[usize::from(cy) * usize::from(wide) + usize::from(cx)] = blocked;
                }
            }
            maps.push(map);
        }

        Levels {
            maps,
            width,
            height,
        }
    }

    /// **How many cells a level has**, per side: `ceil(tiles / 2^level)`, so an odd-sized map keeps its last column and row.
    #[must_use]
    pub fn cells(&self, level: u8) -> (u16, u16) {
        cells_of(self.width, self.height, level)
    }

    /// **Whether a cell is blocked.** A cell off the map is blocked: nothing may be routed there.
    #[must_use]
    pub fn blocked(&self, level: u8, cx: u16, cy: u16) -> bool {
        let level = usize::from(level);
        let Some(map) = self.maps.get(level) else {
            return true;
        };
        let (wide, tall) = cells_of(
            self.width,
            self.height,
            u8::try_from(level).unwrap_or(u8::MAX),
        );
        if cx >= wide || cy >= tall {
            return true;
        }
        map.get(usize::from(cy) * usize::from(wide) + usize::from(cx))
            .copied()
            .unwrap_or(true)
    }

    /// **The coarsest level whose cell holding `tile` is usable** - the binary's descent (`FUN_007ee980`), whose loop walks from level
    /// 3 down until the cell it is looking at is usable, and which floors at level 0.
    ///
    /// The binary tests bit `0x20` of the cell's **second** byte there, and its per-search stamper writes the **first**; that second
    /// byte is the persistent passability of the cell - the same byte `FUN_007ed8f0` reads for a usable child (`& 0xf == 0xf`, all
    /// four directions clear). So the test is "is this cell usable", and the descent stops at the first level where it is. Recorded as
    /// an interpretation in `docs/pathfinding-levels-evidence.md`, because the byte's producer has not been read.
    #[must_use]
    pub fn descent(&self, tile: (u16, u16)) -> u8 {
        let mut level = u8::try_from(DEPTH - 1).unwrap_or(3);
        while level > 0 {
            let (cx, cy) = Self::cell_of(level, tile);
            if !self.blocked(level, cx, cy) {
                break;
            }
            level -= 1;
        }
        level
    }

    /// **The cell a tile falls in** at a level: `tile >> level`, which is the binary's own indexing.
    #[must_use]
    pub fn cell_of(level: u8, tile: (u16, u16)) -> (u16, u16) {
        (tile.0 >> level, tile.1 >> level)
    }
}

/// `ceil(tiles / 2^level)` per side, computed in 32-bit so `1 << level` cannot overflow a `u16`.
fn cells_of(width: u16, height: u16, level: u8) -> (u16, u16) {
    let step = 1u32 << level;
    let wide = u32::from(width).div_ceil(step);
    let tall = u32::from(height).div_ceil(step);
    (
        u16::try_from(wide).unwrap_or(u16::MAX),
        u16::try_from(tall).unwrap_or(u16::MAX),
    )
}
