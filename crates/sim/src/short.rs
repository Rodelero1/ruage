//! **The short-range pathfinder: a route's corners are the obstacles' corners.**
//!
//! `plan_path` answers *which tiles* to cross, and `pull_route` throws away the nodes a clear line lets a unit skip. What is
//! left is a chain of **tile corners** - and a tile's origin is the point where four cells meet, so a body of radius 0.2
//! tiles turns half a tile from a wall and a larger body overlaps it. The owner saw that as "villagers went through the
//! house" (2026-11-04).
//!
//! This module answers the other question: *which point*, given that the body has a radius. The algorithm is the one the
//! original's own maintainers documented (`docs/pathfinding-from-the-talk.md`, Forgotten Empires' C++ talk): shrink the body
//! to a point, **expand every obstruction by its radius**, ray at the goal, and walk the outline of the union - here as a
//! **visibility graph** over the corners of the merged rectangles, which yields the same geodesic and is far easier to test
//! ("an edge exists only where a straight line never enters an expanded rectangle").
//!
//! Everything here is integer arithmetic: the project keeps `f64` out of the simulation, and the talk's own lesson is that
//! pathing precision, not pathing logic, is what breaks - so the segment-versus-rectangle test is exact rational arithmetic
//! rather than a sampled or floating one.
//!
//! It is called **once per route**, never per tick, and it refuses (returns `None`) rather than guessing when the work would
//! be disproportionate: the caller keeps the pulled tile chain, which is a straight walk between tile corners.

use std::collections::BTreeSet;

use ruage_fixed::Fixed;

use crate::path::Walk;

/// How far beyond the endpoints' box the window reaches, in tiles. An obstacle just outside the direct line still matters
/// for the corner it puts in the path.
const MARGIN: i64 = 3;
/// The largest window, in tiles on a side, this will solve. A long strategic walk keeps its tile corners instead of paying
/// for a map-sized graph: the corners a player watches are the ones in a base.
const MAX_SIDE: i64 = 40;
/// The most rectangles a window may produce before the solver declines.
const MAX_RECTS: usize = 96;
/// The most graph nodes (four per rectangle, plus the two endpoints) it will build.
const MAX_NODES: usize = 400;

/// A rectangle of **tiles**, inclusive of both corners.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct Rect {
    x0: u16,
    y0: u16,
    x1: u16,
    y1: u16,
}

/// An axis-aligned rectangle in **raw units**, as `(x0, y0, x1, y1)`, half-open like a tile.
type Box4 = (i64, i64, i64, i64);

/// The points a unit with `radius` should walk through to get from `from` to `to`, in order, or `None` when the window is
/// too big to solve or no way exists.
///
/// `avoid` is the tiles the caller's search routed around - a repath's bodies (D98a). They are obstacles here for the same
/// reason they were obstacles there: a route straightened through the body it was planned around is worse than no smoothing.
pub fn short_range(
    walk: &Walk,
    scale: i64,
    from: (Fixed, Fixed),
    to: (Fixed, Fixed),
    radius: i64,
    avoid: &BTreeSet<(u16, u16)>,
) -> Option<Vec<(Fixed, Fixed)>> {
    short_range_around(walk, scale, from, to, radius, avoid, &[])
}

