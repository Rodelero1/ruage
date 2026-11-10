//! **The ring sits at the bottom of the body, round its feet** (D179).
//!
//! The owner, twice: first *"it would help to align the center of the ellipse with the bottom side of the square/triangle"*, then, after the body was
//! anchored by its base, *"the circle is supposed to be at the bottom, not waist. The unit is above the circle."* He is right about the picture: with
//! both the body and the ring centred on the same point, half the ellipse lies **on the ground below** the body, which reads as a circle the unit is
//! standing *behind* rather than a circle round its feet.
//!
//! The rule this file states: the ring's **lowest row is the unit's base**, and the rest of the ellipse is on the body.
//!
//! Written before the change (D7, SPEC §11).

use std::path::PathBuf;

use ruage_view::{Camera, Highlights, Snapshot, palette, render_with_solids_and, solids_of};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

#[test]
fn the_ring_sits_round_the_feet_not_under_them() {
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
    let id = ruage_sim::EntityId(9_960);
    let (tx, ty) = (40i64, 40i64);
    world
        .submit_checked(ruage_sim::Command::Spawn {
            id,
            player: 1,
            kind: ruage_sim::EntityKind::Unit,
            unit: Some(villager),
            position: (
                ruage_fixed::Fixed::from_raw(tx * scale + scale / 2),
                ruage_fixed::Fixed::from_raw(ty * scale + scale / 2),
            ),
            speed: ruage_fixed::Fixed::ZERO,
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    world.step().expect("step");

    let snapshot = Snapshot::of(&world, &map, 1);
    let solids = solids_of(&world, &snapshot);
    let camera = Camera::new((900, 700)).centred_on((tx as f32, ty as f32));
    let selected = [id.0];
    let highlights = Highlights {
        selected: &selected,
        order: None,
    };
    let frame = render_with_solids_and(&snapshot, &camera, &highlights, &solids);

    let (_, base) = camera.project(ruage_view::drawn_at(
        *snapshot
            .entities
            .iter()
            .find(|entity| entity.id == id.0)
            .map(|entity| &entity.position)
            .expect("the villager"),
    ));
    let mut ring_low = i32::MIN;
    let mut ring_high = i32::MAX;
    for py in 0..700i32 {
        for px in 0..900i32 {
            if frame.pixel(px as u32, py as u32) == palette::SELECTION {
                ring_low = ring_low.max(py);
                ring_high = ring_high.min(py);
            }
        }
    }
    assert!(ring_low > i32::MIN, "the ring was drawn");
    // **The ring ends where the body ends** - no half of it lying on the grass under the unit.
    assert!(
        (ring_low - base).abs() <= 2,
        "the ring's lowest row is {ring_low} and the unit's feet are at {base}: the ellipse is {} px {} the feet",
        (ring_low - base).abs(),
        if ring_low > base { "below" } else { "above" }
    );
}
