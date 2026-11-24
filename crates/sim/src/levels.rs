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

/// **The engine's tile size in raw units** - 20,000, the number the rest of the crate uses.
const TILE_SCALE: i64 = 20_000;

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
    /// **The cells whose neighbour costs 300** (D342): the same pyramid over the pack's mines, so a coarse cell is
    /// "expensive to be beside" when any of its tiles is a mine - the binary's own per-cell test, applied at that level.
    snug: Vec<Vec<bool>>,
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
        Self::for_radius(ground, 0)
    }

    /// **The pyramid for a body of `reach`** (D306): a tile is usable when that body fits on it - the original's "shrink the unit to a point,
    /// grow every obstruction by its radius", with tangency counting as fitting (the owner's HD test, 2026-11-20).
    #[must_use]
    pub fn for_radius(ground: &Ground, reach: i64) -> Self {
        let (width, height) = ground.size();

        let mut maps = Vec::with_capacity(DEPTH);
        let mut base = vec![false; usize::from(width) * usize::from(height)];
        for y in 0..height {
            for x in 0..width {
                base[usize::from(y) * usize::from(width) + usize::from(x)] =
                    !ground.fits_for(x, y, TILE_SCALE, reach);
            }
        }
        maps.push(base);

        // The 300 row's own mask, built the same way: a mine on level 0, OR-reduced up (D342).
        let mut snug = Vec::with_capacity(DEPTH);
        let mut snug_base = vec![false; usize::from(width) * usize::from(height)];
        for y in 0..height {
            for x in 0..width {
                snug_base[usize::from(y) * usize::from(width) + usize::from(x)] =
                    ground.is_snug(x, y);
            }
        }
        snug.push(snug_base);

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
            // The same OR-reduction over the mine mask.
            let below_snug = &snug[level - 1];
            let mut mine_map = vec![false; usize::from(wide) * usize::from(tall)];
            for cy in 0..tall {
                for cx in 0..wide {
                    let mut mine = false;
                    for dy in 0..2u16 {
                        for dx in 0..2u16 {
                            let child_x = cx * 2 + dx;
                            let child_y = cy * 2 + dy;
                            if child_x >= below_w || child_y >= below_h {
                                continue;
                            }
                            if below_snug
                                [usize::from(child_y) * usize::from(below_w) + usize::from(child_x)]
                            {
                                mine = true;
                            }
                        }
                    }
                    mine_map[usize::from(cy) * usize::from(wide) + usize::from(cx)] = mine;
                }
            }
            snug.push(mine_map);
        }

        Levels {
            maps,
            snug,
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
                let Some(step) = self.step_cost(level, nx, ny) else {
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

    /// **Refine a coarse route to waypoints: the crossings of the shared edges, not the cell centres** (D322).
    ///
    /// `FUN_007ee980` walks the parent chain a coarse node at a time, and `FUN_007eeda0` turns each step into a point on the
    /// boundary between the two cells: it expands the cell to the **tile span it covers** (`x << level .. ((x + 1) << level) - 1`),
    /// scans that span **tile by tile** along the direction of travel, and takes the first tile where the direction is passable,
    /// with the other coordinate interpolated. So a route's waypoints sit where the passage is, on the boundary, and the walker
    /// follows the corridor the cells describe instead of hopping between their middles.
    ///
    /// The first point is the start cell's own centre, which the caller discards (the walk starts where the body stands); the goal
    /// is the caller's, so no trailing centre is emitted.
    #[must_use]
    pub fn refine(
        &self,
        route: &CoarseRoute,
        scale: i64,
        from: (i64, i64),
        to: (i64, i64),
    ) -> Vec<(crate::Fixed, crate::Fixed)> {
        let level = route.level;
        let span = i64::from(1u32 << level.min(15));
        let size = span * scale;
        let centre = |cell: (u16, u16)| {
            (
                crate::Fixed::from_raw(i64::from(cell.0) * size + size / 2),
                crate::Fixed::from_raw(i64::from(cell.1) * size + size / 2),
            )
        };
        let mut points: Vec<(crate::Fixed, crate::Fixed)> = Vec::with_capacity(route.cells.len());
        let Some(first) = route.cells.first().copied() else {
            return points;
        };
        points.push(centre(first));
        for pair in route.cells.windows(2) {
            let (here, next) = (pair[0], pair[1]);
            // The step is 4-directional (the search opens four neighbours), so the shared edge is one line of `span` tiles: a column
            // for a step along x, a row for a step along y.
            let along_x = next.0 != here.0;
            let (boundary, first) = if along_x {
                (
                    (i64::from(next.0.min(here.0)) + 1) * span,
                    i64::from(here.1) * span,
                )
            } else {
                (
                    (i64::from(next.1.min(here.1)) + 1) * span,
                    i64::from(here.0) * span,
                )
            };
            // **Where the straight line crosses this edge** (D343). `FUN_007eeda0` is handed the mover's position and a far
            // point, and computes `(distance to the edge) * delta_across / delta_along + start` - the line's own crossing -
            // before it looks at a single tile. Ours scanned from the span's low end, so on an edge with two crossings it
            // left through the bottom one however the walk was shaped.
            let start = {
                let (travel, across) = if along_x {
                    (to.0 - from.0, (from.1, to.1))
                } else {
                    (to.1 - from.1, (from.0, to.0))
                };
                if travel == 0 {
                    span / 2
                } else {
                    let edge = boundary * scale;
                    let along = if along_x { from.0 } else { from.1 };
                    let value = across.0 + (edge - along) * (across.1 - across.0) / travel;
                    (value.div_euclid(scale) - first).clamp(0, span - 1)
                }
            };
            // **The nearest crossing to that point, forward first** - the binary's order: it keeps the first crossing
            // forward at distance `d`, then walks back over distances `0..d`, and a crossing found there wins because it is
            // strictly nearer. With no forward crossing it walks the whole span back.
            let crossable = |k: i64| -> bool {
                let (Ok(tile), Ok(before), Ok(after)) = (
                    u16::try_from(first + k),
                    u16::try_from(boundary - 1),
                    u16::try_from(boundary),
                ) else {
                    return false;
                };
                let (a, b) = if along_x {
                    ((before, tile), (after, tile))
                } else {
                    ((tile, before), (tile, after))
                };
                !self.blocked(0, a.0, a.1) && !self.blocked(0, b.0, b.1)
            };
            let forward = (start..span).find(|k| crossable(*k));
            let budget = forward.map_or(span, |k| k - start);
            let backward = (0..budget.min(start + 1))
                .map(|d| start - d)
                .find(|k| crossable(*k));
            let chosen = backward.or(forward);
            let offset = chosen.unwrap_or(span / 2);
            let point = if along_x {
                (
                    crate::Fixed::from_raw(boundary * scale),
                    crate::Fixed::from_raw((first + offset) * scale + scale / 2),
                )
            } else {
                (
                    crate::Fixed::from_raw((first + offset) * scale + scale / 2),
                    crate::Fixed::from_raw(boundary * scale),
                )
            };
            points.push(point);
        }
        // No trailing centre: the caller appends the goal it planned to, so a point here would be a waypoint the walker has to
        // visit before its goal, on ground it may not be able to stand on once the goal's own tile is standable.
        points
    }

    /// **What it costs to step into a cell** - the two rows of `FUN_007ed280`'s table this content can express:
    ///
    /// | cell | cost |
    /// | --- | --- |
    /// | blocked | `None` - the search never enters it |
    /// | open, but with a blocked orthogonal neighbour | **300** |
    /// | open, all four orthogonal neighbours clear | **10** |
    ///
    /// **The three rows that are not here, and why** (D327, D328):
    ///
    /// - **100/600** are keyed on the **tile's terrain object**: bit 3 of its flag byte `+0x28` turns a 100 into a 300, bit 6 gives the
    ///   600. Our install is a different build from the decompiled binary - the same string sits 0x8346 lower in its image and the table the
    ///   function indexes (`DAT_00af541c`) lies outside it - so the two bits cannot be named from the sources in hand.
    /// - **2000** is an override keyed on a per-tile grid (`[0x4d]`) of **eight-direction run masks** built by `FUN_007ef3f0`, not on
    ///   standing bodies. The only body-aware test in the path system (`FUN_00801a20`) is in the **flat** search. A `body` parameter used
    ///   to price this row here and **no caller ever passed it true**, so it was a rule the original does not have (D328).
    ///
    /// The distance-from-the-edge term (`(coord - window_lo) + 10` in the direction of travel) is **not** part of this answer: it belongs
    /// to the search, which knows the window, and it is added there.
    #[must_use]
    pub fn step_cost(&self, level: u8, cx: u16, cy: u16) -> Option<i32> {
        if self.blocked(level, cx, cy) {
            return None;
        }
        // **The cell and its four orthogonal neighbours, for a mine** (D342): `FUN_007f11f0` answers 1 only for an
        // object of class 8 or 32, so a cell beside a tree, a wall or a cliff is the plain 10 and only a mine is 300.
        // Before this the test was "any blocked neighbour", which priced every wall the search had to pass.
        // A neighbour off the map counts as clear: nothing can be walked into out there, and the binary's own border row
        // costs the plain 10.
        // The bound is the level's own cell grid, not `u16`: a cell one past the right edge is a valid `u16` and an
        // out-of-range index (measured - ten conformance fixtures died on `the len is 72 but the index is 72`).
        let (wide, tall) = self.cells(level);
        let mine = |x: i32, y: i32| -> bool {
            let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y)) else {
                return false;
            };
            if x >= wide || y >= tall {
                return false;
            }
            self.snug
                .get(usize::from(level))
                .is_some_and(|map| map[usize::from(y) * usize::from(wide) + usize::from(x)])
        };
        let snug = [(0i32, 0i32), (-1, 0), (1, 0), (0, -1), (0, 1)]
            .iter()
            .any(|(dx, dy)| mine(i32::from(cx) + dx, i32::from(cy) + dy));
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