/// **The plan with bodies in it** (D205). A re-plan's avoided units are not ground: each is a body of its own radius
/// standing somewhere in a tile, and the route has to clear the **pair's** reach (`mine + theirs`), not merely the mover's
/// own radius from the tile. Expanding a body's tile by the mover's radius alone - which is what the terrain rectangle does,
/// and what this function did to bodies - leaves room for a waypoint 0.2 of a tile from a 0.2 body, and `bound_to_reach`
/// then refuses it: measured 2026-11-14, the detour's first waypoint landed **400 raw inside** the pair's reach and the
/// unit stood there for the rest of the match. `bodies` is `(tile, pair reach)` pairs.
pub fn short_range_around(
    walk: &Walk,
    scale: i64,
    from: (Fixed, Fixed),
    to: (Fixed, Fixed),
    radius: i64,
    avoid: &BTreeSet<(u16, u16)>,
    bodies: &[((u16, u16), i64)],
) -> Option<Vec<(Fixed, Fixed)>> {
    if from == to {
        return Some(Vec::new());
    }
    if scale <= 0 {
        return None;
    }
    // **A minimum radius closes a pinch.** With no radius at all, two blocked tiles that touch at a corner leave a
    // zero-width gap a line can pass exactly through - and the tile rule forbids that ("no passage between two trees that
    // touch at a corner"). One raw unit of expansion closes every such pinch and is invisible in play.
    let radius = radius.max(1);

    // The window: the endpoints' own tiles, a margin, clipped to the map.
    let (width, height) = walk.size();
    let tile = |value: Fixed| -> i64 { value.raw().div_euclid(scale) };
    let (fx, fy) = (tile(from.0), tile(from.1));
    let (tx, ty) = (tile(to.0), tile(to.1));
    let x0 = (fx.min(tx) - MARGIN).max(0);
    let y0 = (fy.min(ty) - MARGIN).max(0);
    let x1 = (fx.max(tx) + MARGIN).min(i64::from(width) - 1);
    let y1 = (fy.max(ty) + MARGIN).min(i64::from(height) - 1);
    if x1 < x0 || y1 < y0 || x1 - x0 >= MAX_SIDE || y1 - y0 >= MAX_SIDE {
        return None;
    }
    let (Ok(x0), Ok(y0), Ok(x1), Ok(y1)) = (
        u16::try_from(x0),
        u16::try_from(y0),
        u16::try_from(x1),
        u16::try_from(y1),
    ) else {
        return None;
    };

    // **The start's own tile and the goal's are not walls.** A worker stands on the node it is working, and an order on
    // blocked ground means "come as close as you can" - the same two exemptions the search and `step_is_walkable` make.
    let start = (
        u16::try_from(fx).unwrap_or(0),
        u16::try_from(fy).unwrap_or(0),
    );
    let goal = (
        u16::try_from(tx).unwrap_or(0),
        u16::try_from(ty).unwrap_or(0),
    );
    let mut blocked: Vec<(u16, u16)> = Vec::new();
    for y in y0..=y1 {
        for x in x0..=x1 {
            if (x, y) == start || (x, y) == goal {
                continue;
            }
            if walk.blocked(x, y) || avoid.contains(&(x, y)) {
                blocked.push((x, y));
            }
        }
    }
    // Nothing in the window: the route is the straight line, which is the overwhelmingly common case and costs one pass.
    // **A body is something in the window even when no tile is** (D205): the first version of this checked `blocked` alone,
    // so a re-plan asked to go round a unit in open ground answered with the straight line through him.
    if blocked.is_empty() && bodies.is_empty() {
        return Some(vec![to]);
    }

    let rects = rectangles(&blocked);
    // **A body is a disc, so its tile grows by the pair's reach** (D205): `bound_to_reach` will demand `mine + theirs` of
    // every step, so the plan has to keep that much clear or its first waypoint is refused - measured 2026-11-14, the
    // detour's first waypoint landed 400 raw inside the reach and the unit stood there for the rest of the match.
    let body_rects: Vec<(Rect, i64)> = bodies
        .iter()
        .map(|((x, y), reach)| {
            (
                Rect {
                    x0: *x,
                    y0: *y,
                    x1: *x,
                    y1: *y,
                },
                (*reach).max(radius),
            )
        })
        .collect();
    if (rects.is_empty() && body_rects.is_empty()) || rects.len() + body_rects.len() > MAX_RECTS {
        return None;
    }
    // The Minkowski sum: every obstruction grows by the body's radius, so the body may be shrunk to the point `from`/`to`.
    let grow = |rect: &Rect, by: i64| {
        (
            i64::from(rect.x0) * scale - by,
            i64::from(rect.y0) * scale - by,
            (i64::from(rect.x1) + 1) * scale + by,
            (i64::from(rect.y1) + 1) * scale + by,
        )
    };
    let mut boxes: Vec<Box4> = rects.iter().map(|rect| grow(rect, radius)).collect();
    boxes.extend(body_rects.iter().map(|(rect, by)| grow(rect, *by)));

    // **Fence the window.** The talk's own lesson: "whoops, I edge-walked the whole map". An obstacle that runs out of the
    // window is not an obstacle that ends there, and a graph allowed to step over the window's border walks round the cut
    // end of a wall that continues - the sealed-wall test found exactly that. Four thin boxes at the window's tile
    // boundaries make leaving the search area impossible, so an unreachable goal inside it stays unreachable.
    let huge = i64::MAX / 4;
    let (wx0, wy0) = (i64::from(x0) * scale, i64::from(y0) * scale);
    let (wx1, wy1) = ((i64::from(x1) + 1) * scale, (i64::from(y1) + 1) * scale);
    let mut boxes = boxes;
    boxes.push((-huge, -huge, wx0, huge));
    boxes.push((wx1, -huge, huge, huge));
    boxes.push((-huge, -huge, huge, wy0));
    boxes.push((-huge, wy1, huge, huge));

    // The straight line first: if nothing stands on it, the path *is* the goal (the talk's step 2).
    let (a, b) = ((from.0.raw(), from.1.raw()), (to.0.raw(), to.1.raw()));
    if boxes.iter().all(|rect| !enters_interior(a, b, *rect)) {
        return Some(vec![to]);
    }

    // Nodes: the two endpoints and the four corners of every expanded rectangle. `BTreeSet` for a deterministic order -
    // the graph is walked in index order, so two runs build and search the same graph.
    let mut nodes: BTreeSet<(i64, i64)> = BTreeSet::new();
    nodes.insert(a);
    nodes.insert(b);
    // **A corner outside the world is not a way round.** A wall that runs to the map's edge has expanded corners beyond it,
    // and a graph that kept them found a "path" over the edge of the map for a sealed wall - the unit would have walked
    // outside the ground it is allowed to stand on.
    let (limit_x, limit_y) = (i64::from(width) * scale, i64::from(height) * scale);
    let inside = |point: (i64, i64)| {
        point.0 >= 0 && point.1 >= 0 && point.0 <= limit_x && point.1 <= limit_y
    };
    for &(bx0, by0, bx1, by1) in &boxes {
        for point in [(bx0, by0), (bx1, by0), (bx0, by1), (bx1, by1)] {
            if inside(point) {
                nodes.insert(point);
            }
        }
    }
    if nodes.len() > MAX_NODES {
        return None;
    }
    let nodes: Vec<(i64, i64)> = nodes.into_iter().collect();
    let (start_index, goal_index) = (nodes.binary_search(&a).ok()?, nodes.binary_search(&b).ok()?);

    // Visibility: an edge exists where the straight line enters no expanded rectangle's interior. Every edge lies inside
    // the window (both ends do, and the window is a rectangle), and every blocked tile in the window is covered by one of
    // these boxes, so rectangle clearance *is* tile clearance, only stricter by the body's radius.
    // **A start already inside a body's reach must be allowed to leave it** (D205). A man who meets another is, by
    // definition, standing inside the pair's reach; expanding that body's tile therefore puts the plan's own start inside an
    // obstacle and every edge from it is refused - measured 2026-11-14, `short_range_around` returned `None` and the paired
    // fallback straightened the route back through the body. Such a box is ignored for the *first* edge only: a step out of
    // the zone is legal, and the collision bound still refuses any step that ends inside it.
    let escape: Vec<Box4> = boxes
        .iter()
        .copied()
        .filter(|rect| {
            nodes[start_index].0 >= rect.0
                && nodes[start_index].0 <= rect.2
                && nodes[start_index].1 >= rect.1
                && nodes[start_index].1 <= rect.3
        })
        .collect();
    let count = nodes.len();
    let mut visible: Vec<Vec<usize>> = vec![Vec::new(); count];
    for i in 0..count {
        for j in (i + 1)..count {
            let from_start = i == start_index || j == start_index;
            if boxes.iter().all(|rect| {
                !enters_interior(nodes[i], nodes[j], *rect) || (from_start && escape.contains(rect))
            }) {
                visible[i].push(j);
                visible[j].push(i);
            }
        }
    }

    // Dijkstra over squared lengths, ties broken by node order so the route is deterministic.
    let distance = |p: (i64, i64), q: (i64, i64)| -> i128 {
        let (dx, dy) = (i128::from(q.0 - p.0), i128::from(q.1 - p.1));
        dx * dx + dy * dy
    };
    let mut best: Vec<i128> = vec![i128::MAX; count];
    let mut came: Vec<Option<usize>> = vec![None; count];
    let mut done: Vec<bool> = vec![false; count];
    best[start_index] = 0;
    loop {
        let mut here = None;
        let mut here_cost = i128::MAX;
        for index in 0..count {
            if !done[index] && best[index] < here_cost {
                here = Some(index);
                here_cost = best[index];
            }
        }
        let Some(here) = here else {
            break;
        };
        if here == goal_index {
            break;
        }
        done[here] = true;
        for &next in &visible[here] {
            if done[next] {
                continue;
            }
            let step = distance(nodes[here], nodes[next]);
            let candidate = here_cost.saturating_add(step);
            if candidate < best[next] {
                best[next] = candidate;
                came[next] = Some(here);
            }
        }
    }
    if best[goal_index] == i128::MAX {
        return None;
    }
    let mut chain = vec![goal_index];
    let mut cursor = goal_index;
    while let Some(previous) = came[cursor] {
        chain.push(previous);
        cursor = previous;
    }
    chain.reverse();
    // **The start is not a waypoint.** The first node is where the unit already stands, and a route that begins with it
    // costs a tick of standing still - which also made a lead-shot prediction read zero motion for a runner
    // (`an_arrow_misses_a_runner_unless_ballistics_leads_the_shot`, 2026-11-04).
    chain.remove(0);
    Some(
        chain
            .into_iter()
            .map(|index| {
                (
                    Fixed::from_raw(nodes[index].0),
                    Fixed::from_raw(nodes[index].1),
                )
            })
            .collect(),
    )
}

