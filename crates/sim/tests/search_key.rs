//! **The search's key is the game's: `g + octile`, not `g + 25 x euclid`** (D337).
//!
//! `FUN_008007f0` line 177 stores `FUN_007ffd10(start, goal)` - the octile - as the start's frontier value, and line 316 computes
//! a neighbour's as `stored[current] - octile(current, goal) + octile(neighbour, goal) + step(current, neighbour)`, which is
//! `g + octile` because `stored[current]` already is. The `25 x euclid` in the same function (lines 186-191) has one reader: the
//! expansion guard at line 198. It is the **budget**, not the key.
//!
//! The observable: a key decides which of two equal-length corridors the search settles on, and an inadmissible one settles on a
//! **longer** route. One tree is enough to show it. This fixture asserts the route's *cost* rather than its waypoints, because
//! the cost is the rule and the waypoints are one of several routes that satisfy it - a later tie-break change must not break it.

use ruage_data::{Map, Terrain};
use ruage_sim::Ground;

const SCALE: i64 = 20_000;
/// The octile table's costs, from `FUN_007ffc80`: 16 a cardinal step, 23 a diagonal.
const CARDINAL: i64 = 16;
const DIAGONAL: i64 = 23;

fn field(width: u16, height: u16, blocked: &[(u16, u16)]) -> Ground {
    let map = Map {
        width,
        height,
        terrain: vec![Terrain::Grass; usize::from(width) * usize::from(height)],
        resources: Vec::new(),
        town_centers: Vec::new(),
    };
    let mut ground = Ground::from_map(&map);
    for (x, y) in blocked {
        ground.block(*x, *y);
    }
    ground
}

fn cost(from: (u16, u16), chain: &[(u16, u16)]) -> i64 {
    let mut total = 0;
    let mut here = from;
    for &next in chain {
        total += if next.0 != here.0 && next.1 != here.1 {
            DIAGONAL
        } else {
            CARDINAL
        };
        here = next;
    }
    total
}

/// **A walk round one tree costs what the octile key pays, not what a greedy key does.**
///
/// Open ground, (2,2) to (14,14), one tree at (8,8). The straight line is the octile bound, `12 x 23 = 276`; a route that goes
/// round the tree on the diagonal pays one hop sideways and back, `276 + 18 = 294`. The tree forces the detour, so the two
/// answers are distinguishable: with `g + octile` the search returns the 294 route, and with `g + 25 x euclid` it returns a
/// staircase that steps *down* the cell and costs **312** - a 6% longer walk for the same journey, which is what the owner sees
/// as "units walk via squares".
#[test]
fn a_walk_round_one_tree_costs_what_the_octile_key_pays() {
    let ground = field(16, 16, &[(8, 8)]);
    let (from, to) = ((2u16, 2u16), (14u16, 14u16));
    let chain = ground
        .search(from, to, SCALE, &[])
        .expect("open ground is routable");
    let last = *chain.last().expect("a route has an end");
    assert_eq!(last, to, "the walk must end on the goal");

    let paid = cost(from, &chain);
    // The bound for the detour: twelve diagonals for the distance, plus the hop that steps round the tree.
    assert_eq!(
        paid, 294,
        "the route round one tree paid {paid} instead of the octile key's 294 (the bound is 276, a greedy key pays 312): {chain:?}"
    );
}
