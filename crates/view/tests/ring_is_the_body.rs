//! **The selection ellipse is the body** (D176).
//!
//! The owner: *"the ellipse cannot overlap with [the impassable building diamond] at all, not just center of it"*. The collision was already
//! clean - `crates/sim/tests/body_clearance.rs` walks two thousand ticks and finds no unit with its body inside a blocked tile - but the ring
//! was drawn at `radius * ring_scale`, and the pack's `ring_scale` was **1.5**: a 0.2 body wearing a 0.3 ring, so the *picture* overlapped the
//! wall by a tenth of a tile. This test says the pack now draws the ring on the collision circle, and that the view uses the number it is given.

use std::path::PathBuf;

use ruage_data::Economy;
use ruage_view::Snapshot;

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

#[test]
fn the_ring_is_drawn_on_the_collision_circle() {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        return;
    }
    let economy: Economy =
        ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy");
    let resources =
        ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources");
    let settings = ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim");
    let map = ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map");
    let units = ruage_data::load_units_file(&units).expect("units");
    let mut world = ruage_sim::World::new(&settings, 7).expect("world");
    world.seed_from(&map, &economy, &units).expect("seeding");
    world.load_resources(&resources).expect("stockpile");

    let snapshot = Snapshot::of(&world, &map, 1);
    assert!(
        (snapshot.ring_scale - 1.0).abs() < f32::EPSILON,
        "the pack draws the ring at {} times the body: the ellipse is not the collision circle",
        snapshot.ring_scale
    );
    // ... and the number the view multiplies is the body the simulation uses, not the dat's third radius field (2.0 for a villager,
    // which is a selection *size*, not a footprint).
    let villager = snapshot
        .entities
        .iter()
        .find(|entity| entity.unit == Some(83) && entity.kind == ruage_sim::EntityKind::Unit)
        .expect("a villager");
    assert!(
        (villager.radius - 0.2).abs() < 1e-3,
        "the drawn radius is {}, and the villager's body is 0.2",
        villager.radius
    );
}