/// Merge blocked tiles into maximal rectangles: extend a run right along its row, then down while every column of the run
/// is blocked on the next row. Scanned in row-major order over a sorted list, so the decomposition is deterministic.
fn rectangles(blocked: &[(u16, u16)]) -> Vec<Rect> {
    let set: BTreeSet<(u16, u16)> = blocked.iter().copied().collect();
    let mut used: BTreeSet<(u16, u16)> = BTreeSet::new();
    let mut rects = Vec::new();
    for &(x, y) in &set {
        if used.contains(&(x, y)) {
            continue;
        }
        let mut x1 = x;
        while set.contains(&(x1.saturating_add(1), y)) && !used.contains(&(x1.saturating_add(1), y))
        {
            x1 = x1.saturating_add(1);
            if x1 == u16::MAX {
                break;
            }
        }
        let mut y1 = y;
        'down: loop {
            let next = y1.saturating_add(1);
            if next == y1 {
                break;
            }
            for xx in x..=x1 {
                if !set.contains(&(xx, next)) || used.contains(&(xx, next)) {
                    break 'down;
                }
            }
            y1 = next;
        }
        for yy in y..=y1 {
            for xx in x..=x1 {
                used.insert((xx, yy));
            }
        }
        rects.push(Rect {
            x0: x,
            y0: y,
            x1,
            y1,
        });
    }
    rects
}

