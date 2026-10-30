//! Render one frame of a match to a PPM file.
//!
//! ```text
//! cargo run -p ruage-view --bin frame -- <config-dir> <ticks> <out.ppm> [player]
//! ```
//!
//! It is a diagnostic rather than a viewer: no window, no input, no clock - the point is that a frame is a
//! pure function of a world, so it can be produced, compared and looked at without a display.

use std::path::PathBuf;
use std::process::ExitCode;

use ruage_data::{Economy, Map, Units};
use ruage_sim::{Command, EntityKind, World};
use ruage_view::{Camera, Snapshot, render};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 3 {
        eprintln!("usage: frame <config-dir> <ticks> <out.ppm> [player]");
        return ExitCode::from(2);
    }
    let config = PathBuf::from(&args[0]);
    let ticks: u32 = match args[1].parse() {
        Ok(ticks) => ticks,
        Err(err) => {
            eprintln!("ticks: {err}");
            return ExitCode::from(2);
        }
    };
    let out = PathBuf::from(&args[2]);
    let player: u8 = args.get(3).and_then(|p| p.parse().ok()).unwrap_or(1);

    let resources = match ruage_data::load_resources_file(&config.join("resources.toml")) {
        Ok(resources) => resources,
        Err(err) => {
            eprintln!("resources.toml: {err}");
            return ExitCode::FAILURE;
        }
    };
    let load = || -> Result<(ruage_data::Sim, Map, Economy, Units, ruage_data::Combat), String> {
        Ok((
            ruage_data::load_sim_file(&config.join("sim.toml")).map_err(|e| e.to_string())?,
            ruage_data::load_map_file(&config.join("map.toml"), &resources)
                .map_err(|e| e.to_string())?,
            ruage_data::load_economy_file(&config.join("economy.toml"))
                .map_err(|e| e.to_string())?,
            ruage_data::load_units_file(&config.join("gamedata/units.toml"))
                .map_err(|e| e.to_string())?,
            ruage_data::load_combat_file(&config.join("combat.toml")).map_err(|e| e.to_string())?,
        ))
    };
    let (settings, map, economy, units, combat) = match load() {
        Ok(parts) => parts,
        Err(err) => {
            eprintln!("config: {err}");
            return ExitCode::FAILURE;
        }
    };

    let mut world = match World::new(&settings, 7) {
        Ok(world) => world,
        Err(err) => {
            eprintln!("world: {err}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(err) = world.seed_from(&map, &economy, &units) {
        eprintln!("seeding: {err}");
        return ExitCode::FAILURE;
    }
    world.load_resources(&resources).ok();
    world.load_combat(&combat);

    // Something to look at: the first villager of the player under test walks out, so the frame is not a
    // still life of a town centre.
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
    for _ in 0..ticks {
        if let Err(err) = world.step() {
            eprintln!("tick: {err}");
            return ExitCode::FAILURE;
        }
    }

    let snapshot = Snapshot::of(&world, &map, player);
    // Centre on the player's own base rather than on a fixed corner: with a 4x4 town centre the base fills most of
    // the frame, and a camera pointed at empty ground proves nothing about what was drawn.
    let focus = snapshot
        .entities
        .iter()
        .find(|entity| entity.player == player && entity.kind == EntityKind::Building)
        .map_or((20.0, 20.0), |entity| entity.position);
    let camera = Camera::new((640, 480)).centred_on(focus);
    let frame = render(&snapshot, &camera);
    if let Err(err) = std::fs::write(&out, frame.to_ppm()) {
        eprintln!("write {}: {err}", out.display());
        return ExitCode::FAILURE;
    }
    println!(
        "tick={} entities={} visible={} -> {}",
        snapshot.tick,
        snapshot.entities.len(),
        snapshot.visible.iter().map(|w| w.count_ones()).sum::<u32>(),
        out.display()
    );
    ExitCode::SUCCESS
}
