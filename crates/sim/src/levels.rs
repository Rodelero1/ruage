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

/// **A coarse route**: the level it was searched at, and the cells from the start's to the goal's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoarseRoute {
    /// The level the search ran on (`Levels::descent` of the start, and no finer than the goal's).
    pub level: u8,
    /// The cells in order, each one orthogonal step from the next.
    pub cells: Vec<(u16, u16)>,
}

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

    /// **The coarse route**: a 4-directional A\* over the level's cells, from the cell holding `from` to the cell holding `to`, at the
    /// level `descent` picks for the start.
    ///
    /// Everything here is `FUN_007ed280` + `FUN_007ee980`, measured in `docs/pathfinding-levels-evidence.md`:
    ///
    /// - **four directions**, never diagonal (`param_4` = 1, 2, 4, 8);
    /// - `g = step + parent's g`, with `step` from [`Levels::step_cost`];
    /// - `h` = Manhattan between the cells. The binary uses `window_hi - coord` on one axis, which needs its search window; without the
    ///   window (see below) Manhattan is the same measure on a 4-directional grid;
    /// - the frontier is **insertion-sorted on `f`**, and a new node is placed **before** equal `f` - the newest first, which is what
    ///   `FUN_007ed500` does with its `f <= existing` test;
    /// - a closed cell is never reopened.
    ///
    /// **The one deviation: no search window.** The binary clips every expansion to `in_ECX[0x42..0x45]`, a box whose padding is set
    /// where it has not been read. Without the clip this search can find routes the clipped one would refuse - a superset, never an
    /// illegal route - and the padding is recorded as unread rather than guessed.
    #[must_use]
    pub fn coarse_route(&self, from: (u16, u16), to: (u16, u16)) -> Option<CoarseRoute> {
        let level = self.descent(from).min(self.descent(to));
        let (start, goal) = (Self::cell_of(level, from), Self::cell_of(level, to));
        if self.blocked(level, goal.0, goal.1) {
            return None;
        }
        let (wide, tall) = self.cells(level);

        // The frontier is a sorted list, as the binary keeps it: `(f, g, cell, parent index)`, ordered by f and newest-first among
        // equals. A `Vec` with a binary-search insert is the same order with a better constant than a linked list.
        struct Node {
            f: i64,
            g: i64,
            cell: (u16, u16),
            parent: Option<usize>,
        }
        let mut nodes: Vec<Node> = Vec::new();
        let mut open: Vec<usize> = Vec::new();
        let mut closed = vec![false; usize::from(wide) * usize::from(tall)];
        let index =
            |cell: (u16, u16)| usize::from(cell.1) * usize::from(wide) + usize::from(cell.0);
        let manhattan = |a: (u16, u16), b: (u16, u16)| {
            (i64::from(a.0) - i64::from(b.0)).abs() + (i64::from(a.1) - i64::from(b.1)).abs()
        };

        let first = nodes.len();
        nodes.push(Node {
            f: manhattan(start, goal),
            g: 0,
            cell: start,
            parent: None,
        });
        open.push(first);

        let mut found = None;
        while !open.is_empty() {
            // Pop the best f. `open` is kept sorted, so it is the front.
            let current = open.remove(0);
            let (cell, g, parent) = (nodes[current].cell, nodes[current].g, Some(current));
            if cell == goal {
                found = Some(current);
                break;
            }
            if closed[index(cell)] {
                continue;
            }
            closed[index(cell)] = true;

            for (dx, dy) in [(0i32, -1i32), (1, 0), (0, 1), (-1, 0)] {
                let (nx, ny) = (i32::from(cell.0) + dx, i32::from(cell.1) + dy);
                let (Ok(nx), Ok(ny)) = (u16::try_from(nx), u16::try_from(ny)) else {
                    continue;
                };
                if nx >= wide || ny >= tall || closed[index((nx, ny))] {
                    continue;
                }
                let Some(step) = self.step_cost(level, nx, ny, false) else {
                    continue;
                };
                let next_g = g + i64::from(step);
                let next_f = next_g + manhattan((nx, ny), goal);
                // A cell already on the frontier is improved only if this path is strictly better.
                if let Some(at) = open
                    .iter()
                    .position(|at| nodes[*at].cell == (nx, ny) && nodes[*at].g <= next_g)
                {
                    let _ = at;
                    continue;
                }
                let node = nodes.len();
                nodes.push(Node {
                    f: next_f,
                    g: next_g,
                    cell: (nx, ny),
                    parent,
                });
                // Insertion sort on f, before equals (newest first), as `FUN_007ed500` does.
                let at = open
                    .iter()
                    .position(|other| nodes[*other].f >= next_f)
                    .unwrap_or(open.len());
                open.insert(at, node);
            }
        }

        let end = found?;
        let mut cells = Vec::new();
        let mut at = Some(end);
        while let Some(index) = at {
            cells.push(nodes[index].cell);
            at = nodes[index].parent;
        }
        cells.reverse();
        Some(CoarseRoute { level, cells })
    }

    /// **Refine a coarse route to waypoints**: one point per cell, at its centre - `n * 2^level + 0.5` tiles - which is where
    /// `FUN_007ee980`'s chain walk makes `FUN_007fd880` push a point, and what the movement code is handed.
    #[must_use]
    pub fn refine(&self, route: &CoarseRoute, scale: i64) -> Vec<(crate::Fixed, crate::Fixed)> {
        let size = (1i64 << route.level) * scale;
        route
            .cells
            .iter()
            .map(|cell| {
                (
                    crate::Fixed::from_raw(i64::from(cell.0) * size + size / 2),
                    crate::Fixed::from_raw(i64::from(cell.1) * size + size / 2),
                )
            })
            .collect()
    }

    /// **What it costs to step into a cell** - `FUN_007ed280`'s table, with the two rows our content cannot express named rather than
    /// invented:
    ///
    /// | cell | cost |
    /// | --- | --- |
    /// | blocked | `None` - the search never enters it |
    /// | a body standing on it | **2000** |
    /// | open, but with a blocked orthogonal neighbour | **300** |
    /// | open, all four orthogonal neighbours clear | **10** |
    ///
    /// The original's **100** ("blocked but a body may cross") and **600** ("not crossable, and the search still opens it") need terrain
    /// costs this content model does not carry: our tiles are walkable or not, with no cost between. When terrain costs land, those two
    /// rows are where they go.
    ///
    /// The distance-from-the-edge term (`(coord - window_lo) + 10` in the direction of travel) is **not** part of this answer: it belongs
    /// to the search, which knows the window, and it is added there.
    #[must_use]
    pub fn step_cost(&self, level: u8, cx: u16, cy: u16, body: bool) -> Option<i32> {
        if self.blocked(level, cx, cy) {
            return None;
        }
        if body {
            return Some(2000);
        }
        // The four orthogonal neighbours in the level's own cells; a neighbour off the map counts as clear, because nothing can be
        // walked into out there and the original's own border row costs the plain 10.
        let snug = [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)]
            .iter()
            .any(|(dx, dy)| {
                let nx = i32::from(cx) + dx;
                let ny = i32::from(cy) + dy;
                match (u16::try_from(nx), u16::try_from(ny)) {
                    (Ok(nx), Ok(ny)) => self.blocked(level, nx, ny),
                    _ => false,
                }
            });
        Some(if snug { 300 } else { 10 })
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
