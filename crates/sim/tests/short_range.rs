//! **M2: the short-range pathfinder.** A route's corners are the obstacles' corners, not tile centres, and the line it
//! chooses keeps the body's radius away from every wall it passes.
//!
//! The owner's report that started this: *"villagers went through the house"* - a unit turning at the *origin* of a cell,
//! which is the point where four tiles meet, so a body of radius 0.2 tiles brushes a wall and a radius 0.5 body overlaps it.
//! `docs/pathfinding-from-the-talk.md` is the source of the algorithm (Forgotten Empires' own talk); this file pins the
//! properties the engine needs from it.
//!
//! Written before the implementation (D7, SPEC §11). D113.

use std::collections::BTreeSet;

use ruage_data::Map;
use ruage_fixed::Fixed;
use ruage_sim::{Walk, short_range};

/// The project's scale, so a tile is 20,000 raw units and the fixtures read in tiles.
const SCALE: i64 = 20_000;
/// A villager's body: the dat's `radius[0]`.
const RADIUS: i64 = SCALE / 5;

fn map(width: u16, height: u16) -> Map {
    ruage_data::Map {
        width,
        height,
        terrain: vec![ruage_data::Terrain::Grass; usize::from(width) * usize::from(height)],
        resources: Vec::new(),
        town_centers: Vec::new(),
    }
}

fn walk(width: u16, height: u16, blocked: &[(u16, u16)]) -> Walk {
    let mut walk = Walk::from_map(&map(width, height));
    for (x, y) in blocked {
        walk.block(*x, *y);
    }
    walk
}

fn at(x: i64, y: i64) -> (Fixed, Fixed) {
    (Fixed::from_raw(x * SCALE), Fixed::from_raw(y * SCALE))
}

fn tiles(points: &[(Fixed, Fixed)]) -> Vec<(f64, f64)> {
    points
        .iter()
        .map(|p| {
            (
                p.0.raw() as f64 / SCALE as f64,
                p.1.raw() as f64 / SCALE as f64,
            )
        })
        .collect()
}

/// The squared distance from a point to a tile's square, in raw units: zero when the point is inside it.
fn distance_to_tile(point: (Fixed, Fixed), tile: (u16, u16)) -> i64 {
    let (x0, y0) = (i64::from(tile.0) * SCALE, i64::from(tile.1) * SCALE);
    let (x1, y1) = (x0 + SCALE, y0 + SCALE);
    let (px, py) = (point.0.raw(), point.1.raw());
    let dx = (x0 - px).max(0).max(px - x1);
    let dy = (y0 - py).max(0).max(py - y1);
    dx * dx + dy * dy
}

/// Open ground: the path *is* the goal, and nothing else - the start is where the unit already stands, so it is not a
/// waypoint (a route beginning with it costs a tick of standing still).
#[test]
fn open_ground_is_one_segment() {
    let walk = walk(16, 16, &[]);
    let points = short_range(&walk, SCALE, at(2, 2), at(12, 9), RADIUS, &BTreeSet::new())
        .expect("open ground is always a path");
    assert_eq!(points.len(), 1, "open ground gave {} points", points.len());
    assert_eq!(points[0], at(12, 9), "the path must end on the goal");
}

/// A block in the way: the path goes round, and every one of its points keeps the body's radius from the block. This is the
/// property the tile-corner route could not offer - a corner at a cell's origin is half a tile from the wall, not a radius.
#[test]
fn a_block_in_the_way_is_rounded_at_arms_length() {
    let walk = walk(16, 16, &[(6, 7), (7, 7), (8, 7)]);
    let points = short_range(&walk, SCALE, at(2, 7), at(12, 7), RADIUS, &BTreeSet::new())
        .expect("a wall three tiles long is walkable round");
    assert!(
        points.len() >= 3,
        "a wall has to bend the path: {:?}",
        tiles(&points)
    );
    for point in &points {
        for tile in [(6u16, 7u16), (7, 7), (8, 7)] {
            let gap = distance_to_tile(*point, tile);
            assert!(
                gap >= RADIUS * RADIUS,
                "a point of the path ({}, {}) is {} raw units from tile {:?}, inside the body's {}",
                point.0.raw(),
                point.1.raw(),
                gap as f64,
                tile,
                RADIUS
            );
        }
    }
    // And it is not a detour for its own sake: round one three-tile wall is shorter than the tile route that goes out and
    // back along the wall's ends by a whole extra tile.
    let length: f64 = points
        .windows(2)
        .map(|pair| {
            let dx = (pair[1].0.raw() - pair[0].0.raw()) as f64 / SCALE as f64;
            let dy = (pair[1].1.raw() - pair[0].1.raw()) as f64 / SCALE as f64;
            (dx * dx + dy * dy).sqrt()
        })
        .sum();
    assert!(
        length < 11.0,
        "the path is {length:.2} tiles for a ten-tile walk: it wandered"
    );
}

