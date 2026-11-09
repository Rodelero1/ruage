//! **A unit standing at the centre of a tile is drawn at the centre of that tile's diamond.**
//!
//! The owner, with a screenshot of three blue squares whose centres sit well inside a building's dark grey: *"the picture clearly
//! contradicts the statement."* It does, and this is why - `Camera::project` puts a **tile** at the centre of its diamond, so the tile
//! named `(40, 41)` covers `39.5..40.5` on screen. The simulation's tile `(40, 41)` covers `40.0..41.0` instead, and a settled unit
//! stands at its tile's **centre**, `(40.5, 41.5)` - which the view therefore paints on a corner of four diamonds, half a tile off, so a
//! villager walking beside a house paints a quarter of itself *inside* it.

use std::path::PathBuf;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};
use ruage_view::{Camera, Snapshot, palette, render};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

#[test]
fn a_unit_at_a_tile_centre_is_drawn_at_that_tiles_centre() {
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
    let mut world = World::new(&settings, 7).expect("world");
    world.seed_from(&map, &economy, &units).expect("seeding");
    world.load_resources(&resources).expect("stockpile");

    let scale = world.scale();
    let villager = world.role_unit("villager").expect("villager");
    let (tx, ty) = (42i64, 42i64);
    // **Where a settled unit really is**: the centre of its tile, not the tile's integer point.
    let (px, py) = ((tx * scale) + scale / 2, (ty * scale) + scale / 2);
    world
        .submit_checked(Command::Spawn {
            id: EntityId(9_990),
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: (Fixed::from_raw(px), Fixed::from_raw(py)),
            speed: Fixed::ZERO,
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    world.step().expect("step");
    assert_eq!(
        world.tile_of_position(world.position(EntityId(9_990)).expect("pos")),
        (tx as u16, ty as u16),
        "the villager stands in the tile under test"
    );

    let snapshot = Snapshot::of(&world, &map, 1);
    let camera = Camera::new((900, 700)).centred_on((tx as f32, ty as f32));
    let frame = render(&snapshot, &camera);
    let colour = palette::PLAYER_ONE;

    let (mut sum_x, mut sum_y, mut n) = (0i64, 0i64, 0i64);
    for py in 0..700i32 {
        for px in 0..900i32 {
            if frame.pixel(px as u32, py as u32) == colour {
                sum_x += i64::from(px);
                sum_y += i64::from(py);
                n += 1;
            }
        }
    }
    assert!(n > 40, "the marker was found: {n} pixels");
    let (cx, cy) = ((sum_x / n) as f32, (sum_y / n) as f32);
    // The centre of the diamond the **tile** is drawn in - the same projection the terrain and buildings use.
    let (wx, wy) = camera.project((tx as f32, ty as f32));
    let (want_x, want_y) = (wx as f32, wy as f32);
    assert!(
        (cx - want_x).abs() <= 2.5 && (cy - want_y).abs() <= 2.5,
        "the villager at the centre of tile ({tx},{ty}) was drawn at ({cx:.1},{cy:.1}) but its tile's diamond is centred at \
         ({want_x:.1},{want_y:.1}) - off by ({:.1},{:.1}) pixels, half a tile",
        cx - want_x,
        cy - want_y
    );
}