/// Whether the segment `a..b` enters the **interior** of the box. Touching an edge or a corner does not count: a unit may
/// walk exactly along a wall's face, which is what "expanded by the radius" means.
///
/// Liang-Barsky, in exact rational arithmetic (no floats anywhere in this crate's simulation, and no sampled segment that
/// could step over a thin rectangle).
fn enters_interior(a: (i64, i64), b: (i64, i64), rect: Box4) -> bool {
    let (bx0, by0, bx1, by1) = rect;
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    // The parameter interval of the segment that lies inside the closed box, as fractions `num / den` with `den > 0`.
    let mut low = (0i128, 1i128);
    let mut high = (1i128, 1i128);
    for (p, q) in [
        (-i128::from(dx), i128::from(a.0) - i128::from(bx0)),
        (i128::from(dx), i128::from(bx1) - i128::from(a.0)),
        (-i128::from(dy), i128::from(a.1) - i128::from(by0)),
        (i128::from(dy), i128::from(by1) - i128::from(a.1)),
    ] {
        if p == 0 {
            // Parallel to this pair of faces. **On** one of them (`q == 0`) is not an interior: walking exactly along a
            // wall's expanded face is the tightest legal line, and refusing it made the whole graph unwalkable round a
            // plain wall - the first version of this test said "no path" for a three-tile block.
            if q <= 0 {
                return false;
            }
            continue;
        }
        // **Normalise before comparing.** `p` is negative for the two faces that bound `t` from below, and a fraction with
        // a negative denominator compares backwards - the first version of this refused every segment that had such a
        // bound, which is most of them, and the tests said "no path" for a wall a unit can plainly walk round.
        let bound = normalise((q, p));
        if p < 0 {
            // A lower bound on t: keep the larger.
            if greater(bound, low) {
                low = bound;
            }
        } else if greater(high, bound) {
            high = bound;
        }
        // A lower bound of 1 means the box is entered only at the far end, at best.
        if !greater(high, low) {
            return false;
        }
    }
    // A positive-length interval inside the closed box is a positive-length interval inside the interior.
    greater(high, low) && greater(high, (0i128, 1i128)) && greater((1i128, 1i128), low)
}

/// `left > right` for fractions with positive denominators.
fn greater(left: (i128, i128), right: (i128, i128)) -> bool {
    left.0 * right.1 > right.0 * left.1
}

/// A fraction with a positive denominator.
fn normalise(fraction: (i128, i128)) -> (i128, i128) {
    if fraction.1 < 0 {
        (-fraction.0, -fraction.1)
    } else {
        fraction
    }
}
