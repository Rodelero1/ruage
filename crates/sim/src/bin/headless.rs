//! Headless runner: seed a world from config, advance N ticks, print its state hash.
//!
//! This is the smallest thing that makes "can be cloned, built and run" true (D4 = B), and it
//! is how a replay is checked from the outside: two runs of the same command must print the
//! same hash, and a mismatch is a bug rather than bad luck.
//!
//! Usage:
//!   ruage-headless --config config/sim.toml --map config/map.toml \
//!                  --economy config/economy.toml --units config/gamedata/units.toml \
//!                  --seed 7 --ticks 2000
//!
//! There are no default game values, and nothing about units is accepted on the command line:
//! who starts, where, and with what stats comes from the map, the economy and the unit table
//! (D6). A `--speed` flag would be a number invented by the CLI. A repeated flag is refused for
//! the same reason: silently taking the first is how a run of 10 ticks is reported as 10,000.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;

use ruage_data::{load_economy_file, load_map_file, load_sim_file, load_units_file};
use ruage_sim::World;

const USAGE: &str = "usage: ruage-headless --config <sim.toml> --map <map.toml> \
                     --economy <economy.toml> --units <units.toml> --seed <u64> --ticks <u64>";

const KNOWN: [&str; 6] = [
    "--config",
    "--map",
    "--economy",
    "--units",
    "--seed",
    "--ticks",
];

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

fn number<T: std::str::FromStr>(args: &BTreeMap<&str, &str>, name: &str) -> Result<T, String>
where
    T::Err: std::fmt::Display,
{
    required(args, name)?
        .parse()
        .map_err(|err| format!("{name}: {err}"))
}

fn run(args: &[String]) -> Result<(), String> {
    let parsed = parse(args)?;

    let config = required(&parsed, "--config")?;
    let map_path = required(&parsed, "--map")?;
    let economy_path = required(&parsed, "--economy")?;
    let units_path = required(&parsed, "--units")?;
    let seed: u64 = number(&parsed, "--seed")?;
    let ticks: u64 = number(&parsed, "--ticks")?;

    let settings = load_sim_file(Path::new(config)).map_err(|err| format!("{err}"))?;
    let map = load_map_file(Path::new(map_path)).map_err(|err| format!("{err}"))?;
    let economy = load_economy_file(Path::new(economy_path)).map_err(|err| format!("{err}"))?;
    let units = load_units_file(Path::new(units_path)).map_err(|err| format!("{err}"))?;

    let mut world = World::new(&settings, seed).map_err(|err| format!("{err}"))?;
    world
        .seed_from(&map, &economy, &units)
        .map_err(|err| format!("{err}"))?;

    for _ in 0..ticks {
        world
            .step()
            .map_err(|err| format!("tick {}: {err}", world.tick()))?;
    }

    let players = world
        .entities()
        .iter()
        .map(|entity| entity.player)
        .filter(|player| *player != 0)
        .max()
        .unwrap_or(0);

    println!(
        "tick={} entities={} players={} hash={:016x}",
        world.tick(),
        world.entities().len(),
        players,
        world.state_hash()
    );
    Ok(())
}
