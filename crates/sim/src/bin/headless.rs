//! Headless runner: advance a world N ticks from a seed and print its state hash.
//!
//! This is the smallest thing that makes "can be cloned, built and run" true (D4 = B), and it
//! is how a replay is checked from the outside: two runs of the same command must print the
//! same hash, and a mismatch is a bug rather than bad luck.
//!
//! Usage:
//!   ruage-headless --config config/sim.toml --seed 7 --ticks 100 [--speed 1.35]
//!
//! Everything it needs comes from the user's own files: the engine ships no game data.

use std::process::ExitCode;

use ruage_fixed::Fixed;
use ruage_sim::{EntityId, World};

const USAGE: &str = "usage: ruage-headless --config <sim.toml> --seed <u64> --ticks <u64> \
                     [--speed <decimal>]";

fn arg<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let config = arg(args, "--config").ok_or("--config is required")?;
    let seed: u64 = arg(args, "--seed")
        .ok_or("--seed is required")?
        .parse()
        .map_err(|err| format!("--seed: {err}"))?;
    let ticks: u64 = arg(args, "--ticks")
        .ok_or("--ticks is required")?
        .parse()
        .map_err(|err| format!("--ticks: {err}"))?;
    let speed_text = arg(args, "--speed").unwrap_or("1.35");

    let settings =
        ruage_data::load_sim_file(std::path::Path::new(config)).map_err(|err| format!("{err}"))?;

    // The scale comes from config; a speed that the scale cannot represent is refused rather
    // than rounded, which is the whole point of the fixed-point domain.
    let speed = Fixed::parse(speed_text, settings.fixed_point_scale)
        .map_err(|err| format!("--speed {speed_text}: {err}"))?;

    let mut world = World::new(&settings, seed).map_err(|err| err.to_string())?;
    world
        .spawn_now_checked(EntityId(1), speed, 100)
        .map_err(|err| err.to_string())?;

    for _ in 0..ticks {
        world.step().map_err(|err| err.to_string())?;
    }

    let position = world
        .position(EntityId(1))
        .ok_or("entity 1 vanished, which cannot happen")?;

    println!(
        "tick={} hash={:016x} x={} y={} entities={} scale={} tick_rate={}",
        world.tick(),
        world.state_hash(),
        position.0.raw(),
        position.1.raw(),
        world.entities().len(),
        settings.fixed_point_scale,
        world.tick_rate_hz(),
    );
    Ok(())
}
