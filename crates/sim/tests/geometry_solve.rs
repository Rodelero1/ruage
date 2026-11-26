//! Re-ray loop with the no-failure contract: open ground reaches, a walled-in
//! goal answers `Closest` without ever crossing a wall, a surrounded start
//! expands once then stops beside itself, and the turn count names a hole
//! apart from open ground.

use ruage_fixed::Fixed;
use ruage_sim::{Clear, WayRoute, solve, verify_path, walk_shape};

const SCALE: i64 = 20_000;

fn point(tx: i64, ty: i64) -> (Fixed, Fixed) {
    (Fixed::from_raw(tx * SCALE), Fixed::from_raw(ty * SCALE))
}

fn raw_points(route: &WayRoute) -> Vec<(i64, i64)> {
    let points = match route {
        WayRoute::Reached(points) | WayRoute::Closest(points) => points,
    };
    points.iter().map(|(x, y)| (x.raw(), y.raw())).collect()
}

fn point_inside_box() -> (Fixed, Fixed) {
    point(5, 0)
}

fn walled_goal() -> Vec<Clear> {
    vec![Clear {
        at: (5 * SCALE, 0),
        half: (10_000, 10_000),
        reach: 0,
    }]
}

fn surrounded() -> Vec<Clear> {
    vec![
        Clear {
            at: (0, 30_000),
            half: (40_000, 5_000),
            reach: 0,
        },
        Clear {
            at: (0, -30_000),
            half: (40_000, 5_000),
            reach: 0,
        },
        Clear {
            at: (30_000, 0),
            half: (5_000, 40_000),
            reach: 0,
        },
        Clear {
            at: (-30_000, 0),
            half: (5_000, 40_000),
            reach: 0,
        },
    ]
}

fn u_boxes() -> Vec<Clear> {
    // A U opening upward: a floor with two arms, faces touching so the three
    // boxes merge into one shape.
    vec![
        Clear {
            at: (0, -30_000),
            half: (30_000, 5_000),
            reach: 0,
        },
        Clear {
            at: (-30_000, 0),
            half: (5_000, 25_000),
            reach: 0,
        },
        Clear {
            at: (30_000, 0),
            half: (5_000, 25_000),
            reach: 0,
        },
    ]
}

fn open_u_shape() -> Vec<(i64, i64)> {
    walk_shape(0, &u_boxes())
}

fn closed_hole() -> Vec<(i64, i64)> {
    // The hole the `surrounded` walls leave: the inner faces' own rectangle
    // (30k - 5k = 25k each side), listed counterclockwise. `walk_shape`
    // returns outer boundaries, so the hole is named from the walls' inner
    // faces directly; it still certifies clean because every edge lies on a
    // grown face, which the box test calls a touch.
    vec![
        (-25_000, -25_000),
        (25_000, -25_000),
        (25_000, 25_000),
        (-25_000, 25_000),
        (-25_000, -25_000),
    ]
}

/// Right turns minus left turns around a closed ring (last repeats first):
/// an outer boundary walked clockwise counts +4, a hole loop walked
/// counterclockwise counts -4. Exact integer cross products, no float.
fn turn_balance(ring: &[(i64, i64)]) -> i64 {
    let mut distinct = ring.to_vec();
    if distinct.len() > 1 && distinct.first() == distinct.last() {
        distinct.pop();
    }
    let n = distinct.len();
    if n < 3 {
        return 0;
    }
    let mut balance = 0i64;
    for i in 0..n {
        let (ax, ay) = distinct[i];
        let (bx, by) = distinct[(i + 1) % n];
        let (cx, cy) = distinct[(i + 2) % n];
        let cross = (i128::from(bx) - i128::from(ax)) * (i128::from(cy) - i128::from(by))
            - (i128::from(by) - i128::from(ay)) * (i128::from(cx) - i128::from(bx));
        if cross > 0 {
            balance -= 1;
        } else if cross < 0 {
            balance += 1;
        }
    }
    balance
}

#[test]
fn open_ground_reaches() {
    let route = solve(20_000, point(0, 0), point(5, 0), 0, &[], &[], true);
    assert!(matches!(route, WayRoute::Reached(_)));
}

#[test]
fn walled_goal_answers_closest_never_through_wall() {
    let route = solve(
        20_000,
        point(0, 0),
        point_inside_box(),
        0,
        &walled_goal(),
        &[],
        true,
    );
    assert!(matches!(route, WayRoute::Closest(_)));
    assert!(verify_path(&raw_points(&route), &walled_goal()).is_none());
}

#[test]
fn surrounded_expands_once_then_closest() {
    let route = solve(
        20_000,
        point(0, 0),
        point(9, 9),
        0,
        &surrounded(),
        &[],
        true,
    );
    assert!(matches!(route, WayRoute::Closest(_)));
}

#[test]
fn turn_count_names_hole_vs_open() {
    assert_ne!(turn_balance(&open_u_shape()), turn_balance(&closed_hole()));
}

/// The deck's named edges, re-derived as routing witnesses (never copied):
/// a walk across the whole map stays clean, and a walk from inside a hole
/// answers without crossing a wall.
fn whole_map() -> Vec<Clear> {
    // One wall across a 100-tile walk, short enough to walk round inside
    // the search area: the mouths past its ends are the way through.
    vec![Clear {
        at: (1_000_000, 0),
        half: (10_000, 100_000),
        reach: 0,
    }]
}

