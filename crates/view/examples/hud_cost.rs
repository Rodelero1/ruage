//! **What the HUD costs a frame.** A measurement, not a test: the panel is drawn once per frame and its cost must stay small
//! enough to leave the frame budget alone, so every change to it is priced before it lands (the owner's rule for every step).
//!
//! Run: `cargo run --release -p ruage-view --example hud_cost` - it prints microseconds a frame for the whole HUD, built from
//! a real match at tick 200 with the player's first unit selected.

use std::path::PathBuf;
use std::time::Instant;

use ruage_sim::{Command, EntityKind, World};
use ruage_view::{Camera, Frame, Snapshot, ui};

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

    let player = 1u8;
    if let Some(villager) = world
        .entities()
        .iter()
        .find(|entity| entity.player == player && entity.kind == EntityKind::Unit)
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

    let snapshot = Snapshot::of(&world, &map, player);
    let selected: Vec<u32> = snapshot
        .entities
        .iter()
        .filter(|entity| entity.player == player && entity.kind == EntityKind::Unit)
        .map(|entity| entity.id)
        .take(1)
        .collect();
    let hud = ui::Hud::of_selected(&snapshot, &resources, &selected);
    let camera = Camera::new((960, 720));
    let mut frame = Frame::new(960, 720);
    let _ = camera;

    // Warm the caches: the first frame allocates a frame buffer and a layout.
    for _ in 0..50 {
        ui::hud(&mut frame, &hud, &snapshot, ruage_view::Speed::One);
    }
    let runs = 2_000u32;
    let start = Instant::now();
    for _ in 0..runs {
        ui::hud(&mut frame, &hud, &snapshot, ruage_view::Speed::One);
    }
    let each = start.elapsed().as_secs_f64() * 1_000_000.0 / f64::from(runs);
    println!(
        "hud: {each:.1} us a frame ({runs} runs, {} entities)",
        snapshot.entities.len()
    );
}
