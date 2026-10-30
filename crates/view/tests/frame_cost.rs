//! What a frame costs: the window redraws on every simulation tick, so the price of `Snapshot::of` + `render` is the price
//! of the loop. Ignored by default (it is a measurement, not a rule); run with `--release -- --ignored --nocapture`.

use std::path::PathBuf;
use std::time::Instant;

use ruage_view::{Camera, Snapshot, render};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

#[test]
#[ignore = "measurement, not a rule"]
fn a_frame_costs_less_than_a_tick() {
    let resources =
        ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources");
    let settings = ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim");
    let map = ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map");
    let economy = ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy");
    let units = ruage_data::load_units_file(&repo("config/gamedata/units.toml")).expect("units");
    let combat = ruage_data::load_combat_file(&repo("config/combat.toml")).expect("combat");
    let mut world = ruage_sim::World::new(&settings, 7).expect("world");
    world.seed_from(&map, &economy, &units).expect("seed");
    world.load_resources(&resources).expect("pack");
    world.load_combat(&combat);
    world.apply_content_policy();
    for _ in 0..2_000 {
        world.step().expect("step");
    }

    let camera = Camera::new((960, 720)).centred_on((16.0, 16.0));
    let frames = 100u32;
    let start = Instant::now();
    for _ in 0..frames {
        let snapshot = Snapshot::of(&world, &map, 1);
        let frame = render(&snapshot, &camera);
        std::hint::black_box(&frame);
    }
    let per_frame = start.elapsed().as_micros() as f64 / f64::from(frames);
    println!(
        "frame: {per_frame:.0} us each at 960x720, {} entities",
        world.entities().len()
    );
    assert!(
        per_frame < 50_000.0,
        "a frame should cost far less than a tick"
    );
}