fn solve_whole_map() -> WayRoute {
    solve(
        SCALE,
        point(0, 0),
        point(100, 0),
        0,
        &whole_map(),
        &[],
        true,
    )
}

fn hole() -> Vec<Clear> {
    surrounded()
}

fn solve_hole() -> WayRoute {
    solve(SCALE, point(0, 0), point(9, 9), 0, &hole(), &[], true)
}

#[test]
fn deck_edges_hold() {
    assert!(verify_path(&raw_points(&solve_whole_map()), &whole_map()).is_none());
    assert!(verify_path(&raw_points(&solve_hole()), &hole()).is_none());
}

/// Two boxes sharing a face, walked at routing level: the seam is contact,
/// not a wall, so the walk crosses to the far side and stays clean.
fn aligned_overlap_pair() -> Vec<Clear> {
    vec![
        Clear {
            at: (0, 0),
            half: (10_000, 10_000),
            reach: 0,
        },
        Clear {
            at: (20_000, 0),
            half: (10_000, 10_000),
            reach: 0,
        },
    ]
}

#[test]
fn aligned_overlap_solves_clean() {
    let avoid = aligned_overlap_pair();
    let route = solve(SCALE, point(-2, 0), point(4, 0), 0, &avoid, &[], true);
    assert!(matches!(route, WayRoute::Reached(_)));
    assert!(verify_path(&raw_points(&route), &avoid).is_none());
}

/// Open pocket vs closed hole at routing level: a goal inside the U's mouth
/// is reachable from the side, while a goal inside the closed frame is
/// answered beside it, never through a wall.
#[test]
fn open_pocket_reaches_but_hole_answers_closest() {
    let mouth = u_boxes();
    let into_pocket = solve(SCALE, point(5, 0), point(0, 0), 0, &mouth, &[], true);
    assert!(matches!(into_pocket, WayRoute::Reached(_)));
    assert!(verify_path(&raw_points(&into_pocket), &mouth).is_none());

    let frame = hole();
    let into_hole = solve(SCALE, point(9, 9), point(0, 0), 0, &frame, &[], true);
    assert!(matches!(into_hole, WayRoute::Closest(_)));
    assert!(verify_path(&raw_points(&into_hole), &frame).is_none());
}

/// The whole-map guard: every waypoint of the cross-map walk lies inside
/// the twice-expanded search area (the from/to box grown by 8 tiles each
/// way), so the walk cannot wander the map looking for a way round.
#[test]
fn whole_map_walk_stays_inside_the_search_area() {
    let avoid = whole_map();
    let route = solve_whole_map();
    assert!(
        matches!(route, WayRoute::Reached(_)),
        "the wall is short enough to walk round inside the search area"
    );
    let points = raw_points(&route);
    assert!(verify_path(&points, &avoid).is_none());
    let step = 8 * SCALE;
    let (lo_x, hi_x) = (-step, 100 * SCALE + step);
    let (lo_y, hi_y) = (-step, step);
    for (x, y) in &points {
        assert!(
            (lo_x..=hi_x).contains(x) && (lo_y..=hi_y).contains(y),
            "waypoint ({x}, {y}) left the search area"
        );
    }
}

/// Surrounded means expand-once-then-closest: the answer stays beside the
/// start (first waypoint is where the walk stands) instead of wandering.
#[test]
fn surrounded_stays_beside_itself() {
    let avoid = hole();
    let route = solve_hole();
    assert!(matches!(route, WayRoute::Closest(_)));
    let points = raw_points(&route);
    assert!(verify_path(&points, &avoid).is_none());
    assert_eq!(points.first(), Some(&(0, 0)));
}

/// Determinism is the anti-goal's guard: the same walk twice answers
/// identically, fixed-point only, no float to creep between runs.
#[test]
fn same_walk_twice_answers_identically() {
    for _ in 0..3 {
        assert_eq!(solve_whole_map(), solve_whole_map());
        assert_eq!(solve_hole(), solve_hole());
    }
}

/// The no-failure contract at the deck edges: every named edge answers with
/// a polyline, never an empty refusal — Reached or Closest, always points.
#[test]
fn deck_edges_answer_never_empty() {
    assert!(!raw_points(&solve_whole_map()).is_empty());
    assert!(!raw_points(&solve_hole()).is_empty());
}

/// The guard under pressure: a wall longer than the search area leaves no
/// way round inside the bounds, so the walk stops beside the wall (Closest)
/// instead of wandering past the bounds or crossing. Clean either way.
#[test]
fn wall_longer_than_the_search_area_answers_beside_it() {
    let avoid = vec![Clear {
        at: (1_000_000, 0),
        half: (10_000, 500_000),
        reach: 0,
    }];
    let route = solve(SCALE, point(0, 0), point(100, 0), 0, &avoid, &[], true);
    assert!(matches!(route, WayRoute::Closest(_)));
    let points = raw_points(&route);
    assert!(verify_path(&points, &avoid).is_none());
    let step = 8 * SCALE;
    for (x, y) in &points {
        assert!(
            (-step..=100 * SCALE + step).contains(x) && (-step..=step).contains(y),
            "waypoint ({x}, {y}) left the search area"
        );
    }
}
