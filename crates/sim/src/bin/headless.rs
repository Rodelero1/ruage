//! Headless runner: advance a world N ticks from a seed and print its state hash.
//!
//! This is the smallest thing that makes "can be cloned, built and run" true (D4 = B), and it
//! is how a replay is checked from the outside: two runs of the same command must print the
//! same hash, and a mismatch is a bug rather than bad luck.
//!
//! Usage:
//!   ruage-headless --config config/sim.toml --seed 7 --ticks 100 --speed 1.35 --hp 100
//!
//! There are no default game values. `--speed` and `--hp` are required because a default would
//! be a number invented by the CLI — exactly what D6 forbids — and a plausible wrong answer is
//! worse than an error. A repeated flag is refused for the same reason: silently taking the
//! first is how a run of 10 ticks gets reported as 10,000.

use std::collections::BTreeMap;
use std::process::ExitCode;

use ruage_fixed::Fixed;
use ruage_sim::{EntityId, World};

/// The single entity this runner creates. An id is an address, not a game value.
const ENTITY: EntityId = EntityId(1);

const USAGE: &str = "usage: ruage-headless --config <sim.toml> --seed <u64> --ticks <u64> \
                     --speed <decimal> --hp <i32>";

const KNOWN: [&str; 5] = ["--config", "--seed", "--ticks", "--speed", "--hp"];

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }

    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

/// Strict flag parsing: unknown, valueless, missing and repeated flags are all errors.
fn parse(args: &[String]) -> Result<BTreeMap<&str, &str>, String> {
    let mut out = BTreeMap::new();
    let mut i = 0;
    while i < args.len() {
        let flag = args[i].as_str();
        let Some(&name) = KNOWN.iter().find(|k| **k == flag) else {
            return Err(format!("{flag} is not a known flag"));
        };
        let value = args
            .get(i + 1)
            .ok_or_else(|| format!("{name} needs a value"))?;
        if out.insert(name, value.as_str()).is_some() {
            return Err(format!("{name} is repeated"));
        }
        i += 2;
    }
    Ok(out)
}

fn required<'a>(args: &BTreeMap<&'a str, &'a str>, name: &str) -> Result<&'a str, String> {
    args.get(name)
        .copied()
        .ok_or_else(|| format!("{name} is required"))
}

fn run(args: &[String]) -> Result<(), String> {
    let parsed = parse(args)?;

    let config = required(&parsed, "--config")?;
    let seed: u64 = required(&parsed, "--seed")?
        .parse()
        .map_err(|err| format!("--seed: {err}"))?;
    let ticks: u64 = required(&parsed, "--ticks")?
        .parse()
        .map_err(|err| format!("--ticks: {err}"))?;
    let speed_text = required(&parsed, "--speed")?;
    let hp: i32 = required(&parsed, "--hp")?
        .parse()
        .map_err(|err| format!("--hp: {err}"))?;

    let settings =
        ruage_data::load_sim_file(std::path::Path::new(config)).map_err(|err| format!("{err}"))?;

    // The scale comes from config; a speed the scale cannot represent is refused rather than
    // rounded, which is the whole point of the fixed-point domain.
    let speed = Fixed::parse(speed_text, settings.fixed_point_scale)
        .map_err(|err| format!("--speed {speed_text}: {err}"))?;

    let mut world = World::new(&settings, seed).map_err(|err| err.to_string())?;
    world
        .spawn_now_checked(ENTITY, speed, hp)
        .map_err(|err| err.to_string())?;

    for _ in 0..ticks {
        world.step().map_err(|err| err.to_string())?;
    }

    let position = world
        .position(ENTITY)
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
