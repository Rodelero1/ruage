//! The camera is the game's projection: a 2:1 isometric diamond grid, and picking must be its exact inverse or clicks
//! land on the tile next to the one the player aimed at. Both are pure arithmetic, so both are tested without a window.

use ruage_view::{Camera, pick_at};

fn camera() -> Camera {
    Camera::new((960, 720)).centred_on((16.0, 16.0))
}

/// A tile is twice as wide as it is tall, and the two grid axes go down-right and down-left.
#[test]
fn the_projection_is_two_to_one_isometric() {
    let camera = camera();
    let centre = camera.project((16.0, 16.0));
    let east = camera.project((17.0, 16.0));
    let south = camera.project((16.0, 17.0));

    assert_eq!(
        (east.0 - centre.0, east.1 - centre.1),
        (48, 24),
        "one tile east is down-right"
    );
    assert_eq!(
        (south.0 - centre.0, south.1 - centre.1),
        (-48, 24),
        "one tile south is down-left"
    );
}

/// The centre of every diamond picks its own tile: this is the click the player makes most often.
#[test]
fn the_centre_of_a_diamond_is_its_tile() {
    let camera = camera();
    for tile in [(16, 16), (17, 16), (16, 17), (20, 12), (9, 30)] {
        #[allow(clippy::cast_precision_loss)]
        let at = camera.project((tile.0 as f32, tile.1 as f32));
        assert_eq!(camera.pick(at), tile, "centre of {tile:?} must pick itself");
    }
}

/// Where a click falls *inside* a diamond near its edges. This was parked as a known defect ("needs a picture"),
/// and the picture was never the question: `project` maps a tile to the **centre** of its diamond, so continuous
/// tile coordinates run from `tile - 0.5` to `tile + 0.5` inside a diamond and the inverse must **round**, not
/// floor. Flooring was right for the old top-down camera, where a tile's pixel was its top-left corner; it puts
/// every click in the upper-left half of a diamond one tile off. Every centre round-trips either way, which is
/// why this only showed near an edge - and why a quarter tile from the centre is the test that finds it.
#[test]
fn a_click_just_inside_a_diamond_picks_that_tile() {
    let camera = camera();
    let centre = camera.project((16.0, 16.0));
    // A quarter tile from the centre, toward all four vertices of the diamond.
    for offset in [(0, 12), (0, -12), (24, 0), (-24, 0), (18, 6), (-18, -6)] {
        assert_eq!(
            camera.pick((centre.0 + offset.0, centre.1 + offset.1)),
            (16, 16),
            "a click {offset:?} from the centre of (16,16)'s diamond must pick (16,16)"
        );
    }
    // And clearly outside: past the bottom vertex is the tile below, past the right vertex the tile east and up.
    assert_eq!(camera.pick((centre.0, centre.1 + 30)), (17, 17));
    assert_eq!(camera.pick((centre.0 + 54, centre.1)), (17, 15));
}

/// The camera is state, and zoom scales the diamonds without moving the tile under the centre of the screen.
#[test]
fn zoom_keeps_the_centre_tile_and_scales_the_diamond() {
    let base = camera();
    let mut zoomed = camera();
    zoomed.zoom = 2.0;

    let one = base.project((17.0, 16.0)).0 - base.project((16.0, 16.0)).0;
    let two = zoomed.project((17.0, 16.0)).0 - zoomed.project((16.0, 16.0)).0;
    assert_eq!(two, one * 2, "at 2x a tile is twice as wide in pixels");

    let at = base.project((16.0, 16.0));
    assert_eq!(base.pick(at), (16, 16));
    assert_eq!(
        zoomed.pick(zoomed.project((16.0, 16.0))),
        (16, 16),
        "and picking still inverts it"
    );
}

/// The whole chain: a world, a tile, a pixel, back to a world entity.
#[test]
fn a_click_on_the_screen_reaches_the_entity_under_it() {
    use std::path::PathBuf;
    let repo = |rel: &str| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(rel)
    };
    let resources =
        ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources");
    let settings = ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim");
    let map = ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map");
    let economy = ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy");
    let units = ruage_data::load_units_file(&repo("config/gamedata/units.toml")).expect("units");
    let mut world = ruage_sim::World::new(&settings, 7).expect("world");
    world.seed_from(&map, &economy, &units).expect("seed");

    let camera = camera();
    let pixel = camera.project((16.0, 16.0));
    let tile = camera.pick(pixel);
    assert!(
        pick_at(&world, tile).is_some(),
        "the town centre is picked through the projection, at {tile:?}"
    );
}

/// The camera is **state**: a resize changes the viewport it looks through and nothing else, so the tile under the middle
/// of the window is still the tile the camera is on. Rebuilding a camera per frame is the defect this pins - it looks
/// identical until there is a pan or a zoom key, and then neither can ever work.
#[test]
fn resizing_the_window_keeps_the_camera_on_its_tile() {
    let mut camera = camera();
    camera.resized((1280, 800));
    assert_eq!(camera.viewport, (1280, 800));

    let at = camera.project((16.0, 16.0));
    assert_eq!(at, (640, 400 + 24), "the origin tile is not at the middle");
    assert_eq!(camera.pick(at), (16, 16), "and picking still finds it");

    // Panning then resizing keeps the pan: the origin is the camera's position, not something a resize resets.
    camera.scroll((3.0, -2.0));
    camera.resized((800, 600));
    let at = camera.project((19.0, 14.0));
    assert_eq!(
        camera.pick(at),
        (19, 14),
        "a resize lost the camera's position"
    );
    assert_eq!(at.0, 400, "the origin tile is not at the new middle");
}

/// Zoom is three levels, and stepping past either end stays there. The game has 0.5x/1x/2x, not a continuum, so a wheel
/// notch is a notch rather than a small multiplication that accumulates rounding.
#[test]
fn zoom_steps_through_the_three_levels_and_stops_at_the_ends() {
    let mut camera = camera();
    assert_eq!(camera.zoom, 1.0);
    camera.zoom_in();
    assert_eq!(camera.zoom, 2.0);
    camera.zoom_in();
    assert_eq!(camera.zoom, 2.0, "zoomed past the closest level");
    camera.zoom_out();
    assert_eq!(camera.zoom, 1.0);
    camera.zoom_out();
    assert_eq!(camera.zoom, 0.5);
    camera.zoom_out();
    assert_eq!(camera.zoom, 0.5, "zoomed past the furthest level");

    // And the tile under the middle of the screen is still (16,16) at every level.
    for level in [0.5f32, 1.0, 2.0] {
        camera.zoom = level;
        let at = camera.project((16.0, 16.0));
        assert_eq!(camera.pick(at), (16, 16), "at {level}x");
    }
}