/// A gap between two blocks is a way through, and the path takes it rather than going round the outside.
#[test]
fn a_gap_between_two_blocks_is_used() {
    let walk = walk(20, 20, &[(8, 6), (8, 7), (8, 9), (8, 10)]);
    let points = short_range(&walk, SCALE, at(4, 8), at(12, 8), RADIUS, &BTreeSet::new())
        .expect("the gap at (8, 8) is a way through");
    // Through the gap means a **segment** crosses the gap's column inside the gap's row: the path's own points are corners of
    // the expanded blocks (0.2 tiles off), so no point would ever sit exactly on the gap's tile.
    let gap = (8 * SCALE, 8 * SCALE, 9 * SCALE, 9 * SCALE);
    let crosses = points.windows(2).any(|pair| {
        let (a, b) = (pair[0], pair[1]);
        let (ay, by) = (a.1.raw(), b.1.raw());
        // The x where the segment reaches the gap's left edge, interpolated in integers.
        let (dx, gx) = (b.0.raw() - a.0.raw(), gap.0 - a.0.raw());
        if dx == 0 || (gx < 0) != (dx < 0) || gx.abs() > dx.abs() {
            return false;
        }
        let y = ay + (by - ay) * gx / dx;
        y > gap.1 && y < gap.3 && {
            let _ = by;
            true
        }
    });
    assert!(
        crosses,
        "the path did not pass through the one-tile gap: {:?}",
        tiles(&points)
    );
}

/// A solid wall across the whole map has no way round, and says so rather than inventing a line through it.
#[test]
fn a_wall_across_the_world_is_no_path() {
    let mut blocked = Vec::new();
    for y in 0..20u16 {
        blocked.push((9u16, y));
    }
    let walk = walk(20, 20, &blocked);
    assert!(
        short_range(
            &walk,
            SCALE,
            at(4, 10),
            at(14, 10),
            RADIUS,
            &BTreeSet::new()
        )
        .is_none(),
        "a sealed wall produced a path through it"
    );
}

/// The same question twice gives the same answer, corner for corner: a route is state, and two runs of a replay must walk
/// the same line.
#[test]
fn the_path_is_the_same_every_time() {
    let walk = walk(20, 20, &[(9, 7), (9, 8), (10, 8), (11, 8), (11, 9)]);
    let first = short_range(&walk, SCALE, at(3, 4), at(16, 13), RADIUS, &BTreeSet::new())
        .expect("a path exists");
    for _ in 0..4 {
        let again = short_range(&walk, SCALE, at(3, 4), at(16, 13), RADIUS, &BTreeSet::new())
            .expect("a path exists");
        assert_eq!(first, again, "two solves disagree");
    }
}

/// The start's own tile is never an obstacle: a worker stands on the node it is working, and a viler must be able to leave.
#[test]
fn the_start_tile_is_not_a_wall() {
    let walk = walk(12, 12, &[(3, 3)]);
    let points = short_range(&walk, SCALE, at(3, 3), at(9, 3), RADIUS, &BTreeSet::new())
        .expect("a unit standing on a blocked tile can still walk off it");
    assert_eq!(
        points.last().copied(),
        Some(at(9, 3)),
        "the path does not end on the goal"
    );
    assert!(
        !points.contains(&at(3, 3)),
        "the start came back as a waypoint: {points:?}"
    );
}

/// A goal on blocked ground - a tree a villager is sent to - is a point, not a wall: the path ends on it.
#[test]
fn a_goal_on_a_blocked_tile_is_still_reached() {
    let walk = walk(12, 12, &[(9, 6)]);
    let points = short_range(&walk, SCALE, at(3, 6), at(9, 6), RADIUS, &BTreeSet::new())
        .expect("the goal is a point the unit may be sent to");
    assert_eq!(points.last().copied(), Some(at(9, 6)));
}
