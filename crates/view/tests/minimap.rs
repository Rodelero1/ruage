//! **The minimap is the map turned 45 degrees**, with a box that shows what the camera can see and can be grabbed.
//!
//! The owner's note (2026-11-04):
//!
//!   "hud should be also 45 degrees turned. Just like map. It should also show rectangle (empty fill, white margin) in it
//!    based on where on the map I am, also it should change the size based on the proportion of the map I see, and it should
//!    be possible to grab the rectangle (not just by the margin but by the inside, too) and move it on the map (and that
//!    should move the view, too), also by left clicking on the hud the minimap rectangle should move its center to the point
//!    of click on hud and view should move too."
//!
//! Written before the implementation (D7, SPEC §11).

use ruage_view::Camera;
use ruage_view::ui;

const RECT: (i32, i32, u32, u32) = (700, 560, 160, 142);
const W: u16 = 64;
const H: u16 = 64;

/// **A tile and the pixel it drew on agree**, both ways: the diamond projection is one function and its inverse, so a click
/// lands on the tile the picture shows. The corners of the map are the diamond's four points.
#[test]
fn the_projection_and_its_inverse_agree() {
    for tile in [(0u16, 0u16), (63, 0), (0, 63), (63, 63), (32, 32), (10, 50)] {
        let pixel = ui::minimap_point(tile, RECT, W, H);
        let back =
            ui::minimap_tile(pixel, RECT, W, H).expect("the projected point is inside the diamond");
        assert_eq!(
            back, tile,
            "tile {tile:?} drew at {pixel:?} and that pixel reads as {back:?}"
        );
    }
    // The four corners of the map are the diamond's top, right, bottom and left points.
    let (x, y, w, h) = RECT;
    let top = ui::minimap_point((0, 0), RECT, W, H);
    let right = ui::minimap_point((63, 0), RECT, W, H);
    let bottom = ui::minimap_point((63, 63), RECT, W, H);
    let left = ui::minimap_point((0, 63), RECT, W, H);
    assert!(
        top.1 < right.1 && top.1 < left.1,
        "the top corner is not the top"
    );
    assert!(
        bottom.1 > right.1 && bottom.1 > left.1,
        "the bottom corner is not the bottom"
    );
    assert!(
        left.0 < top.0 && left.0 < bottom.0,
        "the left corner is not the left"
    );
    assert!(
        right.0 > top.0 && right.0 > bottom.0,
        "the right corner is not the right"
    );
    // And the diamond fits the box it was given.
    for point in [top, right, bottom, left] {
        assert!(
            point.0 >= x && point.0 < x + w as i32 && point.1 >= y && point.1 < y + h as i32,
            "{point:?} is outside the minimap's box {RECT:?}"
        );
    }
}

/// **A click outside the diamond is not a click on the map.** The box is a rectangle and the map inside it is a diamond, so
/// the corners of the box are empty ground - and a click there must not move the camera to a tile that is not on the map.
#[test]
fn the_corners_of_the_box_are_not_the_map() {
    let (x, y, w, h) = RECT;
    for corner in [
        (x + 1, y + 1),
        (x + w as i32 - 2, y + 1),
        (x + 1, y + h as i32 - 2),
        (x + w as i32 - 2, y + h as i32 - 2),
    ] {
        assert!(
            ui::minimap_tile(corner, RECT, W, H).is_none(),
            "{corner:?} is a corner of the box and read as a tile"
        );
    }
    // The middle of the box is the middle of the map.
    let middle = (x + w as i32 / 2, y + h as i32 / 2);
    let tile = ui::minimap_tile(middle, RECT, W, H).expect("the middle is on the map");
    assert!(
        tile.0.abs_diff(32) <= 1 && tile.1.abs_diff(32) <= 1,
        "the middle of the box is {tile:?}, not the middle of the map"
    );
}

/// **The box shows what the camera sees, and shrinks as the camera zooms in.** The view is a rectangle on the map, so on the
/// diamond it is a slanted parallelogram; its width on screen is the proportion of the map the camera covers.
#[test]
fn the_view_box_is_the_camera_and_shrinks_with_zoom() {
    let mut camera = Camera::new((960, 720));
    camera.origin = (20.0, 20.0);
    let wide = ui::minimap_view(&camera, RECT, W, H);
    let wide_span = span(&wide);

    camera.zoom_by(2.0);
    let close = ui::minimap_view(&camera, RECT, W, H);
    let close_span = span(&close);
    assert!(
        close_span < wide_span,
        "zooming in made the view box bigger: {wide_span} then {close_span}"
    );

    // The box is drawn where the camera is: at the map's origin it sits at the top of the diamond, not in the middle.
    let mut corner = Camera::new((960, 720));
    corner.origin = (0.0, 0.0);
    corner.zoom_by(4.0);
    let at_origin = ui::minimap_view(&corner, RECT, W, H);
    let top = ui::minimap_point((0, 0), RECT, W, H);
    let middle = ui::minimap_point((32, 32), RECT, W, H);
    let dist =
        |a: (i32, i32), b: (i32, i32)| i64::from(a.0 - b.0).pow(2) + i64::from(a.1 - b.1).pow(2);
    let to_top = at_origin
        .iter()
        .map(|p| dist(*p, top))
        .min()
        .unwrap_or(i64::MAX);
    let to_middle = at_origin
        .iter()
        .map(|p| dist(*p, middle))
        .min()
        .unwrap_or(i64::MAX);
    assert!(
        to_top < to_middle,
        "a camera at the map's origin draws its box in the middle of the diamond"
    );
}

/// The width of a quad: the largest distance between any two of its points, which is what "how much of the map" means.
fn span(points: &[(i32, i32); 4]) -> i64 {
    let mut widest = 0;
    for i in 0..4 {
        for j in i + 1..4 {
            let dx = i64::from(points[i].0 - points[j].0);
            let dy = i64::from(points[i].1 - points[j].1);
            widest = widest.max(dx * dx + dy * dy);
        }
    }
    widest
}
