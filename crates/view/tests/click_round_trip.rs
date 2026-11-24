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

use std::path::PathBuf;

use ruage_view::{Camera, drawn_at};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// **The whole chain, as the player experiences it**: click a pixel, submit the order the window would submit, let him walk, and
/// the centre of his circle must be **on that pixel**. This is the fixture that would have caught all three defects at once - the
/// order carrying a tile-space point, the marker drawn in tile space, and the ring raised off the ground point.
#[test]
fn the_circle_ends_on_the_pixel_that_was_clicked() {
    let units_path = repo("config/gamedata/units.toml");
    if !units_path.exists() {
        return;
    }
    let resources =
        ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources");
    let settings = ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim");
    let map = ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map");
    let economy = ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy");
    let units = ruage_data::load_units_file(&units_path).expect("units");
    let mut world = ruage_sim::World::new(&settings, 7).expect("world");
    world.seed_from(&map, &economy, &units).expect("seeding");
    world.load_resources(&resources).expect("stockpile");

    let scale = world.scale();
    let villager = world.role_unit("villager").expect("villager");
    let id = ruage_sim::EntityId(9_995);
    world
        .submit_checked(ruage_sim::Command::Spawn {
            id,
            player: 1,
            kind: ruage_sim::EntityKind::Unit,
            unit: Some(villager),
            position: (
                ruage_fixed::Fixed::from_raw(30 * scale + scale / 2),
                ruage_fixed::Fixed::from_raw(30 * scale + scale / 2),
            ),
            speed: ruage_fixed::Fixed::from_raw(scale / 2),
            hp: 40,
            amount: 0,
        })
        .expect("a villager");
    world.step().expect("the spawn lands");

    let camera = Camera::new((900, 700)).centred_on((30.0, 30.0));
    // A pixel **nowhere near a diamond's centre**: this is a place the player could point at.
    let clicked = (
        camera.project((32.0, 31.0)).0 + 13,
        camera.project((32.0, 31.0)).1 - 9,
    );
    let at = camera.world_point(clicked);
    world
        .submit_checked(ruage_sim::Command::WalkTo {
            target: None,
            unit: id,
            to: (
                ruage_fixed::Fixed::from_raw((f64::from(at.0) * scale as f64).round() as i64),
                ruage_fixed::Fixed::from_raw((f64::from(at.1) * scale as f64).round() as i64),
            ),
        })
        .expect("a walk order");
    for _ in 0..600 {
        world.step().expect("step");
    }

    let snapshot = ruage_view::Snapshot::of(&world, &map, 1);
    let position = snapshot
        .entities
        .iter()
        .find(|entity| entity.id == id.0)
        .map(|entity| entity.position)
        .expect("the villager is in the snapshot");
    // **The centre of his circle**, exactly as the renderer computes it for a unit.
    let centre = camera.project(drawn_at(position));
    assert!(
        (centre.0 - clicked.0).abs() <= 1 && (centre.1 - clicked.1).abs() <= 1,
        "the circle's centre came to {centre:?} for a click at {clicked:?} ({:?} px out)",
        ((centre.0 - clicked.0).abs()).max((centre.1 - clicked.1).abs())
    );
}

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
