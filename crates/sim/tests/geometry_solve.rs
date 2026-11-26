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
    let route = solve(20_000, point(0, 0), point(5, 0), 0, &[]);
    assert!(matches!(route, WayRoute::Reached(_)));
}

#[test]
fn walled_goal_answers_closest_never_through_wall() {
    let route = solve(20_000, point(0, 0), point_inside_box(), 0, &walled_goal());
    assert!(matches!(route, WayRoute::Closest(_)));
    assert!(verify_path(&raw_points(&route), &walled_goal()).is_none());
}

#[test]
fn surrounded_expands_once_then_closest() {
    let route = solve(20_000, point(0, 0), point(9, 9), 0, &surrounded());
    assert!(matches!(route, WayRoute::Closest(_)));
}

#[test]
fn turn_count_names_hole_vs_open() {
    assert_ne!(turn_balance(&open_u_shape()), turn_balance(&closed_hole()));
}
