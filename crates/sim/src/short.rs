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
/// The most rectangles a window may produce before the solver declines.
///
/// **A bound on the work, not on the distance** (2026-11-19). This module used to refuse any window wider than 40 tiles on
/// a side - our number, and a *geometric* limit the original does not have: its bound on a search is a **budget of `h`
/// expansions, capped at 20,000** (`FUN_008007f0`), i.e. a count of work. A 58-tile walk across open ground builds two
/// nodes and costs nothing, so refusing it was the divergence `the_original_solves_a_long_walk_in_its_hierarchy` names -
/// and on this map the flat search reaches a goal anywhere, so no hierarchy is needed to answer it. The remaining two
/// constants are the honest analogue of that budget: they cap rectangles and nodes, not the span between two points.
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
    short_range_clear(walk, scale, from, to, radius, avoid, &[])
}

/// **A small enough pencil** (D211): the same search, with bodies as **discs** instead of tiles.
///
/// The original plans on continuous rectangles - each body a box of `pos +- radius`, grown by the mover's radius - so it can
/// thread a gap narrower than a tile. A graph built only from tile corners cannot: beside a man standing in a lane there is
/// no free *tile* to route through, the search fails, the fallback straightens the route, and the walker ghosts through him
/// (measured 2026-11-14: 4000 raw against an 8000 reach). `bodies` is `(centre, clearance)`: the route keeps `clearance`
/// clear of each centre, and the graph gains eight **shoulder points** round every body so the detour can bend inside a tile.
pub fn short_range_clear(
    walk: &Walk,
    scale: i64,
    from: (Fixed, Fixed),
    to: (Fixed, Fixed),
    radius: i64,
    avoid: &BTreeSet<(u16, u16)>,
    bodies: &[((Fixed, Fixed), i64)],
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

    // **A start inside a body is re-seeded** (D212). The original does exactly this in its low-level search: when the start
    // point falls inside an obstruction it offsets it by `2 x radius` along a fixed ring of directions, re-seeds from the
    // first free one, and returns "no low-level path" only when every one is blocked (`FUN_007fb220`). Without it a walker
    // whose nose is inside the man who just stopped in front of him can never plan round him - the search has no legal
    // first step - so the old straight route survives and he walks through. The anchor becomes the route's first waypoint.
    let mut from = from;
    let mut anchor: Option<(Fixed, Fixed)> = None;
    let inside = |point: (i64, i64), bodies: &[((Fixed, Fixed), i64)]| -> i64 {
        bodies
            .iter()
            .filter(|(centre, clearance)| {
                contains_disc(point, (centre.0.raw(), centre.1.raw()), *clearance)
            })
            .map(|(_, clearance)| *clearance)
            .max()
            .unwrap_or(0)
    };
    let trapped = inside((from.0.raw(), from.1.raw()), bodies);
    if trapped > 0 {
        // A fixed ring, in tenths of a raw unit: the four axes and the four diagonals (the binary's table is a sampled ring
        // of unit vectors; these eight are the same shape and are exact in integers).
        const RING: [(i64, i64); 8] = [
            (100, 0),
            (-100, 0),
            (0, 100),
            (0, -100),
            (71, 71),
            (-71, -71),
            (-71, 71),
            (71, -71),
        ];
        let out = trapped * 2;
        for (dx, dy) in RING {
            let candidate = (from.0.raw() + out * dx / 100, from.1.raw() + out * dy / 100);
            if inside(candidate, bodies) > 0 {
                continue;
            }
            // **And the step out must not cross him** (D212): a candidate on the far side is "free" but the straight line
            // to it runs through the body - measured 3702 raw against an 8000 reach, i.e. the anchor made the overlap
            // worse. A point beyond the man he is stuck inside is a *destination*, not a place to start: the anchor must
            // move away from every disc the start is inside, and clear every other one.
            let here = (from.0.raw(), from.1.raw());
            if bodies.iter().any(|(centre, clearance)| {
                !step_out_ok(
                    here,
                    candidate,
                    (centre.0.raw(), centre.1.raw()),
                    *clearance,
                )
            }) {
                continue;
            }
            let tile_x = candidate.0.div_euclid(scale);
            let tile_y = candidate.1.div_euclid(scale);
            let (map_w, map_h) = walk.size();
            if !(0..i64::from(map_w)).contains(&tile_x) || !(0..i64::from(map_h)).contains(&tile_y)
            {
                continue;
            }
            if walk.blocked(
                u16::try_from(tile_x).unwrap_or(u16::MAX),
                u16::try_from(tile_y).unwrap_or(u16::MAX),
            ) {
                continue;
            }
            anchor = Some((Fixed::from_raw(candidate.0), Fixed::from_raw(candidate.1)));
            from = anchor.expect("just set");
            break;
        }
        // Every offset blocked: the original answers 3 here, which its caller reads as "no low-level path".
        anchor?;
    }

    let with_anchor = |mut points: Vec<(Fixed, Fixed)>| -> Vec<(Fixed, Fixed)> {
        if let Some(anchor) = anchor {
            points.insert(0, anchor);
        }
        points
    };

    // The window: the endpoints' own tiles, a margin, clipped to the map.
    let (width, height) = walk.size();
    let tile = |value: Fixed| -> i64 { value.raw().div_euclid(scale) };
    let (fx, fy) = (tile(from.0), tile(from.1));
    let (tx, ty) = (tile(to.0), tile(to.1));
    let x0 = (fx.min(tx) - MARGIN).max(0);
    let y0 = (fy.min(ty) - MARGIN).max(0);
    let x1 = (fx.max(tx) + MARGIN).min(i64::from(width) - 1);
    let y1 = (fy.max(ty) + MARGIN).min(i64::from(height) - 1);
    if x1 < x0 || y1 < y0 {
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
    // **Nothing in the window, nothing in the way.** The straight line is the route, which is the overwhelmingly common
    // case and costs one pass - but "nothing in the window" means nothing in the *tiles*, and this shortcut used to
    // return before `bodies` was read at all. An open field with one man in it therefore answered with a line through
    // him: measured, `group_overlap::a_mate_who_is_stopped_becomes_solid`, closest 5012 raw against the 8000 the pair's
    // footprints need, with the re-plan that was asked for producing a "detour" whose second leg passed 0.25 tiles from
    // his centre. A body is an obstacle to this search exactly as a wall is (D211), so the shortcut must ask both.
    let clear_of_bodies = |a: (i64, i64), b: (i64, i64)| {
        bodies.iter().all(|(centre, clearance)| {
            clears_disc(a, b, (centre.0.raw(), centre.1.raw()), *clearance)
        })
    };
    if blocked.is_empty() && clear_of_bodies((from.0.raw(), from.1.raw()), (to.0.raw(), to.1.raw()))
    {
        return Some(with_anchor(vec![to]));
    }

    let rects = rectangles(&blocked);
    // **An empty rectangle set is not a failure when a body stands in the window** (D211 follow-up): the window can be
    // clear of terrain and still hold the man this search was asked to route round, and his octagon is then the whole of
    // the graph. `rectangles` cannot return an empty set for a non-empty window, so this test only ever fired on the case
    // above - which used to be answered before this point.
    if rects.len() > MAX_RECTS {
        return None;
    }
    // The Minkowski sum: every obstruction grows by the body's radius, so the body may be shrunk to the point `from`/`to`.
    let boxes: Vec<Box4> = rects
        .iter()
        .map(|rect| {
            (
                i64::from(rect.x0) * scale - radius,
                i64::from(rect.y0) * scale - radius,
                (i64::from(rect.x1) + 1) * scale + radius,
                (i64::from(rect.y1) + 1) * scale + radius,
            )
        })
        .collect();

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
    let clear_of_bodies = bodies
        .iter()
        .all(|(centre, clearance)| clears_disc(a, b, (centre.0.raw(), centre.1.raw()), *clearance));
    if clear_of_bodies && boxes.iter().all(|rect| !enters_interior(a, b, *rect)) {
        return Some(with_anchor(vec![to]));
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
    // **A body is a polygon, not a point** (D211). Each disc contributes the eight vertices of the octagon that
    // *circumscribes* its clearance circle, so the edges between adjacent vertices are tangent to it and a route can run
    // along them at exactly the clearance - which is what lets a detour bend **inside one tile**. This is the mechanism the
    // D211 review described and the code did not have: `clearance.rs`'s two fixtures passed without it, because the
    // stander's own *tile* was in the avoid set and a whole-tile rectangle existed to bend round, so nothing ever asked
    // for a sub-tile bend.
    //
    // The octagon's radius is `1.083 x clearance` and its diagonal vertices are at `0.765 x clearance` - a regular
    // octagon's circumradius is `1/cos(22.5 deg) = 1.0824` times its edge distance, and rounding *out* by a fifth of a
    // per-cent keeps every edge at or beyond the circle in integer arithmetic. That slack is 0.0009 of the clearance (7
    // raw units for two villagers), far below anything the simulation can see, and it is deliberate: rounding *in* would
    // make every tangent edge cut the disc and the graph would have no legal step at all.
    for (centre, clearance) in bodies {
        let (cx, cy) = (centre.0.raw(), centre.1.raw());
        let axis = clearance * 1083 / 1000;
        let diagonal = clearance * 765 / 1000;
        for point in [
            (cx + axis, cy),
            (cx - axis, cy),
            (cx, cy + axis),
            (cx, cy - axis),
            (cx + diagonal, cy + diagonal),
            (cx + diagonal, cy - diagonal),
            (cx - diagonal, cy + diagonal),
            (cx - diagonal, cy - diagonal),
        ] {
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
    let count = nodes.len();
    let mut visible: Vec<Vec<usize>> = vec![Vec::new(); count];
    for i in 0..count {
        for j in (i + 1)..count {
            // **Two kinds of obstacle now** (D211): terrain, which is boxes, and bodies, which are discs. A man who has just
            // met somebody is inside that man's clearance by definition, so an edge *leaving* the start's disc is allowed
            // iff it moves **away** from the centre - one rule, `step_out_ok`, shared with the re-seed, because a step that
            // ends on the far side of the man he is stuck inside is a step through him.
            let clear_of_boxes = boxes
                .iter()
                .all(|rect| !enters_interior(nodes[i], nodes[j], *rect));
            let clear_of_bodies = bodies.iter().all(|(centre, clearance)| {
                let at = (centre.0.raw(), centre.1.raw());
                if i == start_index {
                    step_out_ok(nodes[i], nodes[j], at, *clearance)
                } else if j == start_index {
                    step_out_ok(nodes[j], nodes[i], at, *clearance)
                } else {
                    clears_disc(nodes[i], nodes[j], at, *clearance)
                }
            });
            if clear_of_boxes && clear_of_bodies {
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
    let points: Vec<(Fixed, Fixed)> = chain
        .into_iter()
        .map(|index| {
            (
                Fixed::from_raw(nodes[index].0),
                Fixed::from_raw(nodes[index].1),
            )
        })
        .collect();
    Some(with_anchor(points))
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
/// **Does the segment stay `clearance` away from the point?** Integer only: the perpendicular distance is `cross / length`,
/// so the test is `cross^2 >= clearance^2 * length^2` - no division, no sqrt, exact in i128.
fn clears_disc(a: (i64, i64), b: (i64, i64), centre: (i64, i64), clearance: i64) -> bool {
    let (abx, aby) = (i128::from(b.0 - a.0), i128::from(b.1 - a.1));
    let (acx, acy) = (i128::from(centre.0 - a.0), i128::from(centre.1 - a.1));
    let reach = i128::from(clearance) * i128::from(clearance);
    let length = abx * abx + aby * aby;
    if length == 0 {
        return acx * acx + acy * acy >= reach;
    }
    // Within the perpendicular band the nearest point is the foot; outside it, one of the ends.
    let dot = acx * abx + acy * aby;
    if !(0..=length).contains(&dot) {
        let (bcx, bcy) = (i128::from(centre.0 - b.0), i128::from(centre.1 - b.1));
        // **Both ends, not either** (found 2026-11-14). The foot lies outside the segment, so the distance to it is
        // monotone along it and the nearest point is an end: the segment is clear only when **both** ends are. This read
        // `d(a) >= reach || d(b) >= reach`, which calls a segment clear whenever its *far* end is far - so a line that
        // begins inside a man's clearance and ends on his other side passed, which is exactly the "walk through him"
        // this predicate exists to refuse (measured: the route's second leg 0.25 tiles from a centre it had to keep
        // 0.4 from).
        return acx * acx + acy * acy >= reach && bcx * bcx + bcy * bcy >= reach;
    }
    let cross = (abx * acy - aby * acx).abs();
    cross * cross >= reach * length
}

/// Whether a point is inside a body's clearance circle - the licence to *leave* one, never to enter it.
fn contains_disc(point: (i64, i64), centre: (i64, i64), clearance: i64) -> bool {
    let (dx, dy) = (
        i128::from(point.0 - centre.0),
        i128::from(point.1 - centre.1),
    );
    dx * dx + dy * dy < i128::from(clearance) * i128::from(clearance)
}

/// **May a step leave the point it starts on?** One rule, asked by the re-seed and by the graph's first edges, because
/// the two ask the same question and the first version answered it two different ways - both wrong.
///
/// A step from **outside** a disc must stay clear of it. A step from **inside** one can never be clear (every line from an
/// interior point passes within the clearance), so the only legal ones are those that move *away* from that centre: the
/// walker is already overlapping - a man stopped in front of him - and leaving is the whole point, while a step that ends
/// on his far side is a step through him. Measured both ways: "any edge off the start" let the walker aim straight at his
/// goal through the man (3702 raw against an 8000 reach), and "any edge that widens the gap" still admitted the far side,
/// which is the same walk in one hop.
fn step_out_ok(from: (i64, i64), to: (i64, i64), centre: (i64, i64), clearance: i64) -> bool {
    if !contains_disc(from, centre, clearance) {
        return clears_disc(from, to, centre, clearance);
    }
    let (step_x, step_y) = (i128::from(to.0 - from.0), i128::from(to.1 - from.1));
    let (out_x, out_y) = (i128::from(centre.0 - from.0), i128::from(centre.1 - from.1));
    // `<= 0` and not `< 0`: a step exactly along the tangent neither approaches the centre nor enters the disc, and a man
    // wedged in a crowd may have nothing else - refusing it would leave him no legal first step at all.
    step_x * out_x + step_y * out_y <= 0
}

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
