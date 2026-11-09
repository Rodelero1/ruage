//! **A unit's marker cannot paint outside the tile it stands on** (D166).
//!
//! The owner, four rounds in: *"The overlaps happen just the same."* Tiles **share edges**, so a marker wider than its own diamond is a marker
//! inside the next tile - and when the neighbour is a building, it reads as a unit standing in the building. Shrinking the marker moved the
//! threshold; **clipping** ends the class.

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
fn every_pixel_of_a_marker_is_inside_its_own_tile() {
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
    // One villager on open ground, away from the base - **at its tile's centre**, where a settled unit really stands.
    let (tx, ty) = (40i64, 40i64);
    let (px, py) = (tx * scale + scale / 2, ty * scale + scale / 2);
    world
        .submit_checked(Command::Spawn {
            id: EntityId(9_970),
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

    let snapshot = Snapshot::of(&world, &map, 1);
    let camera = Camera::new((900, 700)).centred_on((tx as f32, ty as f32));
    let frame = render(&snapshot, &camera);
    let (rx, ry) = camera.radii();
    let (cx, cy) = camera.project((tx as f32, ty as f32));
    let colour = ruage_view::palette::PLAYER_ONE;

    let mut painted = 0;
    for py in 0..700i32 {
        for px in 0..900i32 {
            if frame.pixel(px as u32, py as u32) != colour {
                continue;
            }
            painted += 1;
            let norm = (px - cx).abs() as f32 / rx + (py - cy).abs() as f32 / ry;
            assert!(
                norm <= 1.0,
                "the marker painted ({px},{py}) at {norm:.2} of the tile's diamond - it is in the next tile"
            );
        }
    }
    assert!(painted > 40, "the marker was found: {painted} pixels");
    let _ = palette::PLAYER_ONE;
}
