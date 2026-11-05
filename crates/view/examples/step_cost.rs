//! **What a tick and a snapshot cost**, so a change that touches either is priced rather than described.
//!
//! Run: `cargo run --release -p ruage-sim --example step_cost`. It seeds the acceptance map, plays 200 ticks to settle, then
//! times `World::step` and `Snapshot::of` separately: the first is what the simulation pays per tick, the second what the view
//! pays per frame - and they are touched by different slices, so a single number would hide one of them.

use std::path::PathBuf;
use std::time::Instant;

use ruage_sim::{Command, EntityKind, World};
use ruage_view::Snapshot;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let config = root.join("config");
    let resources =
        ruage_data::load_resources_file(&config.join("resources.toml")).expect("resources");
    let settings = ruage_data::load_sim_file(&config.join("sim.toml")).expect("sim");
    let map = ruage_data::load_map_file(&config.join("map.toml"), &resources).expect("map");
    let economy = ruage_data::load_economy_file(&config.join("economy.toml")).expect("economy");
    let units = ruage_data::load_units_file(&config.join("gamedata/units.toml")).expect("units");
    let techs = ruage_data::load_techs_file(&config.join("gamedata/techs.toml")).expect("techs");
    let effects =
        ruage_data::load_effects_file(&config.join("gamedata/effects.toml")).expect("effects");
    let civs = ruage_data::load_civs_file(&config.join("gamedata/civs.toml")).expect("civs");

    let mut world = World::new(&settings, 7).expect("world");
    world.seed_from(&map, &economy, &units).expect("seed");
    world.load_resources(&resources).expect("resources");
    world.load_research(&techs, &effects).expect("research");
    world.load_civs(&civs).expect("civs");
    if let Some(villager) = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
        .map(|entity| entity.id)
    {
        world.submit(Command::Move {
            unit: villager,
            to: (24, 20),
        });
    }
    for _ in 0..200 {
        world.step().expect("tick");
    }

    // A snapshot of a settled match: 150 entities, farms included.
    let runs = 400u32;
    let _ = Snapshot::of(&world, &map, 1);
    let start = Instant::now();
    for _ in 0..runs {
        let snapshot = Snapshot::of(&world, &map, 1);
        std::hint::black_box(&snapshot);
    }
    let snapshot_us = start.elapsed().as_secs_f64() * 1_000_000.0 / f64::from(runs);

    // Ticks, on the same settled world: the same 400 ticks' worth of work.
    let start = Instant::now();
    for _ in 0..runs {
        world.step().expect("tick");
    }
    let step_us = start.elapsed().as_secs_f64() * 1_000_000.0 / f64::from(runs);

    println!(
        "snapshot {snapshot_us:.1} us | step {step_us:.1} us | {} entities",
        world.entities().len()
    );
}
