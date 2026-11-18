//! **A pixel the player clicks and the pixel the body is drawn on** (the owner, 2026-11-18: *"the unit never goes to the exact spot
//! ... Center of the unit circle is not aligned with the spot (not even close)"*).
//!
//! The two conversions in play:
//!
//! * `Camera::world_point(pixel)` - a screen pixel to a **world point in tile units** an order carries;
//! * `Camera::project(drawn_at(position))` - a body's position to the pixel its sprite and ring are drawn on.
//!
//! If they are not exact inverses, every click lands somewhere the player did not point at, and no amount of sim-side precision
//! helps. This measures the round trip on a grid of pixels and prints the worst error.

use ruage_view::{Camera, drawn_at};

#[test]
fn a_click_round_trips_through_the_pick_and_the_projection() {
    let camera = Camera::new((1280, 720)).centred_on((30.0, 30.0));
    let mut worst = (0i32, (0, 0), (0.0f32, 0.0f32));
    for y in (100..620).step_by(97) {
        for x in (120..1160).step_by(113) {
            let pixel = (x, y);
            let world = camera.world_point(pixel);
            let back = camera.project(drawn_at(world));
            let (dx, dy) = (back.0 - pixel.0, back.1 - pixel.1);
            let error = dx.abs().max(dy.abs());
            if error > worst.0 {
                worst = (error, pixel, world);
            }
        }
    }
    // The tolerance is a pixel: the projection rounds, and a body is drawn with floats cast to i32.
    assert!(
        worst.0 <= 1,
        "a click at {:?} (world point {:?}) is drawn {:?} pixels away from where it was clicked",
        worst.1,
        worst.2,
        worst.0
    );
}
