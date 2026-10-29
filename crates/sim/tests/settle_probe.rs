//! Measurement, not an assertion: what the blind-site pass costs a world that has no foundations (item 6).
//!
//! `settle_blind_sites` used to ask "is anything under construction?" by walking every entity, every tick. The
//! answer was "no" in every ordinary match, and it was paid for anyway. Run:
//!
//! ```text
//! cargo test --release --test settle_probe -- --include-ignored --nocapture
//! ```

use std::path::PathBuf;
use std::time::Instant;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

#[test]
#[ignore = "measurement, not an assertion"]
fn a_world_with_no_foundations_pays_nothing_for_blind_sites() {
    let units_path = repo("config/gamedata/units.toml");
    if !units_path.exists() {
        eprintln!("skipping: no units.toml (generated, D8)");
        return;
    }
    let resources =
        ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources.toml");
    let settings = ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim.toml");
    let map = ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map.toml");
    let economy =
        ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy.toml");
    let units = ruage_data::load_units_file(&units_path).expect("units.toml");

    let mut world = World::new(&settings, 11).expect("world");
    world.seed_from(&map, &economy, &units).expect("seeding");
    world
        .load_resources(&resources)
        .expect("the pack fits the stockpile");
    let scale = world.scale();
    // Finished buildings, so nothing is under construction and the pass has nothing to find.
    let count: i64 = std::env::var("PROBE_BUILDINGS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(2_000);
    for i in 0..count {
        let id = EntityId(20_000 + u32::try_from(i).expect("fits"));
        let x = 2 + i % 60;
        let y = 2 + (i / 60) % 60;
        world
            .submit_checked(Command::Spawn {
                id,
                player: 1,
                kind: EntityKind::Building,
                unit: Some(70),
                position: (Fixed::from_raw(x * scale), Fixed::from_raw(y * scale)),
                speed: Fixed::ZERO,
                hp: 550,
                amount: 0,
            })
            .expect("a building");
    }
    // A hundred steps to settle, then the measurement.
    for _ in 0..100 {
        world.step().expect("stepping");
    }
    let ticks = 1_000;
    let start = Instant::now();
    for _ in 0..ticks {
        world.step().expect("stepping");
    }
    let each = start.elapsed() / ticks;
    println!(
        "{count} buildings, no foundations: {each:?} per tick (hash {:x})",
        world.state_hash()
    );
}
