//! A\* over the static tile grid.
//!
//! Costs are integers (10 for a cardinal step, 14 for a diagonal) rather than the f64 weights in
//! `movement.toml`: those values are still `-1.0` sentinels (U5/U7), and a pathing heuristic is an
//! algorithm constant, not a game value. When the weights are measured they replace `CARDINAL`
//! and `DIAGONAL` here and nowhere else.
//!
//! Determinism: the open set is a binary heap ordered by `(f, g, y, x)`, so two runs expand the
//! same nodes in the same order. No hash-map iteration order reaches the result.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

use ruage_data::{Map, Terrain};

const CARDINAL: u32 = 10;
const DIAGONAL: u32 = 14;

/// Which tiles a unit may stand on. Built once from the map, because terrain does not change
/// during a match yet.
pub struct Walk {
    width: u16,
    height: u16,
    blocked: Vec<bool>,
}

impl Walk {
    pub fn from_map(map: &Map) -> Self {
        Walk {
            width: map.width,
            height: map.height,
            blocked: map
                .terrain
                .iter()
                .map(|tile| *tile != Terrain::Grass)
                .collect(),
        }
    }

    pub fn blocked(&self, x: u16, y: u16) -> bool {
        if x >= self.width || y >= self.height {
            return true;
        }
        self.blocked[usize::from(y) * usize::from(self.width) + usize::from(x)]
    }

    pub fn contains(&self, x: u16, y: u16) -> bool {
        x < self.width && y < self.height
    }
}

fn octile(from: (u16, u16), to: (u16, u16)) -> u32 {
    let dx = i64::from(from.0) - i64::from(to.0);
    let dy = i64::from(from.1) - i64::from(to.1);
    let (dx, dy) = (dx.unsigned_abs(), dy.unsigned_abs());
    let diagonal = dx.min(dy);
    let straight = dx.max(dy) - diagonal;
    (straight * u64::from(CARDINAL) + diagonal * u64::from(DIAGONAL)) as u32
}

/// The tiles to walk through, excluding the start and including the goal. `None` when the goal is
/// unreachable. A blocked goal is allowed: resource nodes stand on forest, and a villager has to
/// be able to reach the tree it is working.
pub fn find_path(walk: &Walk, from: (u16, u16), to: (u16, u16)) -> Option<Vec<(u16, u16)>> {
    if !walk.contains(to.0, to.1) || !walk.contains(from.0, from.1) {
        return None;
    }
    if from == to {
        return Some(Vec::new());
    }
    // A blocked *start* is allowed and necessary: a villager stands on the tile of the tree it is
    // working, and must be able to walk off it. Only the route is searched, not the standing spot.

    let mut open: BinaryHeap<Reverse<(u32, u32, u16, u16)>> = BinaryHeap::new();
    let mut best: BTreeMap<(u16, u16), u32> = BTreeMap::new();
    let mut came: BTreeMap<(u16, u16), (u16, u16)> = BTreeMap::new();

    best.insert(from, 0);
    open.push(Reverse((octile(from, to), 0, from.1, from.0)));

    while let Some(Reverse((_, cost, y, x))) = open.pop() {
        let here = (x, y);
        if here == to {
            let mut path = vec![here];
            let mut cursor = here;
            while let Some(&previous) = came.get(&cursor) {
                if previous == from {
                    break;
                }
                path.push(previous);
                cursor = previous;
            }
            path.reverse();
            return Some(path);
        }
        if cost > *best.get(&here).unwrap_or(&u32::MAX) {
            continue;
        }

        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let (Some(nx), Some(ny)) = (
                    u16::try_from(i32::from(x) + dx).ok(),
                    u16::try_from(i32::from(y) + dy).ok(),
                ) else {
                    continue;
                };
                let diagonal = dx != 0 && dy != 0;
                let is_goal = (nx, ny) == to;
                if !walk.contains(nx, ny) {
                    continue;
                }
                // Walls may not be cut across: a diagonal step needs both of its corners open.
                if diagonal && (walk.blocked(nx, y) || walk.blocked(x, ny)) {
                    continue;
                }
                if !is_goal && walk.blocked(nx, ny) {
                    continue;
                }
                let step = if diagonal { DIAGONAL } else { CARDINAL };
                let next_cost = cost + step;
                let entry = best.entry((nx, ny)).or_insert(u32::MAX);
                if next_cost < *entry {
                    *entry = next_cost;
                    came.insert((nx, ny), here);
                    open.push(Reverse((
                        next_cost + octile((nx, ny), to),
                        next_cost,
                        ny,
                        nx,
                    )));
                }
            }
        }
    }
    None
}
